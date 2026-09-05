/**
 * Main HTTP API client for slskr
 */

import {
  ClientConfig,
  HealthStatus,
  VersionInfo,
  Capabilities,
  Configuration,
  Statistics,
  Session,
  SessionPrivileges,
  User,
  UserInfo,
  Search,
  SearchDetails,
  SearchCreateRequest,
  Message,
  MessageSendRequest,
  Transfer,
  TransferCreateRequest,
  Room,
  BrowseEntry,
  BrowseResult,
  BrowseRequest,
  Share,
  DownloadFilter,
  Event,
  PaginationParams,
  CacheStats,
} from './types';
import {
  ApiError,
  NetworkError,
  ResponseContractError,
  TimeoutError,
} from './errors';

const MAX_HTTP_RESPONSE_BYTES = 8 * 1024 * 1024;
const MAX_HTTP_ERROR_BYTES = 64 * 1024;
const MAX_DATE_MILLISECONDS = 8_640_000_000_000_000;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function responseList<T>(response: unknown, resource: string, ...keys: string[]): T[] {
  let entries: unknown[] | undefined;
  if (Array.isArray(response)) {
    entries = response;
  } else if (isRecord(response)) {
    for (const key of keys) {
      if (Array.isArray(response[key])) {
        entries = response[key];
        break;
      }
    }
  }
  if (!entries || entries.some((entry) => !isRecord(entry))) {
    throw new ResponseContractError(resource);
  }
  return entries as T[];
}

function responseObject(response: unknown, resource = 'object'): Record<string, unknown> {
  if (!isRecord(response)) {
    throw new ResponseContractError(resource);
  }
  return response;
}

function requiredObject<T>(response: unknown, resource: string): T {
  return responseObject(response, resource) as T;
}

function requiredIdentifier(
  object: Record<string, unknown>,
  resource: string,
  ...keys: string[]
): string {
  for (const key of keys) {
    const value = object[key];
    if (typeof value === 'string' && value.trim() !== '') {
      return value;
    }
    if (typeof value === 'number' && Number.isSafeInteger(value)) {
      return String(value);
    }
  }
  throw new ResponseContractError(resource);
}

function requiredText(value: unknown, resource: string): string {
  if (typeof value !== 'string' || value.trim() === '') {
    throw new ResponseContractError(resource);
  }
  return value;
}

function numberValue(value: unknown, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function normalizeTimestamp(value: unknown): string {
  if (typeof value === 'number' && Number.isFinite(value)) {
    const milliseconds = value > 10_000_000_000 ? value : value * 1000;
    return isoTimestamp(milliseconds);
  }
  return typeof value === 'string' ? value : '';
}

function normalizeEpochSeconds(value: unknown): string {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return isoTimestamp(value * 1000);
  }
  return typeof value === 'string' ? value : '';
}

function isoTimestamp(milliseconds: number): string {
  if (!Number.isFinite(milliseconds) || Math.abs(milliseconds) > MAX_DATE_MILLISECONDS) {
    return '';
  }
  const date = new Date(milliseconds);
  return Number.isNaN(date.getTime()) ? '' : date.toISOString();
}

function normalizeSearchStatus(value: unknown): Search['status'] {
  switch (String(value ?? '').toLowerCase()) {
    case 'failed':
      return 'failed';
    case 'cancelled':
    case 'canceled':
      return 'cancelled';
    case 'completed':
    case 'complete':
    case 'expired':
      return 'completed';
    default:
      return 'active';
  }
}

function normalizeSearchResult(response: unknown): SearchDetails['results'][number] {
  const object = responseObject(response, 'search result');
  return {
    ...object,
    username: String(object.username ?? object.peer_username ?? ''),
    filename: String(object.filename ?? ''),
    size: numberValue(object.size),
    ...(object.bitRate !== undefined || object.bit_rate !== undefined || object.bitrate !== undefined
      ? { bitrate: numberValue(object.bitRate ?? object.bit_rate ?? object.bitrate) }
      : {}),
    ...(object.length !== undefined || object.length_seconds !== undefined
      ? { length: numberValue(object.length ?? object.length_seconds) }
      : {}),
  };
}

function normalizeSearch(response: unknown): Search {
  const object = responseObject(response, 'search');
  const results = Array.isArray(object.results) ? object.results : [];
  return {
    ...object,
    id: requiredIdentifier(object, 'search', 'id', 'searchId', 'token'),
    query: String(object.query ?? object.searchText ?? ''),
    status: normalizeSearchStatus(object.status ?? object.state),
    results_count: numberValue(
      object.results_count ?? object.result_count ?? object.resultsCount,
      results.length
    ),
    started_at: normalizeTimestamp(object.started_at ?? object.startedAt ?? object.created_at),
  } as Search;
}

function normalizeSearchDetails(response: unknown): SearchDetails {
  const object = responseObject(response, 'search details');
  return {
    ...normalizeSearch(response),
    results: Array.isArray(object.results)
      ? object.results.map(normalizeSearchResult)
      : [],
  };
}

function normalizeMessage(response: unknown): Message {
  const object = responseObject(response, 'message');
  return {
    ...object,
    id: requiredIdentifier(object, 'message', 'id'),
    sender: String(object.sender ?? object.username ?? ''),
    content: String(object.content ?? object.body ?? object.message ?? ''),
    timestamp: normalizeTimestamp(object.timestamp ?? object.created_at ?? object.createdAtMs),
  };
}

function normalizeTransferStatus(value: unknown): Transfer['status'] {
  switch (String(value ?? '').toLowerCase()) {
    case 'succeeded':
    case 'completed':
    case 'complete':
      return 'completed';
    case 'failed':
    case 'error':
    case 'errored':
    case 'rejected':
      return 'failed';
    case 'cancelled':
    case 'canceled':
      return 'cancelled';
    default:
      return 'active';
  }
}

function normalizeTransfer(response: unknown): Transfer {
  const object = responseObject(response, 'transfer');
  const direction = object.direction === 1
    || String(object.direction ?? '').toLowerCase() === 'upload'
    ? 'upload'
    : 'download';
  return {
    ...object,
    id: requiredIdentifier(object, 'transfer', 'id'),
    direction,
    status: normalizeTransferStatus(object.status),
    peer_username: String(object.peer_username ?? object.username ?? ''),
    filename: String(object.filename ?? ''),
    bytes_transferred: numberValue(object.bytes_transferred ?? object.bytesTransferred),
    started_at: normalizeTimestamp(object.started_at ?? object.startedAt),
  } as Transfer;
}

function normalizeEvent(response: unknown): Event {
  const object = responseObject(response, 'event');
  const type = requiredText(object.type ?? object.kind, 'event');
  let data: unknown = object.data;
  if (typeof data === 'string') {
    try {
      data = JSON.parse(data);
    } catch {
      data = undefined;
    }
  }
  if (data === null || typeof data !== 'object' || Array.isArray(data)) {
    data = object.payload && typeof object.payload === 'object' && !Array.isArray(object.payload)
      ? object.payload
      : {};
  }
  return {
    ...object,
    id: requiredIdentifier(object, 'event', 'id'),
    type: type as Event['type'],
    data: data as Record<string, any>,
    timestamp: normalizeTimestamp(object.timestamp ?? object.created_at),
  };
}

function sessionFromSnapshot(response: unknown): Session {
  const snapshot = responseObject(response, 'session');
  const rawState = snapshot.state ?? snapshot.status;
  const statuses: Session['status'][] = [
    'connecting',
    'connected',
    'disconnecting',
    'disconnected',
  ];
  if (typeof rawState !== 'string' || !statuses.includes(rawState as Session['status'])) {
    throw new ResponseContractError('session');
  }
  const status = rawState as Session['status'];
  const connectedAt = snapshot.connected_at;
  const normalizedConnectedAt = normalizeEpochSeconds(connectedAt);
  return {
    id: 'server',
    type: 'server',
    status,
    ...(normalizedConnectedAt ? { connected_at: normalizedConnectedAt } : {}),
  };
}

function normalizeBrowseStatus(value: unknown): BrowseRequest['status'] {
  switch (String(value ?? '').toLowerCase()) {
    case 'ready':
    case 'partial':
    case 'accepted':
      return 'accepted';
    case 'failed':
    case 'cancelled':
    case 'rejected':
      return 'rejected';
    default:
      return 'pending';
  }
}

function normalizeRoom(response: unknown): Room {
  const object = responseObject(response, 'room');
  const name = requiredText(object.name ?? object.room, 'room');
  const rawUsers = Array.isArray(object.users)
    ? object.users
    : Array.isArray(object.members)
      ? object.members
      : undefined;
  const users = rawUsers?.filter((user): user is string => typeof user === 'string');
  return {
    ...object,
    name,
    user_count: numberValue(
      object.user_count ?? object.userCount ?? object.memberCount,
      users?.length ?? 0,
    ),
    ...(users ? { users } : {}),
  } as Room;
}

function nullableNumber(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function normalizeUser(response: unknown): User {
  const object = responseObject(response, 'user');
  return {
    ...object,
    username: requiredText(object.username, 'user'),
    watched: object.watched === true,
    status: typeof object.status === 'string' ? object.status : null,
    average_speed: nullableNumber(object.average_speed ?? object.averageSpeed),
    upload_count: nullableNumber(object.upload_count ?? object.uploadCount),
    file_count: nullableNumber(object.file_count ?? object.fileCount),
    directory_count: nullableNumber(object.directory_count ?? object.directoryCount),
    updated_at: numberValue(object.updated_at ?? object.updatedAt),
  };
}

function normalizeUserInfo(response: unknown, fallbackUsername: string): UserInfo {
  const object = responseObject(response, 'user info');
  const picture = object.picture;
  return {
    ...object,
    username: String(object.username ?? fallbackUsername),
    description: typeof object.description === 'string' ? object.description : '',
    hasFreeUploadSlot: object.hasFreeUploadSlot === true || object.has_free_upload_slot === true,
    hasPicture: object.hasPicture === true || object.has_picture === true,
    picture: typeof picture === 'string' || picture === null ? picture : null,
    queueLength: numberValue(object.queueLength ?? object.queue_length),
    uploadSlots: numberValue(object.uploadSlots ?? object.upload_slots),
    uploadSpeed: numberValue(object.uploadSpeed ?? object.upload_speed),
    uploadCount: numberValue(object.uploadCount ?? object.upload_count),
    fileCount: numberValue(object.fileCount ?? object.file_count),
    directoryCount: numberValue(object.directoryCount ?? object.directory_count),
  };
}

function browseRequestFromResponse(response: unknown, fallbackUsername = ''): BrowseRequest {
  const object = responseObject(response, 'browse request');
  const username = requiredText(
    object.username ?? object.from ?? object.id ?? fallbackUsername,
    'browse request',
  );
  const requestedAt = object.requested_at ?? object.requestedAt;
  const requested_at = normalizeEpochSeconds(requestedAt);
  return {
    id: String(object.id ?? username),
    from: String(object.from ?? username),
    status: normalizeBrowseStatus(object.status),
    requested_at,
  };
}

function normalizeBrowseResult(response: unknown): BrowseResult {
  const object = responseObject(response, 'browse result');
  const hasEntries = Array.isArray(object.entries);
  const hasDirectories = Array.isArray(object.directories);
  if (!hasEntries && !hasDirectories) {
    throw new ResponseContractError('browse result');
  }
  const rawEntries = hasEntries ? object.entries as unknown[] : [];
  const directEntries = rawEntries.filter((entry): entry is BrowseEntry => isRecord(entry));
  if (hasEntries && directEntries.length !== rawEntries.length) {
    throw new ResponseContractError('browse result');
  }
  if (directEntries.length > 0 || Array.isArray(object.entries)) {
    return {
      entries: directEntries,
      ...(typeof object.folder === 'string' ? { folder: object.folder } : {}),
    };
  }
  const directories = hasDirectories ? object.directories as unknown[] : [];
  if (directories.some((directory) => !isRecord(directory))) {
    throw new ResponseContractError('browse result');
  }
  const entries = directories.flatMap((directory) => {
    const files = (directory as Record<string, unknown>).files;
    if (!Array.isArray(files)) return [];
    if (files.some((entry) => !isRecord(entry))) {
      throw new ResponseContractError('browse result');
    }
    return files as BrowseEntry[];
  });
  return {
    entries,
    ...(typeof object.folder === 'string' ? { folder: object.folder } : {}),
  };
}

function normalizeShare(response: unknown): Share {
  return responseObject(response, 'share');
}

function normalizeDownloadFilter(response: unknown): DownloadFilter {
  const object = responseObject(response, 'download filter');
  const rawTerms = Array.isArray(object.exclude)
    ? object.exclude
    : Array.isArray(object.terms)
      ? object.terms
      : undefined;
  if (!rawTerms || rawTerms.some((term) => typeof term !== 'string')) {
    throw new ResponseContractError('download filter');
  }
  const terms = rawTerms as string[];
  return {
    ...object,
    exclude: terms,
    ...(typeof object.maxTerms === 'number' ? { maxTerms: object.maxTerms } : {}),
    ...(typeof object.maxTermLength === 'number' ? { maxTermLength: object.maxTermLength } : {}),
  };
}

function normalizeCacheStats(response: unknown): CacheStats {
  const object = responseObject(response, 'cache stats');
  const numericFields = [
    'totalRetrievals',
    'cacheHits',
    'cacheMisses',
    'cacheHitRatio',
    'expiredEntriesCleaned',
  ];
  if (numericFields.some((field) => (
    typeof object[field] !== 'number' || !Number.isFinite(object[field])
  ))) {
    throw new ResponseContractError('cache stats');
  }
  const hits = typeof object.cacheHits === 'number' ? object.cacheHits : 0;
  const misses = typeof object.cacheMisses === 'number' ? object.cacheMisses : 0;
  const total_requests = typeof object.totalRetrievals === 'number'
    ? object.totalRetrievals
    : hits + misses;
  const hit_rate = typeof object.cacheHitRatio === 'number'
    ? object.cacheHitRatio
    : total_requests === 0 ? 0 : hits / total_requests;
  return {
    hits,
    misses,
    evictions: typeof object.expiredEntriesCleaned === 'number' ? object.expiredEntriesCleaned : 0,
    total_requests,
    hit_rate,
  };
}

export class SlskrClient {
  private baseUrl: string;
  private token: string;
  private timeout: number;
  private retries: number;
  private retryDelay: number;
  private debug: boolean;

  constructor(config: ClientConfig) {
    const parsedUrl = new URL(config.baseUrl);
    if (!['http:', 'https:'].includes(parsedUrl.protocol) || parsedUrl.username || parsedUrl.password) {
      throw new Error('baseUrl must be an absolute HTTP or HTTPS URL without credentials');
    }
    parsedUrl.pathname = parsedUrl.pathname.replace(/\/+$/, '');
    parsedUrl.search = '';
    parsedUrl.hash = '';
    this.baseUrl = parsedUrl.toString().replace(/\/$/, '');
    this.token = config.token;
    this.timeout = config.timeout ?? 30000;
    this.retries = config.retries ?? 3;
    this.retryDelay = config.retryDelay ?? 1000;
    this.debug = config.debug ?? false;
  }

  private debugBody(body: any): any {
    if (body === null || body === undefined) {
      return body;
    }
    if (Array.isArray(body)) {
      return body.map((item) => this.debugBody(item));
    }
    if (typeof body !== 'object') {
      return body;
    }
    const redacted: Record<string, any> = {};
    for (const [key, value] of Object.entries(body)) {
      if (/(api[-_]?key|authorization|credential|pass(word)?|secret|session|token)/i.test(key)) {
        redacted[key] = '[REDACTED]';
      } else {
        redacted[key] = this.debugBody(value);
      }
    }
    return redacted;
  }

  // =========================================================================
  // Health & Version
  // =========================================================================

  async health(): Promise<HealthStatus> {
    return requiredObject<HealthStatus>(await this.get<unknown>('/api/health', {}), 'health');
  }

  async version(): Promise<VersionInfo> {
    return requiredObject<VersionInfo>(await this.get<unknown>('/api/version', {}), 'version');
  }

  // =========================================================================
  // Configuration
  // =========================================================================

  async getConfig(): Promise<Configuration> {
    return requiredObject<Configuration>(await this.getAuth<unknown>('/api/config'), 'configuration');
  }

  async getStats(): Promise<Statistics> {
    return requiredObject<Statistics>(await this.getAuth<unknown>('/api/stats'), 'statistics');
  }

  // =========================================================================
  // Capabilities
  // =========================================================================

  async getCapabilities(): Promise<Capabilities> {
    return requiredObject<Capabilities>(
      await this.get<unknown>('/api/capabilities', {}),
      'capabilities',
    );
  }

  // =========================================================================
  // Sessions
  // =========================================================================

  async getSessions(): Promise<Session[]> {
    const response = await this.getAuth<unknown>('/api/session');
    return [sessionFromSnapshot(response)];
  }

  async createSession(kind: string, parameters?: Record<string, any>): Promise<Session> {
    if (kind !== 'server') {
      throw new Error(`Unsupported session type: ${kind}`);
    }
    if (parameters && ('username' in parameters || 'password' in parameters)) {
      await this.putAuth('/api/server', parameters);
    } else {
      await this.postAuth('/api/session/connect', {});
    }
    return (await this.getSessions())[0];
  }

  async pingSession(_id: string): Promise<{ status: string; latency_ms: number }> {
    const started = Date.now();
    const response = requiredObject<{ accepted?: unknown }>(
      await this.postAuth<unknown>('/api/session/ping', {}),
      'session ping',
    );
    if (typeof response.accepted !== 'boolean') {
      throw new ResponseContractError('session ping');
    }
    return {
      status: response.accepted ? 'accepted' : 'unknown',
      latency_ms: Date.now() - started,
    };
  }

  async disconnectSession(_id: string): Promise<void> {
    await this.postAuth('/api/session/disconnect', {});
  }

  async getSessionPrivileges(_id: string): Promise<SessionPrivileges> {
    await this.postAuth('/api/session/privileges/check', {});
    const snapshot = responseObject(await this.getAuth<unknown>('/api/session'), 'session');
    const seconds = typeof snapshot.privileges_seconds === 'number'
      ? snapshot.privileges_seconds
      : 0;
    return {
      user_id: String(snapshot.username ?? ''),
      privileges: seconds > 0 ? ['privileged'] : [],
    };
  }

  // =========================================================================
  // Users
  // =========================================================================

  async listUsers(params?: PaginationParams): Promise<User[]> {
    const response = await this.getAuth<unknown>('/api/users', params);
    return responseList<unknown>(response, 'users', 'users', 'entries').map(normalizeUser);
  }

  async getUser(username: string): Promise<UserInfo> {
    return normalizeUserInfo(
      await this.getAuth<unknown>(`/api/users/${this.pathSegment(username)}/info`),
      username,
    );
  }

  // =========================================================================
  // Search
  // =========================================================================

  async listSearches(params?: PaginationParams): Promise<Search[]> {
    const response = await this.getAuth<unknown>('/api/searches', params);
    return responseList<unknown>(response, 'searches', 'searches', 'entries').map(normalizeSearch);
  }

  async createSearch(request: SearchCreateRequest): Promise<Search> {
    return normalizeSearch(await this.postAuth<unknown>('/api/searches', request));
  }

  async getSearchDetails(id: string, params?: PaginationParams): Promise<SearchDetails> {
    return normalizeSearchDetails(await this.getAuth<unknown>(`/api/searches/${this.pathSegment(id)}`, params));
  }

  // =========================================================================
  // Messages
  // =========================================================================

  async listMessages(params?: PaginationParams): Promise<Message[]> {
    const response = await this.getAuth<unknown>('/api/messages', params);
    return responseList<unknown>(response, 'messages', 'messages', 'entries').map(normalizeMessage);
  }

  async getUserMessages(username: string, params?: PaginationParams): Promise<Message[]> {
    const response = await this.getAuth<unknown>(
      `/api/messages/${this.pathSegment(username)}`,
      params
    );
    return responseList<unknown>(response, 'messages', 'messages', 'entries').map(normalizeMessage);
  }

  async sendMessage(request: MessageSendRequest): Promise<Message> {
    return normalizeMessage(await this.postAuth<unknown>('/api/messages', {
      username: request.recipient,
      body: request.content,
    }));
  }

  async acknowledgeMessage(id: string): Promise<void> {
    await this.postAuth(`/api/messages/${this.pathSegment(id)}/ack`, {});
  }

  // =========================================================================
  // Transfers
  // =========================================================================

  async listTransfers(params?: {
    direction?: 'upload' | 'download';
    status?: string;
  } & PaginationParams): Promise<Transfer[]> {
    const query = params && {
      ...params,
      ...(params.direction
        ? { direction: params.direction === 'upload' ? 1 : 0 }
        : {}),
    };
    const response = await this.getAuth<unknown>('/api/transfers', query);
    return responseList<unknown>(response, 'transfers', 'transfers', 'entries').map(normalizeTransfer);
  }

  async createTransfer(request: TransferCreateRequest): Promise<Transfer> {
    return normalizeTransfer(await this.postAuth<unknown>('/api/transfers', {
      ...request,
      direction: request.direction === 'upload' ? 1 : 0,
    }));
  }

  async getTransfer(id: string): Promise<Transfer> {
    return normalizeTransfer(await this.getAuth<unknown>(`/api/transfers/${this.pathSegment(id)}`));
  }

  async cancelTransfer(id: string): Promise<void> {
    await this.deleteAuth(`/api/transfers/${this.pathSegment(id)}`);
  }

  // =========================================================================
  // Rooms
  // =========================================================================

  async listRooms(params?: PaginationParams): Promise<Room[]> {
    const response = await this.getAuth<unknown>('/api/rooms', params);
    return responseList<unknown>(response, 'rooms', 'rooms', 'entries').map(normalizeRoom);
  }

  async getRoom(name: string): Promise<Room> {
    return normalizeRoom(await this.getAuth<unknown>(`/api/rooms/${this.pathSegment(name)}`));
  }

  async joinRoom(name: string): Promise<Room> {
    return normalizeRoom(
      await this.postAuth<unknown>(`/api/rooms/${this.pathSegment(name)}/join`, {}),
    );
  }

  async leaveRoom(name: string): Promise<void> {
    await this.deleteAuth(`/api/rooms/${this.pathSegment(name)}/join`);
  }

  // =========================================================================
  // Shares & Filters
  // =========================================================================

  async listShares(params?: PaginationParams): Promise<Share[]> {
    const response = await this.getAuth<unknown>('/api/shares', params);
    return responseList<unknown>(response, 'shares', 'shares', 'local', 'entries').map(normalizeShare);
  }

  async refreshShares(): Promise<Record<string, unknown>> {
    return requiredObject<Record<string, unknown>>(
      await this.postAuth<unknown>('/api/shares/rescan', {}),
      'share rescan',
    );
  }

  async getFilters(): Promise<DownloadFilter> {
    return normalizeDownloadFilter(await this.getAuth<unknown>('/api/config/download-filter'));
  }

  async updateFilters(filters: DownloadFilter): Promise<DownloadFilter> {
    return normalizeDownloadFilter(
      await this.putAuth<unknown>('/api/config/download-filter', filters),
    );
  }

  // =========================================================================
  // Browse
  // =========================================================================

  async browseUser(username: string, params?: { folder?: string } & PaginationParams): Promise<BrowseResult> {
    const response = await this.getAuth<unknown>(
      `/api/users/${this.pathSegment(username)}/browse`,
      params
    );
    return normalizeBrowseResult(response);
  }

  async requestBrowse(username: string, folder?: string): Promise<BrowseRequest> {
    const path = folder === undefined
      ? `/api/users/${this.pathSegment(username)}/browse/request`
      : `/api/users/${this.pathSegment(username)}/browse/folder`;
    const response = await this.postAuth<unknown>(path, folder === undefined ? {} : { folder });
    return browseRequestFromResponse(response, username);
  }

  async getBrowseRequests(params?: { status?: string } & PaginationParams): Promise<BrowseRequest[]> {
    const response = await this.getAuth<unknown>('/api/browse/requests', params);
    return responseList<unknown>(response, 'browse requests', 'requests', 'entries')
      .map((request) => browseRequestFromResponse(request));
  }

  async respondToBrowseRequest(
    id: string,
    action: 'accept' | 'reject',
    folder?: string
  ): Promise<BrowseResult> {
    const username = this.pathSegment(id);
    const path = action === 'reject'
      ? `/api/users/${username}/browse/cancel`
      : `/api/users/${username}/browse/folder`;
    const body = action === 'reject'
      ? { reason: 'rejected by client' }
      : { folder: folder ?? '' };
    return normalizeBrowseResult(await this.postAuth<unknown>(path, body));
  }

  // =========================================================================
  // Events
  // =========================================================================

  async getEvents(
    params?: { type?: string; topic?: string; query?: string } & PaginationParams,
  ): Promise<Event[]> {
    const query = params && {
      limit: params.limit,
      offset: params.offset,
      ...(params.type ? { kind: params.type } : {}),
      ...(params.topic ? { topic: params.topic } : {}),
      ...(params.query ? { q: params.query } : {}),
    };
    const response = await this.getAuth<unknown>('/api/events', query);
    return responseList<unknown>(response, 'events', 'events', 'entries').map(normalizeEvent);
  }

  // =========================================================================
  // Cache
  // =========================================================================

  async getCacheStats(): Promise<CacheStats> {
    return normalizeCacheStats(await this.getAuth<unknown>('/api/mediacore/retrieve/stats'));
  }

  async invalidateCache(keys: string[] = []): Promise<Record<string, unknown>> {
    return requiredObject<Record<string, unknown>>(
      await this.postAuth<unknown>('/api/mediacore/retrieve/cache/clear', { keys }),
      'cache invalidation',
    );
  }

  // =========================================================================
  // HTTP Methods
  // =========================================================================

  private async get<T>(path: string, query?: Record<string, any>): Promise<T> {
    const url = this.buildUrl(path, query);
    return this.request<T>('GET', url);
  }

  private async getAuth<T>(path: string, query?: Record<string, any>): Promise<T> {
    const url = this.buildUrl(path, query);
    return this.request<T>('GET', url, undefined, true);
  }

  private async post<T>(path: string, body?: any): Promise<T> {
    return this.request<T>('POST', this.baseUrl + path, body, false);
  }

  private async postAuth<T>(path: string, body?: any): Promise<T> {
    return this.request<T>('POST', this.baseUrl + path, body, true);
  }

  private async put<T>(path: string, body?: any): Promise<T> {
    return this.request<T>('PUT', this.baseUrl + path, body, false);
  }

  private async putAuth<T>(path: string, body?: any): Promise<T> {
    return this.request<T>('PUT', this.baseUrl + path, body, true);
  }

  private async deleteAuth(path: string): Promise<void> {
    await this.request('DELETE', this.baseUrl + path, {}, true);
  }

  // =========================================================================
  // Core Request Handler
  // =========================================================================

  private async request<T>(
    method: string,
    url: string,
    body?: any,
    authenticated: boolean = false,
    attempt: number = 0
  ): Promise<T> {
    let timeoutId: ReturnType<typeof setTimeout> | undefined;
    try {
      if (this.debug) {
        console.debug('[slskr] request', method, url, this.debugBody(body));
      }

      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
      };

      if (authenticated) {
        headers['Authorization'] = `Bearer ${this.token}`;
      }

      const controller = new AbortController();
      timeoutId = this.timeout > 0
        ? setTimeout(() => controller.abort(), this.timeout)
        : undefined;

      const response = await fetch(url, {
        method,
        headers,
        body: body === undefined ? undefined : JSON.stringify(body),
        signal: controller.signal,
        redirect: 'error',
      });
      if (!response.ok) {
        const parsedError = await this.readJson(response, MAX_HTTP_ERROR_BYTES).catch(() => ({}));
        const errorData =
          parsedError !== null &&
          typeof parsedError === 'object' &&
          !Array.isArray(parsedError)
            ? parsedError as Record<string, unknown>
            : {};
        const errorCode =
          typeof errorData.code === 'string'
            ? errorData.code
            : typeof errorData.error === 'string'
              ? errorData.error
              : `HTTP ${response.status}`;
        const message =
          typeof errorData.message === 'string'
            ? errorData.message
            : typeof errorData.detail === 'string'
              ? errorData.detail
              : typeof errorData.error === 'string'
                ? errorData.error
                : `HTTP ${response.status}`;
        const details = typeof errorData.details === 'string' ? errorData.details : undefined;
        throw new ApiError(
          response.status,
          errorCode,
          message,
          details
        );
      }

      if (response.status === 204) {
        return undefined as T;
      }

      const data = await this.readJson(response, MAX_HTTP_RESPONSE_BYTES);
      return data as T;
    } catch (error) {
      if (error instanceof ApiError || error instanceof NetworkError) {
        throw error;
      }

      if (
        (error instanceof Error && error.name === 'AbortError') ||
        (typeof error === 'object' &&
          error !== null &&
          'name' in error &&
          error.name === 'AbortError')
      ) {
        throw new TimeoutError(`Request timeout after ${this.timeout}ms`);
      }

      if (method === 'GET' && attempt < this.retries) {
        await new Promise((resolve) => setTimeout(resolve, this.retryDelay));
        return this.request<T>(method, url, body, authenticated, attempt + 1);
      }

      throw new NetworkError(
        `Failed to ${method} ${url}`,
        error instanceof Error ? error : undefined
      );
    } finally {
      if (timeoutId !== undefined) {
        clearTimeout(timeoutId);
      }
    }
  }

  private async readJson(response: Response, maximum: number): Promise<unknown> {
    const declaredLength = response.headers.get('content-length');
    if (declaredLength !== null && Number(declaredLength) > maximum) {
      throw new NetworkError(`HTTP response body exceeds ${maximum} bytes`);
    }

    const reader = response.body?.getReader();
    if (!reader) {
      const text = await response.text();
      if (new TextEncoder().encode(text).byteLength > maximum) {
        throw new NetworkError(`HTTP response body exceeds ${maximum} bytes`);
      }
      if (!text.trim()) {
        return undefined;
      }
      return JSON.parse(text);
    }

    const chunks: Uint8Array[] = [];
    let length = 0;
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      length += value.byteLength;
      if (length > maximum) {
        await reader.cancel();
        throw new NetworkError(`HTTP response body exceeds ${maximum} bytes`);
      }
      chunks.push(value);
    }

    const body = new Uint8Array(length);
    let offset = 0;
    for (const chunk of chunks) {
      body.set(chunk, offset);
      offset += chunk.byteLength;
    }
    const text = new TextDecoder().decode(body);
    if (!text.trim()) {
      return undefined;
    }
    return JSON.parse(text);
  }

  // =========================================================================
  // Utilities
  // =========================================================================

  private buildUrl(path: string, query?: Record<string, any>): string {
    let url = this.baseUrl + path;

    if (query && Object.keys(query).length > 0) {
      const params = new URLSearchParams();
      Object.entries(query).forEach(([key, value]) => {
        if (value !== undefined && value !== null) {
          params.append(key, String(value));
        }
      });
      const queryString = params.toString();
      if (queryString) {
        url += '?' + queryString;
      }
    }

    return url;
  }

  private pathSegment(value: string): string {
    return encodeURIComponent(value);
  }
}

export default SlskrClient;
