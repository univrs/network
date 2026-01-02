# Phase 3 Stress Testing: Multi-Node Spawn Harness Design

> **Status**: DESIGN DOCUMENT
> **Date**: 2026-01-01
> **Author**: Test Architect

---

## Executive Summary

This document describes the architecture for a multi-node spawn harness capable of stress testing 10+ P2P nodes in the univrs-network codebase. The design builds upon existing `TestCluster` infrastructure while extending it for larger scale testing.

---

## Architecture Overview

```
+------------------------------------------------------------------+
|                    STRESS TEST HARNESS                            |
+------------------------------------------------------------------+
|                                                                   |
|  +------------------------+    +---------------------------+      |
|  |   StressTestConfig     |    |    TopologyManager        |      |
|  |  - node_count: usize   |    |  - FullMesh              |      |
|  |  - base_port: u16      |    |  - HubSpoke              |      |
|  |  - topology: Topology  |    |  - Random(connectivity)  |      |
|  |  - metrics: bool       |    |  - Ring                  |      |
|  +------------------------+    +---------------------------+      |
|             |                           |                         |
|             v                           v                         |
|  +----------------------------------------------------------+    |
|  |                    StressCluster                          |    |
|  |  - nodes: Vec<StressNode>                                 |    |
|  |  - topology: TopologyManager                              |    |
|  |  - metrics_collector: MetricsCollector                    |    |
|  |  - shutdown_tx: broadcast::Sender<()>                     |    |
|  +----------------------------------------------------------+    |
|             |                                                     |
|             v                                                     |
|  +----------------------------------------------------------+    |
|  |                    StressNode                             |    |
|  |  - id: NodeId                                             |    |
|  |  - keypair: Keypair (Ed25519)                             |    |
|  |  - port: u16                                              |    |
|  |  - handle: NetworkHandle                                  |    |
|  |  - event_rx: broadcast::Receiver<NetworkEvent>            |    |
|  |  - enr_bridge: Arc<EnrBridge>                             |    |
|  |  - metrics: NodeMetrics                                   |    |
|  +----------------------------------------------------------+    |
|                                                                   |
+------------------------------------------------------------------+
                              |
                              v
+------------------------------------------------------------------+
|                    LIBP2P LAYER                                   |
|  +------+  +------+  +------+  +------+  +------+                |
|  |Node 0|--|Node 1|--|Node 2|--|Node 3|--|Node N|                |
|  +------+  +------+  +------+  +------+  +------+                |
|     |         |         |         |         |                     |
|     +---------+---------+---------+---------+                     |
|                  gossipsub mesh                                   |
+------------------------------------------------------------------+
```

---

## Component Definitions

### 1. StressTestConfig

```rust
/// Configuration for stress test cluster
#[derive(Debug, Clone)]
pub struct StressTestConfig {
    /// Number of nodes to spawn (10-100)
    pub node_count: usize,

    /// Base port for TCP listeners (nodes use base + offset*2)
    pub base_port: u16,

    /// Topology pattern for node connections
    pub topology: Topology,

    /// Enable detailed metrics collection
    pub enable_metrics: bool,

    /// Timeout for mesh formation (seconds)
    pub mesh_timeout_secs: u64,

    /// Minimum peers per node before considered "connected"
    pub min_peers: usize,

    /// Enable verbose logging per node
    pub verbose: bool,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            node_count: 10,
            base_port: 20000,
            topology: Topology::HubSpoke,
            enable_metrics: true,
            mesh_timeout_secs: 60,
            min_peers: 1,
            verbose: false,
        }
    }
}
```

### 2. Topology Enum

```rust
/// Network topology patterns for stress testing
#[derive(Debug, Clone, Copy)]
pub enum Topology {
    /// Every node connects to every other node
    /// Connections: n*(n-1)/2
    FullMesh,

    /// All nodes connect through central hub (node 0)
    /// Connections: n-1
    HubSpoke,

    /// Each node connects to next, last connects to first
    /// Connections: n
    Ring,

    /// Random connections with specified connectivity ratio
    /// Connections: n * connectivity * (n-1) / 2
    Random { connectivity: f64 },

    /// Hierarchical tree structure
    /// Connections: n-1 (parent-child edges)
    Tree { branching_factor: usize },
}
```

### 3. StressNode

```rust
/// A single node in the stress test cluster
pub struct StressNode {
    /// Unique node identifier
    pub id: String,

    /// Node index (0..n)
    pub index: usize,

    /// Ed25519 keypair for identity
    pub keypair: libp2p::identity::Keypair,

    /// TCP listen port
    pub port: u16,

    /// Network handle for commands
    pub handle: NetworkHandle,

    /// Event receiver for monitoring
    pub event_rx: broadcast::Receiver<NetworkEvent>,

    /// ENR bridge for economics primitives
    pub enr_bridge: Arc<EnrBridge>,

    /// Full multiaddr for bootstrap connections
    pub multiaddr: String,

    /// Runtime metrics
    pub metrics: Arc<RwLock<NodeMetrics>>,

    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
}

impl StressNode {
    /// Get current peer count
    pub async fn peer_count(&self) -> usize {
        self.handle.get_peers().await.map(|p| p.len()).unwrap_or(0)
    }

    /// Get node's credit balance
    pub async fn balance(&self) -> u64 {
        self.enr_bridge.credits.local_balance().await.amount
    }

    /// Check if node is healthy
    pub async fn is_healthy(&self) -> bool {
        self.peer_count().await > 0
    }
}
```

### 4. StressCluster

```rust
/// Multi-node cluster for stress testing
pub struct StressCluster {
    /// All nodes in the cluster
    pub nodes: Vec<StressNode>,

    /// Configuration used to create cluster
    pub config: StressTestConfig,

    /// Topology manager for connection patterns
    topology_manager: TopologyManager,

    /// Aggregated metrics collector
    metrics_collector: MetricsCollector,

    /// Global shutdown signal
    shutdown_tx: broadcast::Sender<()>,

    /// Creation timestamp
    created_at: Instant,
}

impl StressCluster {
    /// Spawn a new stress test cluster
    pub async fn spawn(config: StressTestConfig)
        -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    {
        // Implementation in Spawn Sequence section
    }

    /// Connect nodes according to topology
    pub async fn connect_all(&mut self) -> Result<()>;

    /// Wait for mesh formation
    pub async fn wait_for_mesh(&self, timeout_secs: u64) -> Result<()>;

    /// Get cluster-wide statistics
    pub fn stats(&self) -> ClusterStats;

    /// Graceful shutdown all nodes
    pub async fn shutdown(self) -> Result<ShutdownReport>;

    /// Get specific node by index
    pub fn node(&self, index: usize) -> &StressNode;

    /// Iterate over all nodes
    pub fn iter(&self) -> impl Iterator<Item = &StressNode>;

    /// Node count
    pub fn len(&self) -> usize;
}
```

---

## Spawn Sequence

```
                    SPAWN SEQUENCE DIAGRAM
                    ======================

    User                StressCluster           StressNode            NetworkService
      |                      |                      |                      |
      |  spawn(config)       |                      |                      |
      |--------------------->|                      |                      |
      |                      |                      |                      |
      |              [Phase 1: Keypair Generation]                         |
      |                      |                      |                      |
      |                      |  for i in 0..count:  |                      |
      |                      |  generate_ed25519()  |                      |
      |                      |--------------------->|                      |
      |                      |  (keypair, port)     |                      |
      |                      |<---------------------|                      |
      |                      |                      |                      |
      |              [Phase 2: Node Creation]                              |
      |                      |                      |                      |
      |                      |  for each keypair:   |                      |
      |                      |  NetworkService::new()|                     |
      |                      |-------------------------------------------->|
      |                      |  (service, handle, event_rx)                |
      |                      |<--------------------------------------------|
      |                      |                      |                      |
      |                      |  tokio::spawn(       |                      |
      |                      |    service.run()     |                      |
      |                      |  )                   |                      |
      |                      |-------------------------------------------->|
      |                      |                      |        [running]     |
      |                      |                      |                      |
      |              [Phase 3: Bootstrap Connections]                      |
      |                      |                      |                      |
      |                      |  apply_topology()    |                      |
      |                      |--------------------->|                      |
      |                      |  handle.dial(addr)   |                      |
      |                      |<---------------------|                      |
      |                      |                      |                      |
      |              [Phase 4: Mesh Formation Wait]                        |
      |                      |                      |                      |
      |                      |  wait_for_mesh()     |                      |
      |                      |--------------------->|                      |
      |                      |  poll peer_count()   |                      |
      |                      |<---------------------|                      |
      |                      |                      |                      |
      |  Ok(cluster)         |                      |                      |
      |<---------------------|                      |                      |
      |                      |                      |                      |
```

### Detailed Spawn Implementation

```rust
impl StressCluster {
    pub async fn spawn(config: StressTestConfig)
        -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    {
        assert!(config.node_count >= 2, "Need at least 2 nodes");
        assert!(config.node_count <= 100, "Max 100 nodes for stress test");

        // Calculate base port with process isolation
        let cluster_offset = PORT_COUNTER.fetch_add(
            config.node_count as u16 * 2,
            Ordering::SeqCst
        ) % 10000;

        let base_port = config.base_port
            .wrapping_add((std::process::id() as u16 % 100) * 100)
            .wrapping_add(cluster_offset);

        // Phase 1: Generate all keypairs and addresses upfront
        let mut node_specs = Vec::with_capacity(config.node_count);
        for i in 0..config.node_count {
            let port = base_port + (i as u16 * 2);
            let keypair = libp2p::identity::Keypair::generate_ed25519();
            let peer_id = keypair.public().to_peer_id();
            let multiaddr = format!("/ip4/127.0.0.1/tcp/{}/p2p/{}", port, peer_id);
            node_specs.push((keypair, port, multiaddr));
        }

        // Phase 2: Create and spawn all nodes
        let (shutdown_tx, _) = broadcast::channel(1);
        let mut nodes = Vec::with_capacity(config.node_count);

        for (i, (keypair, port, multiaddr)) in node_specs.into_iter().enumerate() {
            // Determine bootstrap peers based on topology
            let bootstrap_peers = Self::compute_bootstrap_peers(
                i,
                &nodes,
                &config.topology
            );

            let net_config = NetworkConfig {
                listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{}", port)],
                bootstrap_peers,
                enable_mdns: false,  // Disable for deterministic testing
                enable_tcp: true,
                enable_quic: false,
                max_connections: 50, // Lower limit per node
                ..Default::default()
            };

            let (service, handle, event_rx) = NetworkService::new(
                keypair.clone(),
                net_config
            )?;

            let enr_bridge = service.enr_bridge().clone();

            // Spawn network service task
            let node_idx = i;
            let shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                tokio::select! {
                    result = service.run() => {
                        if let Err(e) = result {
                            eprintln!("Node {} error: {}", node_idx, e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        // Graceful shutdown signal received
                    }
                }
            });

            nodes.push(StressNode {
                id: keypair.public().to_peer_id().to_base58(),
                index: i,
                keypair,
                port,
                handle,
                event_rx,
                enr_bridge,
                multiaddr,
                metrics: Arc::new(RwLock::new(NodeMetrics::default())),
                shutdown: Arc::new(AtomicBool::new(false)),
            });
        }

        // Brief startup delay
        tokio::time::sleep(Duration::from_millis(100 * config.node_count as u64)).await;

        Ok(Self {
            nodes,
            config,
            topology_manager: TopologyManager::new(config.topology),
            metrics_collector: MetricsCollector::new(),
            shutdown_tx,
            created_at: Instant::now(),
        })
    }

    /// Compute bootstrap peers based on topology
    fn compute_bootstrap_peers(
        node_index: usize,
        existing_nodes: &[StressNode],
        topology: &Topology,
    ) -> Vec<String> {
        match topology {
            Topology::HubSpoke => {
                // All nodes bootstrap to node 0
                if node_index == 0 {
                    vec![]
                } else if let Some(hub) = existing_nodes.first() {
                    vec![hub.multiaddr.clone()]
                } else {
                    vec![]
                }
            }

            Topology::Ring => {
                // Each node bootstraps to previous node
                if node_index == 0 {
                    vec![]
                } else {
                    vec![existing_nodes[node_index - 1].multiaddr.clone()]
                }
            }

            Topology::FullMesh => {
                // Bootstrap to first node, mesh forms via gossipsub
                if node_index == 0 {
                    vec![]
                } else if let Some(first) = existing_nodes.first() {
                    vec![first.multiaddr.clone()]
                } else {
                    vec![]
                }
            }

            Topology::Random { connectivity } => {
                // Random subset of existing nodes
                use rand::seq::SliceRandom;
                let target_count = ((existing_nodes.len() as f64) * connectivity) as usize;
                let target_count = target_count.max(1).min(existing_nodes.len());

                existing_nodes
                    .choose_multiple(&mut rand::thread_rng(), target_count)
                    .map(|n| n.multiaddr.clone())
                    .collect()
            }

            Topology::Tree { branching_factor } => {
                // Bootstrap to parent node
                if node_index == 0 {
                    vec![]
                } else {
                    let parent_index = (node_index - 1) / branching_factor;
                    vec![existing_nodes[parent_index].multiaddr.clone()]
                }
            }
        }
    }
}
```

---

## Connection Establishment

### Topology-Specific Connection Patterns

```
    FULL MESH (10 nodes)                  HUB-SPOKE (10 nodes)
    ====================                  ====================

         1 --- 2                              1   2   3
        /|\   /|\                              \  |  /
       0-+-3-+-4                                \ | /
        \|/ \|/                               0--HUB--4
         5---6                                  / | \
        /     \                                /  |  \
       7-------8                              5   6   7
         \   /
          \ /
           9                                      8   9

    Connections: 45                       Connections: 9
    Time to form: ~10s                    Time to form: ~3s


    RING (10 nodes)                       RANDOM (10 nodes, 30%)
    ===============                       ======================

        0 --- 1                               1     2
       /       \                             / \   / \
      9         2                           0---3-4---5
       \       /                               \ X /
        8 --- 3                                 6-7
       /       \                               / \
      7         4                             8   9
       \       /
        6 --- 5

    Connections: 10                       Connections: ~14
    Time to form: ~5s                     Time to form: ~4s
```

### Connection Wait Strategy

```rust
impl StressCluster {
    /// Wait for mesh formation with progress reporting
    pub async fn wait_for_mesh(&self, timeout_secs: u64)
        -> Result<MeshFormationReport, TimeoutError>
    {
        let deadline = Duration::from_secs(timeout_secs);
        let start = Instant::now();

        timeout(deadline, async {
            loop {
                let mut progress = MeshProgress::default();

                for node in &self.nodes {
                    let peers = node.handle.get_peers().await?;
                    progress.peer_counts.push(peers.len());

                    if peers.len() >= self.config.min_peers {
                        progress.connected_count += 1;
                    }
                }

                // Progress logging every second
                if progress.should_log() {
                    info!(
                        "Mesh formation: {}/{} nodes connected, peers: {:?}",
                        progress.connected_count,
                        self.nodes.len(),
                        progress.peer_counts
                    );
                }

                if progress.connected_count == self.nodes.len() {
                    // Additional stabilization wait
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    return Ok(MeshFormationReport {
                        duration: start.elapsed(),
                        final_peer_counts: progress.peer_counts,
                        topology: self.config.topology,
                    });
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }).await
        .map_err(|_| TimeoutError::MeshFormation(timeout_secs))?
    }
}
```

---

## Cleanup Procedure

```
                    SHUTDOWN SEQUENCE
                    =================

    StressCluster        StressNode          NetworkService
         |                   |                    |
         |  shutdown()       |                    |
         |------------------>|                    |
         |                   |                    |
         |  [Phase 1: Signal All Nodes]           |
         |                   |                    |
         |  shutdown_tx      |                    |
         |  .send(())        |                    |
         |------------------>|                    |
         |                   |  shutdown flag     |
         |                   |------------------->|
         |                   |                    |
         |  [Phase 2: Wait for Graceful Stop]     |
         |                   |                    |
         |  for each node:   |                    |
         |  handle.shutdown()|                    |
         |------------------>|                    |
         |                   |  NetworkCommand::  |
         |                   |  Shutdown          |
         |                   |------------------->|
         |                   |                    |
         |                   |  [drain events]    |
         |                   |<-------------------|
         |                   |                    |
         |  [Phase 3: Collect Final Metrics]      |
         |                   |                    |
         |  collect_metrics()|                    |
         |------------------>|                    |
         |  NodeMetrics      |                    |
         |<------------------|                    |
         |                   |                    |
         |  [Phase 4: Report Generation]          |
         |                   |                    |
         |  Ok(ShutdownReport)                    |
         |<------------------|                    |
         |                   |                    |
```

### Shutdown Implementation

```rust
impl StressCluster {
    /// Graceful shutdown with cleanup report
    pub async fn shutdown(self) -> Result<ShutdownReport, ShutdownError> {
        let shutdown_start = Instant::now();

        // Phase 1: Broadcast shutdown signal
        let _ = self.shutdown_tx.send(());

        // Phase 2: Shutdown each node's handle
        let mut node_reports = Vec::with_capacity(self.nodes.len());

        for node in &self.nodes {
            let node_start = Instant::now();

            // Collect final metrics before shutdown
            let final_peers = node.handle.get_peers().await.unwrap_or_default();
            let final_stats = node.handle.get_stats().await.ok();

            // Send shutdown command
            let shutdown_result = tokio::time::timeout(
                Duration::from_secs(5),
                node.handle.shutdown()
            ).await;

            node_reports.push(NodeShutdownReport {
                node_id: node.id.clone(),
                node_index: node.index,
                shutdown_duration: node_start.elapsed(),
                final_peer_count: final_peers.len(),
                final_stats,
                clean_shutdown: shutdown_result.is_ok(),
            });
        }

        // Phase 3: Aggregate metrics
        let total_messages: u64 = node_reports.iter()
            .filter_map(|r| r.final_stats.as_ref())
            .map(|s| s.messages_sent + s.messages_received)
            .sum();

        let total_bytes: u64 = node_reports.iter()
            .filter_map(|r| r.final_stats.as_ref())
            .map(|s| s.bytes_sent + s.bytes_received)
            .sum();

        Ok(ShutdownReport {
            cluster_duration: self.created_at.elapsed(),
            shutdown_duration: shutdown_start.elapsed(),
            node_count: self.nodes.len(),
            node_reports,
            total_messages,
            total_bytes,
            clean_shutdown_rate: node_reports.iter()
                .filter(|r| r.clean_shutdown)
                .count() as f64 / self.nodes.len() as f64,
        })
    }
}
```

---

## Resource Requirements

### Port Allocation

| Node Count | Port Range | Buffer |
|------------|------------|--------|
| 10 nodes   | base..base+20 | 20 ports |
| 25 nodes   | base..base+50 | 50 ports |
| 50 nodes   | base..base+100 | 100 ports |
| 100 nodes  | base..base+200 | 200 ports |

**Port Calculation Formula:**
```
node_port = base_port + (node_index * 2)
# Each node needs 2 ports: TCP and potentially QUIC
```

### Memory Estimates

| Component | Per-Node Memory | 10 Nodes | 50 Nodes | 100 Nodes |
|-----------|-----------------|----------|----------|-----------|
| libp2p Swarm | ~5 MB | 50 MB | 250 MB | 500 MB |
| Gossipsub State | ~2 MB | 20 MB | 100 MB | 200 MB |
| Kademlia DHT | ~1 MB | 10 MB | 50 MB | 100 MB |
| ENR Bridge | ~1 MB | 10 MB | 50 MB | 100 MB |
| Event Channels | ~0.5 MB | 5 MB | 25 MB | 50 MB |
| **Total** | **~10 MB** | **~100 MB** | **~500 MB** | **~1 GB** |

### CPU Requirements

| Topology | 10 Nodes | 50 Nodes | 100 Nodes |
|----------|----------|----------|-----------|
| Hub-Spoke | ~20% | ~50% | ~100% |
| Ring | ~15% | ~40% | ~80% |
| Full Mesh | ~30% | ~150% | ~400% |

*Note: Full mesh at 100 nodes requires multi-core scaling*

### Connection Limits

```rust
// Per-node gossipsub limits (from existing config)
const MESH_N: usize = 2;          // Target mesh peers
const MESH_N_LOW: usize = 1;      // Min before grafting
const MESH_N_HIGH: usize = 4;     // Max before pruning
const MAX_CONNECTIONS: u32 = 50;  // libp2p connection limit
```

---

## Existing Code to Reuse

### 1. TestCluster Foundation
**Location:** `/home/ardeshir/repos/univrs-network/crates/mycelial-network/tests/helpers/cluster.rs`

```rust
// Reusable components:
- PORT_COUNTER: AtomicU16 - Process-unique port allocation
- TestNode struct - Base structure for stress nodes
- wait_for_mesh() - Mesh formation wait logic
- shutdown() - Cleanup pattern
```

### 2. NetworkConfig
**Location:** `/home/ardeshir/repos/univrs-network/crates/mycelial-network/src/config.rs`

```rust
// local_test() factory method provides good defaults:
NetworkConfig::local_test(port)
```

### 3. NetworkService
**Location:** `/home/ardeshir/repos/univrs-network/crates/mycelial-network/src/service.rs`

```rust
// Core service creation:
NetworkService::new(keypair, config) -> (Self, NetworkHandle, Receiver<NetworkEvent>)

// Runtime execution:
service.run() -> Result<()>
```

### 4. ENR Bridge
**Location:** `/home/ardeshir/repos/univrs-network/crates/mycelial-network/src/enr_bridge/mod.rs`

```rust
// Economic primitives for stress testing:
enr_bridge.trigger_election()
enr_bridge.broadcast_gradient()
enr_bridge.credits.local_balance()
```

### 5. Gate Tests Pattern
**Location:** `/home/ardeshir/repos/univrs-network/crates/mycelial-network/tests/gate_election.rs`

```rust
// Pattern for integration tests:
#[tokio::test]
#[ignore = "Integration test"]
async fn test_with_cluster() {
    let cluster = TestCluster::spawn(5).await?;
    cluster.wait_for_mesh(1, 15).await?;
    // ... test logic
    cluster.shutdown().await;
}
```

---

## Implementation Recommendations

### Phase 1: Extend Existing TestCluster (Week 1)

1. **Remove 10-node limit** in existing `TestCluster::spawn()`
2. **Add Topology enum** parameter to spawn()
3. **Implement topology-specific bootstrap** logic
4. **Add metrics collection** to TestNode

### Phase 2: Create StressCluster Wrapper (Week 2)

1. **Create `stress_harness.rs`** in `tests/helpers/`
2. **Implement StressTestConfig** with all options
3. **Add progress reporting** during mesh formation
4. **Implement detailed shutdown reports**

### Phase 3: Metrics and Reporting (Week 3)

1. **Create MetricsCollector** for aggregated stats
2. **Add resource monitoring** (memory, connections)
3. **Generate HTML/JSON reports** for test runs
4. **Add CI integration** for automated stress tests

### Suggested File Structure

```
crates/mycelial-network/tests/
  helpers/
    mod.rs              # Existing - add stress harness export
    cluster.rs          # Existing - extend for stress testing
    stress_harness.rs   # NEW - StressCluster implementation
    topology.rs         # NEW - Topology enum and manager
    metrics.rs          # NEW - Metrics collection
  stress_tests/
    mod.rs              # NEW - Stress test module
    scale_10_nodes.rs   # NEW - 10 node stress tests
    scale_50_nodes.rs   # NEW - 50 node stress tests
    topology_tests.rs   # NEW - Topology validation
```

---

## Example Usage

```rust
use mycelial_network::tests::helpers::{StressCluster, StressTestConfig, Topology};

#[tokio::test]
#[ignore = "Stress test - requires significant resources"]
async fn stress_test_50_nodes_hub_spoke() {
    let config = StressTestConfig {
        node_count: 50,
        base_port: 30000,
        topology: Topology::HubSpoke,
        enable_metrics: true,
        mesh_timeout_secs: 120,
        min_peers: 1,
        verbose: false,
    };

    // Spawn cluster
    let cluster = StressCluster::spawn(config)
        .await
        .expect("Failed to spawn stress cluster");

    // Wait for mesh
    let mesh_report = cluster.wait_for_mesh(120)
        .await
        .expect("Mesh formation timeout");

    println!("Mesh formed in {:?}", mesh_report.duration);

    // Run stress test scenarios...

    // Cleanup
    let shutdown_report = cluster.shutdown()
        .await
        .expect("Shutdown failed");

    assert!(shutdown_report.clean_shutdown_rate > 0.95);
    println!("Total messages: {}", shutdown_report.total_messages);
}
```

---

## Appendix: Comparison with Docker Compose

| Aspect | In-Process (StressCluster) | Docker Compose |
|--------|---------------------------|----------------|
| Startup Time | Fast (~1s per 10 nodes) | Slow (~10s per node) |
| Memory Overhead | Low (~10 MB/node) | High (~50 MB/container) |
| Debugging | Easy (shared memory) | Hard (container logs) |
| Network Realism | Medium (localhost only) | High (real network) |
| Port Conflicts | Avoided via counter | Manual management |
| Cleanup | Automatic (Drop trait) | Manual docker-compose down |
| Max Nodes | ~100 (single process) | ~20 (resource limits) |

**Recommendation:** Use in-process StressCluster for development and CI, Docker Compose for production simulation testing.

---

*Document generated: 2026-01-01*
