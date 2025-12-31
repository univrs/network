//! REST API endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::AppState;
use super::messages::PeerListEntry;
use super::economics_state::{
    CreditLine, Proposal, Vouch, ResourcePool, EconomicsSummary,
};

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

// ─────────────────────────────────────────────────────────────────────────────
// Economics API Endpoints
// ─────────────────────────────────────────────────────────────────────────────

/// Get economics state summary
pub async fn get_economics_summary(
    State(state): State<Arc<AppState>>,
) -> Json<EconomicsSummary> {
    Json(state.economics.get_summary())
}

/// List all credit lines
pub async fn list_credit_lines(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<CreditLine>> {
    Json(state.economics.get_all_credit_lines())
}

/// Get credit lines for a specific peer
pub async fn get_credit_lines_for_peer(
    State(state): State<Arc<AppState>>,
    Path(peer_id): Path<String>,
) -> Json<Vec<CreditLine>> {
    Json(state.economics.get_credit_lines_for_peer(&peer_id))
}

/// List all proposals
pub async fn list_proposals(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<Proposal>> {
    Json(state.economics.get_all_proposals())
}

/// List active proposals
pub async fn list_active_proposals(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<Proposal>> {
    Json(state.economics.get_active_proposals())
}

/// Get a specific proposal
pub async fn get_proposal(
    State(state): State<Arc<AppState>>,
    Path(proposal_id): Path<String>,
) -> Json<Option<Proposal>> {
    Json(state.economics.get_proposal(&proposal_id))
}

/// Get vouches for a peer
pub async fn get_vouches_for_peer(
    State(state): State<Arc<AppState>>,
    Path(peer_id): Path<String>,
) -> Json<Vec<Vouch>> {
    Json(state.economics.get_vouches_for_peer(&peer_id))
}

/// Get vouches from a peer
pub async fn get_vouches_from_peer(
    State(state): State<Arc<AppState>>,
    Path(peer_id): Path<String>,
) -> Json<Vec<Vouch>> {
    Json(state.economics.get_vouches_from_peer(&peer_id))
}

/// Get peer reputation
pub async fn get_peer_reputation(
    State(state): State<Arc<AppState>>,
    Path(peer_id): Path<String>,
) -> Json<f64> {
    Json(state.economics.get_reputation(&peer_id))
}

/// Get resource pool
pub async fn get_resource_pool(
    State(state): State<Arc<AppState>>,
) -> Json<ResourcePool> {
    Json(state.economics.get_resource_pool())
}

/// Peer economics details
#[derive(Serialize)]
pub struct PeerEconomics {
    pub peer_id: String,
    pub reputation: f64,
    pub credit_lines: Vec<CreditLine>,
    pub vouches_received: Vec<Vouch>,
    pub vouches_given: Vec<Vouch>,
}

/// Get all economics data for a peer
pub async fn get_peer_economics(
    State(state): State<Arc<AppState>>,
    Path(peer_id): Path<String>,
) -> Json<PeerEconomics> {
    Json(PeerEconomics {
        peer_id: peer_id.clone(),
        reputation: state.economics.get_reputation(&peer_id),
        credit_lines: state.economics.get_credit_lines_for_peer(&peer_id),
        vouches_received: state.economics.get_vouches_for_peer(&peer_id),
        vouches_given: state.economics.get_vouches_from_peer(&peer_id),
    })
}
