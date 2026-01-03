//! TestCluster - Spawn multiple network nodes for integration testing
//!
//! This module provides a TestCluster that spawns 2-100 nodes with:
//! - Automatic port allocation (process-unique)
//! - Hierarchical bootstrap topology (avoids bottleneck on node 0 for large clusters)
//! - Direct bootstrap connections (no mDNS to avoid cross-test interference)
//! - Mesh formation waiting with configurable timeout
//! - Clean shutdown
//! - Network partition simulation for testing distributed system behavior
//!
//! Bootstrap topology:
//! - Nodes 0-9: star topology (bootstrap to node 0)
//! - Nodes 10+: hierarchical (nodes 10-19 -> node 1, nodes 20-29 -> node 2, etc.)

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;

use libp2p::PeerId;
use mycelial_network::{
    config::NetworkConfig,
    enr_bridge::EnrBridge,
    event::NetworkEvent,
    service::{NetworkHandle, NetworkService},
};

/// Global port counter to ensure unique ports across tests
/// Uses larger increments to avoid TIME_WAIT conflicts
static PORT_COUNTER: AtomicU16 = AtomicU16::new(0);

/// Port range allocated per test (with buffer for TIME_WAIT)
const PORT_RANGE_PER_TEST: u16 = 250;

/// A test node with its handle and ENR bridge
pub struct TestNode {
    pub handle: NetworkHandle,
    pub event_rx: broadcast::Receiver<NetworkEvent>,
    pub enr_bridge: Arc<EnrBridge>,
    pub node_index: usize,
    pub listen_addr: String,
    pub peer_id: PeerId,
}

impl TestNode {
    /// Get the node's local balance
    pub async fn balance(&self) -> u64 {
        self.enr_bridge.credits.local_balance().await.amount
    }
}

/// TestCluster spawns and manages multiple network nodes
pub struct TestCluster {
    pub nodes: Vec<TestNode>,
    shutdown_handles: Vec<NetworkHandle>,
}

impl TestCluster {
    /// Spawn a cluster of `count` nodes (2-100 supported)
    ///
    /// Nodes use direct bootstrap connections (not mDNS) to avoid
    /// interference from parallel test runs. For clusters larger than 10 nodes,
    /// a hierarchical bootstrap topology is used to avoid overwhelming node 0.
    pub async fn spawn(count: usize) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        assert!(count >= 2, "Need at least 2 nodes for a cluster");
        assert!(count <= 100, "Max 100 nodes for test cluster");

        // Get unique base port for this test cluster
        // Use large increments to avoid TIME_WAIT conflicts from previous tests
        let test_index = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let cluster_offset = (test_index as u32 * PORT_RANGE_PER_TEST as u32) % 30000;
        let base_port = 20000u16
            .wrapping_add((std::process::id() as u16 % 50) * 200)
            .wrapping_add(cluster_offset as u16);

        // First pass: create nodes and collect their addresses
        let mut keypairs: Vec<(libp2p::identity::Keypair, PeerId)> = Vec::with_capacity(count);
        let mut listen_addrs = Vec::with_capacity(count);

        for i in 0..count {
            let port = base_port + (i as u16 * 2);
            let keypair = libp2p::identity::Keypair::generate_ed25519();
            let peer_id = keypair.public().to_peer_id();
            let addr = format!("/ip4/127.0.0.1/tcp/{}/p2p/{}", port, peer_id);
            keypairs.push((keypair, peer_id));
            listen_addrs.push((port, addr));
        }

        let mut nodes = Vec::with_capacity(count);
        let mut shutdown_handles = Vec::with_capacity(count);

        // Second pass: create nodes with hierarchical bootstrap topology
        // This avoids overwhelming node 0 with connections in large clusters:
        // - Nodes 0-9: bootstrap to node 0 (node 0 has no bootstrap)
        // - Nodes 10-19: bootstrap to node 1
        // - Nodes 20-29: bootstrap to node 2
        // - etc.
        for i in 0..count {
            let (port, _) = &listen_addrs[i];
            let (keypair, peer_id) = keypairs.remove(0);

            // Hierarchical bootstrap: distribute load across multiple nodes
            let bootstrap_peers = if i == 0 {
                // First node has no bootstrap peers
                vec![]
            } else if i < 10 {
                // First 10 nodes bootstrap to node 0 (star for small clusters)
                vec![listen_addrs[0].1.clone()]
            } else {
                // Larger clusters: bootstrap to node (i / 10)
                // This creates a tree: nodes 10-19 -> node 1, nodes 20-29 -> node 2, etc.
                let bootstrap_idx = i / 10;
                vec![listen_addrs[bootstrap_idx].1.clone()]
            };

            let config = NetworkConfig {
                listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{}", port)],
                bootstrap_peers,
                enable_mdns: false, // Disable mDNS to avoid cross-test interference
                enable_tcp: true,
                enable_quic: false,
                ..Default::default()
            };

            let (service, handle, event_rx, enr_bridge) = NetworkService::new(keypair, config)?;
            let listen_addr = listen_addrs[i].1.clone();

            // Spawn the network service
            let idx = i;
            let handle_clone = handle.clone();
            tokio::spawn(async move {
                if let Err(e) = service.run().await {
                    eprintln!("Node {} error: {}", idx, e);
                }
            });

            nodes.push(TestNode {
                handle: handle.clone(),
                event_rx,
                enr_bridge,
                node_index: i,
                listen_addr,
                peer_id,
            });
            shutdown_handles.push(handle_clone);

            // For larger clusters, add a brief delay between spawns to allow
            // hierarchical bootstrap parents to be ready before children dial
            if count > 10 && i > 0 && i % 10 == 0 {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }

        // Give nodes time to start listening before returning
        // Scale wait time based on cluster size to accommodate cascading bootstrap
        let initial_wait = if count <= 5 {
            200 // Small clusters: 200ms
        } else if count <= 20 {
            500 // Medium clusters: 500ms
        } else {
            1000 // Large clusters: 1s
        };
        tokio::time::sleep(Duration::from_millis(initial_wait)).await;

        Ok(Self {
            nodes,
            shutdown_handles,
        })
    }

    /// Wait for all nodes to discover each other and form mesh
    ///
    /// Returns Ok when all nodes have at least `min_peers` connections
    pub async fn wait_for_mesh(
        &self,
        min_peers: usize,
        timeout_secs: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let deadline = Duration::from_secs(timeout_secs);

        timeout(deadline, async {
            loop {
                let mut all_connected = true;

                for node in &self.nodes {
                    let peers = node.handle.get_peers().await?;
                    if peers.len() < min_peers {
                        all_connected = false;
                        break;
                    }
                }

                if all_connected {
                    // Additional wait for gossipsub mesh to stabilize
                    // Scale based on cluster size - larger clusters need more heartbeat cycles
                    let stabilization_wait = if self.nodes.len() <= 5 {
                        500
                    } else if self.nodes.len() <= 20 {
                        1000
                    } else {
                        2000
                    };
                    tokio::time::sleep(Duration::from_millis(stabilization_wait)).await;
                    return Ok::<(), Box<dyn std::error::Error + Send + Sync>>(());
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .map_err(|_| "Timeout waiting for mesh formation")?
    }

    /// Get number of nodes in the cluster
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get a reference to a specific node
    pub fn node(&self, index: usize) -> &TestNode {
        &self.nodes[index]
    }

    /// Shutdown all nodes
    pub async fn shutdown(self) {
        for handle in self.shutdown_handles {
            let _ = handle.shutdown().await;
        }
    }

    // === Partition Testing Methods ===

    /// Create a network partition between two groups of nodes.
    ///
    /// After calling this, nodes in group_a cannot communicate with nodes in group_b
    /// and vice versa. Messages and connections across the partition are blocked.
    ///
    /// # Example
    /// ```rust,ignore
    /// // Partition nodes [0, 1] from nodes [2, 3]
    /// cluster.create_partition(&[0, 1], &[2, 3]).await?;
    /// ```
    pub async fn create_partition(
        &self,
        group_a: &[usize],
        group_b: &[usize],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get peer IDs for each group
        let ids_a: Vec<PeerId> = group_a.iter().map(|&i| self.nodes[i].peer_id).collect();
        let ids_b: Vec<PeerId> = group_b.iter().map(|&i| self.nodes[i].peer_id).collect();

        // Tell group A to block all peers in group B
        for &i in group_a {
            for id in &ids_b {
                self.nodes[i].handle.block_peer(*id).await?;
            }
        }

        // Tell group B to block all peers in group A
        for &i in group_b {
            for id in &ids_a {
                self.nodes[i].handle.block_peer(*id).await?;
            }
        }

        // Give time for disconnect to propagate
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Heal a partition between two groups of nodes.
    ///
    /// After calling this, nodes can communicate across the former partition again.
    pub async fn heal_partition(
        &self,
        group_a: &[usize],
        group_b: &[usize],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get peer IDs for each group
        let ids_a: Vec<PeerId> = group_a.iter().map(|&i| self.nodes[i].peer_id).collect();
        let ids_b: Vec<PeerId> = group_b.iter().map(|&i| self.nodes[i].peer_id).collect();

        // Unblock group B peers from group A
        for &i in group_a {
            for id in &ids_b {
                self.nodes[i].handle.unblock_peer(*id).await?;
            }
        }

        // Unblock group A peers from group B
        for &i in group_b {
            for id in &ids_a {
                self.nodes[i].handle.unblock_peer(*id).await?;
            }
        }

        // Trigger reconnection by dialing across the partition
        for &idx_a in group_a.iter().take(1) {
            for &idx_b in group_b.iter().take(1) {
                let addr: libp2p::Multiaddr = self.nodes[idx_b].listen_addr.parse()?;
                let _ = self.nodes[idx_a].handle.dial(addr).await;
            }
        }

        Ok(())
    }

    /// Heal all partitions - unblock all peers on all nodes.
    pub async fn heal_all_partitions(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for node in &self.nodes {
            node.handle.unblock_all_peers().await?;
        }
        Ok(())
    }

    /// Isolate a single node from all other nodes in the cluster.
    pub async fn isolate_node(
        &self,
        node_idx: usize,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let isolated_peer = self.nodes[node_idx].peer_id;

        // Block from perspective of the isolated node
        for (i, node) in self.nodes.iter().enumerate() {
            if i != node_idx {
                // The isolated node blocks all others
                self.nodes[node_idx].handle.block_peer(node.peer_id).await?;
                // All others block the isolated node
                node.handle.block_peer(isolated_peer).await?;
            }
        }

        // Give time for disconnect to propagate
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Rejoin an isolated node to the cluster.
    pub async fn rejoin_node(
        &self,
        node_idx: usize,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let rejoining_peer = self.nodes[node_idx].peer_id;

        // Unblock from all perspectives
        for (i, node) in self.nodes.iter().enumerate() {
            if i != node_idx {
                self.nodes[node_idx]
                    .handle
                    .unblock_peer(node.peer_id)
                    .await?;
                node.handle.unblock_peer(rejoining_peer).await?;
            }
        }

        // Trigger reconnection
        if node_idx > 0 {
            let addr: libp2p::Multiaddr = self.nodes[0].listen_addr.parse()?;
            let _ = self.nodes[node_idx].handle.dial(addr).await;
        } else if self.nodes.len() > 1 {
            let addr: libp2p::Multiaddr = self.nodes[1].listen_addr.parse()?;
            let _ = self.nodes[node_idx].handle.dial(addr).await;
        }

        Ok(())
    }

    // === Election Helper Methods ===

    /// Wait for a specific node to have no election in progress.
    ///
    /// This is useful before triggering a new election to avoid
    /// `ElectionInProgress` errors from race conditions during mesh formation.
    ///
    /// Returns Ok(()) when no election is in progress, or Err if timeout expires.
    pub async fn wait_for_no_election(
        &self,
        node_idx: usize,
        timeout_secs: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let deadline = Duration::from_secs(timeout_secs);

        timeout(deadline, async {
            loop {
                if !self.nodes[node_idx].enr_bridge.election_in_progress().await {
                    return Ok::<(), Box<dyn std::error::Error + Send + Sync>>(());
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        })
        .await
        .map_err(|_| "Timeout waiting for election to clear")?
    }

    /// Wait for all nodes to have no election in progress.
    ///
    /// Useful before tests that need to trigger elections from scratch.
    pub async fn wait_for_all_elections_clear(
        &self,
        timeout_secs: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let deadline = Duration::from_secs(timeout_secs);

        timeout(deadline, async {
            loop {
                let mut any_in_progress = false;
                for node in &self.nodes {
                    if node.enr_bridge.election_in_progress().await {
                        any_in_progress = true;
                        break;
                    }
                }
                if !any_in_progress {
                    return Ok::<(), Box<dyn std::error::Error + Send + Sync>>(());
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        })
        .await
        .map_err(|_| "Timeout waiting for all elections to clear")?
    }
}

// Note: Explicit shutdown() should be called before dropping
// Nodes will timeout and stop on their own if not explicitly shut down
