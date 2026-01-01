//! State synchronization across the network
//!
//! This module provides mechanisms for synchronizing state between peers
//! using gossipsub messaging with last-write-wins semantics for simple
//! fields and grow-only counters for reputation.

use chrono::{DateTime, Utc};
use mycelial_core::{
    credit::CreditRelationship,
    peer::{PeerId, PeerInfo},
    reputation::Reputation,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

use crate::cache::StateCache;
use crate::error::{Result, StateError};
use crate::storage::SqliteStore;

/// State update types that can be synced across the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateUpdate {
    /// Peer information update
    PeerUpdate {
        peer_id: String,
        info: PeerInfoUpdate,
        timestamp: DateTime<Utc>,
    },
    /// Reputation update (grow-only counters)
    ReputationUpdate {
        peer_id: String,
        successful_interactions: u64,
        failed_interactions: u64,
        timestamp: DateTime<Utc>,
    },
    /// Credit relationship update
    CreditUpdate {
        creditor: String,
        debtor: String,
        credit_limit: f64,
        balance: f64,
        active: bool,
        timestamp: DateTime<Utc>,
    },
    /// Generic key-value update
    KeyValueUpdate {
        key: String,
        value: Vec<u8>,
        version: u64,
        timestamp: DateTime<Utc>,
    },
}

/// Peer information that can be synced
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfoUpdate {
    /// Public key (base58 encoded string)
    pub public_key: String,
    pub addresses: Vec<String>,
    pub name: Option<String>,
}

/// Vector clock for tracking causality
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorClock {
    clocks: HashMap<String, u64>,
}

impl VectorClock {
    /// Create a new vector clock
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the clock for a peer
    pub fn increment(&mut self, peer_id: &str) {
        let counter = self.clocks.entry(peer_id.to_string()).or_insert(0);
        *counter += 1;
    }

    /// Get the clock value for a peer
    pub fn get(&self, peer_id: &str) -> u64 {
        self.clocks.get(peer_id).copied().unwrap_or(0)
    }

    /// Merge with another vector clock (take max of each)
    pub fn merge(&mut self, other: &VectorClock) {
        for (peer_id, &value) in &other.clocks {
            let current = self.clocks.entry(peer_id.clone()).or_insert(0);
            *current = (*current).max(value);
        }
    }

    /// Check if this clock is concurrent with another
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }

    /// Check if this clock happens-before another
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut dominated = false;
        for (peer_id, &value) in &self.clocks {
            let other_value = other.get(peer_id);
            if value > other_value {
                return false;
            }
            if value < other_value {
                dominated = true;
            }
        }
        for (peer_id, &value) in &other.clocks {
            if self.get(peer_id) < value {
                dominated = true;
            }
        }
        dominated
    }
}

/// State synchronization manager
pub struct StateSync {
    /// Local peer ID
    local_peer_id: String,
    /// Vector clock for this peer
    clock: RwLock<VectorClock>,
    /// Last seen timestamps for updates (for LWW)
    last_seen: RwLock<HashMap<String, DateTime<Utc>>>,
    /// Pending updates to be sent
    pending_updates: RwLock<Vec<StateUpdate>>,
    /// Cache reference for quick lookups
    cache: Arc<StateCache>,
}

impl StateSync {
    /// Create a new state sync manager
    pub fn new(local_peer_id: String, cache: Arc<StateCache>) -> Self {
        Self {
            local_peer_id,
            clock: RwLock::new(VectorClock::new()),
            last_seen: RwLock::new(HashMap::new()),
            pending_updates: RwLock::new(Vec::new()),
            cache,
        }
    }

    /// Create a peer update
    pub fn create_peer_update(&self, peer_info: &PeerInfo) -> StateUpdate {
        self.clock.write().increment(&self.local_peer_id);

        StateUpdate::PeerUpdate {
            peer_id: peer_info.id.as_str().to_string(),
            info: PeerInfoUpdate {
                public_key: peer_info.public_key.clone(),
                addresses: peer_info.addresses.clone(),
                name: peer_info.name.clone(),
            },
            timestamp: Utc::now(),
        }
    }

    /// Create a reputation update (grow-only counters)
    pub fn create_reputation_update(&self, peer_id: &str, reputation: &Reputation) -> StateUpdate {
        self.clock.write().increment(&self.local_peer_id);

        StateUpdate::ReputationUpdate {
            peer_id: peer_id.to_string(),
            successful_interactions: reputation.successful_interactions,
            failed_interactions: reputation.failed_interactions,
            timestamp: Utc::now(),
        }
    }

    /// Create a credit update
    pub fn create_credit_update(&self, relationship: &CreditRelationship) -> StateUpdate {
        self.clock.write().increment(&self.local_peer_id);

        StateUpdate::CreditUpdate {
            creditor: relationship.creditor.as_str().to_string(),
            debtor: relationship.debtor.as_str().to_string(),
            credit_limit: relationship.credit_limit,
            balance: relationship.balance,
            active: relationship.active,
            timestamp: Utc::now(),
        }
    }

    /// Create a key-value update
    pub fn create_kv_update(&self, key: &str, value: Vec<u8>, version: u64) -> StateUpdate {
        self.clock.write().increment(&self.local_peer_id);

        StateUpdate::KeyValueUpdate {
            key: key.to_string(),
            value,
            version,
            timestamp: Utc::now(),
        }
    }

    /// Apply an update received from the network
    pub async fn apply_update(&self, update: &StateUpdate, store: &SqliteStore) -> Result<bool> {
        match update {
            StateUpdate::PeerUpdate {
                peer_id,
                info,
                timestamp,
            } => {
                self.apply_peer_update(peer_id, info, timestamp, store)
                    .await
            }
            StateUpdate::ReputationUpdate {
                peer_id,
                successful_interactions,
                failed_interactions,
                timestamp,
            } => {
                self.apply_reputation_update(
                    peer_id,
                    *successful_interactions,
                    *failed_interactions,
                    timestamp,
                    store,
                )
                .await
            }
            StateUpdate::CreditUpdate {
                creditor,
                debtor,
                credit_limit,
                balance,
                active,
                timestamp,
            } => {
                self.apply_credit_update(
                    creditor,
                    debtor,
                    *credit_limit,
                    *balance,
                    *active,
                    timestamp,
                    store,
                )
                .await
            }
            StateUpdate::KeyValueUpdate {
                key,
                value,
                version,
                timestamp,
            } => {
                self.apply_kv_update(key, value, *version, timestamp, store)
                    .await
            }
        }
    }

    /// Apply a peer update using last-write-wins
    async fn apply_peer_update(
        &self,
        peer_id: &str,
        info: &PeerInfoUpdate,
        timestamp: &DateTime<Utc>,
        store: &SqliteStore,
    ) -> Result<bool> {
        let update_key = format!("peer:{}", peer_id);

        // Check if we have a newer update
        {
            let last_seen = self.last_seen.read();
            if let Some(last_ts) = last_seen.get(&update_key) {
                if last_ts >= timestamp {
                    debug!("Skipping stale peer update for {}", peer_id);
                    return Ok(false);
                }
            }
        }

        // Get existing peer or create new one
        let peer_info = match store.get_peer(peer_id).await? {
            Some((mut existing, _)) => {
                // Update existing peer
                existing.public_key = info.public_key.clone();
                existing.addresses = info.addresses.clone();
                existing.name = info.name.clone();
                existing.last_seen = Utc::now();
                existing
            }
            None => {
                // Create new peer
                PeerInfo {
                    id: PeerId(peer_id.to_string()),
                    public_key: info.public_key.clone(),
                    addresses: info.addresses.clone(),
                    first_seen: Utc::now(),
                    last_seen: Utc::now(),
                    name: info.name.clone(),
                }
            }
        };

        store.upsert_peer(&peer_info, None).await?;

        // Update last seen timestamp
        self.last_seen.write().insert(update_key, *timestamp);

        // Update cache
        let reputation = Reputation::default();
        self.cache.peers.insert(peer_info, reputation);

        debug!("Applied peer update for {}", peer_id);
        Ok(true)
    }

    /// Apply a reputation update using grow-only counters (max merge)
    async fn apply_reputation_update(
        &self,
        peer_id: &str,
        successful: u64,
        failed: u64,
        _timestamp: &DateTime<Utc>,
        store: &SqliteStore,
    ) -> Result<bool> {
        // Get existing reputation
        let (peer_info, mut reputation) = match store.get_peer(peer_id).await? {
            Some((info, rep)) => (info, rep),
            None => {
                debug!("Skipping reputation update for unknown peer {}", peer_id);
                return Ok(false);
            }
        };

        // Grow-only counter merge: take the max
        let updated = successful > reputation.successful_interactions
            || failed > reputation.failed_interactions;

        if updated {
            reputation.successful_interactions = reputation.successful_interactions.max(successful);
            reputation.failed_interactions = reputation.failed_interactions.max(failed);

            // Recalculate score
            let total = reputation.successful_interactions + reputation.failed_interactions;
            if total > 0 {
                reputation.score = reputation.successful_interactions as f64 / total as f64;
            }

            store.update_peer_reputation(peer_id, &reputation).await?;

            // Update cache
            self.cache.peers.insert(peer_info, reputation);

            debug!("Applied reputation update for {}", peer_id);
        }

        Ok(updated)
    }

    /// Apply a credit update using last-write-wins
    async fn apply_credit_update(
        &self,
        creditor: &str,
        debtor: &str,
        credit_limit: f64,
        balance: f64,
        active: bool,
        timestamp: &DateTime<Utc>,
        store: &SqliteStore,
    ) -> Result<bool> {
        let update_key = format!("credit:{}:{}", creditor, debtor);

        // Check if we have a newer update
        {
            let last_seen = self.last_seen.read();
            if let Some(last_ts) = last_seen.get(&update_key) {
                if last_ts >= timestamp {
                    debug!("Skipping stale credit update for {}:{}", creditor, debtor);
                    return Ok(false);
                }
            }
        }

        let relationship = CreditRelationship {
            creditor: PeerId(creditor.to_string()),
            debtor: PeerId(debtor.to_string()),
            credit_limit,
            balance,
            active,
            established: *timestamp,
            last_transaction: *timestamp,
        };

        store.upsert_credit_relationship(&relationship).await?;

        // Update last seen timestamp
        self.last_seen.write().insert(update_key, *timestamp);

        // Update cache
        self.cache.credits.insert(relationship);

        debug!("Applied credit update for {}:{}", creditor, debtor);
        Ok(true)
    }

    /// Apply a key-value update using version numbers
    async fn apply_kv_update(
        &self,
        key: &str,
        value: &[u8],
        version: u64,
        _timestamp: &DateTime<Utc>,
        store: &SqliteStore,
    ) -> Result<bool> {
        // Get existing version
        if let Some((_, existing_version)) = store.get_sync_value(key).await? {
            if existing_version as u64 >= version {
                debug!("Skipping stale key-value update for {}", key);
                return Ok(false);
            }
        }

        store.set_sync_value(key, value).await?;

        debug!("Applied key-value update for {}", key);
        Ok(true)
    }

    /// Queue an update to be sent
    pub fn queue_update(&self, update: StateUpdate) {
        self.pending_updates.write().push(update);
    }

    /// Get and clear pending updates
    pub fn drain_pending_updates(&self) -> Vec<StateUpdate> {
        let mut pending = self.pending_updates.write();
        std::mem::take(&mut *pending)
    }

    /// Serialize an update for network transmission
    pub fn serialize_update(update: &StateUpdate) -> Result<Vec<u8>> {
        serde_json::to_vec(update).map_err(|e| StateError::Serialization(e.to_string()))
    }

    /// Deserialize an update from network data
    pub fn deserialize_update(data: &[u8]) -> Result<StateUpdate> {
        serde_json::from_slice(data).map_err(|e| StateError::Deserialization(e.to_string()))
    }

    /// Get the current vector clock
    pub fn get_clock(&self) -> VectorClock {
        self.clock.read().clone()
    }

    /// Merge a remote vector clock
    pub fn merge_clock(&self, remote: &VectorClock) {
        self.clock.write().merge(remote);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_clock() {
        let mut clock1 = VectorClock::new();
        let mut clock2 = VectorClock::new();

        clock1.increment("peer1");
        clock1.increment("peer1");
        clock2.increment("peer2");

        assert_eq!(clock1.get("peer1"), 2);
        assert_eq!(clock1.get("peer2"), 0);

        assert!(clock1.is_concurrent(&clock2));

        clock1.merge(&clock2);
        assert_eq!(clock1.get("peer2"), 1);
    }

    #[test]
    fn test_state_update_serialization() {
        let update = StateUpdate::PeerUpdate {
            peer_id: "test_peer".to_string(),
            info: PeerInfoUpdate {
                public_key: "3mJr7AoUXx2Wqd5s8N4Df".to_string(), // base58 encoded
                addresses: vec!["/ip4/127.0.0.1/tcp/4001".to_string()],
                name: Some("Test".to_string()),
            },
            timestamp: Utc::now(),
        };

        let serialized = StateSync::serialize_update(&update).unwrap();
        let deserialized = StateSync::deserialize_update(&serialized).unwrap();

        match deserialized {
            StateUpdate::PeerUpdate { peer_id, info, .. } => {
                assert_eq!(peer_id, "test_peer");
                assert_eq!(info.name, Some("Test".to_string()));
            }
            _ => panic!("Wrong update type"),
        }
    }

    #[tokio::test]
    async fn test_state_sync_create_updates() {
        let cache = Arc::new(StateCache::new());
        let sync = StateSync::new("local_peer".to_string(), cache);

        // Create peer update
        let peer_info = PeerInfo {
            id: PeerId("test_peer".to_string()),
            public_key: "3mJr7AoUXx2Wqd5s8N4Df".to_string(), // base58 encoded
            addresses: vec![],
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            name: None,
        };

        let update = sync.create_peer_update(&peer_info);
        match update {
            StateUpdate::PeerUpdate { peer_id, .. } => {
                assert_eq!(peer_id, "test_peer");
            }
            _ => panic!("Wrong update type"),
        }

        // Check vector clock was incremented
        assert_eq!(sync.get_clock().get("local_peer"), 1);
    }
}
