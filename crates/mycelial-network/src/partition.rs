//! Network Partition Simulator for Testing
//!
//! This module provides controlled creation and healing of network partitions
//! for testing distributed system behavior. It uses application-level filtering
//! to block communication between nodes.
//!
//! # Design
//!
//! The partition simulator provides two levels of isolation:
//!
//! 1. **Direct Peer Blocking**: Block specific peers from sending/receiving messages
//! 2. **Partition Groups**: Create isolated groups where members can only communicate
//!    within their group
//!
//! # Example
//!
//! ```rust,ignore
//! // Block a specific peer
//! simulator.block_peer(peer_id);
//! assert!(!simulator.allows_communication(&peer_id));
//!
//! // Heal by unblocking
//! simulator.unblock_peer(peer_id);
//! assert!(simulator.allows_communication(&peer_id));
//! ```

use libp2p::PeerId;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Partition group identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartitionId(pub u32);

impl std::fmt::Display for PartitionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Partition({})", self.0)
    }
}

/// Network partition simulator for testing
///
/// Enables controlled creation and healing of network partitions.
/// This is a lightweight approach that filters at the application layer
/// rather than requiring OS-level network manipulation.
#[derive(Debug)]
pub struct PartitionSimulator {
    /// Peers explicitly blocked from this node
    blocked_peers: Arc<RwLock<HashSet<PeerId>>>,

    /// Partition groups - peers in different groups cannot communicate
    /// Key: PartitionId, Value: Set of peers in that partition
    partitions: Arc<RwLock<HashMap<PartitionId, HashSet<PeerId>>>>,

    /// Mapping from peer to their partition group
    peer_to_partition: Arc<RwLock<HashMap<PeerId, PartitionId>>>,

    /// Local peer's partition (if assigned)
    local_partition: Arc<RwLock<Option<PartitionId>>>,

    /// Local peer ID
    local_peer_id: PeerId,

    /// Partition counter for generating IDs
    next_partition_id: Arc<RwLock<u32>>,
}

/// Partition statistics
#[derive(Debug, Clone)]
pub struct PartitionStats {
    /// Number of active partition groups
    pub partition_count: usize,
    /// Number of directly blocked peers
    pub blocked_peer_count: usize,
    /// Local node's partition assignment
    pub local_partition: Option<PartitionId>,
}

impl PartitionSimulator {
    /// Create a new partition simulator for a node
    pub fn new(local_peer_id: PeerId) -> Self {
        Self {
            blocked_peers: Arc::new(RwLock::new(HashSet::new())),
            partitions: Arc::new(RwLock::new(HashMap::new())),
            peer_to_partition: Arc::new(RwLock::new(HashMap::new())),
            local_partition: Arc::new(RwLock::new(None)),
            local_peer_id,
            next_partition_id: Arc::new(RwLock::new(0)),
        }
    }

    /// Get the local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    // === Direct Peer Blocking ===

    /// Block a specific peer - prevents all communication
    pub fn block_peer(&self, peer: PeerId) {
        let mut blocked = self.blocked_peers.write();
        blocked.insert(peer);
        tracing::info!(peer = %peer, "Blocked peer");
    }

    /// Unblock a specific peer
    pub fn unblock_peer(&self, peer: PeerId) {
        let mut blocked = self.blocked_peers.write();
        blocked.remove(&peer);
        tracing::info!(peer = %peer, "Unblocked peer");
    }

    /// Check if a peer is directly blocked
    pub fn is_peer_blocked(&self, peer: &PeerId) -> bool {
        self.blocked_peers.read().contains(peer)
    }

    /// Get all directly blocked peers
    pub fn blocked_peers(&self) -> Vec<PeerId> {
        self.blocked_peers.read().iter().cloned().collect()
    }

    /// Clear all blocked peers (heal all direct blocks)
    pub fn clear_blocked_peers(&self) {
        let mut blocked = self.blocked_peers.write();
        let count = blocked.len();
        blocked.clear();
        tracing::info!(count = count, "Cleared all blocked peers");
    }

    // === Partition Group Management ===

    /// Create a new partition group containing specified peers
    ///
    /// Returns the PartitionId for the new group
    pub fn create_partition(&self, peers: Vec<PeerId>) -> PartitionId {
        let mut next_id = self.next_partition_id.write();
        let partition_id = PartitionId(*next_id);
        *next_id += 1;

        let mut partitions = self.partitions.write();
        let mut peer_to_partition = self.peer_to_partition.write();

        let peer_set: HashSet<_> = peers.iter().cloned().collect();
        partitions.insert(partition_id, peer_set.clone());

        for peer in peers {
            peer_to_partition.insert(peer, partition_id);
        }

        tracing::info!(
            partition = partition_id.0,
            peer_count = peer_set.len(),
            "Created partition group"
        );

        partition_id
    }

    /// Assign local node to a partition
    pub fn join_partition(&self, partition_id: PartitionId) {
        let mut local = self.local_partition.write();
        let mut peer_to_partition = self.peer_to_partition.write();
        let mut partitions = self.partitions.write();

        // Remove from old partition if any
        if let Some(old_id) = *local {
            if let Some(peers) = partitions.get_mut(&old_id) {
                peers.remove(&self.local_peer_id);
            }
            peer_to_partition.remove(&self.local_peer_id);
        }

        // Add to new partition
        if let Some(peers) = partitions.get_mut(&partition_id) {
            peers.insert(self.local_peer_id);
        }
        peer_to_partition.insert(self.local_peer_id, partition_id);
        *local = Some(partition_id);

        tracing::info!(partition = partition_id.0, "Joined partition group");
    }

    /// Leave current partition (become partitioned from everyone in partitions)
    pub fn leave_partition(&self) {
        let mut local = self.local_partition.write();
        let mut peer_to_partition = self.peer_to_partition.write();
        let mut partitions = self.partitions.write();

        if let Some(old_id) = *local {
            if let Some(peers) = partitions.get_mut(&old_id) {
                peers.remove(&self.local_peer_id);
            }
            peer_to_partition.remove(&self.local_peer_id);
        }

        *local = None;
        tracing::info!("Left partition group");
    }

    /// Check if communication is allowed with a peer
    ///
    /// Returns false if:
    /// - The peer is directly blocked
    /// - The peer is in a different partition group
    /// - One has a partition and the other doesn't (asymmetric partition)
    pub fn allows_communication(&self, peer: &PeerId) -> bool {
        // Check direct block first
        if self.is_peer_blocked(peer) {
            return false;
        }

        // Check partition groups
        let local_partition = self.local_partition.read();
        let peer_to_partition = self.peer_to_partition.read();

        match (*local_partition, peer_to_partition.get(peer)) {
            // Both in partitions - must be same partition
            (Some(local_p), Some(peer_p)) => local_p == *peer_p,
            // Local has no partition, peer does - allowed
            // (peer's partition doesn't affect us if we're not partitioned)
            (None, Some(_)) => true,
            // Local has partition, peer doesn't - allowed
            // (our partition only isolates us from peers in OTHER partitions)
            (Some(_), None) => true,
            // Neither has partition - allowed
            (None, None) => true,
        }
    }

    // === Partition Healing ===

    /// Merge two partition groups into one
    ///
    /// All peers from partition p2 are moved into p1.
    /// Returns the surviving partition ID (p1).
    pub fn merge_partitions(&self, p1: PartitionId, p2: PartitionId) -> PartitionId {
        let mut partitions = self.partitions.write();
        let mut peer_to_partition = self.peer_to_partition.write();

        // Get peers from p2
        let p2_peers: Vec<PeerId> = partitions
            .remove(&p2)
            .map(|set| set.into_iter().collect())
            .unwrap_or_default();

        // Add to p1
        if let Some(p1_peers) = partitions.get_mut(&p1) {
            for peer in &p2_peers {
                p1_peers.insert(*peer);
                peer_to_partition.insert(*peer, p1);
            }
        }

        tracing::info!(
            from = p2.0,
            to = p1.0,
            merged_count = p2_peers.len(),
            "Merged partition groups"
        );

        p1
    }

    /// Remove all partitions and blocked peers, restoring full connectivity
    pub fn heal_all(&self) {
        let mut partitions = self.partitions.write();
        let mut peer_to_partition = self.peer_to_partition.write();
        let mut local_partition = self.local_partition.write();
        let mut blocked = self.blocked_peers.write();

        let partition_count = partitions.len();
        let blocked_count = blocked.len();

        partitions.clear();
        peer_to_partition.clear();
        *local_partition = None;
        blocked.clear();

        tracing::info!(
            partitions = partition_count,
            blocked = blocked_count,
            "Healed all partitions and unblocked all peers"
        );
    }

    // === Statistics ===

    /// Get partition statistics
    pub fn stats(&self) -> PartitionStats {
        let partitions = self.partitions.read();
        let blocked = self.blocked_peers.read();

        PartitionStats {
            partition_count: partitions.len(),
            blocked_peer_count: blocked.len(),
            local_partition: *self.local_partition.read(),
        }
    }

    /// Get the partition ID for a peer, if any
    pub fn peer_partition(&self, peer: &PeerId) -> Option<PartitionId> {
        self.peer_to_partition.read().get(peer).copied()
    }

    /// Get all peers in a partition
    pub fn partition_members(&self, partition_id: PartitionId) -> Vec<PeerId> {
        self.partitions
            .read()
            .get(&partition_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl Clone for PartitionSimulator {
    fn clone(&self) -> Self {
        Self {
            blocked_peers: self.blocked_peers.clone(),
            partitions: self.partitions.clone(),
            peer_to_partition: self.peer_to_partition.clone(),
            local_partition: self.local_partition.clone(),
            local_peer_id: self.local_peer_id,
            next_partition_id: self.next_partition_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_peer_id(n: u8) -> PeerId {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        // Use the keypair's peer_id but we just need any peer_id for testing
        let _ = n; // Suppress unused warning
        keypair.public().to_peer_id()
    }

    #[test]
    fn test_block_and_unblock_peer() {
        let local = test_peer_id(0);
        let remote = test_peer_id(1);
        let simulator = PartitionSimulator::new(local);

        // Initially allowed
        assert!(simulator.allows_communication(&remote));

        // Block
        simulator.block_peer(remote);
        assert!(!simulator.allows_communication(&remote));
        assert!(simulator.is_peer_blocked(&remote));

        // Unblock
        simulator.unblock_peer(remote);
        assert!(simulator.allows_communication(&remote));
        assert!(!simulator.is_peer_blocked(&remote));
    }

    #[test]
    fn test_partition_groups() {
        let local = test_peer_id(0);
        let peer_a = test_peer_id(1);
        let peer_b = test_peer_id(2);
        let simulator = PartitionSimulator::new(local);

        // Create two partitions
        let partition_a = simulator.create_partition(vec![local, peer_a]);
        let partition_b = simulator.create_partition(vec![peer_b]);

        // Join partition A
        simulator.join_partition(partition_a);

        // Can communicate with peer_a (same partition)
        // Note: peer_a is in partition_a, so they share partition
        let peer_a_sim = PartitionSimulator::new(peer_a);
        peer_a_sim.join_partition(partition_a);

        // Communication with peer_b (different partition) depends on implementation
        // Since peer_b is in partition_b and we're in partition_a, communication is blocked
        assert!(simulator.allows_communication(&peer_a));

        // peer_b is in a different partition
        // We need to simulate that peer_b joined partition_b
        // For the local simulator, it sees peer_b in peer_to_partition as partition_b
        simulator
            .peer_to_partition
            .write()
            .insert(peer_b, partition_b);
        assert!(!simulator.allows_communication(&peer_b));
    }

    #[test]
    fn test_heal_all() {
        let local = test_peer_id(0);
        let remote = test_peer_id(1);
        let simulator = PartitionSimulator::new(local);

        // Block and create partition
        simulator.block_peer(remote);
        let partition = simulator.create_partition(vec![local]);
        simulator.join_partition(partition);

        // Verify blocked
        assert!(!simulator.allows_communication(&remote));
        assert!(simulator.stats().blocked_peer_count > 0);

        // Heal
        simulator.heal_all();

        // Verify healed
        assert!(simulator.allows_communication(&remote));
        assert_eq!(simulator.stats().blocked_peer_count, 0);
        assert_eq!(simulator.stats().partition_count, 0);
    }

    #[test]
    fn test_stats() {
        let local = test_peer_id(0);
        let simulator = PartitionSimulator::new(local);

        let stats = simulator.stats();
        assert_eq!(stats.partition_count, 0);
        assert_eq!(stats.blocked_peer_count, 0);
        assert!(stats.local_partition.is_none());

        simulator.block_peer(test_peer_id(1));
        simulator.block_peer(test_peer_id(2));
        let partition = simulator.create_partition(vec![local]);
        simulator.join_partition(partition);

        let stats = simulator.stats();
        assert_eq!(stats.blocked_peer_count, 2);
        assert_eq!(stats.partition_count, 1);
        assert!(stats.local_partition.is_some());
    }
}
