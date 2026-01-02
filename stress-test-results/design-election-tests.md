# Nexus Election Stress Tests Design

**Phase 3 Stress Testing Swarm - Test Architect Specification**

**Document Version:** 1.0
**Date:** 2026-01-01
**Status:** Design Phase

---

## Overview

This document specifies stress tests for the Nexus election system in `univrs-network`. The election mechanism enables distributed consensus for selecting a Nexus coordinator across the P2P network.

### Election System Summary

Based on analysis of `/home/ardeshir/repos/univrs-network/crates/mycelial-network/src/enr_bridge/nexus.rs`:

**Timing Constants:**
- `ELECTION_TIMEOUT_MS`: 30,000ms (total election timeout)
- `CANDIDACY_PHASE_MS`: 10,000ms (candidacy collection window)
- `VOTING_PHASE_MS`: 15,000ms (voting window)
- `MIN_VOTE_FRACTION`: 0.5 (50% quorum requirement)

**Election Phases:**
1. `Idle` - No election active
2. `Candidacy` - Collecting candidate submissions (10s)
3. `Voting` - Collecting votes (15s)
4. `Confirming` - Announcing results

**Message Types:**
- `ElectionAnnouncement` - Initiates election
- `NexusCandidacy` - Candidate submission with metrics
- `ElectionVote` - Vote for a candidate
- `ElectionResult` - Winner announcement

**Error Conditions:**
- `ElectionInProgress` - Election already running
- `NoCandidates` - No eligible candidates
- `InsufficientVotes` - Failed to reach quorum
- `IneligibleCandidate` - Candidate metrics below threshold

---

## Test Scenario 1: Basic Election (10 Nodes)

### Description
Verify that a 10-node cluster with no existing Nexus successfully elects exactly one winner.

### Setup Conditions
```rust
// Spawn 10-node cluster
let cluster = TestCluster::spawn(10).await?;
cluster.wait_for_mesh(3, 30).await?;  // Each node has 3+ peers

// Set all nodes as eligible candidates
for i in 0..10 {
    cluster.node(i).enr_bridge.update_metrics(LocalNodeMetrics {
        uptime: 0.96 + (i as f64 * 0.003),  // Varied scores
        bandwidth: 20_000_000 + (i as u64 * 1_000_000),
        reputation: 0.80 + (i as f64 * 0.015),
        connection_count: 25,
    }).await;
}

// Verify no nexus currently exists
for i in 0..10 {
    assert!(cluster.node(i).enr_bridge.election.current_nexus().await.is_none());
}
```

### Trigger Mechanism
```rust
// Node 0 triggers election
let election_id = cluster.node(0).enr_bridge.trigger_election("stress-test-region".to_string()).await?;
```

### Expected Outcome
1. Election announcement propagates to all 10 nodes within 2 seconds
2. All 10 nodes submit candidacy within candidacy phase (10s)
3. All nodes cast votes for the candidate with highest `election_score`
4. Exactly one winner is determined and announced
5. All nodes update their `current_nexus` to the winner
6. Winner has role `NexusRoleType::Nexus`, others have `Leaf` role

### Pass/Fail Criteria
| Criterion | Pass Condition |
|-----------|---------------|
| Announcement Propagation | All 10 nodes see `election_in_progress()` = true within 2s |
| Candidate Count | All 10 eligible nodes submit candidacy |
| Vote Count | >= 5 votes received (50% quorum) |
| Winner Uniqueness | Exactly one winner across all nodes |
| State Consistency | All nodes agree on same winner |
| Role Assignment | Winner is Nexus, 9 others are Leaf |

### Timeout
- **Total Test Timeout:** 45 seconds
- **Phase Timeouts:**
  - Mesh formation: 30s
  - Announcement propagation: 5s
  - Candidacy phase: 12s (10s + 2s buffer)
  - Voting phase: 18s (15s + 3s buffer)
  - Result propagation: 5s

---

## Test Scenario 2: Multiple Simultaneous Candidates

### Description
Verify election correctness when 5 nodes announce candidacy at exactly the same time (within 100ms window).

### Setup Conditions
```rust
let cluster = TestCluster::spawn(8).await?;
cluster.wait_for_mesh(2, 20).await?;

// Set nodes 0-4 as eligible, nodes 5-7 as ineligible
for i in 0..5 {
    cluster.node(i).enr_bridge.update_metrics(LocalNodeMetrics {
        uptime: 0.97,
        bandwidth: 30_000_000,
        reputation: 0.85 + (i as f64 * 0.02),  // Varied reputation
        connection_count: 20,
    }).await;
}

for i in 5..8 {
    cluster.node(i).enr_bridge.update_metrics(LocalNodeMetrics {
        uptime: 0.70,  // Ineligible
        bandwidth: 5_000_000,
        reputation: 0.50,
        connection_count: 5,
    }).await;
}
```

### Trigger Mechanism
```rust
use tokio::time::{sleep, Duration};
use tokio::sync::Barrier;
use std::sync::Arc;

let barrier = Arc::new(Barrier::new(5));

// 5 nodes trigger simultaneously
let mut handles = vec![];
for i in 0..5 {
    let b = barrier.clone();
    let node = cluster.node(i).enr_bridge.clone();
    handles.push(tokio::spawn(async move {
        b.wait().await;  // Synchronize all 5
        node.trigger_election(format!("simultaneous-{}", i)).await
    }));
}

let results = futures::future::join_all(handles).await;
```

### Expected Outcome
1. Only ONE election proceeds (others get `ElectionInProgress` error)
2. The first election_id wins, later ones are rejected
3. All 5 eligible nodes can still submit candidacy to the winning election
4. Ineligible nodes (5-7) do not submit candidacy
5. Node with highest score wins

### Pass/Fail Criteria
| Criterion | Pass Condition |
|-----------|---------------|
| Single Election | Only 1 election_id is active at any time |
| Error Handling | 4 out of 5 trigger attempts return `ElectionInProgress` |
| Candidacy Filter | Ineligible nodes (5-7) do not appear in candidates list |
| Deterministic Winner | Same winner on all retries with same setup |
| No Deadlock | Election completes within timeout |

### Timeout
- **Total Test Timeout:** 50 seconds
- **Simultaneity Window:** 100ms (all triggers within this window)

---

## Test Scenario 3: Nexus Failure and Re-election

### Description
Simulate current Nexus node failure and verify automatic or triggered re-election succeeds.

### Setup Conditions
```rust
let cluster = TestCluster::spawn(6).await?;
cluster.wait_for_mesh(2, 20).await?;

// Set all nodes eligible
for i in 0..6 {
    cluster.node(i).enr_bridge.update_metrics(LocalNodeMetrics {
        uptime: 0.97,
        bandwidth: 25_000_000,
        reputation: 0.85,
        connection_count: 20,
    }).await;
}

// Complete initial election, ensure node 0 wins
cluster.node(0).enr_bridge.update_metrics(LocalNodeMetrics {
    uptime: 0.99,
    bandwidth: 100_000_000,
    reputation: 0.99,
    connection_count: 50,
}).await;

let election_id = cluster.node(0).enr_bridge.trigger_election("nexus-failure-test".to_string()).await?;

// Wait for election to complete
wait_for_election_complete(&cluster, election_id, 35).await?;

// Verify node 0 is nexus
let nexus = cluster.node(0).enr_bridge.election.current_nexus().await;
assert_eq!(nexus, Some(cluster.node(0).node_id()));
```

### Trigger Mechanism
```rust
// Simulate nexus failure by shutting down node 0
cluster.shutdown_node(0).await;

// Wait for failure detection (depends on heartbeat/gossip)
tokio::time::sleep(Duration::from_secs(5)).await;

// Surviving node triggers re-election
let new_election_id = cluster.node(1).enr_bridge.trigger_election("re-election".to_string()).await?;
```

### Expected Outcome
1. After node 0 shutdown, remaining nodes detect failure
2. Re-election can be triggered by any surviving node
3. New election excludes the dead node
4. New winner is selected from remaining 5 nodes
5. All surviving nodes update to new nexus

### Pass/Fail Criteria
| Criterion | Pass Condition |
|-----------|---------------|
| Failure Detection | Nodes detect node 0 is unreachable within 10s |
| Re-election Success | New election starts and completes |
| Dead Node Excluded | Node 0 does not appear as candidate in re-election |
| New Winner Valid | New winner is from nodes 1-5 |
| State Recovery | All surviving nodes agree on new nexus |
| No Orphan State | No node still considers dead node as nexus after re-election |

### Timeout
- **Total Test Timeout:** 90 seconds
- **Phase Timeouts:**
  - Initial election: 40s
  - Node shutdown: 5s
  - Failure detection: 15s
  - Re-election: 40s

---

## Test Scenario 4: Split Brain During Election

### Description
Simulate network partition during an active election and verify system behavior (should either complete on one side or fail safely on both).

### Setup Conditions
```rust
let cluster = TestCluster::spawn(8).await?;
cluster.wait_for_mesh(3, 25).await?;

// All nodes eligible
for i in 0..8 {
    cluster.node(i).enr_bridge.update_metrics(LocalNodeMetrics {
        uptime: 0.97,
        bandwidth: 25_000_000,
        reputation: 0.85 + (i as f64 * 0.01),
        connection_count: 20,
    }).await;
}
```

### Trigger Mechanism
```rust
// Start election
let election_id = cluster.node(0).enr_bridge.trigger_election("split-brain-test".to_string()).await?;

// Wait for announcement to propagate
tokio::time::sleep(Duration::from_millis(1500)).await;

// Create partition: nodes 0-3 vs nodes 4-7
// Inject network fault via transport layer
cluster.partition(
    vec![0, 1, 2, 3],  // Partition A
    vec![4, 5, 6, 7],  // Partition B
).await;

// Let election proceed under partition
tokio::time::sleep(Duration::from_secs(20)).await;

// Heal partition
cluster.heal_partition().await;

// Wait for stabilization
tokio::time::sleep(Duration::from_secs(10)).await;
```

### Expected Outcome

**Acceptable Outcome A (Majority Partition Wins):**
- Partition with more nodes reaches quorum
- Winner elected in majority partition
- After heal, minority partition accepts majority's result

**Acceptable Outcome B (Both Partitions Fail):**
- Neither partition reaches quorum (50% of original 8 = 4 votes needed)
- Both sides get `InsufficientVotes` error
- After heal, new election can be triggered successfully

**Unacceptable Outcome:**
- Two different winners elected (actual split brain)
- Permanent inconsistency after heal

### Pass/Fail Criteria
| Criterion | Pass Condition |
|-----------|---------------|
| No Dual Winners | Never have 2 different `current_nexus` values post-heal |
| Quorum Enforcement | Elections without quorum fail with `InsufficientVotes` |
| State Convergence | All 8 nodes agree on nexus within 30s after heal |
| No Stale State | `election_in_progress()` returns false for all nodes after resolution |

### Timeout
- **Total Test Timeout:** 120 seconds
- **Phase Timeouts:**
  - Initial propagation: 5s
  - Partition duration: 20s
  - Post-heal stabilization: 30s
  - Final verification: 10s

---

## Test Scenario 5: Rapid Election Storm

### Description
Trigger 10 elections within 60 seconds and verify system stability - no crashes, no resource leaks, eventual consistency.

### Setup Conditions
```rust
let cluster = TestCluster::spawn(5).await?;
cluster.wait_for_mesh(2, 15).await?;

// All nodes eligible
for i in 0..5 {
    cluster.node(i).enr_bridge.update_metrics(LocalNodeMetrics {
        uptime: 0.97 + (i as f64 * 0.005),
        bandwidth: 25_000_000,
        reputation: 0.85 + (i as f64 * 0.02),
        connection_count: 20,
    }).await;
}
```

### Trigger Mechanism
```rust
let mut election_results = Vec::new();
let start_time = std::time::Instant::now();

// Trigger 10 elections in rapid succession
for round in 0..10 {
    let node_index = round % 5;  // Rotate trigger across nodes

    // Wait for any in-progress election to complete
    wait_for_no_election(&cluster, 35).await?;

    let election_id = cluster.node(node_index)
        .enr_bridge
        .trigger_election(format!("rapid-{}", round))
        .await?;

    // Wait for this election to complete
    let winner = wait_for_election_complete(&cluster, election_id, 35).await?;

    election_results.push((round, election_id, winner));

    // Small delay between elections (not too long, stress test)
    tokio::time::sleep(Duration::from_millis(500)).await;
}

let total_duration = start_time.elapsed();
assert!(total_duration < Duration::from_secs(60), "All 10 elections must complete within 60s");
```

### Expected Outcome
1. All 10 elections complete without error
2. Each election produces exactly one winner
3. No memory leaks (heap usage stable)
4. No thread/task leaks (tokio runtime healthy)
5. All elections complete within 60 second window

### Pass/Fail Criteria
| Criterion | Pass Condition |
|-----------|---------------|
| All Complete | 10 out of 10 elections finish successfully |
| No Errors | Zero `ElectionError` exceptions (except `ElectionInProgress` which is expected) |
| Time Bound | Total time < 60 seconds |
| Memory Stable | Heap growth < 50MB over all elections |
| No Hangs | No election takes > 35s to complete |
| State Clean | `election_in_progress()` = false for all nodes at end |
| Winner Consistency | Each election has same winner across all nodes |

### Timeout
- **Total Test Timeout:** 90 seconds (60s + 30s buffer)
- **Per-Election Timeout:** 35 seconds
- **Inter-Election Delay:** 500ms minimum

---

## Test Infrastructure Requirements

### TestCluster Extensions Needed

```rust
impl TestCluster {
    /// Shutdown a specific node (simulating failure)
    pub async fn shutdown_node(&mut self, index: usize) -> Result<(), Error>;

    /// Create network partition between two sets of nodes
    pub async fn partition(&mut self, group_a: Vec<usize>, group_b: Vec<usize>) -> Result<(), Error>;

    /// Heal network partition, restore full connectivity
    pub async fn heal_partition(&mut self) -> Result<(), Error>;

    /// Get node ID for a specific node
    pub fn node_id(&self, index: usize) -> NodeId;
}

impl TestNode {
    /// Get this node's NodeId
    pub fn node_id(&self) -> NodeId;
}
```

### Helper Functions Needed

```rust
/// Wait for election to complete with winner
async fn wait_for_election_complete(
    cluster: &TestCluster,
    election_id: u64,
    timeout_secs: u64,
) -> Result<NodeId, Error>;

/// Wait for no election to be in progress
async fn wait_for_no_election(
    cluster: &TestCluster,
    timeout_secs: u64,
) -> Result<(), Error>;

/// Verify all nodes agree on current nexus
async fn verify_nexus_consensus(
    cluster: &TestCluster,
) -> Result<Option<NodeId>, Error>;
```

### Metrics Collection

Each test should collect:
- Election duration (ms)
- Message count (announcements, candidacies, votes, results)
- Memory usage before/after
- Number of retries/errors
- Time to convergence post-election

---

## Test Execution Matrix

| Scenario | Node Count | Estimated Duration | Priority |
|----------|------------|-------------------|----------|
| Basic Election | 10 | 45s | P0 (Critical) |
| Multiple Candidates | 8 | 50s | P0 (Critical) |
| Nexus Failure | 6 | 90s | P1 (High) |
| Split Brain | 8 | 120s | P1 (High) |
| Rapid Elections | 5 | 90s | P2 (Medium) |

**Total Estimated Time:** ~7 minutes for full suite

---

## Implementation Notes

### Files to Reference
- Election logic: `/home/ardeshir/repos/univrs-network/crates/mycelial-network/src/enr_bridge/nexus.rs`
- Messages: `/home/ardeshir/repos/univrs-network/crates/mycelial-network/src/enr_bridge/messages.rs`
- Test helpers: `/home/ardeshir/repos/univrs-network/crates/mycelial-network/tests/helpers/cluster.rs`
- Existing tests: `/home/ardeshir/repos/univrs-network/crates/mycelial-network/tests/gate_election.rs`

### Eligibility Thresholds
From `univrs_enr::nexus::is_nexus_eligible`:
- Uptime: > 0.95 (95%)
- Bandwidth: > 10,000,000 (10 MB/s)
- Reputation: > 0.70 (70%)

### Election Score Calculation
Uses `calculate_election_score(&NexusCandidate)` which weights:
- Uptime (high weight)
- Bandwidth (medium weight)
- Reputation (high weight)
- Current leaf count (low weight for load balancing)

---

## Next Steps

1. **Implement Test Infrastructure:** Add `partition()` and `heal_partition()` methods to TestCluster
2. **Create Test File:** `stress_test_election.rs` in tests directory
3. **Add Metrics Collection:** Instrument tests with timing and memory metrics
4. **CI Integration:** Add stress tests to nightly CI pipeline (not PR checks due to duration)
5. **Chaos Engineering:** Consider adding random delays/failures to stress tests

---

*Document prepared by Test Architect - Phase 3 Stress Testing Swarm*
