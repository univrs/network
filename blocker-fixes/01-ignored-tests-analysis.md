# Ignored Tests Analysis - univrs-network

**Date:** 2026-01-01
**Repository:** univrs-network
**Purpose:** Document all ignored integration tests and provide roadmap for enabling them

---

## Summary

The `univrs-network` repository contains **9 ignored integration tests** across 3 test files in the `mycelial-network` crate. All tests share the same ignore reason:

```rust
#[ignore = "Integration test - requires clean network environment"]
```

These tests are "Phase 0 Gate Tests" that verify core P2P network functionality:
1. Gradient propagation across cluster nodes
2. Credit transfers with the 2% entropy tax
3. Nexus election protocol

---

## Test Files Overview

| File | Location | Ignored Tests |
|------|----------|---------------|
| `gate_gradient.rs` | `/home/ardeshir/repos/univrs-network/crates/mycelial-network/tests/gate_gradient.rs` | 2 |
| `gate_credits.rs` | `/home/ardeshir/repos/univrs-network/crates/mycelial-network/tests/gate_credits.rs` | 4 |
| `gate_election.rs` | `/home/ardeshir/repos/univrs-network/crates/mycelial-network/tests/gate_election.rs` | 3 |

---

## Detailed Test Analysis

### 1. gate_gradient.rs - Gradient Propagation Tests

#### Test: `test_gradient_propagates_to_all_nodes`

**Location:** Line 26
**Feature Tested:** Resource gradient broadcast propagation across a 3-node cluster

**What It Does:**
- Spawns a 3-node cluster using `TestCluster::spawn(3)`
- Waits for mesh formation (minimum 1 peer per node)
- Node 0 broadcasts a distinctive `ResourceGradient` with specific values (cpu: 0.42, memory: 0.73)
- Verifies all other nodes receive and store the gradient within 15 seconds

**Why Ignored:**
The test requires a "clean network environment" - meaning no port conflicts, no Docker bridge interference (172.17.x.x, 172.28.x.x, 172.29.x.x ranges are explicitly blocked), and proper network isolation between test runs.

**Dependencies:**
- TCP port availability in range ~20000-40000 (dynamic based on process ID)
- No mDNS cross-test interference (disabled in config)
- `univrs-enr` crate for `ResourceGradient` type

**Estimated Effort:** LOW (1-2 hours)
- The test infrastructure (`TestCluster`) is well-built
- Main blocker is likely CI environment configuration
- May need to add port range configuration or use ephemeral ports

---

#### Test: `test_gradient_propagates_5_nodes`

**Location:** Line 125
**Feature Tested:** Gradient propagation in a larger 5-node cluster

**What It Does:**
- Spawns a 5-node cluster
- Waits for mesh formation (minimum 1 peer)
- Node 2 (middle node) broadcasts gradient
- Verifies all 4 other nodes receive the gradient within 15 seconds

**Why Ignored:** Same as above - requires clean network environment

**Dependencies:** Same as above

**Estimated Effort:** LOW (included with above fix)

---

### 2. gate_credits.rs - Credit Transfer Tests

#### Test: `test_credit_transfer_with_tax`

**Location:** Line 28
**Feature Tested:** Credit transfer with 2% entropy tax calculation

**What It Does:**
- Spawns a 3-node cluster (each starts with 1000 credits via `INITIAL_NODE_CREDITS`)
- Node 0 transfers 100 credits to Node 1
- Verifies sender balance: 1000 - 100 - 2 (tax) = 898
- Verifies receiver balance: 1000 + 100 = 1100
- Verifies observer (Node 2) balance unchanged

**Why Ignored:** Requires clean network environment for gossipsub message propagation

**Dependencies:**
- `univrs-enr` crate for `NodeId`, `Credits` types
- EnrBridge credit management system
- Network message propagation within 10 seconds timeout

**Estimated Effort:** MEDIUM (2-4 hours)
- Credit system has more state to manage
- May need to verify async message handling
- Timeout values may need tuning in CI

---

#### Test: `test_self_transfer_rejected`

**Location:** Line 130
**Feature Tested:** Self-transfer rejection validation

**What It Does:**
- Attempts to transfer credits from a node to itself
- Verifies the transfer is rejected with error
- Verifies balance unchanged

**Why Ignored:** Requires clean network environment

**Dependencies:** Same as credit transfer tests

**Estimated Effort:** LOW (included with credit fix)

---

#### Test: `test_insufficient_balance_rejected`

**Location:** Line 174
**Feature Tested:** Overdraft rejection

**What It Does:**
- Attempts to transfer 2000 credits when balance is 1000
- Verifies transfer is rejected
- Verifies balance unchanged

**Why Ignored:** Requires clean network environment

**Dependencies:** Same as credit transfer tests

**Estimated Effort:** LOW (included with credit fix)

---

#### Test: `test_multiple_transfers`

**Location:** Line 221
**Feature Tested:** Sequential transfers with cumulative tax

**What It Does:**
- Performs two transfers from Node 0:
  - 100 credits to Node 1 (tax: 2)
  - 200 credits to Node 2 (tax: 4)
- Verifies Node 0 balance: 1000 - 100 - 2 - 200 - 4 = 694

**Why Ignored:** Requires clean network environment

**Dependencies:** Same as credit transfer tests

**Estimated Effort:** LOW (included with credit fix)

---

### 3. gate_election.rs - Nexus Election Tests

#### Test: `test_election_announcement_propagates`

**Location:** Line 28
**Feature Tested:** Election announcement propagation across cluster

**What It Does:**
- Spawns a 5-node cluster
- Sets eligible metrics on all nodes (uptime > 0.95, high reputation)
- Node 0 triggers an election for "test-region"
- Verifies all nodes see `election_in_progress` within 10 seconds

**Why Ignored:** Requires clean network environment for gossipsub propagation

**Dependencies:**
- `LocalNodeMetrics` struct from `enr_bridge`
- Election system with region-based elections
- Gossipsub topic subscription for election messages

**Estimated Effort:** MEDIUM (3-5 hours)
- Election protocol is more complex
- Involves state machines for candidacy/voting phases
- May have timing-sensitive behavior

---

#### Test: `test_election_completes_with_winner`

**Location:** Line 119
**Feature Tested:** Full election cycle to completion

**What It Does:**
- Spawns 3-node cluster
- Sets Node 1 with highest metrics (uptime: 0.99, bandwidth: 500, reputation: 0.95)
- Sets Nodes 0, 2 with lower metrics
- Triggers election from Node 0
- Waits for election to complete or timeout (30 seconds)
- Logs result (winner or timeout - timeout is acceptable for MVP)

**Why Ignored:** Requires clean network environment

**Dependencies:**
- Full election protocol (announcement -> candidacy -> voting -> result)
- `current_nexus()` method for result verification

**Estimated Effort:** HIGH (4-8 hours)
- Most complex test involving full protocol cycle
- May need protocol timing adjustments
- Vote collection and counting logic

---

#### Test: `test_ineligible_node_cannot_win`

**Location:** Line 216
**Feature Tested:** Election eligibility enforcement

**What It Does:**
- Spawns 3-node cluster
- Sets Node 0 as ineligible (uptime: 0.80, below 0.95 threshold)
- Sets Node 1 as eligible
- Node 0 triggers election (should succeed)
- Verifies Node 0 remains Leaf role (not a candidate)

**Why Ignored:** Requires clean network environment

**Dependencies:**
- Eligibility thresholds in election module
- Role type checking (`NexusRoleType::Leaf`)

**Estimated Effort:** MEDIUM (2-4 hours)
- Requires understanding eligibility rules
- Need to verify role state transitions

---

## Test Infrastructure Analysis

### TestCluster (helpers/cluster.rs)

The test infrastructure is well-designed:

```rust
pub struct TestCluster {
    pub nodes: Vec<TestNode>,
    shutdown_handles: Vec<NetworkHandle>,
}
```

**Key Features:**
1. **Automatic port allocation** using atomic counter and process ID to avoid conflicts
2. **Direct bootstrap connections** (no mDNS) to prevent cross-test interference
3. **Mesh formation waiting** with configurable timeout
4. **Clean shutdown** via handle shutdown method

**Port Calculation:**
```rust
let base_port = 20000u16
    .wrapping_add((std::process::id() as u16 % 100) * 100)
    .wrapping_add(cluster_offset)
    % 40000
    + 20000;
```

This creates unique port ranges per process, but may still conflict in parallel CI runs.

---

## Root Cause Analysis

### Why "Clean Network Environment" is Required

1. **Port Conflicts:**
   - Tests spawn multiple nodes on TCP ports
   - Parallel test execution can cause port collisions
   - The current port calculation may not be sufficient for CI parallelization

2. **Address Filtering:**
   - Tests explicitly filter Docker bridge addresses (172.17.x.x, 172.28-29.x.x)
   - WSL magic IP (10.255.255.254) is also blocked
   - This may cause issues in containerized CI environments

3. **Timing Dependencies:**
   - Tests have timeouts (10-30 seconds)
   - Mesh formation requires network stability
   - CI environments may have variable network latency

---

## Recommendations for Enabling Tests

### Priority Order

| Priority | Test Group | Estimated Total Effort | Value |
|----------|------------|----------------------|-------|
| 1 | Gradient Tests | 1-2 hours | Core P2P validation |
| 2 | Credit Tests | 2-4 hours | Economics verification |
| 3 | Election Tests | 4-8 hours | Consensus validation |

### Technical Fixes Required

1. **Port Management:**
   - Consider using ephemeral ports (port 0) and extracting actual bound port
   - Or implement a port reservation system with file locking

2. **CI Environment:**
   - Add `--test-threads=1` to run integration tests sequentially
   - Create a dedicated test network namespace (if running in Linux containers)

3. **Timeout Tuning:**
   - Consider making timeouts configurable via environment variables
   - Increase defaults for CI (e.g., 30s -> 60s for slow environments)

4. **Test Isolation:**
   - Add random topic prefixes to prevent cross-test pollution
   - Ensure full cleanup in `TestCluster::shutdown()`

### Running the Ignored Tests Locally

```bash
# Run all ignored tests in the mycelial-network crate
cd ~/repos/univrs-network
cargo test --package mycelial-network --test gate_gradient -- --ignored
cargo test --package mycelial-network --test gate_credits -- --ignored
cargo test --package mycelial-network --test gate_election -- --ignored

# Run with verbose output
cargo test --package mycelial-network -- --ignored --nocapture
```

---

## Non-Ignored Tests Summary

The repository has extensive unit test coverage that runs normally:

| Crate | Test Module | Status |
|-------|-------------|--------|
| mycelial-network | service_tests.rs | Active |
| mycelial-network | lib.rs (tests module) | Active |
| mycelial-network | behaviour, economics, peer | Active |
| mycelial-network | enr_bridge/* | Active |
| mycelial-network | raft/* | Active |
| mycelial-node | integration/rest_api.rs | Active |
| mycelial-node | integration/websocket_messages.rs | Active |
| mycelial-node | integration/rest_handlers.rs | Active |
| mycelial-node | integration/websocket_handlers.rs | Active |
| mycelial-node | integration/dashboard_compatibility.rs | Active |
| mycelial-core | Various modules | Active |
| mycelial-state | storage, cache, sync | Active |
| mycelial-protocol | messages | Active |

---

## Conclusion

The 9 ignored tests are well-written integration tests that validate critical P2P functionality. They are ignored due to environmental requirements, not code issues. The test infrastructure (`TestCluster`) is robust and handles:

- Automatic port allocation
- Bootstrap peer connections
- Mesh formation detection
- Graceful shutdown

Enabling these tests requires:
1. CI environment configuration (sequential execution, network isolation)
2. Minor timeout tuning for variable latency environments
3. Potentially ephemeral port usage for better isolation

Total estimated effort: **8-14 hours** for full enablement across all test groups.
