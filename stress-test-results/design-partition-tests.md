# Network Partition Recovery Test Design

## Phase 3 Stress Testing - Partition Scenarios

This document outlines the design for network partition recovery tests, based on analysis of the existing mycelial-network implementation.

---

## Codebase Context

### Key Components Analyzed

1. **SeptalGateManager** (`/crates/mycelial-network/src/enr_bridge/septal.rs`)
   - Circuit breaker pattern with three states: `Open`, `HalfOpen`, `Closed`
   - Failure tracking with `FAILURE_THRESHOLD` before gate trips
   - `WoroninManager` for blocking transactions to/from isolated nodes
   - State transitions broadcast via `SEPTAL_TOPIC` gossipsub

2. **DistributedElection** (`/crates/mycelial-network/src/enr_bridge/nexus.rs`)
   - Election phases: `Idle`, `Candidacy`, `Voting`, `Confirming`
   - Timeout constants:
     - `ELECTION_TIMEOUT_MS`: 30,000ms
     - `CANDIDACY_PHASE_MS`: 10,000ms
     - `VOTING_PHASE_MS`: 15,000ms
   - Quorum requirement: `MIN_VOTE_FRACTION` = 0.5 (50% of participants)

3. **CreditSynchronizer** (`/crates/mycelial-network/src/enr_bridge/credits.rs`)
   - Local ledger with optimistic gossip updates
   - `INITIAL_NODE_CREDITS`: 1000
   - Replay protection via nonces
   - Entropy tax: 2% on transfers

4. **StateSync** (`/crates/mycelial-state/src/sync.rs`)
   - Vector clocks for causality tracking
   - Last-write-wins for peer and credit updates
   - Grow-only counters for reputation (max merge)

5. **RaftConfig** (`/crates/mycelial-network/src/raft/config.rs`)
   - Heartbeat interval: 100ms (50ms for testing)
   - Election timeout: 300-500ms (150-300ms for testing)

---

## Partition Simulation Approaches

### 1. Application-Level Partition (Recommended for Phase 3)

**Description**: Intercept message passing at the gossipsub layer.

**Implementation**:
```rust
/// Partition controller for test scenarios
pub struct PartitionController {
    /// Set of peer pairs that can communicate
    allowed_connections: Arc<RwLock<HashSet<(PeerId, PeerId)>>>,
    /// Partition groups: nodes in same group can communicate
    partition_groups: Arc<RwLock<HashMap<u8, Vec<PeerId>>>>,
}

impl PartitionController {
    /// Create symmetric partition: split network into groups
    pub fn create_partition(&self, groups: Vec<Vec<usize>>, cluster: &TestCluster) {
        let mut allowed = self.allowed_connections.write();
        allowed.clear();

        for group in &groups {
            for i in group {
                for j in group {
                    if i != j {
                        let peer_i = cluster.node(*i).handle.local_peer_id();
                        let peer_j = cluster.node(*j).handle.local_peer_id();
                        allowed.insert((peer_i, peer_j));
                        allowed.insert((peer_j, peer_i));
                    }
                }
            }
        }
    }

    /// Heal partition: allow all nodes to communicate
    pub fn heal_partition(&self) {
        self.allowed_connections.write().clear();
        // Empty set means no filtering (all allowed)
    }

    /// Check if message should be delivered
    pub fn should_deliver(&self, from: &PeerId, to: &PeerId) -> bool {
        let allowed = self.allowed_connections.read();
        if allowed.is_empty() {
            return true; // No partition active
        }
        allowed.contains(&(*from, *to))
    }
}
```

**Advantages**:
- No system-level dependencies (works on any OS)
- Precise control over which messages are dropped
- Can simulate asymmetric partitions
- Integrates with existing TestCluster infrastructure

**Disadvantages**:
- Requires modifying NetworkService to support message filtering
- Does not test actual TCP connection handling

### 2. Proxy-Based Partition (Alternative)

**Description**: Use a proxy layer between nodes.

```rust
/// TCP proxy that can simulate partitions
pub struct PartitionProxy {
    /// Mapping from virtual port to actual node port
    port_mapping: HashMap<u16, u16>,
    /// Whether to drop traffic between ports
    blocked_routes: HashSet<(u16, u16)>,
}

impl PartitionProxy {
    pub async fn run(&self, virtual_port: u16, actual_port: u16) {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", virtual_port)).await?;

        while let Ok((stream, _)) = listener.accept().await {
            if !self.is_blocked(virtual_port, actual_port) {
                // Forward traffic
                let upstream = TcpStream::connect(format!("127.0.0.1:{}", actual_port)).await?;
                tokio::spawn(async move {
                    let (mut read, mut write) = stream.into_split();
                    let (mut up_read, mut up_write) = upstream.into_split();
                    tokio::select! {
                        _ = tokio::io::copy(&mut read, &mut up_write) => {},
                        _ = tokio::io::copy(&mut up_read, &mut write) => {},
                    }
                });
            }
            // Else: connection silently dropped (simulates partition)
        }
    }
}
```

**Advantages**:
- Tests actual TCP behavior
- More realistic simulation
- Works with existing code without modification

**Disadvantages**:
- More complex setup
- Port management overhead
- Harder to debug

### 3. iptables-Based Partition (Linux Only)

**Description**: Use iptables to block traffic between specific ports.

```bash
# Create partition: Block traffic between group A (ports 20000-20004) and B (20006-20010)
iptables -A INPUT -p tcp --dport 20000:20004 --sport 20006:20010 -j DROP
iptables -A OUTPUT -p tcp --sport 20000:20004 --dport 20006:20010 -j DROP

# Heal partition
iptables -D INPUT -p tcp --dport 20000:20004 --sport 20006:20010 -j DROP
iptables -D OUTPUT -p tcp --sport 20000:20004 --dport 20006:20010 -j DROP
```

**Advantages**:
- Most realistic (tests actual network stack)
- No code changes required

**Disadvantages**:
- Requires root/sudo
- Linux-only
- Harder to parallelize tests
- Risk of leaving rules in place if test crashes

**Recommended**: Use Application-Level Partition for Phase 3, with proxy-based as stretch goal.

---

## Test Scenarios

### Scenario 1: Simple Partition (5+5 Split)

**Setup**:
- 10 nodes total
- Split into Group A (nodes 0-4) and Group B (nodes 5-9)
- Each group operates independently for 30 seconds
- Heal partition
- Verify convergence

**Test Steps**:
```rust
#[tokio::test]
#[ignore = "Stress test - requires dedicated resources"]
async fn test_simple_partition_recovery() {
    let cluster = TestCluster::spawn(10).await.unwrap();
    cluster.wait_for_mesh(2, 30).await.unwrap();

    // Record initial state
    let initial_nexus = cluster.node(0).enr_bridge.current_nexus().await;
    let initial_credits: Vec<_> = (0..10)
        .map(|i| cluster.node(i).balance())
        .collect::<FuturesOrdered<_>>()
        .collect()
        .await;

    // Create partition
    let controller = PartitionController::new();
    controller.create_partition(
        vec![vec![0, 1, 2, 3, 4], vec![5, 6, 7, 8, 9]],
        &cluster
    );

    // Let partition persist (allow elections in both groups)
    tokio::time::sleep(Duration::from_secs(30)).await;

    // Verify groups have independent state
    // Group A and B may have different nexus now
    let nexus_a = cluster.node(0).enr_bridge.current_nexus().await;
    let nexus_b = cluster.node(5).enr_bridge.current_nexus().await;

    // Heal partition
    controller.heal_partition();

    // Wait for convergence (max 60 seconds)
    let converged = timeout(Duration::from_secs(60), async {
        loop {
            // Check all nodes agree on nexus
            let nexuses: Vec<_> = (0..10)
                .map(|i| cluster.node(i).enr_bridge.current_nexus())
                .collect::<FuturesOrdered<_>>()
                .collect()
                .await;

            if nexuses.windows(2).all(|w| w[0] == w[1]) {
                return true;
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }).await;

    assert!(converged.is_ok(), "Network did not converge within 60 seconds");

    cluster.shutdown().await;
}
```

**Convergence Criteria**:
1. All nodes agree on nexus (same winner)
2. All nodes can reach all other nodes (gossip resumes)
3. Credit ledgers eventually consistent (within 5% tolerance due to entropy tax)
4. No Septal gates in Closed state (unless truly unhealthy)

**Timeout Values**:
- Partition duration: 30 seconds
- Convergence timeout: 60 seconds
- Poll interval: 500ms

### Scenario 2: Asymmetric Partition (8+2 Split)

**Purpose**: Test minority isolation detection via SeptalGate.

**Setup**:
- 10 nodes total
- Group A: 8 nodes (majority)
- Group B: 2 nodes (minority)
- Minority should detect isolation

**Expected Behavior**:
1. Majority (Group A) continues normal operation
2. Minority (Group B) nodes:
   - Fail to reach quorum for elections (`has_quorum()` returns false with 2/10 = 20% < 50%)
   - SeptalGate may trip due to repeated failures to reach other nodes
   - Should enter a "degraded" state

**Test Steps**:
```rust
#[tokio::test]
#[ignore = "Stress test - requires dedicated resources"]
async fn test_asymmetric_partition_minority_detection() {
    let cluster = TestCluster::spawn(10).await.unwrap();
    cluster.wait_for_mesh(2, 30).await.unwrap();

    // Create asymmetric partition
    let controller = PartitionController::new();
    controller.create_partition(
        vec![vec![0, 1, 2, 3, 4, 5, 6, 7], vec![8, 9]],
        &cluster
    );

    // Allow time for isolation detection (3x election timeout)
    tokio::time::sleep(Duration::from_secs(45)).await;

    // Majority should still function
    let majority_nexus = cluster.node(0).enr_bridge.current_nexus().await;
    assert!(majority_nexus.is_some(), "Majority should still have nexus");

    // Minority should detect issues
    // Check SeptalGate statistics for nodes 8 and 9
    let minority_stats_8 = cluster.node(8).enr_bridge.septal.stats().await;
    let minority_stats_9 = cluster.node(9).enr_bridge.septal.stats().await;

    // Minority nodes should have closed gates for majority nodes
    assert!(
        minority_stats_8.closed_gates + minority_stats_9.closed_gates >= 6,
        "Minority should have detected isolation from majority"
    );

    // Heal and verify recovery
    controller.heal_partition();

    // Wait for Septal gates to recover (half-open -> open)
    tokio::time::sleep(Duration::from_secs(30)).await;

    let recovered_stats = cluster.node(8).enr_bridge.septal.stats().await;
    assert!(
        recovered_stats.closed_gates == 0,
        "All gates should be open after recovery"
    );

    cluster.shutdown().await;
}
```

**Septal Gate Triggers**:
1. `record_failure()` called when peer unreachable
2. After `FAILURE_THRESHOLD` failures, gate trips to Closed
3. `WoroninManager` activates to block transactions
4. Recovery via `attempt_half_open()` after timeout

### Scenario 3: Transient Partition (5-second disconnect)

**Purpose**: Verify short partitions do not trigger full re-election.

**Setup**:
- 10 nodes
- Brief disconnect (5 seconds)
- Should maintain existing nexus

**Test Steps**:
```rust
#[tokio::test]
#[ignore = "Stress test - requires dedicated resources"]
async fn test_transient_partition_no_reelection() {
    let cluster = TestCluster::spawn(10).await.unwrap();
    cluster.wait_for_mesh(2, 30).await.unwrap();

    // Ensure election completes first
    cluster.node(0).enr_bridge.trigger_election("test-region".to_string()).await.unwrap();
    tokio::time::sleep(Duration::from_secs(35)).await; // Wait for election

    let initial_nexus = cluster.node(0).enr_bridge.current_nexus().await;
    assert!(initial_nexus.is_some(), "Should have nexus before partition");

    // Create brief partition
    let controller = PartitionController::new();
    controller.create_partition(
        vec![vec![0, 1, 2, 3, 4], vec![5, 6, 7, 8, 9]],
        &cluster
    );

    // Brief disconnect - shorter than candidacy phase (10s)
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Heal immediately
    controller.heal_partition();

    // Wait for gossip to resume
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Nexus should NOT have changed
    let final_nexus = cluster.node(0).enr_bridge.current_nexus().await;
    assert_eq!(
        initial_nexus, final_nexus,
        "Brief partition should not trigger re-election"
    );

    // No elections should be in progress
    for i in 0..10 {
        let in_progress = cluster.node(i).enr_bridge.election_in_progress().await;
        assert!(!in_progress, "Node {} should not have election in progress", i);
    }

    cluster.shutdown().await;
}
```

**Threshold Analysis**:
- 5 seconds < `CANDIDACY_PHASE_MS` (10 seconds)
- Connection failures may be recorded but `FAILURE_THRESHOLD` not reached
- Heartbeat loss detected but Raft timeout (150-300ms) allows brief recovery

### Scenario 4: Cascading Failure

**Purpose**: Verify graceful degradation as nodes fail one by one.

**Setup**:
- 10 nodes
- Remove nodes one at a time
- Verify quorum maintained until 5 nodes remain

**Test Steps**:
```rust
#[tokio::test]
#[ignore = "Stress test - requires dedicated resources"]
async fn test_cascading_failure_quorum() {
    let cluster = TestCluster::spawn(10).await.unwrap();
    cluster.wait_for_mesh(2, 30).await.unwrap();

    // Initial election
    cluster.node(0).enr_bridge.trigger_election("cascade-test".to_string()).await.unwrap();
    tokio::time::sleep(Duration::from_secs(35)).await;

    let mut remaining_nodes: Vec<usize> = (0..10).collect();
    let controller = PartitionController::new();

    // Remove nodes one by one from index 9 down to 5
    for removed_count in 1..=5 {
        let node_to_remove = 10 - removed_count;

        // Partition out the node
        remaining_nodes.retain(|&n| n != node_to_remove);
        controller.create_partition(
            vec![remaining_nodes.clone(), vec![node_to_remove]],
            &cluster
        );

        // Wait for detection
        tokio::time::sleep(Duration::from_secs(10)).await;

        // Check if remaining nodes still have quorum
        let node0_nexus = cluster.node(0).enr_bridge.current_nexus().await;

        if remaining_nodes.len() >= 5 {
            // Should still have nexus (quorum possible)
            assert!(
                node0_nexus.is_some(),
                "With {} nodes remaining, should have nexus",
                remaining_nodes.len()
            );
        }

        // Log progress
        println!(
            "Removed node {}, {} remaining, nexus: {:?}",
            node_to_remove,
            remaining_nodes.len(),
            node0_nexus
        );
    }

    // Remove one more to break quorum (4 remaining)
    remaining_nodes.retain(|&n| n != 4);
    controller.create_partition(
        vec![remaining_nodes.clone(), vec![4, 5, 6, 7, 8, 9]],
        &cluster
    );

    // Attempt new election - should fail due to insufficient votes
    let result = cluster.node(0).enr_bridge.trigger_election("no-quorum".to_string()).await;

    // Wait for election to timeout
    tokio::time::sleep(Duration::from_secs(35)).await;

    // Finalize should fail
    let finalize_result = cluster.node(0).enr_bridge.finalize_election().await;
    assert!(
        matches!(finalize_result, Err(ElectionError::InsufficientVotes)),
        "Election should fail without quorum"
    );

    cluster.shutdown().await;
}
```

**Quorum Calculation**:
- `MIN_VOTE_FRACTION` = 0.5
- With N participants, need ceil(N * 0.5) votes
- 10 nodes: need 5 votes
- 5 nodes: need 3 votes
- 4 nodes: need 2 votes (but may not achieve if partitioned)

---

## Recovery Verification Checklist

### 1. Gossip Resumes
```rust
async fn verify_gossip_resumed(cluster: &TestCluster, timeout_secs: u64) -> bool {
    // Send test message from each node
    for i in 0..cluster.node_count() {
        cluster.node(i).handle
            .publish("test-topic", format!("ping-from-{}", i).as_bytes())
            .await
            .ok();
    }

    // Wait and collect received messages
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Verify all nodes received messages from all others
    // (Implementation depends on message tracking infrastructure)
    true
}
```

### 2. State Converges
```rust
async fn verify_state_converged(cluster: &TestCluster) -> bool {
    // Check nexus agreement
    let nexuses: Vec<_> = futures::future::join_all(
        (0..cluster.node_count()).map(|i| cluster.node(i).enr_bridge.current_nexus())
    ).await;

    let all_same_nexus = nexuses.windows(2).all(|w| w[0] == w[1]);

    // Check role consistency
    let roles: Vec<_> = futures::future::join_all(
        (0..cluster.node_count()).map(|i| cluster.node(i).enr_bridge.current_role())
    ).await;

    // Exactly one Nexus, rest are Leaf
    let nexus_count = roles.iter().filter(|r| r.is_nexus()).count();

    all_same_nexus && nexus_count == 1
}
```

### 3. Elections Finalize
```rust
async fn verify_elections_finalized(cluster: &TestCluster) -> bool {
    for i in 0..cluster.node_count() {
        if cluster.node(i).enr_bridge.election_in_progress().await {
            return false;
        }
    }
    true
}
```

### 4. Credits Balance
```rust
async fn verify_credits_balance(cluster: &TestCluster, tolerance_percent: f64) -> bool {
    // Get all balances
    let balances: Vec<u64> = futures::future::join_all(
        (0..cluster.node_count()).map(|i| async move {
            cluster.node(i).enr_bridge.credits.local_balance().await.amount
        })
    ).await;

    // Calculate expected total (accounting for entropy tax)
    let initial_total = INITIAL_NODE_CREDITS * cluster.node_count() as u64;
    let actual_total: u64 = balances.iter().sum();

    // Credits only decrease (due to entropy tax), never increase beyond initial
    let deviation = (initial_total - actual_total) as f64 / initial_total as f64;
    deviation <= tolerance_percent
}
```

---

## Timeout Values Summary

| Parameter | Value | Source |
|-----------|-------|--------|
| Election total timeout | 30,000ms | `ELECTION_TIMEOUT_MS` |
| Candidacy phase | 10,000ms | `CANDIDACY_PHASE_MS` |
| Voting phase | 15,000ms | `VOTING_PHASE_MS` |
| Quorum fraction | 0.5 (50%) | `MIN_VOTE_FRACTION` |
| Raft heartbeat (test) | 50ms | `RaftConfig::for_testing()` |
| Raft election timeout | 150-300ms | `RaftConfig::for_testing()` |
| Connection idle | 30,000ms | `NetworkConfig.idle_timeout_secs` |
| Gossip mesh stabilization | 500ms | TestCluster wait |

**Recommended Test Timeouts**:
- Simple partition recovery: 60 seconds
- Asymmetric partition detection: 45 seconds
- Transient partition test: 15 seconds
- Cascading failure (per step): 10 seconds
- Full convergence verification: 120 seconds

---

## Implementation Priority

1. **Phase 3.1**: Implement `PartitionController` at application level
2. **Phase 3.2**: Add message filtering hooks to NetworkService
3. **Phase 3.3**: Implement Scenarios 1-4 as integration tests
4. **Phase 3.4**: Add convergence verification utilities
5. **Stretch**: Proxy-based partition for TCP-level testing

---

## Files to Modify/Create

### New Files
- `/crates/mycelial-network/tests/partition_controller.rs` - Partition simulation
- `/crates/mycelial-network/tests/stress_partition.rs` - Partition test scenarios
- `/crates/mycelial-network/tests/helpers/convergence.rs` - Verification utilities

### Files to Extend
- `/crates/mycelial-network/src/service.rs` - Add message filter callback
- `/crates/mycelial-network/tests/helpers/cluster.rs` - Add partition methods

---

## Metrics to Collect

During partition tests, collect:
1. Time to detect partition (SeptalGate trip)
2. Time to recover (SeptalGate Open again)
3. Number of failed message deliveries
4. Election count during partition
5. Credit balance drift
6. Gossipsub mesh reformation time
