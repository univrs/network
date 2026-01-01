//! TestCluster - Spawn multiple network nodes for integration testing
//!
//! This module provides a TestCluster that spawns 3-5 nodes with:
//! - Automatic port allocation (process-unique)
//! - Direct bootstrap connections (no mDNS to avoid cross-test interference)
//! - Mesh formation waiting
//! - Cleanup on drop

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;

use mycelial_network::{
    config::NetworkConfig,
    enr_bridge::EnrBridge,
    event::NetworkEvent,
    service::{NetworkHandle, NetworkService},
};

/// Global port counter to ensure unique ports across tests
static PORT_COUNTER: AtomicU16 = AtomicU16::new(0);

/// A test node with its handle and ENR bridge
pub struct TestNode {
    pub handle: NetworkHandle,
    pub event_rx: broadcast::Receiver<NetworkEvent>,
    pub enr_bridge: Arc<EnrBridge>,
    pub node_index: usize,
    pub listen_addr: String,
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
    /// Spawn a cluster of `count` nodes (3-5 recommended)
    ///
    /// Nodes use direct bootstrap connections (not mDNS) to avoid
    /// interference from parallel test runs.
    pub async fn spawn(count: usize) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        assert!(count >= 2, "Need at least 2 nodes for a cluster");
        assert!(count <= 10, "Max 10 nodes for test cluster");

        // Get unique base port for this test cluster
        // Use modular arithmetic to stay in valid port range
        let cluster_offset = PORT_COUNTER.fetch_add(count as u16 * 2, Ordering::SeqCst) % 10000;
        let base_port = 20000u16
            .wrapping_add((std::process::id() as u16 % 100) * 100)
            .wrapping_add(cluster_offset)
            % 40000
            + 20000;

        // First pass: create nodes and collect their addresses
        let mut keypairs = Vec::with_capacity(count);
        let mut listen_addrs = Vec::with_capacity(count);

        for i in 0..count {
            let port = base_port + (i as u16 * 2);
            let keypair = libp2p::identity::Keypair::generate_ed25519();
            let peer_id = keypair.public().to_peer_id();
            let addr = format!("/ip4/127.0.0.1/tcp/{}/p2p/{}", port, peer_id);
            keypairs.push(keypair);
            listen_addrs.push((port, addr));
        }

        let mut nodes = Vec::with_capacity(count);
        let mut shutdown_handles = Vec::with_capacity(count);

        // Second pass: create nodes with bootstrap peers pointing to first node
        for i in 0..count {
            let (port, _) = &listen_addrs[i];
            let keypair = keypairs.remove(0);

            // All nodes except first bootstrap to first node
            // First node has no bootstrap peers
            let bootstrap_peers = if i > 0 {
                vec![listen_addrs[0].1.clone()]
            } else {
                vec![]
            };

            let config = NetworkConfig {
                listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{}", port)],
                bootstrap_peers,
                enable_mdns: false, // Disable mDNS to avoid cross-test interference
                enable_tcp: true,
                enable_quic: false,
                ..Default::default()
            };

            let (service, handle, event_rx) = NetworkService::new(keypair, config)?;
            let enr_bridge = service.enr_bridge().clone();
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
            });
            shutdown_handles.push(handle_clone);
        }

        // Give nodes time to start listening before returning
        tokio::time::sleep(Duration::from_millis(100)).await;

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
                    tokio::time::sleep(Duration::from_millis(500)).await;
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
}

// Note: Explicit shutdown() should be called before dropping
// Nodes will timeout and stop on their own if not explicitly shut down
