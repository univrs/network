# Septal Gate (Circuit Breaker) Stress Test Design

## Overview

This document defines stress test scenarios for the Septal Gate implementation in the univrs-network P2P layer. Septal gates act as circuit breakers that isolate unhealthy nodes to protect network integrity.

**Implementation References:**
- Core types: `/home/ardeshir/repos/univrs-enr/src/septal/gate.rs`
- Woronin body: `/home/ardeshir/repos/univrs-enr/src/septal/woronin.rs`
- Healing: `/home/ardeshir/repos/univrs-enr/src/septal/healing.rs`
- Network manager: `/home/ardeshir/repos/univrs-network/crates/mycelial-network/src/enr_bridge/septal.rs`

---

## State Machine Diagram

```
                         +---------+
                         |  Open   |
                         | (Normal)|
                         +----+----+
                              |
          failures >= 5 AND   |
          weighted_score >= 0.7
                              |
                              v
                         +---------+
              +----------|  Closed |<---------+
              |          |(Woronin)|          |
              |          +----+----+          |
              |               |               |
              |    60s timeout elapsed        |
              |               |               |
              |               v               |
              |          +---------+          |
              |          |HalfOpen |          |
              |          |(Testing)|          |
              |          +----+----+          |
              |               |               |
         recovery             |           recovery
          passes              |            fails
              |               |               |
              v               |               |
         +---------+          |          +---------+
         |  Open   |<---------+          |  Closed |
         +---------+   (healthy)         +---------+
```

---

## Configuration Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `FAILURE_THRESHOLD` | 5 | Consecutive failures before gate trips |
| `RECOVERY_TIMEOUT_MS` | 60,000 | Wait time before attempting half-open (60s) |
| `HALF_OPEN_TEST_INTERVAL_MS` | 10,000 | Interval between recovery tests (10s) |
| `ISOLATION_THRESHOLD` | 0.7 | Weighted health score threshold |
| `PING_TIMEOUT_MS` | 5,000 | Timeout for health pings (5s) |
| `HEALTH_CHECK_INTERVAL_MS` | 10,000 | Interval between health checks (10s) |

### SeptalGateConfig Weights

| Weight | Default | Description |
|--------|---------|-------------|
| `timeout_weight` | 0.4 | 40% weight for timeout failures |
| `credit_default_weight` | 0.3 | 30% weight for credit defaults |
| `reputation_weight` | 0.3 | 30% weight for reputation drops |

**Constraint:** `timeout_weight + credit_default_weight + reputation_weight = 1.0`

---

## Test Scenarios

### Scenario 1: Timeout Trigger

**Description:** Node stops responding, gate closes after threshold failures.

**Setup:**
- Spawn 5-node cluster
- Wait for mesh formation
- Select target node

**Test Steps:**
1. Simulate target node becoming unresponsive (stop heartbeats/pings)
2. Wait for health checks to accumulate failures
3. After 5 consecutive timeout failures, verify gate transitions Open -> Closed
4. Verify Woronin body activated for target node
5. Verify transactions to/from target are blocked

**Assertions:**
```rust
// Before failure
assert!(manager.allows_traffic(&target_node).await);
assert_eq!(manager.get_gate_state(&target_node).await, SeptalGateState::Open);

// Record 5 failures (simulating timeout)
for _ in 0..FAILURE_THRESHOLD {
    manager.record_failure(target_node, "Connection timeout").await;
}

// After threshold reached
assert!(!manager.allows_traffic(&target_node).await);
assert_eq!(manager.get_gate_state(&target_node).await, SeptalGateState::Closed);
assert!(manager.is_isolated(&target_node).await);
assert!(manager.should_block_transaction(&target_node, &other_node).await);
```

**Pass Criteria:**
- [ ] Gate transitions to Closed after exactly 5 failures
- [ ] Transition broadcast received by all cluster nodes
- [ ] Woronin body blocks transactions involving isolated node
- [ ] Stats show: `closed_gates = 1`, `isolated_nodes = 1`

**Fail Criteria:**
- [ ] Gate closes before 5 failures (threshold too low)
- [ ] Gate fails to close after 5+ failures (threshold ignored)
- [ ] Other nodes not aware of state change (gossip failure)

---

### Scenario 2: Credit Default Trigger

**Description:** Node balance goes negative, triggering gate closure.

**Setup:**
- Spawn 5-node cluster with initial credits
- Target node has 100 credits initially

**Test Steps:**
1. Simulate credit depletion on target node
2. Set credit_score to 1.0 in health status (representing default)
3. Verify health status `should_isolate()` returns true when weighted score >= 0.7
4. Record failures as credit checks fail
5. Verify gate closes after threshold

**Health Score Calculation:**
```
weighted_score = timeout_score * 0.4 + credit_score * 0.3 + reputation_score * 0.3
              = 0.0 * 0.4 + 1.0 * 0.3 + 0.0 * 0.3
              = 0.3 (below 0.7 threshold)

// Combined with timeout:
weighted_score = 0.5 * 0.4 + 1.0 * 0.3 + 0.5 * 0.3
              = 0.2 + 0.3 + 0.15
              = 0.65 (still below 0.7)

// Full failure:
weighted_score = 1.0 * 0.4 + 1.0 * 0.3 + 1.0 * 0.3
              = 1.0 (above 0.7)
```

**Assertions:**
```rust
let unhealthy = HealthStatus {
    is_healthy: false,
    timeout_score: 0.5,
    credit_score: 1.0,  // Credit default
    reputation_score: 0.5,
    last_check: Timestamp::now(),
};

// Score = 0.2 + 0.3 + 0.15 = 0.65, below threshold
assert!(!unhealthy.should_isolate(&config));

let severe = HealthStatus {
    timeout_score: 1.0,
    credit_score: 1.0,
    reputation_score: 1.0,
    ..unhealthy
};

// Score = 1.0, above threshold
assert!(severe.should_isolate(&config));
```

**Pass Criteria:**
- [ ] Credit default alone (0.3 score) does not trigger isolation
- [ ] Credit default + partial other failures (0.65) does not trigger
- [ ] Full failure (1.0 score) triggers isolation after threshold failures
- [ ] Gate closes only after 5 consecutive health check failures

---

### Scenario 3: Reputation Drop Trigger

**Description:** Node misbehaves, reputation drops, gate closes.

**Setup:**
- Spawn 5-node cluster
- All nodes start with reputation 1.0

**Test Steps:**
1. Simulate misbehavior detection (e.g., invalid messages)
2. Decrease reputation_score in health status
3. Monitor weighted health score
4. Verify gate closes when score >= 0.7 and failure count >= 5

**Misbehavior Scenarios:**
- Sending malformed messages
- Double-spending credits
- Violating protocol rules
- Failing to respond to required messages

**Assertions:**
```rust
// Reputation only failure
let bad_rep = HealthStatus {
    is_healthy: false,
    timeout_score: 0.0,
    credit_score: 0.0,
    reputation_score: 1.0,  // Bad reputation
    last_check: Timestamp::now(),
};

// Score = 0.3, well below threshold
assert!(!bad_rep.should_isolate(&config));

// Multi-factor failure
let multi_fail = HealthStatus {
    is_healthy: false,
    timeout_score: 0.8,
    credit_score: 0.8,
    reputation_score: 0.8,
    last_check: Timestamp::now(),
};

// Score = 0.32 + 0.24 + 0.24 = 0.8, above threshold
assert!(multi_fail.should_isolate(&config));
```

**Pass Criteria:**
- [ ] Single factor (reputation only) does not trigger isolation
- [ ] Multi-factor failure (timeout + credit + reputation) triggers isolation
- [ ] Reputation recovery resets failure count

---

### Scenario 4: Recovery Cycle

**Description:** Closed gate waits timeout, enters half-open, tests recovery.

**Setup:**
- Spawn 3-node cluster
- Trip gate for target node (accumulate 5 failures)
- Mock health checker for controlled testing

**Test Steps:**
1. Verify gate is Closed
2. Wait RECOVERY_TIMEOUT_MS (60 seconds) - or mock time
3. Verify gate transitions Closed -> HalfOpen
4. **Recovery Success Path:**
   - Health check returns healthy
   - Verify gate transitions HalfOpen -> Open
   - Verify Woronin body deactivated
5. **Recovery Failure Path:**
   - Health check returns unhealthy
   - Verify gate transitions HalfOpen -> Closed
   - Verify Woronin body remains active
   - Verify new isolation_start timestamp

**State Transition Timeline:**
```
T=0:      Gate trips -> Closed (isolation_start set)
T=60s:    Timeout elapsed -> HalfOpen (attempt_half_open() returns true)
T=60+10s: Health check -> Open (if healthy) OR Closed (if unhealthy)
```

**Assertions:**
```rust
// Initial: gate is closed
let node = NodeId::from_bytes([1u8; 32]);
let mut gate = SeptalGate::new(node);
gate.trip();
assert!(gate.state.is_closed());

// Before timeout: cannot transition
assert!(!gate.attempt_half_open());
assert!(gate.state.is_closed());

// After timeout (mock): can transition
// In test, manually set isolation_start to past
gate.isolation_start = Some(Timestamp {
    millis: Timestamp::now().millis - RECOVERY_TIMEOUT_MS - 1,
});
assert!(gate.attempt_half_open());
assert!(gate.state.is_half_open());

// Recovery success
gate.recover();
assert!(gate.state.is_open());
assert_eq!(gate.failure_count, 0);
assert!(gate.isolation_start.is_none());

// Recovery failure
gate.state = SeptalGateState::HalfOpen;
gate.fail_recovery();
assert!(gate.state.is_closed());
assert!(gate.isolation_start.is_some()); // New timestamp
```

**Pass Criteria:**
- [ ] Cannot transition to HalfOpen before 60s timeout
- [ ] Can transition to HalfOpen after 60s timeout
- [ ] Successful recovery: HalfOpen -> Open, Woronin deactivated
- [ ] Failed recovery: HalfOpen -> Closed, new timeout starts
- [ ] State changes broadcast to all nodes

---

### Scenario 5: Cascade Prevention

**Description:** One node fails, verify isolation does not cascade to healthy nodes.

**Setup:**
- Spawn 5-node cluster (A, B, C, D, E)
- All nodes connected in mesh

**Test Steps:**
1. Isolate node A (trip its gate from all other nodes' perspective)
2. Verify B, C, D, E can still communicate
3. Simulate node B interacting with isolated node A
4. Verify node B does NOT get isolated (no cascade)
5. Verify transactions between healthy nodes succeed
6. Verify only transactions involving A are blocked

**Network Topology:**
```
     A (isolated)
    /|\
   B-C-D
    \|/
     E

After A isolation:
- B, C, D, E fully connected
- A cannot transact with anyone
- B <-> C, C <-> D, etc. all work
```

**Assertions:**
```rust
// Setup: 5 nodes
let nodes: Vec<NodeId> = (0..5).map(|i| NodeId::from_bytes([i as u8; 32])).collect();
let [a, b, c, d, e] = [nodes[0], nodes[1], nodes[2], nodes[3], nodes[4]];

// Isolate node A from all nodes' perspective
for manager in &managers {
    for _ in 0..FAILURE_THRESHOLD {
        manager.record_failure(a, "cascade test").await;
    }
}

// Verify A is isolated
assert!(managers[0].is_isolated(&a).await);
assert!(managers[1].should_block_transaction(&a, &b).await);

// Verify healthy nodes NOT isolated
assert!(!managers[0].is_isolated(&b).await);
assert!(!managers[0].is_isolated(&c).await);
assert!(!managers[0].is_isolated(&d).await);
assert!(!managers[0].is_isolated(&e).await);

// Verify healthy nodes can transact
assert!(!managers[0].should_block_transaction(&b, &c).await);
assert!(!managers[0].should_block_transaction(&c, &d).await);
assert!(!managers[0].should_block_transaction(&d, &e).await);
assert!(!managers[0].should_block_transaction(&e, &b).await);

// Verify stats
let stats = managers[0].stats().await;
assert_eq!(stats.closed_gates, 1);  // Only A
assert_eq!(stats.isolated_nodes, 1);
assert_eq!(stats.open_gates, 4);    // B, C, D, E
```

**Cascade Prevention Mechanisms:**
1. **Per-node gate state**: Each node's gate is independent
2. **Local failure tracking**: Failures are counted per-peer, not globally
3. **Gossip verification**: Nodes verify state changes before applying
4. **No automatic propagation**: Isolation of A does not automatically isolate A's peers

**Pass Criteria:**
- [ ] Single node isolation does not affect other nodes' states
- [ ] Healthy nodes maintain full connectivity
- [ ] Only direct transactions with isolated node are blocked
- [ ] Stats correctly reflect single isolation

---

## Stress Test Variations

### High Concurrency Test

**Setup:** 20 nodes with rapid failure injection

**Steps:**
1. Spawn 20-node cluster
2. Randomly inject failures across all nodes (100 failures/second)
3. Verify system remains stable
4. Count gates in each state after 60 seconds
5. Verify recovery works after failure injection stops

**Metrics to Monitor:**
- Gate state distribution (open/half-open/closed)
- Recovery success rate
- Gossip message latency
- Memory usage

### Rapid State Transitions

**Setup:** Single node with mocked fast timers

**Steps:**
1. Trip gate (Open -> Closed)
2. Wait mock 60s (Closed -> HalfOpen)
3. Fail recovery (HalfOpen -> Closed)
4. Wait mock 60s (Closed -> HalfOpen)
5. Pass recovery (HalfOpen -> Open)
6. Repeat cycle 100 times
7. Verify all transitions recorded correctly

### Network Partition Test

**Setup:** 5 nodes split into 2 partitions

**Steps:**
1. Create partition: [A, B] and [C, D, E]
2. Verify cross-partition communication fails
3. Verify gates eventually close for unreachable nodes
4. Heal partition
5. Verify gates recover to Open state

---

## Woronin Body Verification

### Transaction Blocking

```rust
#[test]
fn test_woronin_blocks_transactions() {
    let mut woronin = WoroninManager::new();
    let isolated = NodeId::from_bytes([1u8; 32]);
    let healthy1 = NodeId::from_bytes([2u8; 32]);
    let healthy2 = NodeId::from_bytes([3u8; 32]);

    // Activate Woronin body for isolated node
    woronin.activate(isolated, "test isolation");

    // Transactions involving isolated node blocked
    assert!(woronin.should_block(&isolated, &healthy1));
    assert!(woronin.should_block(&healthy1, &isolated));

    // Transactions between healthy nodes allowed
    assert!(!woronin.should_block(&healthy1, &healthy2));

    // Track blocked transactions
    woronin.record_blocked(&isolated);
    woronin.record_blocked(&isolated);
    assert_eq!(woronin.get(&isolated).unwrap().blocked_transactions, 2);
}
```

### Deactivation

```rust
#[test]
fn test_woronin_deactivation() {
    let mut woronin = WoroninManager::new();
    let node = NodeId::from_bytes([1u8; 32]);

    woronin.activate(node, "test");
    assert!(woronin.is_isolated(&node));

    let body = woronin.deactivate(&node);
    assert!(body.is_some());
    assert!(!woronin.is_isolated(&node));
}
```

---

## Test Infrastructure Requirements

### TestCluster Extensions

The existing `TestCluster` helper (from `helpers/cluster.rs`) needs these extensions:

```rust
impl TestCluster {
    /// Get septal gate manager for a node
    pub fn septal_manager(&self, index: usize) -> &SeptalGateManager {
        &self.nodes[index].enr_bridge.septal
    }

    /// Inject failure for a peer from a node's perspective
    pub async fn inject_failure(&self, observer_idx: usize, target: NodeId, reason: &str) {
        self.septal_manager(observer_idx)
            .record_failure(target, reason)
            .await;
    }

    /// Wait for gate to reach specific state
    pub async fn wait_for_gate_state(
        &self,
        observer_idx: usize,
        target: NodeId,
        expected: SeptalGateState,
        timeout_secs: u64,
    ) -> Result<(), &'static str> {
        let deadline = Duration::from_secs(timeout_secs);
        timeout(deadline, async {
            loop {
                let state = self.septal_manager(observer_idx)
                    .get_gate_state(&target)
                    .await;
                if state == expected {
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .map_err(|_| "Timeout waiting for gate state")?
    }
}
```

### Mock Health Checker

```rust
struct MockHealthChecker {
    health_map: HashMap<NodeId, bool>,
}

impl HealthChecker for MockHealthChecker {
    fn check_health(&self, node: &NodeId) -> HealthStatus {
        let healthy = self.health_map.get(node).copied().unwrap_or(true);
        HealthStatus {
            is_healthy: healthy,
            timeout_score: if healthy { 0.0 } else { 1.0 },
            credit_score: if healthy { 0.0 } else { 1.0 },
            reputation_score: if healthy { 0.0 } else { 1.0 },
            last_check: Timestamp::now(),
        }
    }
}
```

---

## Pass/Fail Summary Table

| Scenario | Primary Assertion | Timeout |
|----------|-------------------|---------|
| 1. Timeout Trigger | Gate closes after 5 failures | 30s |
| 2. Credit Default | Weighted score >= 0.7 required | 30s |
| 3. Reputation Drop | Multi-factor failure required | 30s |
| 4. Recovery Cycle | Correct state transitions | 120s |
| 5. Cascade Prevention | Only target isolated | 60s |

---

## Metrics Collection

During stress tests, collect:

1. **Gate State Counts**
   - `stats.open_gates`
   - `stats.half_open_gates`
   - `stats.closed_gates`
   - `stats.isolated_nodes`

2. **Transition History**
   - `manager.recent_transitions().await`
   - Track from_state, to_state, reason, timestamp

3. **Woronin Metrics**
   - `blocked_transactions` per isolated node
   - `duration_active()` for each Woronin body

4. **Performance**
   - Time to close gate (failure injection to Closed state)
   - Time to recover (Closed to Open)
   - Gossip propagation latency for state changes

---

## Implementation Priority

1. **P0 (Must Have)**
   - Scenario 1: Timeout Trigger
   - Scenario 5: Cascade Prevention

2. **P1 (Should Have)**
   - Scenario 4: Recovery Cycle
   - Transaction blocking verification

3. **P2 (Nice to Have)**
   - Scenario 2: Credit Default Trigger
   - Scenario 3: Reputation Drop Trigger
   - High concurrency stress tests

---

## Appendix: State Machine Reference

From `/home/ardeshir/repos/univrs-enr/src/septal/mod.rs`:

```
State machine:
- Open: Normal operation, traffic flows freely
- HalfOpen: Testing recovery, limited traffic
- Closed: Isolated, no traffic allowed (Woronin active)
```

From `/home/ardeshir/repos/univrs-enr/src/septal/gate.rs`:

```
Open --[failures exceed threshold]--> Closed
  ^                                      |
  |                                      |
  +--[recovery test passes]-- HalfOpen <-+
                                 |        [timeout]
                                 |
                                 +--[recovery test fails]--> Closed
```
