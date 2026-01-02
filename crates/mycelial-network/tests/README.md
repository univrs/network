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
| `test_cluster_20_nodes` | 20-node cluster (scale test) | Gradient reaches 75%+ of nodes within 30s |
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

**Important**: Tests must be run with `--test-threads=1` to avoid port conflicts
between concurrent test clusters.

```bash
# Run all gate tests (MUST use --test-threads=1)
cargo test --package mycelial-network --test gate_gradient --test gate_credits --test gate_election -- --test-threads=1

# Run specific test
cargo test --package mycelial-network --test gate_credits test_credit_transfer_with_tax -- --test-threads=1 --nocapture

# Run with verbose logging
RUST_LOG=mycelial_network=debug cargo test --test gate_gradient -- --test-threads=1 --nocapture
```

## Running in Docker (Recommended)

For isolated, reproducible test execution, use the Docker setup:

```bash
# From univrs-network directory
cd /path/to/univrs-network

# Build the test image (includes all workspace dependencies)
docker compose -f docker-compose.test.yml build

# Run all integration tests
docker compose -f docker-compose.test.yml run --rm integration-tests

# Run specific test file
docker compose -f docker-compose.test.yml run --rm integration-tests \
  cargo test --package mycelial-network --release --test gate_credits -- --ignored --nocapture

# Clean up
docker compose -f docker-compose.test.yml down -v
```

### Docker Setup Files

| File | Purpose |
|------|---------|
| `Dockerfile.integration` | Multi-stage build with all univrs-* dependencies |
| `docker-compose.test.yml` | Orchestrates test container with cargo caching |

### Why Docker?

- **Network Isolation**: Eliminates interference from host network interfaces (Docker bridges, WSL2 virtual NICs)
- **Reproducibility**: Same environment across all developer machines
- **Clean State**: Each run starts fresh without stale peer discovery
- **CI/CD Ready**: Can be integrated into GitHub Actions workflows

## Local Execution

Tests can run locally. Ensure sequential execution to avoid port conflicts:

```bash
# Run all gate tests locally (sequential mode required)
cargo test --package mycelial-network --test gate_gradient --test gate_credits --test gate_election -- --test-threads=1 --nocapture
```

### Network Requirements (Local)

For reliable local test execution:
- Run on a host without Docker bridge interfaces active
- Or configure tests to bind only to `127.0.0.1`
- Tests use ports in range 20000-60000

## Phase 0 Gate Criteria

These tests validate the ENR bridge integration meets Phase 0 requirements:

1. **Gradient Propagation**: Resource gradients broadcast via gossipsub reach all mesh peers
2. **Credit Transfers**: Transfers correctly deduct 2% entropy tax from sender
3. **Nexus Election**: Distributed election converges on a single winner
4. **Septal Gates**: Circuit breakers isolate failing nodes (unit tested in `enr_bridge/septal.rs`)
