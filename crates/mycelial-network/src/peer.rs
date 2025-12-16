//! Peer management and tracking
//!
//! This module provides peer tracking, connection state management,
//! and peer scoring.

use chrono::{DateTime, Utc};
use libp2p::{Multiaddr, PeerId};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Information about a connected peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer ID
    pub peer_id: String,
    /// Known addresses
    pub addresses: Vec<String>,
    /// Connection state
    pub state: ConnectionState,
    /// When first seen
    pub first_seen: DateTime<Utc>,
    /// When last seen
    pub last_seen: DateTime<Utc>,
    /// Agent version string
    pub agent_version: Option<String>,
    /// Protocol version
    pub protocol_version: Option<String>,
    /// Supported protocols
    pub protocols: Vec<String>,
    /// Connection score (reputation)
    pub score: f64,
    /// Number of successful interactions
    pub successful_interactions: u64,
    /// Number of failed interactions
    pub failed_interactions: u64,
}

impl PeerInfo {
    /// Create new peer info
    pub fn new(peer_id: PeerId) -> Self {
        let now = Utc::now();
        Self {
            peer_id: peer_id.to_string(),
            addresses: Vec::new(),
            state: ConnectionState::Disconnected,
            first_seen: now,
            last_seen: now,
            agent_version: None,
            protocol_version: None,
            protocols: Vec::new(),
            score: 0.5, // Neutral starting score
            successful_interactions: 0,
            failed_interactions: 0,
        }
    }

    /// Update last seen timestamp
    pub fn touch(&mut self) {
        self.last_seen = Utc::now();
    }

    /// Add an address for this peer
    pub fn add_address(&mut self, addr: Multiaddr) {
        let addr_str = addr.to_string();
        if !self.addresses.contains(&addr_str) {
            self.addresses.push(addr_str);
        }
    }

    /// Record a successful interaction
    pub fn record_success(&mut self) {
        self.successful_interactions += 1;
        self.update_score();
        self.touch();
    }

    /// Record a failed interaction
    pub fn record_failure(&mut self) {
        self.failed_interactions += 1;
        self.update_score();
        self.touch();
    }

    /// Update the peer score based on interactions
    fn update_score(&mut self) {
        let total = self.successful_interactions + self.failed_interactions;
        if total > 0 {
            // Simple ratio with decay towards neutral
            let ratio = self.successful_interactions as f64 / total as f64;
            // Weighted average with neutral
            self.score = 0.3 * 0.5 + 0.7 * ratio;
        }
    }

    /// Check if peer is trusted (score above threshold)
    pub fn is_trusted(&self, threshold: f64) -> bool {
        self.score >= threshold
    }

    /// Time since last seen
    pub fn time_since_seen(&self) -> chrono::Duration {
        Utc::now().signed_duration_since(self.last_seen)
    }
}

/// Connection state for a peer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connection in progress
    Connecting,
    /// Connected
    Connected,
    /// Connection failed
    Failed,
    /// Banned
    Banned,
}

/// Manages known peers and their state
pub struct PeerManager {
    /// Known peers
    peers: RwLock<HashMap<PeerId, PeerInfo>>,
    /// Maximum number of peers to track
    max_peers: usize,
    /// Trust threshold for considering a peer trusted
    trust_threshold: f64,
}

impl PeerManager {
    /// Create a new peer manager
    pub fn new(max_peers: usize, trust_threshold: f64) -> Self {
        Self {
            peers: RwLock::new(HashMap::new()),
            max_peers,
            trust_threshold,
        }
    }

    /// Get or create peer info
    pub fn get_or_create(&self, peer_id: PeerId) -> PeerInfo {
        let mut peers = self.peers.write();
        peers.entry(peer_id)
            .or_insert_with(|| PeerInfo::new(peer_id))
            .clone()
    }

    /// Update peer info
    pub fn update<F>(&self, peer_id: PeerId, f: F)
    where
        F: FnOnce(&mut PeerInfo),
    {
        let mut peers = self.peers.write();
        if let Some(info) = peers.get_mut(&peer_id) {
            f(info);
        } else {
            let mut info = PeerInfo::new(peer_id);
            f(&mut info);
            peers.insert(peer_id, info);
        }
    }

    /// Set peer connection state
    pub fn set_state(&self, peer_id: PeerId, state: ConnectionState) {
        self.update(peer_id, |info| {
            info.state = state;
            info.touch();
        });
    }

    /// Add an address for a peer
    pub fn add_address(&self, peer_id: PeerId, addr: Multiaddr) {
        self.update(peer_id, |info| {
            info.add_address(addr);
        });
    }

    /// Set peer identification info
    pub fn set_identify_info(
        &self,
        peer_id: PeerId,
        agent_version: String,
        protocol_version: String,
        protocols: Vec<String>,
    ) {
        self.update(peer_id, |info| {
            info.agent_version = Some(agent_version);
            info.protocol_version = Some(protocol_version);
            info.protocols = protocols;
            info.touch();
        });
    }

    /// Record a successful interaction
    pub fn record_success(&self, peer_id: PeerId) {
        self.update(peer_id, |info| info.record_success());
    }

    /// Record a failed interaction
    pub fn record_failure(&self, peer_id: PeerId) {
        self.update(peer_id, |info| info.record_failure());
    }

    /// Get peer info
    pub fn get(&self, peer_id: &PeerId) -> Option<PeerInfo> {
        self.peers.read().get(peer_id).cloned()
    }

    /// Remove a peer
    pub fn remove(&self, peer_id: &PeerId) -> Option<PeerInfo> {
        self.peers.write().remove(peer_id)
    }

    /// Get all connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.peers
            .read()
            .iter()
            .filter(|(_, info)| info.state == ConnectionState::Connected)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get all peers
    pub fn all_peers(&self) -> Vec<PeerInfo> {
        self.peers.read().values().cloned().collect()
    }

    /// Get trusted peers
    pub fn trusted_peers(&self) -> Vec<PeerId> {
        self.peers
            .read()
            .iter()
            .filter(|(_, info)| info.is_trusted(self.trust_threshold))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Count connected peers
    pub fn connected_count(&self) -> usize {
        self.peers
            .read()
            .values()
            .filter(|info| info.state == ConnectionState::Connected)
            .count()
    }

    /// Count all known peers
    pub fn total_count(&self) -> usize {
        self.peers.read().len()
    }

    /// Ban a peer
    pub fn ban(&self, peer_id: PeerId) {
        self.update(peer_id, |info| {
            info.state = ConnectionState::Banned;
            info.score = 0.0;
        });
    }

    /// Check if a peer is banned
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        self.peers
            .read()
            .get(peer_id)
            .map(|info| info.state == ConnectionState::Banned)
            .unwrap_or(false)
    }

    /// Prune stale peers
    pub fn prune_stale(&self, max_age: Duration) {
        let mut peers = self.peers.write();
        let cutoff = Utc::now() - chrono::Duration::from_std(max_age).unwrap_or_default();

        peers.retain(|_, info| {
            // Keep connected peers and recently seen peers
            info.state == ConnectionState::Connected || info.last_seen > cutoff
        });

        // If still over limit, remove lowest scored peers
        while peers.len() > self.max_peers {
            if let Some((&peer_id, _)) = peers
                .iter()
                .filter(|(_, info)| info.state != ConnectionState::Connected)
                .min_by(|a, b| a.1.score.partial_cmp(&b.1.score).unwrap())
            {
                peers.remove(&peer_id);
            } else {
                break;
            }
        }
    }
}

impl Default for PeerManager {
    fn default() -> Self {
        Self::new(1000, 0.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;

    fn random_peer_id() -> PeerId {
        Keypair::generate_ed25519().public().to_peer_id()
    }

    #[test]
    fn test_peer_info_scoring() {
        let peer_id = random_peer_id();
        let mut info = PeerInfo::new(peer_id);

        assert_eq!(info.score, 0.5); // Initial neutral score

        // Record successes
        for _ in 0..10 {
            info.record_success();
        }
        assert!(info.score > 0.5);

        // Record failures
        for _ in 0..10 {
            info.record_failure();
        }
        // Score should decrease but stay above 0
        assert!(info.score > 0.0);
    }

    #[test]
    fn test_peer_manager() {
        let manager = PeerManager::new(100, 0.4);
        let peer_id = random_peer_id();

        // Get or create
        let info = manager.get_or_create(peer_id);
        assert_eq!(info.state, ConnectionState::Disconnected);

        // Update state
        manager.set_state(peer_id, ConnectionState::Connected);
        let info = manager.get(&peer_id).unwrap();
        assert_eq!(info.state, ConnectionState::Connected);

        // Connected count
        assert_eq!(manager.connected_count(), 1);

        // Ban
        manager.ban(peer_id);
        assert!(manager.is_banned(&peer_id));
    }
}
