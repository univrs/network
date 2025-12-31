# Integration Testing Guide

This guide explains how to run the mycelial-network integration tests, which verify gossipsub mesh formation, gradient propagation, credit transfers, and nexus elections.

## Overview

The integration tests spawn multiple network nodes that communicate via libp2p gossipsub. These tests are marked with `#[ignore]` because they require a clean network environment and take longer to run than unit tests.

### Test Suites

| Test File | Tests | Description |
|-----------|-------|-------------|
| `gate_gradient.rs` | 2 | Gradient propagation across mesh |
| `gate_credits.rs` | 4 | Credit transfers with 2% entropy tax |
| `gate_election.rs` | 3 | Nexus election via gossipsub |

## Running Tests Locally

### Prerequisites

- Rust toolchain (stable)
- No conflicting network services on ports 20000-60000

### Run All Integration Tests

```bash
cargo test --test gate_gradient --test gate_credits --test gate_election -- --ignored
```

### Run Individual Test Suites

```bash
# Gradient propagation tests
cargo test --test gate_gradient -- --ignored

# Credit transfer tests
cargo test --test gate_credits -- --ignored

# Election tests
cargo test --test gate_election -- --ignored
```

### Run with Debug Output

```bash
RUST_LOG=mycelial_network=debug cargo test --test gate_gradient -- --ignored --nocapture
```

## Running Tests in Docker

Docker provides an isolated network environment, avoiding conflicts with host network interfaces (Docker bridges, WSL adapters, etc.).

### Prerequisites

- Docker and Docker Compose installed
- Access to parent directory containing all `univrs-*` crates

### Directory Structure

The Docker setup expects this directory structure:

```
parent-directory/
├── univrs-enr/
├── univrs-identity/
├── univrs-state/
└── univrs-network/
    ├── Dockerfile.integration
    └── docker-compose.test.yml
```

### Build and Run

```bash
# From the univrs-network directory
cd /path/to/univrs-network

# Build and run integration tests
docker compose -f docker-compose.test.yml up --build

# Run in detached mode
docker compose -f docker-compose.test.yml up --build -d

# View logs
docker compose -f docker-compose.test.yml logs -f

# Clean up
docker compose -f docker-compose.test.yml down -v
```

### Docker Configuration

**Dockerfile.integration:**
- Base image: `rust:latest`
- Installs: `pkg-config`, `libssl-dev`, `protobuf-compiler`
- Builds tests in release mode for faster execution
- Runs with `--test-threads=1` to avoid port conflicts

**docker-compose.test.yml:**
- Uses bridge network mode for isolated networking
- Caches cargo registry for faster rebuilds
- Sets `RUST_LOG=info` and `RUST_BACKTRACE=1`

### Customizing Test Runs

To run specific tests, modify the CMD in `Dockerfile.integration`:

```dockerfile
# Run only gradient tests
CMD ["cargo", "test", "--package", "mycelial-network", "--release", "--", "--ignored", "gradient"]

# Run with debug logging
CMD ["cargo", "test", "--package", "mycelial-network", "--release", "--", "--ignored", "--nocapture"]
```

Or override at runtime:

```bash
docker compose -f docker-compose.test.yml run integration-tests \
  cargo test --package mycelial-network --release -- --ignored gradient --nocapture
```

## Test Architecture

### TestCluster

The `TestCluster` helper (`tests/helpers/cluster.rs`) spawns multiple nodes:

1. **Port Allocation**: Each cluster gets unique ports (20000-60000 range)
2. **Star Topology**: Node 0 is the bootstrap node; others connect to it
3. **No mDNS**: Disabled to avoid cross-test interference
4. **Mesh Formation**: Waits for gossipsub mesh to stabilize

### Network Topology

```
     Node 1
       |
Node 0 (hub) --- Node 2
       |
     Node 3
```

Bootstrap creates a star topology where:
- Node 0 has N-1 peers (all other nodes)
- Nodes 1-N have 1 peer each (Node 0)

### Address Filtering

The network service filters non-routable addresses to avoid connection issues:
- Localhost (127.0.0.1) - allowed
- Docker bridges (172.17.x.x) - filtered
- WSL adapters (172.28.x.x, 172.29.x.x) - filtered
- Link-local (10.255.255.254) - filtered

## Troubleshooting

### Mesh Formation Timeout

**Symptom:** Tests fail with "Mesh formation timeout"

**Causes:**
- Port conflicts with other services
- Network interface issues (Docker/WSL bridges)
- Firewall blocking localhost connections

**Solutions:**
1. Run in Docker for isolated networking
2. Check for port conflicts: `netstat -tlnp | grep -E "2[0-5][0-9]{3}"`
3. Increase timeout in test if needed

### Election Tests Timing Out

**Symptom:** `test_election_completes_with_winner` times out after 30s

**Note:** This is expected behavior for MVP. The election may not complete within the timeout as vote timing depends on network conditions.

### Parallel Test Failures

**Symptom:** Tests pass individually but fail when run together

**Solution:** Run with single thread:
```bash
cargo test -- --ignored --test-threads=1
```

## CI/CD Integration

For GitHub Actions or similar:

```yaml
integration-tests:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Run integration tests
      run: |
        docker compose -f docker-compose.test.yml up --build --abort-on-container-exit
      working-directory: ./univrs-network
```

## Related Documentation

- [ENR Bridge Architecture](./enr-bridge.md)
- [Gossipsub Configuration](./gossipsub.md)
- [Septal Gates (Circuit Breakers)](./septal-gates.md)
