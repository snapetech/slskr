/**
 * Main HTTP API client for slskr
 */
import { ClientConfig, HealthStatus, VersionInfo, Capabilities, Configuration, Statistics, Session, SessionPrivileges, Search, SearchDetails, SearchCreateRequest, Message, MessageSendRequest, Transfer, TransferCreateRequest, Room, BrowseResult, BrowseRequest, Event, PaginationParams, CacheStats } from './types';
export declare class SlskrClient {
    private baseUrl;
    private token;
    private timeout;
    private retries;
    private retryDelay;
    private debug;
    constructor(config: ClientConfig);
    private debugBody;
    health(): Promise<HealthStatus>;
    version(): Promise<VersionInfo>;
    getConfig(): Promise<Configuration>;
    getStats(): Promise<Statistics>;
    getCapabilities(): Promise<Capabilities>;
    getSessions(): Promise<Session[]>;
    createSession(kind: string, parameters?: Record<string, any>): Promise<Session>;
    pingSession(id: string): Promise<{
        status: string;
        latency_ms: number;
    }>;
    disconnectSession(id: string): Promise<void>;
    getSessionPrivileges(id: string): Promise<SessionPrivileges>;
    listSearches(params?: PaginationParams): Promise<Search[]>;
    createSearch(request: SearchCreateRequest): Promise<Search>;
    getSearchDetails(id: string, params?: PaginationParams): Promise<SearchDetails>;
    listMessages(params?: PaginationParams): Promise<Message[]>;
    getUserMessages(username: string, params?: PaginationParams): Promise<Message[]>;
    sendMessage(request: MessageSendRequest): Promise<Message>;
    acknowledgeMessage(id: string): Promise<void>;
    listTransfers(params?: {
        direction?: 'upload' | 'download';
        status?: string;
    } & PaginationParams): Promise<Transfer[]>;
    createTransfer(request: TransferCreateRequest): Promise<Transfer>;
    getTransfer(id: string): Promise<Transfer>;
    cancelTransfer(id: string): Promise<void>;
    listRooms(params?: PaginationParams): Promise<Room[]>;
    getRoom(name: string): Promise<Room>;
    joinRoom(name: string): Promise<Room>;
    leaveRoom(name: string): Promise<void>;
    browseUser(username: string, params?: {
        folder?: string;
    } & PaginationParams): Promise<BrowseResult>;
    requestBrowse(username: string, folder?: string): Promise<BrowseRequest>;
    getBrowseRequests(params?: {
        status?: string;
    } & PaginationParams): Promise<BrowseRequest[]>;
    respondToBrowseRequest(id: string, action: 'accept' | 'reject', folder?: string): Promise<BrowseResult>;
    getEvents(params?: {
        type?: string;
    } & PaginationParams): Promise<Event[]>;
    getCacheStats(): Promise<CacheStats>;
    invalidateCache(keys: string[]): Promise<void>;
    private get;
    private getAuth;
    private post;
    private postAuth;
    private put;
    private putAuth;
    private deleteAuth;
    private request;
    private readJson;
    private buildUrl;
    private pathSegment;
}
export default SlskrClient;
//# sourceMappingURL=client.d.ts.map