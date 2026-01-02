# WebSocket-EnrBridge Integration Gap Analysis

## Overview

This document analyzes the integration gap between the WebSocket handlers in `mycelial-node` and the `EnrBridge` component in `mycelial-network`. While incoming gossipsub messages ARE properly routed to the EnrBridge, outgoing WebSocket client messages are NOT forwarded to the EnrBridge for processing.

## Current Architecture

### Component Locations

| Component | Path |
|-----------|------|
| WebSocket Handler | `/crates/mycelial-node/src/server/websocket.rs` |
| Message Definitions | `/crates/mycelial-node/src/server/messages.rs` |
| EnrBridge | `/crates/mycelial-network/src/enr_bridge/mod.rs` |
| Network Service | `/crates/mycelial-network/src/service.rs` |
| Main Entry Point | `/crates/mycelial-node/src/main.rs` |

### Feature Flag Dependency

The `univrs-compat` feature must be enabled for EnrBridge:
- **Default enabled** in `mycelial-network/Cargo.toml`: `default = ["univrs-compat"]`
- Guards all EnrBridge-related code with `#[cfg(feature = "univrs-compat")]`

---

## 1. Current WebSocket Message Types (ClientMessage variants)

### Core Messages
| Variant | Description | Status |
|---------|-------------|--------|
| `SendChat` | Send chat message | Publishes to gossip, local echo |
| `GetPeers` | Request peer list | Local state query |
| `GetStats` | Request network stats | Local state query |
| `Subscribe` | Subscribe to topic | Network command |

### Economics Protocol Messages
| Variant | Description | Status |
|---------|-------------|--------|
| `SendVouch` | Vouch for a peer | Publishes to `/mycelial/1.0.0/vouch` |
| `RespondVouch` | Accept/reject vouch | Publishes to `/mycelial/1.0.0/vouch` |
| `CreateCreditLine` | Create credit line | Publishes to `/mycelial/1.0.0/credit` |
| `TransferCredit` | Transfer credits | Publishes to `/mycelial/1.0.0/credit` |
| `CreateProposal` | Create governance proposal | Publishes to `/mycelial/1.0.0/governance` |
| `CastVote` | Vote on proposal | Publishes to `/mycelial/1.0.0/governance` |
| `ReportResource` | Report resource contribution | Publishes to `/mycelial/1.0.0/resource` |

### Room/Seance Messages
| Variant | Description | Status |
|---------|-------------|--------|
| `CreateRoom` | Create new room | Subscribes to topic |
| `JoinRoom` | Join existing room | Subscribes to topic |
| `LeaveRoom` | Leave room | Unsubscribes from topic |
| `GetRooms` | List available rooms | Local query (returns empty) |

### ENR Bridge Messages (GAP IDENTIFIED)
| Variant | Description | Current Behavior | Gap |
|---------|-------------|------------------|-----|
| `ReportGradient` | Report resource availability | LOCAL ECHO ONLY | NOT sent to EnrBridge.broadcast_gradient() |
| `StartElection` | Start nexus election | LOCAL ECHO ONLY | NOT sent to EnrBridge.trigger_election() |
| `RegisterCandidacy` | Register as candidate | LOCAL ECHO ONLY | NOT sent to EnrBridge.election |
| `VoteElection` | Vote for candidate | LOCAL ECHO ONLY | NOT sent to EnrBridge.election.cast_vote() |
| `SendEnrCredit` | Send ENR credits | LOCAL ECHO ONLY | NOT sent to EnrBridge.transfer_credits() |

---

## 2. Current Handler Logic Analysis

### WebSocket Handler Flow (websocket.rs)

```
WebSocket Message Received
    |
    v
Deserialize to ClientMessage
    |
    v
handle_client_message()
    |
    +-- Core Messages --> state.network.publish() --> Gossipsub
    |
    +-- Economics Messages --> state.network.publish() --> Gossipsub (mycelial-protocol topics)
    |
    +-- ENR Bridge Messages --> state.event_tx.send() --> WebSocket Clients ONLY
                                                          (NO EnrBridge integration!)
```

### Incoming Gossipsub Flow (main.rs + service.rs)

```
Gossipsub Message Received
    |
    v
NetworkService.handle_behaviour_event()
    |
    +-- ENR Topics? --> EnrBridge.handle_message() --> Updates local state
    |
    v
NetworkEvent::MessageReceived --> event_rx
    |
    v
handle_network_event() in main.rs
    |
    +-- ENR Topics? --> Decode & broadcast to WebSocket clients
```

**Key Finding**: Incoming ENR messages ARE processed by EnrBridge (service.rs lines 546-558), but outgoing ENR commands from WebSocket are NOT routed to EnrBridge.

---

## 3. EnrBridge Public Methods Available

### EnrBridge Struct (`enr_bridge/mod.rs`)

```rust
pub struct EnrBridge {
    pub gradient: GradientBroadcaster,
    pub credits: CreditSynchronizer,
    pub election: DistributedElection,
    pub septal: SeptalGateManager,
}
```

### Available Methods for Integration

#### Gradient Operations
| Method | Signature | Purpose |
|--------|-----------|---------|
| `broadcast_gradient` | `async fn(&self, ResourceGradient) -> Result<(), BroadcastError>` | Publish gradient to network |
| `network_gradient` | `async fn(&self) -> ResourceGradient` | Get aggregated network gradient |
| `active_node_count` | `async fn(&self) -> usize` | Count of nodes with fresh gradients |

#### Credit Operations
| Method | Signature | Purpose |
|--------|-----------|---------|
| `transfer_credits` | `async fn(&self, to: NodeId, amount: Credits) -> Result<(), TransferError>` | Transfer credits with entropy tax |
| `local_balance` | `async fn(&self) -> Credits` | Get local credit balance |

#### Election Operations
| Method | Signature | Purpose |
|--------|-----------|---------|
| `trigger_election` | `async fn(&self, region_id: String) -> Result<u64, ElectionError>` | Start new election |
| `current_nexus` | `async fn(&self) -> Option<NodeId>` | Get current nexus node |
| `current_role` | `async fn(&self) -> NexusRole` | Get local node's role |
| `update_node_metrics` | `async fn(&self, LocalNodeMetrics)` | Update eligibility metrics |
| `election_in_progress` | `async fn(&self) -> bool` | Check if election active |

#### Septal Gate Operations
| Method | Signature | Purpose |
|--------|-----------|---------|
| `record_peer_failure` | `async fn(&self, peer: NodeId, reason: &str)` | Record failure, may trigger isolation |
| `record_peer_success` | `async fn(&self, peer: NodeId)` | Reset failure count |
| `allows_traffic` | `async fn(&self, peer: &NodeId) -> bool` | Check if traffic allowed |
| `is_peer_isolated` | `async fn(&self, peer: &NodeId) -> bool` | Check isolation status |
| `septal_stats` | `async fn(&self) -> SeptalStats` | Get gate statistics |

#### Message Handling
| Method | Signature | Purpose |
|--------|-----------|---------|
| `handle_message` | `async fn(&self, bytes: &[u8]) -> Result<(), HandleError>` | Process incoming gossip message |
| `maintenance` | `async fn(&self)` | Prune stale data, check elections |

---

## 4. SPECIFIC GAPS: Missing Integration Points

### Gap 1: ReportGradient Not Forwarded

**Location**: `websocket.rs` lines 639-662

**Current Code**:
```rust
ClientMessage::ReportGradient { cpu_available, memory_available, bandwidth_available, storage_available } => {
    let gradient_msg = WsMessage::GradientUpdate { ... };
    let _ = state.event_tx.send(gradient_msg);  // LOCAL ECHO ONLY!
}
```

**Problem**: Only broadcasts to WebSocket clients, never calls `EnrBridge.broadcast_gradient()`.

### Gap 2: StartElection Not Forwarded

**Location**: `websocket.rs` lines 665-677

**Current Code**:
```rust
ClientMessage::StartElection { region_id } => {
    let election_msg = WsMessage::ElectionAnnouncement { ... };
    let _ = state.event_tx.send(election_msg);  // LOCAL ECHO ONLY!
}
```

**Problem**: Only echoes to WebSocket clients, never calls `EnrBridge.trigger_election()`.

### Gap 3: RegisterCandidacy Not Forwarded

**Location**: `websocket.rs` lines 680-701

**Current Code**:
```rust
ClientMessage::RegisterCandidacy { election_id, uptime, ... } => {
    let candidacy_msg = WsMessage::ElectionCandidacy { ... };
    let _ = state.event_tx.send(candidacy_msg);  // LOCAL ECHO ONLY!
}
```

**Problem**: Never calls `EnrBridge.election.update_metrics()` or submits candidacy.

### Gap 4: VoteElection Not Forwarded

**Location**: `websocket.rs` lines 703-721

**Current Code**:
```rust
ClientMessage::VoteElection { election_id, candidate } => {
    let vote_msg = WsMessage::ElectionVote { ... };
    let _ = state.event_tx.send(vote_msg);  // LOCAL ECHO ONLY!
}
```

**Problem**: Never calls `EnrBridge.election.cast_vote()`.

### Gap 5: SendEnrCredit Not Forwarded

**Location**: `websocket.rs` lines 723-748

**Current Code**:
```rust
ClientMessage::SendEnrCredit { to, amount } => {
    let transfer_msg = WsMessage::EnrCreditTransfer { ... };
    let _ = state.event_tx.send(transfer_msg);
    let balance_msg = WsMessage::EnrBalanceUpdate { balance: 0, ... };  // PLACEHOLDER!
    let _ = state.event_tx.send(balance_msg);
}
```

**Problem**: Never calls `EnrBridge.transfer_credits()`, uses placeholder balance.

---

## 5. Wiring Needed: Exact Code Changes Required

### Step 1: Add EnrBridge to AppState

**File**: `/crates/mycelial-node/src/main.rs`

**Change AppState struct**:
```rust
pub struct AppState {
    // ... existing fields ...

    /// ENR bridge for economic operations
    #[cfg(feature = "univrs-compat")]
    pub enr_bridge: Arc<mycelial_network::enr_bridge::EnrBridge>,
}
```

**Problem**: EnrBridge is created inside NetworkService and not exposed. Options:
1. Return EnrBridge reference from `NetworkService::new()`
2. Create separate EnrBridge in main.rs (duplicate state - not ideal)
3. Add method to NetworkHandle to access EnrBridge operations

### Step 2: Modify NetworkService to Expose EnrBridge

**File**: `/crates/mycelial-network/src/service.rs`

**Option A**: Return EnrBridge from `new()`:
```rust
pub fn new(
    keypair: libp2p::identity::Keypair,
    config: NetworkConfig,
) -> Result<(Self, NetworkHandle, broadcast::Receiver<NetworkEvent>, Arc<EnrBridge>)> {
    // ... existing code ...
    Ok((service, handle, event_rx, enr_bridge.clone()))
}
```

**Option B**: Add EnrBridge commands to NetworkCommand enum:
```rust
pub enum NetworkCommand {
    // ... existing commands ...

    #[cfg(feature = "univrs-compat")]
    BroadcastGradient { gradient: ResourceGradient },

    #[cfg(feature = "univrs-compat")]
    TriggerElection { region_id: String, response: oneshot::Sender<Result<u64, ElectionError>> },

    #[cfg(feature = "univrs-compat")]
    TransferCredits { to: NodeId, amount: Credits, response: oneshot::Sender<Result<(), TransferError>> },
}
```

### Step 3: Update WebSocket Handler

**File**: `/crates/mycelial-node/src/server/websocket.rs`

**For each ENR Bridge ClientMessage variant**, replace local echo with actual EnrBridge calls:

```rust
ClientMessage::ReportGradient { cpu_available, memory_available, bandwidth_available, storage_available } => {
    #[cfg(feature = "univrs-compat")]
    {
        let gradient = univrs_enr::nexus::ResourceGradient {
            cpu_available,
            memory_available,
            bandwidth_available,
            storage_available,
            ..Default::default()
        };

        match state.enr_bridge.broadcast_gradient(gradient).await {
            Ok(()) => {
                // Broadcast success to WebSocket clients
                let gradient_msg = WsMessage::GradientUpdate { ... };
                let _ = state.event_tx.send(gradient_msg);
            }
            Err(e) => {
                error!("Failed to broadcast gradient: {}", e);
                let _ = state.event_tx.send(WsMessage::Error { message: e.to_string() });
            }
        }
    }
}

ClientMessage::SendEnrCredit { to, amount } => {
    #[cfg(feature = "univrs-compat")]
    {
        // Parse target NodeId
        let to_node = parse_node_id(&to)?;
        let credits = univrs_enr::core::Credits::new(amount);

        match state.enr_bridge.transfer_credits(to_node, credits).await {
            Ok(()) => {
                let balance = state.enr_bridge.local_balance().await;
                let transfer_msg = WsMessage::EnrCreditTransfer { ... };
                let _ = state.event_tx.send(transfer_msg);
                let balance_msg = WsMessage::EnrBalanceUpdate {
                    balance: balance.amount,  // Actual balance!
                    ...
                };
                let _ = state.event_tx.send(balance_msg);
            }
            Err(e) => {
                let _ = state.event_tx.send(WsMessage::Error { message: e.to_string() });
            }
        }
    }
}
```

### Step 4: Add NodeId Parsing Helper

**File**: `/crates/mycelial-node/src/server/websocket.rs`

```rust
#[cfg(feature = "univrs-compat")]
fn parse_node_id(s: &str) -> Result<univrs_enr::core::NodeId, &'static str> {
    // NodeId is 32 bytes, typically hex-encoded
    let bytes = hex::decode(s).map_err(|_| "Invalid hex")?;
    if bytes.len() != 32 {
        return Err("NodeId must be 32 bytes");
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(univrs_enr::core::NodeId::from_bytes(arr))
}
```

### Step 5: Add Cargo Dependencies

**File**: `/crates/mycelial-node/Cargo.toml`

```toml
[features]
default = []
univrs-compat = ["mycelial-network/univrs-compat"]

[dependencies]
# Add univrs-enr for direct type access
univrs-enr = { workspace = true, optional = true }
hex = "0.4"  # For NodeId parsing
```

---

## 6. Summary of Required Changes

| File | Change Type | Description |
|------|-------------|-------------|
| `mycelial-node/Cargo.toml` | Add | `univrs-compat` feature, `univrs-enr` dep |
| `mycelial-network/src/service.rs` | Modify | Expose EnrBridge from `new()` or add commands |
| `mycelial-node/src/main.rs` | Modify | Add EnrBridge to AppState |
| `mycelial-node/src/server/websocket.rs` | Modify | Wire ENR handlers to EnrBridge methods |
| `mycelial-node/src/server/mod.rs` | Modify | Pass EnrBridge to handlers |

---

## 7. Testing Strategy

After implementing the changes:

1. **Unit Tests**: Mock EnrBridge and verify WebSocket handlers call correct methods
2. **Integration Tests**:
   - Send `ReportGradient` via WebSocket, verify gradient appears in network
   - Send `SendEnrCredit`, verify balance changes
   - Start election via WebSocket, verify election progresses
3. **Multi-Node Tests**:
   - Node A broadcasts gradient, Node B receives it
   - Node A starts election, Node B participates

---

## 8. Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking existing functionality | High | Feature-flag all changes behind `univrs-compat` |
| Type mismatches | Medium | Use shared types from `univrs-enr` |
| Race conditions | Medium | EnrBridge already uses Arc<RwLock<>> internally |
| Balance inconsistency | High | Use actual EnrBridge balance, not placeholders |
