//! Axum-based HTTP router for Phase 8 migration
//! 
//! Replaces the hand-rolled router with Axum framework for better maintainability,
//! middleware support, and code organization.

use axum::{
    extract::{Path, Query, State},
    http::{Method, StatusCode, HeaderMap},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put, delete, patch},
    Router,
    body::Body,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

/// Shared application state for Axum
#[derive(Clone)]
pub struct AxumAppState {
    pub inner: Arc<crate::AppState>,
}

/// Request context with request ID and metadata
#[derive(Clone, Debug)]
pub struct RequestContext {
    pub request_id: String,
    pub remote_addr: String,
    pub user_agent: Option<String>,
}

/// Build the Axum router with all endpoint handlers
pub fn build_router(state: AxumAppState) -> Router {
    Router::new()
        // Health and API info endpoints
        .route("/api/health", get(health_handler))
        .route("/api/version", get(version_handler))
        .route("/api/capabilities", get(capabilities_handler))
        
        // Collections endpoints
        .route("/api/collections", get(list_collections))
        .route("/api/collections", post(create_collection))
        .route("/api/collections/:id", get(get_collection))
        .route("/api/collections/:id", put(update_collection))
        .route("/api/collections/:id", delete(delete_collection))
        .route("/api/collections/:id/items", get(list_collection_items))
        .route("/api/collections/:id/items", post(add_collection_item))
        .route("/api/collections/:id/items/reorder", put(reorder_collection_items))
        
        // Wishlist endpoints
        .route("/api/wishlist", get(list_wishlist))
        .route("/api/wishlist", post(add_wishlist_item))
        .route("/api/wishlist/:id", get(get_wishlist_item))
        .route("/api/wishlist/:id", put(update_wishlist_item))
        .route("/api/wishlist/:id", delete(delete_wishlist_item))
        .route("/api/wishlist/:id/search", post(search_wishlist_item))
        .route("/api/wishlist/import/csv", post(import_wishlist_csv))
        
        // Contacts endpoints
        .route("/api/contacts", get(list_contacts))
        .route("/api/contacts", post(create_contact))
        .route("/api/contacts/nearby", get(list_nearby_contacts))
        .route("/api/contacts/:id", get(get_contact))
        .route("/api/contacts/:id", put(update_contact))
        .route("/api/contacts/:id", delete(delete_contact))
        .route("/api/contacts/from-discovery", post(add_contact_from_discovery))
        .route("/api/contacts/from-invite", post(add_contact_from_invite))
        
        // Profile endpoints
        .route("/api/profile/me", get(get_profile))
        .route("/api/profile/me", put(update_profile))
        .route("/api/profile/:username", get(get_user_profile))
        
        // Searches endpoints
        .route("/api/searches", get(list_searches))
        .route("/api/searches", post(create_search))
        .route("/api/searches/:id", get(get_search))
        .route("/api/searches/:id", put(update_search))
        .route("/api/searches/:id", delete(delete_search))
        .route("/api/searches/:id/responses", get(get_search_responses))
        .route("/api/searches/prune", post(prune_searches))
        
        // Rooms endpoints
        .route("/api/rooms/available", get(list_available_rooms))
        .route("/api/rooms/joined", get(list_joined_rooms))
        .route("/api/rooms/joined", post(join_room))
        .route("/api/rooms/joined/:name", delete(leave_room))
        .route("/api/rooms/joined/:name/messages", get(get_room_messages))
        .route("/api/rooms/joined/:name/users", get(get_room_users))
        
        // Transfers endpoints
        .route("/api/transfers", get(list_transfers))
        .route("/api/transfers", post(create_transfer))
        .route("/api/transfers/:id", get(get_transfer))
        .route("/api/transfers/:id", delete(cancel_transfer))
        .route("/api/transfers/speeds", get(get_transfer_speeds))
        .route("/api/transfers/downloads/stats", get(get_download_stats))
        .route("/api/transfers/downloads/accelerated", get(list_accelerated_downloads))
        .route("/api/transfers/downloads/accelerated", put(update_accelerated_downloads))
        .route("/api/transfers/downloads/stuck", get(list_stuck_downloads))
        .route("/api/transfers/downloads/user-stats", get(get_transfer_user_stats))
        
        // Bridge endpoints
        .route("/api/bridge/status", get(get_bridge_status))
        .route("/api/bridge/admin/clients", get(list_bridge_clients))
        .route("/api/bridge/admin/config", get(get_bridge_config))
        .route("/api/bridge/admin/config", put(update_bridge_config))
        .route("/api/bridge/admin/dashboard", get(get_bridge_dashboard))
        .route("/api/bridge/admin/stats", get(get_bridge_stats))
        .route("/api/bridge/transfer/:id/progress", get(get_transfer_progress))
        .route("/api/bridge/start", post(start_bridge))
        .route("/api/bridge/stop", post(stop_bridge))
        
        // Library endpoints
        .route("/api/library/items", get(list_library_items))
        .route("/api/library/items/:id", get(get_library_item))
        .route("/api/library/health/issues", get(list_health_issues))
        .route("/api/library/health/issues/by-artist", get(list_health_issues_by_artist))
        .route("/api/library/health/issues/by-release", get(list_health_issues_by_release))
        .route("/api/library/health/scans", post(start_health_scan))
        .route("/api/library/health/scans/:id", get(get_health_scan))
        .route("/api/library/health/issues/fix", post(fix_health_issues))
        
        // Server state endpoints
        .route("/api/server", get(get_server_status))
        .route("/api/server", put(update_server_status))
        .route("/api/server", delete(shutdown_server))
        .route("/api/session", get(get_session))
        .route("/api/session", post(create_session))
        .route("/api/application", get(get_application_status))
        .route("/api/application", put(update_application_status))
        .route("/api/application", delete(shutdown_application))
        
        // Middleware
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn(logging_middleware))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// Middleware for adding request ID
async fn request_id_middleware(
    mut req: axum::http::Request<Body>,
    next: Next,
) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    req.extensions_mut().insert(RequestContext {
        request_id: request_id.clone(),
        remote_addr: "0.0.0.0".to_string(), // Would be extracted from ConnectInfo
        user_agent: None,
    });
    
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        "X-Request-ID",
        request_id.parse().unwrap(),
    );
    response
}

// Middleware for request logging
async fn logging_middleware(
    req: axum::http::Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    
    let start = std::time::Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed();
    
    tracing::info!(
        method = %method,
        path = %path,
        status = %response.status(),
        duration_ms = duration.as_millis(),
        "HTTP request"
    );
    
    response
}

// Health endpoint
async fn health_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "status": "ok" })),
    )
}

// Version endpoint
async fn version_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "version": "1.0.0",
            "api_version": "v0"
        })),
    )
}

// Capabilities endpoint
async fn capabilities_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "api_version": "v0",
            "supports": ["login", "peers", "shares", "searches", "transfers", "users", "messages", "rooms"]
        })),
    )
}

// Placeholder handlers for all endpoints
// These would be implemented with proper business logic

#[axum::debug_handler]
async fn list_collections(State(_state): State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"collections": []})))
}

#[axum::debug_handler]
async fn create_collection(
    State(_state): State<AxumAppState>,
) -> impl IntoResponse {
    (StatusCode::CREATED, axum::Json(serde_json::json!({"created": true})))
}

#[axum::debug_handler]
async fn get_collection(
    State(_state): State<AxumAppState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"collection": {}})))
}

#[axum::debug_handler]
async fn update_collection(
    State(_state): State<AxumAppState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"updated": true})))
}

#[axum::debug_handler]
async fn delete_collection(
    State(_state): State<AxumAppState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"deleted": true})))
}

#[axum::debug_handler]
async fn list_collection_items(
    State(_state): State<AxumAppState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"items": []})))
}

#[axum::debug_handler]
async fn add_collection_item(
    State(_state): State<AxumAppState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    (StatusCode::CREATED, axum::Json(serde_json::json!({"created": true})))
}

#[axum::debug_handler]
async fn reorder_collection_items(
    State(_state): State<AxumAppState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"reordered": true})))
}

// Placeholder implementations for remaining endpoints
// In Phase 8 proper implementation, these would contain actual business logic

// Wishlist endpoints
async fn list_wishlist(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"items": []})))
}
async fn add_wishlist_item(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::CREATED, axum::Json(serde_json::json!({"created": true})))
}
async fn get_wishlist_item(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"item": {}})))
}
async fn update_wishlist_item(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"updated": true})))
}
async fn delete_wishlist_item(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"deleted": true})))
}
async fn search_wishlist_item(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"searching": true})))
}
async fn import_wishlist_csv(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::ACCEPTED, axum::Json(serde_json::json!({"imported": true})))
}

// Contacts endpoints
async fn list_contacts(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"contacts": []})))
}
async fn create_contact(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::CREATED, axum::Json(serde_json::json!({"created": true})))
}
async fn list_nearby_contacts(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"nearby": []})))
}
async fn get_contact(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"contact": {}})))
}
async fn update_contact(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"updated": true})))
}
async fn delete_contact(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"deleted": true})))
}
async fn add_contact_from_discovery(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::CREATED, axum::Json(serde_json::json!({"created": true})))
}
async fn add_contact_from_invite(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::CREATED, axum::Json(serde_json::json!({"created": true})))
}

// Profile endpoints
async fn get_profile(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"profile": {}})))
}
async fn update_profile(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"updated": true})))
}
async fn get_user_profile(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"profile": {}})))
}

// Searches endpoints
async fn list_searches(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"searches": []})))
}
async fn create_search(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::CREATED, axum::Json(serde_json::json!({"created": true})))
}
async fn get_search(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"search": {}})))
}
async fn update_search(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"updated": true})))
}
async fn delete_search(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"deleted": true})))
}
async fn get_search_responses(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"responses": []})))
}
async fn prune_searches(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"pruned": true})))
}

// Rooms endpoints
async fn list_available_rooms(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"rooms": []})))
}
async fn list_joined_rooms(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"rooms": []})))
}
async fn join_room(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::CREATED, axum::Json(serde_json::json!({"joined": true})))
}
async fn leave_room(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"left": true})))
}
async fn get_room_messages(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"messages": []})))
}
async fn get_room_users(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"users": []})))
}

// Transfers endpoints
async fn list_transfers(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"transfers": []})))
}
async fn create_transfer(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::CREATED, axum::Json(serde_json::json!({"created": true})))
}
async fn get_transfer(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"transfer": {}})))
}
async fn cancel_transfer(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"cancelled": true})))
}
async fn get_transfer_speeds(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"speeds": []})))
}
async fn get_download_stats(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"stats": {}})))
}
async fn list_accelerated_downloads(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"downloads": []})))
}
async fn update_accelerated_downloads(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"updated": true})))
}
async fn list_stuck_downloads(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"stuck": []})))
}
async fn get_transfer_user_stats(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"stats": []})))
}

// Bridge endpoints
async fn get_bridge_status(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"status": "offline"})))
}
async fn list_bridge_clients(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"clients": []})))
}
async fn get_bridge_config(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"config": {}})))
}
async fn update_bridge_config(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"updated": true})))
}
async fn get_bridge_dashboard(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"dashboard": {}})))
}
async fn get_bridge_stats(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"stats": {}})))
}
async fn get_transfer_progress(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"progress": {}})))
}
async fn start_bridge(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::ACCEPTED, axum::Json(serde_json::json!({"started": true})))
}
async fn stop_bridge(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"stopped": true})))
}

// Library endpoints
async fn list_library_items(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"items": []})))
}
async fn get_library_item(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"item": {}})))
}
async fn list_health_issues(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"issues": []})))
}
async fn list_health_issues_by_artist(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"issues": []})))
}
async fn list_health_issues_by_release(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"issues": []})))
}
async fn start_health_scan(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::ACCEPTED, axum::Json(serde_json::json!({"started": true})))
}
async fn get_health_scan(_state: State<AxumAppState>, _path: Path<String>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"scan": {}})))
}
async fn fix_health_issues(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::ACCEPTED, axum::Json(serde_json::json!({"fixing": true})))
}

// Server state endpoints
async fn get_server_status(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"status": "offline"})))
}
async fn update_server_status(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"updated": true})))
}
async fn shutdown_server(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"shutdown": true})))
}
async fn get_session(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"session": {}})))
}
async fn create_session(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::CREATED, axum::Json(serde_json::json!({"created": true})))
}
async fn get_application_status(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"status": "running"})))
}
async fn update_application_status(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"updated": true})))
}
async fn shutdown_application(_state: State<AxumAppState>) -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"shutdown": true})))
}
