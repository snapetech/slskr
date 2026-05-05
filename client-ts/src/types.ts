/**
 * Type definitions for slskr HTTP API
 */

// ============================================================================
// Request/Response Types
// ============================================================================

export interface ApiResponse<T = any> {
  data?: T;
  error?: string;
  details?: string;
}

export interface PaginationParams {
  limit?: number;
  offset?: number;
}

// ============================================================================
// Health & Version
// ============================================================================

export interface HealthStatus {
  status: 'ok' | 'unhealthy';
  timestamp: string;
}

export interface VersionInfo {
  version: string;
  client_name: string;
  major_version: number;
  minor_version: number;
}

// ============================================================================
// Configuration & Stats
// ============================================================================

export interface Configuration {
  username: string;
  server_address: string;
  shared_directories?: string[];
  transfer_max_active?: number;
}

export interface Statistics {
  total_size?: number;
  file_count?: number;
  uploads?: number;
  downloads?: number;
  transfer_speeds?: {
    up?: number;
    down?: number;
  };
}

// ============================================================================
// Capabilities
// ============================================================================

export interface Capabilities {
  app: string[];
  network: string[];
  storage: string[];
  experimental?: string[];
}

// ============================================================================
// Session Management
// ============================================================================

export interface Session {
  id: string;
  type: 'server' | 'peer';
  status: 'connecting' | 'connected' | 'disconnecting' | 'disconnected';
  connected_at?: string;
}

export interface SessionPrivileges {
  user_id: string;
  privileges: string[];
}

// ============================================================================
// Search
// ============================================================================

export interface SearchResult {
  username: string;
  filename: string;
  size: number;
  bitrate?: number;
  length?: number;
}

export interface Search {
  id: string;
  query: string;
  status: 'active' | 'completed' | 'failed';
  results_count: number;
  started_at: string;
}

export interface SearchDetails extends Search {
  results: SearchResult[];
}

export interface SearchCreateRequest {
  query: string;
  room?: string | null;
  target?: string | null;
}

// ============================================================================
// Messages
// ============================================================================

export interface Message {
  id: string;
  sender: string;
  content: string;
  timestamp: string;
}

export interface MessageSendRequest {
  recipient: string;
  content: string;
}

// ============================================================================
// Transfers
// ============================================================================

export type TransferDirection = 'upload' | 'download';
export type TransferStatus = 'active' | 'completed' | 'failed' | 'cancelled';

export interface Transfer {
  id: string;
  direction: TransferDirection;
  status: TransferStatus;
  peer_username: string;
  filename: string;
  local_path?: string;
  size?: number;
  bytes_transferred: number;
  progress_percent?: number;
  speed_bytes_per_sec?: number;
  eta_seconds?: number;
  started_at: string;
  reason?: string;
}

export interface TransferCreateRequest {
  direction: TransferDirection;
  peer_username: string;
  filename: string;
}

// ============================================================================
// Rooms
// ============================================================================

export interface Room {
  name: string;
  user_count: number;
  users?: string[];
}

// ============================================================================
// Browse
// ============================================================================

export interface BrowseEntry {
  filename: string;
  size: number;
  extension?: string;
}

export interface BrowseResult {
  entries: BrowseEntry[];
  folder?: string;
}

export interface BrowseRequest {
  id: string;
  from: string;
  status: 'pending' | 'accepted' | 'rejected';
  requested_at: string;
}

// ============================================================================
// Events
// ============================================================================

export type EventType =
  | 'search.started'
  | 'search.completed'
  | 'search.result'
  | 'transfer.started'
  | 'transfer.completed'
  | 'transfer.failed'
  | 'message.received'
  | 'connection.status'
  | 'room.joined'
  | 'room.user_joined'
  | 'room.user_left';

export interface Event {
  id: string;
  type: EventType;
  data: Record<string, any>;
  timestamp: string;
}

// ============================================================================
// Batch Operations
// ============================================================================

export interface BatchOperation {
  id: string;
  method: 'GET' | 'POST' | 'PUT' | 'DELETE';
  path: string;
  body?: any;
}

export interface BatchRequest {
  operations: BatchOperation[];
}

export interface BatchResult {
  id: string;
  status: number;
  body: any;
}

export interface BatchResponse {
  results: BatchResult[];
  total_time_ms: number;
}

// ============================================================================
// Cache Control
// ============================================================================

export interface CacheStats {
  hits: number;
  misses: number;
  evictions: number;
  total_requests: number;
  hit_rate: number;
}

// ============================================================================
// WebSocket Events
// ============================================================================

export interface WebSocketSubscription {
  type: 'subscribe' | 'unsubscribe';
  topics: string[];
}

export interface WebSocketMessage {
  type: EventType | 'subscribe' | 'unsubscribe' | 'ping' | 'pong';
  data?: any;
}

// ============================================================================
// Client Configuration
// ============================================================================

export interface ClientConfig {
  baseUrl: string;
  token: string;
  timeout?: number;
  retries?: number;
  retryDelay?: number;
  debug?: boolean;
}

export interface RequestConfig {
  headers?: Record<string, string>;
  timeout?: number;
  retries?: number;
}
