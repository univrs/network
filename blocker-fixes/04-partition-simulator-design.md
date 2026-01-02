# Network Partition Simulator Design

**Status**: Design Document (Implementation Pending)
**Date**: 2026-01-01
**Author**: Systems Architect

## Executive Summary

This document outlines the design for a network partition simulator that enables controlled creation and healing of network partitions for testing distributed system behavior. The simulator leverages the existing Septal Gate infrastructure while adding explicit partition control.

## Background

### Existing Infrastructure

The `univrs-network` codebase already includes sophisticated node isolation mechanisms:

1. **Septal Gates** (`/crates/mycelial-network/src/enr_bridge/septal.rs`)
   - Circuit breaker pattern with Open/HalfOpen/Closed states
   - Automatic isolation based on failure thresholds
   - Recovery mechanism with health probes

2. **Woronin Manager** (from `univrs-enr`)
   - Transaction blocking for isolated nodes
   - `should_block(from, to)` and `is_isolated(node)` checks

3. **NetworkHandle** (`/crates/mycelial-network/src/service.rs`)
   - `disconnect(peer_id)` command exists
   - Routes to `swarm.disconnect_peer_id()`

4. **TestCluster** (`/crates/mycelial-network/tests/helpers/cluster.rs`)
   - Multi-node test environment spawning
   - Mesh formation waiting utilities

### Gap Analysis

The current system lacks:
- **Explicit peer blocking** - preventing reconnection after disconnect
- **Partition group management** - isolating sets of nodes from each other
- **Controlled healing** - reestablishing connectivity on demand
- **Message filtering** - blocking gossipsub messages between partitioned peers

## Recommended Approach: Application-Level Filtering

### Justification

| Approach | Pros | Cons |
|----------|------|------|
| **Application-Level (RECOMMENDED)** | Portable, no root needed, integrates with existing Septal pattern, testable | Slightly higher overhead than transport-level |
| libp2p Swarm-Level | Uses native libp2p APIs | Reconnection prevention is complex, no message filtering |
| Proxy-Based | Most realistic network simulation | Significant complexity, external dependencies |
| iptables/OS-Level | True network isolation | Requires root, not portable, complex test setup |

**Decision**: Application-level filtering is recommended because it:
1. Aligns with the existing Septal Gate/Woronin architecture
2. Requires no platform-specific code
3. Can be easily controlled via test APIs
4. Filters both connections AND messages

## Detailed Design

### 1. Core Data Structures

```rust
// File: crates/mycelial-network/src/partition.rs

use libp2p::PeerId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;

/// Partition group identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartitionId(u32);

/// Network partition simulator for testing
///
/// Enables controlled creation and healing of network partitions.
/// Integrates with existing Septal Gate infrastructure.
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

/// Partition configuration
#[derive(Debug, Clone)]
pub struct PartitionConfig {
    /// Whether to also disconnect existing connections when blocking
    pub disconnect_on_block: bool,

    /// Whether to log partition events
    pub enable_logging: bool,

    /// Whether to broadcast partition state to other nodes (for coordinated partitions)
    pub broadcast_state: bool,
}

impl Default for PartitionConfig {
    fn default() -> Self {
        Self {
            disconnect_on_block: true,
            enable_logging: true,
            broadcast_state: false,
        }
    }
}
```

### 2. PartitionSimulator Implementation

```rust
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

    /// Check if a peer is blocked
    pub fn is_peer_blocked(&self, peer: &PeerId) -> bool {
        self.blocked_peers.read().contains(peer)
    }

    /// Get all blocked peers
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

    /// Leave current partition (become partitioned from everyone)
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

    /// Check if communication is allowed between two peers based on partitions
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
            // Local has no partition, peer does - not allowed
            (None, Some(_)) => false,
            // Local has partition, peer doesn't - not allowed
            (Some(_), None) => false,
            // Neither has partition - allowed
            (None, None) => true,
        }
    }

    // === Partition Healing ===

    /// Heal a partition by merging two partition groups
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

    /// Remove all partitions and restore full connectivity
    pub fn heal_all_partitions(&self) {
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
}

/// Partition statistics
#[derive(Debug, Clone)]
pub struct PartitionStats {
    pub partition_count: usize,
    pub blocked_peer_count: usize,
    pub local_partition: Option<PartitionId>,
}
```

### 3. Integration with NetworkService

The `PartitionSimulator` needs to be integrated at two points:

#### 3.1 Connection Filtering

```rust
// In NetworkService (service.rs)

pub struct NetworkService {
    // ... existing fields ...

    /// Partition simulator for testing (optional)
    #[cfg(any(test, feature = "partition-testing"))]
    partition_simulator: Option<Arc<PartitionSimulator>>,
}

impl NetworkService {
    // In handle_swarm_event, before accepting connections:
    async fn handle_swarm_event(&mut self, event: SwarmEvent<MycelialBehaviourEvent>) {
        match event {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                // Check partition filter before accepting
                #[cfg(any(test, feature = "partition-testing"))]
                if let Some(ref partition) = self.partition_simulator {
                    if !partition.allows_communication(&peer_id) {
                        tracing::debug!(
                            peer = %peer_id,
                            "Disconnecting peer due to partition"
                        );
                        let _ = self.swarm.disconnect_peer_id(peer_id);
                        return;
                    }
                }

                // ... existing connection handling ...
            }
            // ...
        }
    }
}
```

#### 3.2 Message Filtering

```rust
// In handle_behaviour_event:
async fn handle_behaviour_event(&mut self, event: MycelialBehaviourEvent) {
    match event {
        MycelialBehaviourEvent::Gossipsub(gossipsub::Event::Message {
            propagation_source,
            message_id,
            message,
        }) => {
            // Filter messages from partitioned peers
            #[cfg(any(test, feature = "partition-testing"))]
            if let Some(ref partition) = self.partition_simulator {
                if let Some(source) = &message.source {
                    if !partition.allows_communication(source) {
                        tracing::debug!(
                            source = %source,
                            topic = %message.topic,
                            "Dropping message from partitioned peer"
                        );
                        return;
                    }
                }
            }

            // ... existing message handling ...
        }
        // ...
    }
}
```

### 4. TestCluster Integration

```rust
// In tests/helpers/cluster.rs

impl TestCluster {
    /// Create a network partition between two groups of nodes
    ///
    /// After calling this, nodes in group A cannot communicate with nodes in group B.
    ///
    /// # Example
    /// ```rust,ignore
    /// // Partition nodes [0,1] from nodes [2,3,4]
    /// cluster.create_partition(vec![0, 1], vec![2, 3, 4]).await;
    /// ```
    pub async fn create_partition(
        &self,
        group_a: Vec<usize>,
        group_b: Vec<usize>,
    ) -> Result<(PartitionId, PartitionId), Box<dyn std::error::Error + Send + Sync>> {
        // Create partition A
        let peers_a: Vec<PeerId> = group_a
            .iter()
            .map(|&i| self.nodes[i].handle.local_peer_id())
            .collect();

        // Create partition B
        let peers_b: Vec<PeerId> = group_b
            .iter()
            .map(|&i| self.nodes[i].handle.local_peer_id())
            .collect();

        // Create partition groups
        for &idx in &group_a {
            let partition_id = self.nodes[idx]
                .partition_simulator()
                .create_partition(peers_a.clone());
            self.nodes[idx].partition_simulator().join_partition(partition_id);
        }

        for &idx in &group_b {
            let partition_id = self.nodes[idx]
                .partition_simulator()
                .create_partition(peers_b.clone());
            self.nodes[idx].partition_simulator().join_partition(partition_id);
        }

        // Also block cross-group peers explicitly for immediate effect
        for &idx_a in &group_a {
            for &idx_b in &group_b {
                let peer_b = self.nodes[idx_b].handle.local_peer_id();
                self.nodes[idx_a].partition_simulator().block_peer(peer_b);

                let peer_a = self.nodes[idx_a].handle.local_peer_id();
                self.nodes[idx_b].partition_simulator().block_peer(peer_a);
            }
        }

        // Disconnect existing connections
        for &idx_a in &group_a {
            for &idx_b in &group_b {
                let peer_b = self.nodes[idx_b].handle.local_peer_id();
                let _ = self.nodes[idx_a].handle.disconnect(peer_b).await;

                let peer_a = self.nodes[idx_a].handle.local_peer_id();
                let _ = self.nodes[idx_b].handle.disconnect(peer_a).await;
            }
        }

        Ok((PartitionId(0), PartitionId(1)))
    }

    /// Heal a partition between two groups
    pub async fn heal_partition(
        &self,
        group_a: Vec<usize>,
        group_b: Vec<usize>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Unblock all cross-group peers
        for &idx_a in &group_a {
            for &idx_b in &group_b {
                let peer_b = self.nodes[idx_b].handle.local_peer_id();
                self.nodes[idx_a].partition_simulator().unblock_peer(peer_b);

                let peer_a = self.nodes[idx_a].handle.local_peer_id();
                self.nodes[idx_b].partition_simulator().unblock_peer(peer_a);
            }
        }

        // Clear partition groups
        for node in &self.nodes {
            node.partition_simulator().heal_all_partitions();
        }

        // Trigger reconnection by dialing bootstrap peers
        for &idx in &group_a {
            for &idx_b in &group_b.iter().take(1) {
                let addr = &self.nodes[*idx_b].listen_addr;
                let _ = self.nodes[idx].handle.dial(addr.parse()?).await;
            }
        }

        Ok(())
    }

    /// Isolate a single node from all others
    pub async fn isolate_node(&self, node_idx: usize) {
        let isolated_peer = self.nodes[node_idx].handle.local_peer_id();

        // Block from perspective of isolated node
        for (i, node) in self.nodes.iter().enumerate() {
            if i != node_idx {
                let peer = node.handle.local_peer_id();
                self.nodes[node_idx].partition_simulator().block_peer(peer);

                // Block from perspective of other nodes
                node.partition_simulator().block_peer(isolated_peer);

                // Disconnect
                let _ = node.handle.disconnect(isolated_peer).await;
            }
        }
    }

    /// Rejoin an isolated node to the cluster
    pub async fn rejoin_node(&self, node_idx: usize) {
        let rejoining_peer = self.nodes[node_idx].handle.local_peer_id();

        // Unblock from all perspectives
        for (i, node) in self.nodes.iter().enumerate() {
            if i != node_idx {
                let peer = node.handle.local_peer_id();
                self.nodes[node_idx].partition_simulator().unblock_peer(peer);
                node.partition_simulator().unblock_peer(rejoining_peer);
            }
        }

        // Trigger reconnection to first node
        if node_idx > 0 {
            let addr = &self.nodes[0].listen_addr;
            let _ = self.nodes[node_idx].handle.dial(addr.parse().unwrap()).await;
        }
    }
}
```

### 5. Example API Usage

```rust
#[tokio::test]
async fn test_partition_recovery() {
    let cluster = TestCluster::spawn(5).await.expect("Failed to spawn cluster");
    cluster.wait_for_mesh(1, 10).await.unwrap();

    // === Create a partition ===
    // Split: [0,1,2] | [3,4]
    cluster.create_partition(vec![0, 1, 2], vec![3, 4]).await.unwrap();

    // Verify partition
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Nodes 0-2 should only see each other
    let peers_0 = cluster.nodes[0].handle.get_peers().await.unwrap();
    assert!(peers_0.iter().all(|p| {
        *p == cluster.nodes[1].handle.local_peer_id() ||
        *p == cluster.nodes[2].handle.local_peer_id()
    }));

    // Nodes 3-4 should only see each other
    let peers_3 = cluster.nodes[3].handle.get_peers().await.unwrap();
    assert!(peers_3.iter().all(|p| {
        *p == cluster.nodes[4].handle.local_peer_id()
    }));

    // === Heal the partition ===
    cluster.heal_partition(vec![0, 1, 2], vec![3, 4]).await.unwrap();

    // Wait for reconnection
    cluster.wait_for_mesh(1, 10).await.unwrap();

    // Verify all nodes can see each other again
    let peers_0 = cluster.nodes[0].handle.get_peers().await.unwrap();
    assert!(peers_0.len() >= 1); // At least connected to some peers
}

#[tokio::test]
async fn test_node_isolation_and_rejoin() {
    let cluster = TestCluster::spawn(3).await.expect("Failed to spawn cluster");
    cluster.wait_for_mesh(1, 10).await.unwrap();

    // Isolate node 2
    cluster.isolate_node(2).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Node 2 should have no peers
    let peers_2 = cluster.nodes[2].handle.get_peers().await.unwrap();
    assert!(peers_2.is_empty());

    // Nodes 0,1 should still see each other but not node 2
    let peers_0 = cluster.nodes[0].handle.get_peers().await.unwrap();
    assert!(!peers_0.contains(&cluster.nodes[2].handle.local_peer_id()));

    // Rejoin node 2
    cluster.rejoin_node(2).await;
    cluster.wait_for_mesh(1, 10).await.unwrap();

    // Node 2 should have peers again
    let peers_2 = cluster.nodes[2].handle.get_peers().await.unwrap();
    assert!(!peers_2.is_empty());
}

#[tokio::test]
async fn test_message_delivery_during_partition() {
    let cluster = TestCluster::spawn(4).await.expect("Failed to spawn cluster");
    cluster.wait_for_mesh(1, 10).await.unwrap();

    // Partition [0,1] from [2,3]
    cluster.create_partition(vec![0, 1], vec![2, 3]).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Publish message from node 0
    let msg = b"test message during partition".to_vec();
    cluster.nodes[0].handle.publish("/mycelial/1.0.0/test", msg.clone()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Node 1 (same partition) should receive it
    // Node 2,3 (different partition) should NOT receive it

    // (Actual verification depends on event subscription implementation)

    // Heal and verify cross-partition messaging works
    cluster.heal_partition(vec![0, 1], vec![2, 3]).await.unwrap();
    cluster.wait_for_mesh(1, 10).await.unwrap();

    // Now all nodes should receive messages
}
```

## Cleanup/Heal Mechanism

The healing mechanism operates at multiple levels:

### 1. Peer-Level Healing
```rust
// Unblock a specific peer
partition.unblock_peer(peer_id);
```

### 2. Partition Group Healing
```rust
// Merge two partition groups
partition.merge_partitions(partition_a, partition_b);
```

### 3. Full Network Healing
```rust
// Restore full connectivity
partition.heal_all_partitions();
```

### Automatic Reconnection

After healing, connections must be reestablished:

1. **Active Reconnection**: Dial known peers explicitly
2. **mDNS Discovery**: If enabled, will rediscover local peers
3. **Kademlia Bootstrap**: Query DHT for peer addresses
4. **Gossipsub Heartbeat**: Mesh will reform naturally over time

For tests, we recommend explicit dialing for deterministic timing.

## Feature Flag

The partition simulator should be behind a feature flag:

```toml
# Cargo.toml
[features]
partition-testing = []

[dev-dependencies]
# Always enabled for tests
mycelial-network = { path = ".", features = ["partition-testing"] }
```

## Implementation Order

1. **Phase 1**: Core `PartitionSimulator` struct and methods
2. **Phase 2**: Integration with `NetworkService` connection handling
3. **Phase 3**: Integration with message filtering
4. **Phase 4**: `TestCluster` convenience methods
5. **Phase 5**: Integration tests for partition scenarios

## Relationship to Septal Gates

The `PartitionSimulator` complements but does not replace Septal Gates:

| Aspect | PartitionSimulator | Septal Gates |
|--------|-------------------|--------------|
| Purpose | Testing controlled partitions | Production circuit breaking |
| Trigger | Explicit API calls | Automatic on failure threshold |
| Scope | Peer-to-peer blocking | Node health tracking |
| Recovery | Explicit heal calls | Automatic with timeout |
| Message Filtering | Yes | Via Woronin body |

In production, Septal Gates provide automatic fault isolation. For testing, `PartitionSimulator` provides explicit control over network topology.

## Conclusion

This design provides a clean, portable approach to network partition simulation that:

1. Integrates naturally with the existing codebase architecture
2. Provides both low-level (peer blocking) and high-level (partition groups) APIs
3. Enables comprehensive testing of distributed system behavior
4. Can be easily extended for more complex partition scenarios

The application-level approach is recommended over transport-level alternatives due to its portability, testability, and alignment with existing patterns.
