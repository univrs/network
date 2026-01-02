# Credit Transfer Stress Test Design

## Overview

This document specifies stress tests for the credit transfer system in the univrs-network.
Based on analysis of the implementation in:
- `/home/ardeshir/repos/univrs-network/crates/mycelial-network/src/enr_bridge/credits.rs`
- `/home/ardeshir/repos/univrs-enr/src/revival/pool.rs`
- `/home/ardeshir/repos/univrs-network/crates/mycelial-network/tests/gate_credits.rs`

## System Constants

| Constant | Value | Source |
|----------|-------|--------|
| `INITIAL_NODE_CREDITS` | 1000 | `credits.rs:21` |
| `ENTROPY_TAX_RATE` | 0.02 (2%) | `pool.rs:9` |
| `NETWORK_MAINTENANCE_ALLOCATION` | 40% | `pool.rs:12` |
| `NEW_NODE_SUBSIDY_ALLOCATION` | 25% | `pool.rs:13` |
| `LOW_BALANCE_SUPPORT_ALLOCATION` | 20% | `pool.rs:14` |
| `RESERVE_BUFFER_ALLOCATION` | 15% | `pool.rs:15` |

## Key Types

```rust
// Credit transfer structure (from univrs_enr::core)
pub struct CreditTransfer {
    pub from: AccountId,
    pub to: AccountId,
    pub amount: Credits,
    pub entropy_cost: Credits,  // 2% tax
}

// Transfer errors (from credits.rs:285-299)
pub enum TransferError {
    ZeroAmount,
    SelfTransfer,
    InsufficientCredits { available: Credits, required: Credits },
    Encode(EncodeError),
    Publish(String),
}
```

---

## Test Scenario 1: Basic Transfer

### Description
Node A sends 100 credits to Node B. Validates basic transfer mechanics and entropy tax deduction.

### Setup
```
Nodes: 2 (A, B)
Initial State:
  - A.balance = 1000
  - B.balance = 1000
  - revival_pool = 0
```

### Actions
1. A transfers 100 credits to B

### Expected Results
```
A.balance = 1000 - 100 - 2 (tax) = 898
B.balance = 1000 + 100 = 1100
entropy_tax_collected = 2
```

### Invariants Checked
- [x] Sender balance decreased by amount + tax
- [x] Receiver balance increased by amount (NOT amount - tax)
- [x] Tax = floor(amount * 0.02)
- [x] Total credits: 898 + 1100 + 2 = 2000 (conserved)

### Pass Criteria
- Transfer completes within 1 second (local)
- Balances match expected values exactly
- Gossip message published exactly once

---

## Test Scenario 2: Chain Transfer

### Description
Credits flow through a chain: A -> B -> C -> D -> E. Each hop deducts entropy tax.

### Setup
```
Nodes: 5 (A, B, C, D, E)
Initial State:
  - All nodes start with 1000 credits
  - revival_pool = 0

Transfer Amount: 500 credits at each hop
```

### Actions (Sequential)
1. A transfers 500 to B
2. B transfers 500 to C
3. C transfers 500 to D
4. D transfers 500 to E

### Expected Results
```
After A->B:
  A = 1000 - 500 - 10 = 490
  B = 1000 + 500 = 1500
  tax_total = 10

After B->C:
  B = 1500 - 500 - 10 = 990
  C = 1000 + 500 = 1500
  tax_total = 20

After C->D:
  C = 1500 - 500 - 10 = 990
  D = 1000 + 500 = 1500
  tax_total = 30

After D->E:
  D = 1500 - 500 - 10 = 990
  E = 1000 + 500 = 1500
  tax_total = 40

Final State:
  A = 490
  B = 990
  C = 990
  D = 990
  E = 1500
  revival_pool = 40

Total = 490 + 990 + 990 + 990 + 1500 + 40 = 5000
```

### Invariants Checked
- [x] Credits conserved at each step
- [x] Tax accumulates correctly
- [x] Each transfer completes before next begins (no race conditions)

### Pass Criteria
- Chain completes within 10 seconds
- Final balances match expected
- Total credits = initial total (5000)

---

## Test Scenario 3: Concurrent Transfers

### Description
10 simultaneous transfers from different senders to test concurrency safety.

### Setup
```
Nodes: 11 (S1-S10 senders, R receiver)
Initial State:
  - All senders: 1000 credits each
  - Receiver: 1000 credits

Transfer Amount: 100 credits from each sender
```

### Actions (Parallel)
```
S1 -> R: 100
S2 -> R: 100
S3 -> R: 100
...
S10 -> R: 100
```

### Expected Results
```
Each sender Si:
  Si.balance = 1000 - 100 - 2 = 898

Receiver:
  R.balance = 1000 + (100 * 10) = 2000

Total tax collected: 2 * 10 = 20

Conservation check:
  (898 * 10) + 2000 + 20 = 8980 + 2000 + 20 = 11000
```

### Invariants Checked
- [x] No double-spend (each sender debited exactly once)
- [x] Receiver credited exactly 1000 (100 * 10)
- [x] All nonces unique (replay protection)
- [x] Total credits conserved

### Pass Criteria
- All 10 transfers complete within 5 seconds
- No transfer failures due to race conditions
- Final balances match expected

### Stress Metrics
- Target: 100 transfers/second sustained
- Latency: p99 < 100ms

---

## Test Scenario 4: Insufficient Balance

### Description
Attempt transfers that exceed available balance, including edge cases.

### Setup
```
Node: A with 1000 credits
Target: B (any valid node)
```

### Test Cases

#### 4a. Transfer More Than Balance
```
Action: A transfers 2000 to B
Expected: TransferError::InsufficientCredits
Balance after: A = 1000 (unchanged)
```

#### 4b. Transfer Exact Balance (Fails Due to Tax)
```
Action: A transfers 1000 to B
Expected: TransferError::InsufficientCredits
  - Reason: 1000 + 20 (tax) = 1020 > 1000
Balance after: A = 1000 (unchanged)
```

#### 4c. Maximum Possible Transfer
```
Maximum transferable = floor(1000 / 1.02) = 980
Tax = floor(980 * 0.02) = 19
Total cost = 980 + 19 = 999

Action: A transfers 980 to B
Expected: Success
Balance after: A = 1 (1000 - 980 - 19)
```

#### 4d. Zero Transfer
```
Action: A transfers 0 to B
Expected: TransferError::ZeroAmount
Balance after: A = 1000 (unchanged)
```

#### 4e. Self Transfer
```
Action: A transfers 100 to A
Expected: TransferError::SelfTransfer
Balance after: A = 1000 (unchanged)
```

### Invariants Checked
- [x] Failed transfers leave state unchanged
- [x] Error types match expected
- [x] No partial debits on failure

### Pass Criteria
- All error cases return correct error type
- Balances unchanged after rejected transfers
- No state corruption

---

## Test Scenario 5: Credit Conservation

### Description
Verify total system credits remain constant across various operations.

### Setup
```
Nodes: 10
Initial credits per node: 1000
Total initial credits: 10,000
```

### Actions (Random Operations)
```
for i in 1..1000:
    sender = random_node()
    receiver = random_node() != sender
    amount = random(1, sender.balance / 1.02)

    if sender.balance >= amount * 1.02:
        transfer(sender, receiver, amount)
```

### Conservation Formula
```
sum(all_node_balances) + revival_pool.total_balance() = INITIAL_TOTAL

Where revival_pool includes:
  - entropy_tax_collected
  - recycled_credits
  - maintenance_fund
  - reserve_buffer
```

### Invariants Checked (After Every Transfer)
- [x] `sum(balances) + revival_pool == INITIAL_TOTAL`
- [x] All balances >= 0 (enforced by u64)
- [x] revival_pool.entropy_tax_collected is monotonically increasing

### Pass Criteria
- Conservation holds after every operation
- No overflow/underflow errors
- Test completes 1000 operations

### Stress Metrics
- Memory usage stable over test duration
- No gradual drift in totals (floating point errors)

---

## Test Scenario 6: Revival Pool - Failed Node Credits

### Description
When a node fails (detected by septal gate), its credits flow to the revival pool.

### Setup
```
Nodes: 5 (A, B, C, D, E)
A = 1000, B = 1500, C = 800, D = 1200, E = 500
Total = 5000

revival_pool:
  recycled_credits = 0
  entropy_tax_collected = 0
  maintenance_fund = 0
  reserve_buffer = 0
```

### Actions
1. Node C fails (detected by septal circuit breaker)
2. C's 800 credits recycled to revival pool
3. Trigger redistribution

### Expected Results
```
After failure:
  revival_pool.recycled_credits = 800
  C removed from active nodes

After redistribution (800 credits):
  maintenance_fund += floor(800 * 0.40) = 320 (to nexus nodes)
  new_node_subsidy += floor(800 * 0.25) = 200 (to new nodes)
  low_balance_support += floor(800 * 0.20) = 160 (to struggling nodes)
  reserve_buffer += floor(800 * 0.15) = 120

Note: E (balance=500) might receive support if reputation >= 0.5
```

### Invariants Checked
- [x] Total credits conserved (even with node removal)
- [x] Redistribution percentages sum to 100%
- [x] No credits created from nothing
- [x] No credits destroyed (only moved)

### Pass Criteria
- Pool receives exact credits from failed node
- Redistribution follows allocation percentages
- All eligible recipients receive appropriate share

---

## Global Invariants

These invariants MUST hold at all times during any test:

### 1. Credits Never Created
```rust
fn invariant_no_credit_creation(before: &SystemState, after: &SystemState) {
    let before_total = before.total_credits();
    let after_total = after.total_credits();
    assert!(after_total <= before_total,
        "Credits increased: {} -> {}", before_total, after_total);
}
```

### 2. Credits Never Destroyed (Except Entropy Tax)
```rust
fn invariant_no_credit_destruction(before: &SystemState, after: &SystemState, expected_tax: u64) {
    let before_total = before.total_credits();
    let after_total = after.total_credits() + after.revival_pool().entropy_tax_collected;
    assert_eq!(before_total, after_total + expected_tax,
        "Credits lost: {} != {} + {}", before_total, after_total, expected_tax);
}
```

### 3. Entropy Tax Correctness
```rust
fn invariant_entropy_tax(transfer_amount: Credits, actual_tax: Credits) {
    let expected = (transfer_amount.amount as f64 * 0.02).floor() as u64;
    assert_eq!(actual_tax.amount, expected,
        "Tax mismatch: {} != 2% of {}", actual_tax.amount, transfer_amount.amount);
}
```

### 4. No Double Spend
```rust
fn invariant_no_double_spend(nonces: &HashMap<NodeId, u64>, transfer: &CreditTransfer) {
    // Each sender's nonces must be strictly increasing
    if let Some(&last) = nonces.get(&transfer.from.node) {
        assert!(transfer.nonce > last,
            "Potential double spend: nonce {} <= last {}", transfer.nonce, last);
    }
}
```

---

## Performance Targets

| Metric | Target | Stress Target |
|--------|--------|---------------|
| Single transfer latency | < 10ms | < 50ms |
| Transfers per second (single node) | 1000 | 500 |
| Concurrent transfers (10 nodes) | 100/s | 50/s |
| Memory per 1000 transfers | < 1MB | < 5MB |
| Time to verify all invariants | < 1ms | < 10ms |

---

## Test Infrastructure Requirements

### Dependencies
```toml
[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
criterion = "0.5"  # For benchmarks
proptest = "1"     # For property-based tests
```

### Test Harness
```rust
// Required setup for integration tests
struct StressTestCluster {
    nodes: Vec<TestNode>,
    revival_pool: RevivalPool,
    nonce_tracker: HashMap<NodeId, u64>,
}

impl StressTestCluster {
    fn total_credits(&self) -> u64;
    fn verify_conservation(&self) -> Result<(), String>;
    fn random_transfer(&mut self) -> Result<CreditTransfer, TransferError>;
}
```

### Existing Test Foundation
The codebase already has:
- `gate_credits.rs` - Integration tests for credit transfers
- `TestCluster` helper in `tests/helpers/`
- Unit tests in `credits.rs` and `raft/mod.rs`

---

## Implementation Priority

1. **Phase 1**: Scenarios 1, 4 (Basic Transfer, Insufficient Balance)
   - Foundation tests, quick to implement
   - Validates core transfer mechanics

2. **Phase 2**: Scenarios 2, 5 (Chain Transfer, Conservation)
   - Sequential flow validation
   - Critical invariant verification

3. **Phase 3**: Scenarios 3, 6 (Concurrent, Revival Pool)
   - Concurrency stress tests
   - Full system integration

---

## References

- Implementation: `/home/ardeshir/repos/univrs-network/crates/mycelial-network/src/enr_bridge/credits.rs`
- Entropy Tax: `/home/ardeshir/repos/univrs-enr/src/revival/pool.rs`
- Existing Tests: `/home/ardeshir/repos/univrs-network/crates/mycelial-network/tests/gate_credits.rs`
- Raft Integration: `/home/ardeshir/repos/univrs-network/crates/mycelial-network/src/raft/mod.rs`

---

*Document Version: 1.0*
*Generated: Phase 3 Stress Testing Swarm*
