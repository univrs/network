//! REST API endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::AppState;
use super::messages::PeerListEntry;

/// List all peers
pub async fn list_peers(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<PeerListEntry>> {
    let peers = state.store.list_peers().await.unwrap_or_default();
    let entries: Vec<PeerListEntry> = peers.into_iter().map(Into::into).collect();
    Json(entries)
}

/// Get specific peer
pub async fn get_peer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<Option<PeerListEntry>> {
    match state.store.get_peer(&id).await {
        Ok(Some((info, rep))) => Json(Some(PeerListEntry::from((info, rep)))),
        _ => Json(None),
    }
}

/// Network statistics
#[derive(Serialize)]
pub struct NetworkStats {
    pub local_peer_id: String,
    pub peer_count: usize,
    pub message_count: u64,
    pub uptime_seconds: u64,
    pub subscribed_topics: Vec<String>,
}

pub async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Json<NetworkStats> {
    let peers = state.store.list_peers().await.unwrap_or_default();
    Json(NetworkStats {
        local_peer_id: state.local_peer_id.to_string(),
        peer_count: peers.len(),
        message_count: state.message_count.load(std::sync::atomic::Ordering::Relaxed),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        subscribed_topics: state.subscribed_topics.read().clone(),
    })
}

/// Health check endpoint
pub async fn health() -> &'static str {
    "OK"
}

/// Node info endpoint
#[derive(Serialize)]
pub struct NodeInfo {
    pub version: &'static str,
    pub name: String,
    pub peer_id: String,
}

pub async fn node_info(
    State(state): State<Arc<AppState>>,
) -> Json<NodeInfo> {
    Json(NodeInfo {
        version: env!("CARGO_PKG_VERSION"),
        name: state.node_name.clone(),
        peer_id: state.local_peer_id.to_string(),
    })
}
