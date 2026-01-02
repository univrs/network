# WebSocket-EnrBridge Integration - FIXED

## Date
2026-01-01

## Summary
WebSocket handlers for ENR Bridge operations are now properly wired to the EnrBridge component, enabling real network operations instead of local echoes.

## Problem Statement
The original implementation had a gap where:
1. EnrBridge was created inside NetworkService but not exposed
2. WebSocket handlers in `websocket.rs` only performed local echoes for ENR operations
3. Five critical message types were not forwarding to the network:
   - `ReportGradient`
   - `StartElection`
   - `RegisterCandidacy`
   - `VoteElection`
   - `SendEnrCredit`

## Solution Implemented

### 1. Modified NetworkService::new() to expose EnrBridge
**File**: `crates/mycelial-network/src/service.rs`

With `univrs-compat` feature (default), `NetworkService::new()` now returns a 4-tuple:
```rust
pub fn new(
    keypair: libp2p::identity::Keypair,
    config: NetworkConfig,
) -> Result<(Self, NetworkHandle, broadcast::Receiver<NetworkEvent>, Arc<EnrBridge>)>
```

Without the feature, it maintains the original 3-tuple signature for backward compatibility.

### 2. Added EnrBridge to AppState
**File**: `crates/mycelial-node/src/main.rs`

AppState now includes:
```rust
pub struct AppState {
    // ... existing fields ...
    pub enr_bridge: Arc<mycelial_network::enr_bridge::EnrBridge>,
}
```

### 3. Added dependencies to mycelial-node
**File**: `crates/mycelial-node/Cargo.toml`

Added:
```toml
univrs-enr = { workspace = true }
hex = "0.4"
```

### 4. Added election methods to DistributedElection
**File**: `crates/mycelial-network/src/enr_bridge/nexus.rs`

Added two new public methods:
- `submit_candidacy(election_id, metrics)` - Submit candidacy with explicit metrics
- `vote_for_candidate(election_id, candidate)` - Vote for a specific candidate

### 5. Added delegate methods to EnrBridge
**File**: `crates/mycelial-network/src/enr_bridge/mod.rs`

Added:
- `submit_candidacy()` - Delegates to election module
- `vote_for_candidate()` - Delegates to election module

### 6. Wired WebSocket handlers
**File**: `crates/mycelial-node/src/server/websocket.rs`

All five ENR handlers now forward to EnrBridge:

| Handler | EnrBridge Method | Description |
|---------|-----------------|-------------|
| `ReportGradient` | `broadcast_gradient()` | Broadcasts resource gradient to network |
| `StartElection` | `trigger_election()` | Initiates a nexus election for a region |
| `RegisterCandidacy` | `submit_candidacy()` | Submits this node as a candidate |
| `VoteElection` | `vote_for_candidate()` | Casts a vote for a specific candidate |
| `SendEnrCredit` | `transfer_credits()` | Transfers ENR credits to another node |

### 7. Updated doc examples
**File**: `crates/mycelial-network/src/lib.rs`

Updated the main doc example to reflect the new 4-tuple return type.

## Helper Function Added

```rust
fn parse_node_id(s: &str) -> Result<NodeId, String>
```

Parses NodeId from:
- Hex-encoded 32-byte strings (64 chars)
- Base58 peer_id format (with byte conversion)

## Testing

All existing tests pass:
- `mycelial-node`: 8 unit tests, 51 integration tests
- `mycelial-network`: All tests pass, doctests updated

## Files Modified

1. `crates/mycelial-network/src/service.rs` - Export EnrBridge from new()
2. `crates/mycelial-network/src/enr_bridge/nexus.rs` - Add submit_candidacy(), vote_for_candidate()
3. `crates/mycelial-network/src/enr_bridge/mod.rs` - Add delegate methods
4. `crates/mycelial-network/src/lib.rs` - Update doc example
5. `crates/mycelial-node/src/main.rs` - Add EnrBridge to AppState
6. `crates/mycelial-node/src/server/websocket.rs` - Wire all ENR handlers
7. `crates/mycelial-node/Cargo.toml` - Add dependencies

## Backward Compatibility

The implementation uses conditional compilation (`#[cfg(feature = "univrs-compat")]`) to maintain backward compatibility:
- With `univrs-compat` (default): Full EnrBridge integration
- Without feature: Original 3-tuple return type from NetworkService::new()

## Next Steps

1. Add integration tests for the WebSocket -> EnrBridge flow
2. Consider exposing EnrBridge event streams for proactive notifications
3. Add error recovery and retry logic for failed broadcasts
