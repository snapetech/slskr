/**
 * Main HTTP API client for soulseekR
 */

import {
  ClientConfig,
  RequestConfig,
  ApiResponse,
  HealthStatus,
  VersionInfo,
  Capabilities,
  Configuration,
  Statistics,
  Session,
  SessionPrivileges,
  Search,
  SearchDetails,
  SearchCreateRequest,
  Message,
  MessageSendRequest,
  Transfer,
  TransferCreateRequest,
  Room,
  BrowseResult,
  BrowseRequest,
  Event,
  PaginationParams,
  CacheStats,
} from './types';
import { ApiError, NetworkError, TimeoutError, ValidationError } from './errors';

export class SoulseekrClient {
  private baseUrl: string;
  private token: string;
  private timeout: number;
  private retries: number;
  private retryDelay: number;
  private debug: boolean;

  constructor(config: ClientConfig) {
    this.baseUrl = config.baseUrl.replace(/\/$/, '');
    this.token = config.token;
    this.timeout = config.timeout || 30000;
    this.retries = config.retries || 3;
    this.retryDelay = config.retryDelay || 1000;
    this.debug = config.debug || false;
  }

  // =========================================================================
  // Health & Version
  // =========================================================================

  async health(): Promise<HealthStatus> {
    return this.get<HealthStatus>('/api/health', {});
  }

  async version(): Promise<VersionInfo> {
    return this.get<VersionInfo>('/api/version', {});
  }

  // =========================================================================
  // Configuration
  // =========================================================================

  async getConfig(): Promise<Configuration> {
    return this.getAuth<Configuration>('/api/config');
  }

  async getStats(): Promise<Statistics> {
    return this.getAuth<Statistics>('/api/stats');
  }

  // =========================================================================
  // Capabilities
  // =========================================================================

  async getCapabilities(): Promise<Capabilities> {
    return this.get<Capabilities>('/api/capabilities', {});
  }

  // =========================================================================
  // Sessions
  // =========================================================================

  async getSessions(): Promise<Session[]> {
    const response = await this.getAuth<{ sessions: Session[] }>('/api/sessions');
    return response.sessions || [];
  }

  async createSession(kind: string, parameters?: Record<string, any>): Promise<Session> {
    return this.postAuth<Session>('/api/sessions', { kind, parameters });
  }

  async pingSession(id: string): Promise<{ status: string; latency_ms: number }> {
    return this.postAuth(`/api/sessions/${id}/ping`, {});
  }

  async disconnectSession(id: string): Promise<void> {
    await this.deleteAuth(`/api/sessions/${id}`);
  }

  async getSessionPrivileges(id: string): Promise<SessionPrivileges> {
    return this.getAuth<SessionPrivileges>(`/api/sessions/${id}/privileges`);
  }

  // =========================================================================
  // Search
  // =========================================================================

  async listSearches(params?: PaginationParams): Promise<Search[]> {
    const response = await this.getAuth<{ searches: Search[] }>('/api/searches', params);
    return response.searches || [];
  }

  async createSearch(request: SearchCreateRequest): Promise<Search> {
    return this.postAuth<Search>('/api/searches', request);
  }

  async getSearchDetails(id: string, params?: PaginationParams): Promise<SearchDetails> {
    return this.getAuth<SearchDetails>(`/api/searches/${id}`, params);
  }

  // =========================================================================
  // Messages
  // =========================================================================

  async listMessages(params?: PaginationParams): Promise<Message[]> {
    const response = await this.getAuth<{ messages: Message[] }>('/api/messages', params);
    return response.messages || [];
  }

  async getUserMessages(username: string, params?: PaginationParams): Promise<Message[]> {
    const response = await this.getAuth<{ messages: Message[] }>(
      `/api/messages/${username}`,
      params
    );
    return response.messages || [];
  }

  async sendMessage(request: MessageSendRequest): Promise<Message> {
    return this.postAuth<Message>('/api/messages', request);
  }

  async acknowledgeMessage(id: string): Promise<void> {
    await this.putAuth(`/api/messages/${id}/acknowledge`, {});
  }

  // =========================================================================
  // Transfers
  // =========================================================================

  async listTransfers(params?: {
    direction?: 'upload' | 'download';
    status?: string;
  } & PaginationParams): Promise<Transfer[]> {
    const response = await this.getAuth<{ transfers: Transfer[] }>('/api/transfers', params);
    return response.transfers || [];
  }

  async createTransfer(request: TransferCreateRequest): Promise<Transfer> {
    return this.postAuth<Transfer>('/api/transfers', request);
  }

  async getTransfer(id: string): Promise<Transfer> {
    return this.getAuth<Transfer>(`/api/transfers/${id}`);
  }

  async cancelTransfer(id: string): Promise<void> {
    await this.deleteAuth(`/api/transfers/${id}`);
  }

  // =========================================================================
  // Rooms
  // =========================================================================

  async listRooms(params?: PaginationParams): Promise<Room[]> {
    const response = await this.getAuth<{ rooms: Room[] }>('/api/rooms', params);
    return response.rooms || [];
  }

  async getRoom(name: string): Promise<Room> {
    return this.getAuth<Room>(`/api/rooms/${name}`);
  }

  async joinRoom(name: string): Promise<Room> {
    return this.postAuth<Room>(`/api/rooms/${name}`, {});
  }

  async leaveRoom(name: string): Promise<void> {
    await this.deleteAuth(`/api/rooms/${name}`);
  }

  // =========================================================================
  // Browse
  // =========================================================================

  async browseUser(username: string, params?: { folder?: string } & PaginationParams): Promise<BrowseResult> {
    return this.getAuth<BrowseResult>(`/api/browse/${username}`, params);
  }

  async requestBrowse(username: string, folder?: string): Promise<BrowseRequest> {
    return this.postAuth<BrowseRequest>(`/api/browse/${username}`, { folder });
  }

  async getBrowseRequests(params?: { status?: string } & PaginationParams): Promise<BrowseRequest[]> {
    const response = await this.getAuth<{ requests: BrowseRequest[] }>('/api/browse/requests', params);
    return response.requests || [];
  }

  async respondToBrowseRequest(
    id: string,
    action: 'accept' | 'reject',
    folder?: string
  ): Promise<BrowseResult> {
    return this.postAuth<BrowseResult>(`/api/browse/requests/${id}`, { action, folder });
  }

  // =========================================================================
  // Events
  // =========================================================================

  async getEvents(params?: { type?: string } & PaginationParams): Promise<Event[]> {
    const response = await this.getAuth<{ events: Event[] }>('/api/events', params);
    return response.events || [];
  }

  // =========================================================================
  // Cache
  // =========================================================================

  async getCacheStats(): Promise<CacheStats> {
    return this.getAuth<CacheStats>('/api/cache/stats');
  }

  async invalidateCache(keys: string[]): Promise<void> {
    await this.postAuth('/api/cache/invalidate', { keys });
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
    return this.request<T>('GET', url, {}, true);
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
    try {
      if (this.debug) {
        console.debug(`[soulseekr] ${method} ${url}`, body);
      }

      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
      };

      if (authenticated) {
        headers['Authorization'] = `Bearer ${this.token}`;
      }

      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), this.timeout);

      const response = await fetch(url, {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new ApiError(
          response.status,
          errorData.error || `HTTP ${response.status}`,
          errorData.details
        );
      }

      const data = await response.json();
      return data as T;
    } catch (error) {
      if (error instanceof ApiError) {
        throw error;
      }

      if (error instanceof Error && error.name === 'AbortError') {
        throw new TimeoutError(`Request timeout after ${this.timeout}ms`);
      }

      if (attempt < this.retries) {
        await new Promise((resolve) => setTimeout(resolve, this.retryDelay));
        return this.request<T>(method, url, body, authenticated, attempt + 1);
      }

      throw new NetworkError(
        `Failed to ${method} ${url}`,
        error instanceof Error ? error : undefined
      );
    }
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
}

export default SoulseekrClient;
