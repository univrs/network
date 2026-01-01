//! Dashboard backend server
//!
//! This module provides the WebSocket and REST API server for the
//! mycelial node dashboard.

pub mod messages;
pub mod rest;
pub mod websocket;

use axum::{routing::get, Router};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::AppState;

/// Create the server router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(rest::health))
        // Node info
        .route("/api/info", get(rest::node_info))
        // WebSocket endpoint
        .route("/ws", get(websocket::ws_handler))
        // REST endpoints
        .route("/api/peers", get(rest::list_peers))
        .route("/api/peer/:id", get(rest::get_peer))
        .route("/api/stats", get(rest::get_stats))
        // CORS for dashboard
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state)
}
