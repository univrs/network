//! Dashboard backend server
//!
//! This module provides the WebSocket and REST API server for the
//! mycelial node dashboard.

pub mod websocket;
pub mod rest;
pub mod messages;
pub mod economics_state;

use axum::{
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};

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
        // Economics API endpoints
        .route("/api/economics", get(rest::get_economics_summary))
        .route("/api/economics/credit-lines", get(rest::list_credit_lines))
        .route("/api/economics/credit-lines/:peer_id", get(rest::get_credit_lines_for_peer))
        .route("/api/economics/proposals", get(rest::list_proposals))
        .route("/api/economics/proposals/active", get(rest::list_active_proposals))
        .route("/api/economics/proposal/:id", get(rest::get_proposal))
        .route("/api/economics/vouches/to/:peer_id", get(rest::get_vouches_for_peer))
        .route("/api/economics/vouches/from/:peer_id", get(rest::get_vouches_from_peer))
        .route("/api/economics/reputation/:peer_id", get(rest::get_peer_reputation))
        .route("/api/economics/resources", get(rest::get_resource_pool))
        .route("/api/economics/peer/:peer_id", get(rest::get_peer_economics))
        // CORS for dashboard
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
        .with_state(state)
}
