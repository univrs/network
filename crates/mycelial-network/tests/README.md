# Mycelial Network Integration Tests

Phase 0 gate tests for the ENR (Entropy-Nexus-Revival) bridge integration.

## Test Structure

```
tests/
├── README.md           # This file
├── helpers/
│   ├── mod.rs          # Helper module exports
│   └── cluster.rs      # TestCluster - spawns multi-node networks
├── gate_gradient.rs    # Gradient propagation tests
├── gate_credits.rs     # Credit transfer tests
└── gate_election.rs    # Nexus election tests
```

## Test Specifications

### Gradient Tests (`gate_gradient.rs`)

| Test | Description | Assertion |
|------|-------------|-----------|
| `test_gradient_propagates_to_all_nodes` | 3-node cluster | Gradient reaches all nodes within 15s |
| `test_gradient_propagates_5_nodes` | 5-node cluster | Gradient reaches all nodes within 15s |

### Credit Tests (`gate_credits.rs`)

| Test | Description | Assertion |
|------|-------------|-----------|
| `test_credit_transfer_with_tax` | Transfer 100 credits | Sender: 898, Receiver: 1100 (2% tax) |
| `test_self_transfer_rejected` | Self-transfer | Returns error |
| `test_insufficient_balance_rejected` | Over-balance transfer | Returns error |
| `test_multiple_transfers` | Sequential transfers | Correct final balances |

### Election Tests (`gate_election.rs`)

| Test | Description | Assertion |
|------|-------------|-----------|
| `test_nexus_election` | 3-node election | Winner elected within timeout |
| `test_election_convergence` | 5-node election | All nodes agree on winner |

## TestCluster Helper

The `TestCluster` helper spawns multiple network nodes for integration testing:

```rust
use helpers::TestCluster;

// Spawn a 3-node cluster
let cluster = TestCluster::spawn(3).await?;

// Wait for mesh formation (each node sees at least 2 peers)
cluster.wait_for_mesh(2, 10).await?;

// Access individual nodes
let balance = cluster.node(0).balance().await;
let bridge = &cluster.node(0).enr_bridge;

// Cleanup
cluster.shutdown().await;
```

Features:
- Automatic port allocation (process-unique)
- Direct bootstrap connections (no mDNS interference)
- Mesh formation waiting with timeout
- Access to `NetworkHandle` and `EnrBridge` for each node

## Running Tests

Tests are marked `#[ignore]` because they require a clean network environment
(WSL2/Docker bridge interfaces can cause dial errors).

```bash
# Run all gate tests
cargo test --package mycelial-network --test gate_gradient --test gate_credits --test gate_election -- --ignored

# Run specific test
cargo test --package mycelial-network --test gate_credits test_credit_transfer_with_tax -- --ignored --nocapture

# Run with verbose logging
RUST_LOG=mycelial_network=debug cargo test --test gate_gradient -- --ignored --nocapture
```

## Network Requirements

For reliable test execution:
- Run on a host without Docker bridge interfaces active
- Or configure tests to bind only to `127.0.0.1`
- Tests use ports in range 20000-60000

## Phase 0 Gate Criteria

These tests validate the ENR bridge integration meets Phase 0 requirements:

1. **Gradient Propagation**: Resource gradients broadcast via gossipsub reach all mesh peers
2. **Credit Transfers**: Transfers correctly deduct 2% entropy tax from sender
3. **Nexus Election**: Distributed election converges on a single winner
4. **Septal Gates**: Circuit breakers isolate failing nodes (unit tested in `enr_bridge/septal.rs`)
