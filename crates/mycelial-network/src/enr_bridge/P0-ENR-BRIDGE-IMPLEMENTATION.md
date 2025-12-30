# Phase 0: ENR Bridge MVP Implementation Plan

> **Deadline:** Week 2 (Jan 13, 2026)  
> **Objective:** Connect P2P gossip layer to ENR economics  
> **Strategy:** Claude Flow @alpha hive-mind parallel execution

---

## 1. Current State Analysis

### univrs-enr (✅ READY)
All types confirmed present and tested:
```
src/
├── core/types.rs      → Credits, CreditTransfer, NodeId, AccountId, Timestamp
├── core/state.rs      → CreditState (Active→Reserved→Consumed flow)
├── core/invariants.rs → CreditConservation, SeptalSafety
├── entropy/           → EntropyAccount, entropy_price_multiplier()
├── nexus/types.rs     → ResourceGradient, NexusRole, NexusTopology
├── revival/pool.rs    → RevivalPool, calculate_entropy_tax()
├── septal/gate.rs     → SeptalGate, SeptalGateState
└── pricing/           → PriceQuote, calculate_dynamic_price()
```

### univrs-network (🔧 NEEDS enr_bridge)
Workspace structure with libp2p:
```
crates/
├── mycelial-core/       → Shared types
├── mycelial-network/    → libp2p gossipsub + kad
├── mycelial-protocol/   → Message definitions
├── mycelial-state/      → State management
├── mycelial-wasm/       → Browser bindings
└── mycelial-node/       → Node binary
```

Uses: `libp2p` 0.54 with `gossipsub`, `kad`, `quic`, `noise`

---

## 2. MVP Scope (2 Weeks)

### In Scope (MVP)
| Module | Purpose | Key Types |
|--------|---------|-----------|
| `gradient.rs` | Broadcast resource availability via gossip | `ResourceGradient`, `GradientMessage` |
| `credits.rs` | Synchronize credit transfers | `CreditTransfer`, `BalanceQuery` |
| `messages.rs` | Serde-serialized ENR message envelope | `EnrMessage` enum |
| `mod.rs` | Bridge coordinator | `EnrBridge` struct |

### Deferred to Q2
| Module | Reason |
|--------|--------|
| `nexus.rs` | Full distributed election needs OpenRaft |
| `septal.rs` | Circuit breakers need network health metrics |
| Full P2P consensus | Start with centralized ledger fallback |

---

## 3. File Structure

```
mycelial-network/src/enr_bridge/
├── mod.rs           # EnrBridge coordinator, exports
├── gradient.rs      # GradientBroadcaster using gossipsub
├── credits.rs       # CreditSynchronizer with local ledger
├── messages.rs      # EnrMessage enum with CBOR serialization
└── tests/
    └── integration.rs  # 2 gate tests (gradient + credits)
```

**Cargo.toml additions:**
```toml
[dependencies]
univrs-enr = { path = "../../univrs-enr" }
serde_cbor = "0.11"
```

---

## 4. Implementation Details

### 4.1 messages.rs
```rust
//! ENR Message Types for P2P Transport

use serde::{Deserialize, Serialize};
use univrs_enr::{
    Credits, CreditTransfer, NodeId, ResourceGradient, Timestamp,
};

/// Topic names for gossipsub
pub const GRADIENT_TOPIC: &str = "/vudo/enr/gradient/1.0.0";
pub const CREDIT_TOPIC: &str = "/vudo/enr/credits/1.0.0";

/// Envelope for all ENR messages over gossip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnrMessage {
    /// Gradient broadcast from a node
    GradientUpdate(GradientUpdate),
    /// Credit transfer announcement
    CreditTransfer(CreditTransferMsg),
    /// Balance query request
    BalanceQuery(BalanceQueryMsg),
    /// Balance query response
    BalanceResponse(BalanceResponseMsg),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientUpdate {
    pub source: NodeId,
    pub gradient: ResourceGradient,
    pub timestamp: Timestamp,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransferMsg {
    pub transfer: CreditTransfer,
    pub nonce: u64,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceQueryMsg {
    pub requester: NodeId,
    pub target: NodeId,
    pub request_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponseMsg {
    pub request_id: u64,
    pub balance: Credits,
    pub as_of: Timestamp,
}

impl EnrMessage {
    pub fn encode(&self) -> Result<Vec<u8>, serde_cbor::Error> {
        serde_cbor::to_vec(self)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, serde_cbor::Error> {
        serde_cbor::from_slice(bytes)
    }
}
```

### 4.2 gradient.rs
```rust
//! Gradient Broadcasting via Gossipsub

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use univrs_enr::{NodeId, ResourceGradient, Timestamp};
use crate::enr_bridge::messages::{EnrMessage, GradientUpdate, GRADIENT_TOPIC};

/// Maximum age of gradient before considered stale (15 seconds)
pub const MAX_GRADIENT_AGE_MS: u64 = 15_000;

/// Manages gradient state and broadcasting
pub struct GradientBroadcaster {
    local_node: NodeId,
    /// Received gradients from other nodes
    gradients: Arc<RwLock<HashMap<NodeId, GradientUpdate>>>,
    /// Callback to publish to gossipsub
    publish_fn: Box<dyn Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync>,
}

impl GradientBroadcaster {
    pub fn new<F>(local_node: NodeId, publish_fn: F) -> Self
    where
        F: Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync + 'static,
    {
        Self {
            local_node,
            gradients: Arc::new(RwLock::new(HashMap::new())),
            publish_fn: Box::new(publish_fn),
        }
    }

    /// Broadcast local gradient to network
    pub async fn broadcast_update(&self, gradient: ResourceGradient) -> Result<(), String> {
        let update = GradientUpdate {
            source: self.local_node,
            gradient,
            timestamp: Timestamp::now(),
            signature: vec![], // TODO: Sign with Ed25519
        };
        
        let msg = EnrMessage::GradientUpdate(update);
        let bytes = msg.encode().map_err(|e| e.to_string())?;
        
        (self.publish_fn)(GRADIENT_TOPIC.to_string(), bytes)
    }

    /// Handle incoming gradient from gossip
    pub async fn handle_gradient(&self, update: GradientUpdate) -> Result<(), String> {
        // Validate timestamp not in future
        let now = Timestamp::now();
        if update.timestamp.millis > now.millis + 5000 {
            return Err("Gradient timestamp in future".to_string());
        }
        
        // TODO: Verify signature
        
        let mut gradients = self.gradients.write().await;
        
        // Only update if newer
        if let Some(existing) = gradients.get(&update.source) {
            if existing.timestamp.millis >= update.timestamp.millis {
                return Ok(()); // Ignore older update
            }
        }
        
        gradients.insert(update.source, update);
        Ok(())
    }

    /// Get aggregated view of network gradients
    pub async fn get_network_gradient(&self) -> ResourceGradient {
        let gradients = self.gradients.read().await;
        let now = Timestamp::now();
        
        // Filter stale gradients and aggregate
        let fresh: Vec<_> = gradients
            .values()
            .filter(|g| now.millis - g.timestamp.millis < MAX_GRADIENT_AGE_MS)
            .collect();
        
        if fresh.is_empty() {
            return ResourceGradient::zero();
        }
        
        // Simple average aggregation
        let count = fresh.len() as f64;
        ResourceGradient {
            cpu_available: fresh.iter().map(|g| g.gradient.cpu_available).sum::<f64>() / count,
            memory_available: fresh.iter().map(|g| g.gradient.memory_available).sum::<f64>() / count,
            gpu_available: fresh.iter().map(|g| g.gradient.gpu_available).sum::<f64>() / count,
            storage_available: fresh.iter().map(|g| g.gradient.storage_available).sum::<f64>() / count,
            bandwidth_available: fresh.iter().map(|g| g.gradient.bandwidth_available).sum::<f64>() / count,
            credit_balance: fresh.iter().map(|g| g.gradient.credit_balance).sum::<f64>() / count,
        }
    }

    /// Prune stale gradients
    pub async fn prune_stale(&self) {
        let mut gradients = self.gradients.write().await;
        let now = Timestamp::now();
        gradients.retain(|_, g| now.millis - g.timestamp.millis < MAX_GRADIENT_AGE_MS * 2);
    }
}
```

### 4.3 credits.rs
```rust
//! Credit Synchronization with Local Ledger

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use univrs_enr::{
    AccountId, AccountType, Credits, CreditTransfer, NodeId, Timestamp,
    calculate_entropy_tax,
};
use crate::enr_bridge::messages::{
    EnrMessage, CreditTransferMsg, BalanceQueryMsg, BalanceResponseMsg, CREDIT_TOPIC,
};

/// Initial credit grant for new nodes
pub const INITIAL_NODE_CREDITS: u64 = 1000;

/// Local credit ledger (MVP: single-node source of truth)
pub struct CreditSynchronizer {
    local_node: NodeId,
    /// Local ledger: AccountId -> balance
    ledger: Arc<RwLock<HashMap<AccountId, Credits>>>,
    /// Pending transfers awaiting confirmation
    pending: Arc<RwLock<HashMap<u64, CreditTransferMsg>>>,
    /// Next nonce for outgoing transfers
    next_nonce: Arc<RwLock<u64>>,
    /// Callback to publish to gossipsub
    publish_fn: Box<dyn Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync>,
}

impl CreditSynchronizer {
    pub fn new<F>(local_node: NodeId, publish_fn: F) -> Self
    where
        F: Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync + 'static,
    {
        let mut ledger = HashMap::new();
        // Initialize local node with starting credits
        let local_account = AccountId::node_account(local_node);
        ledger.insert(local_account, Credits::new(INITIAL_NODE_CREDITS));
        
        Self {
            local_node,
            ledger: Arc::new(RwLock::new(ledger)),
            pending: Arc::new(RwLock::new(HashMap::new())),
            next_nonce: Arc::new(RwLock::new(1)),
            publish_fn: Box::new(publish_fn),
        }
    }

    /// Get balance for an account
    pub async fn get_balance(&self, account: &AccountId) -> Credits {
        let ledger = self.ledger.read().await;
        ledger.get(account).copied().unwrap_or(Credits::ZERO)
    }

    /// Get local node's balance
    pub async fn local_balance(&self) -> Credits {
        let account = AccountId::node_account(self.local_node);
        self.get_balance(&account).await
    }

    /// Transfer credits to another node
    pub async fn transfer(&self, to: NodeId, amount: Credits) -> Result<CreditTransfer, String> {
        let from_account = AccountId::node_account(self.local_node);
        let to_account = AccountId::node_account(to);
        
        // Calculate entropy tax
        let entropy_cost = calculate_entropy_tax(amount);
        let total_cost = amount.saturating_add(entropy_cost);
        
        // Check balance
        let mut ledger = self.ledger.write().await;
        let from_balance = ledger.get(&from_account).copied().unwrap_or(Credits::ZERO);
        
        if from_balance.amount < total_cost.amount {
            return Err(format!(
                "Insufficient credits: have {}, need {} (including {} tax)",
                from_balance.amount, total_cost.amount, entropy_cost.amount
            ));
        }
        
        // Debit sender
        ledger.insert(from_account.clone(), from_balance.saturating_sub(total_cost));
        
        // Credit receiver
        let to_balance = ledger.get(&to_account).copied().unwrap_or(Credits::ZERO);
        ledger.insert(to_account.clone(), to_balance.saturating_add(amount));
        
        drop(ledger);
        
        // Create transfer record
        let transfer = CreditTransfer::new(from_account, to_account, amount, entropy_cost);
        
        // Get nonce and broadcast
        let nonce = {
            let mut n = self.next_nonce.write().await;
            let current = *n;
            *n += 1;
            current
        };
        
        let msg = CreditTransferMsg {
            transfer: transfer.clone(),
            nonce,
            signature: vec![], // TODO: Sign
        };
        
        let envelope = EnrMessage::CreditTransfer(msg.clone());
        let bytes = envelope.encode().map_err(|e| e.to_string())?;
        (self.publish_fn)(CREDIT_TOPIC.to_string(), bytes)?;
        
        // Track pending
        self.pending.write().await.insert(nonce, msg);
        
        Ok(transfer)
    }

    /// Handle incoming transfer from gossip
    pub async fn handle_transfer(&self, msg: CreditTransferMsg) -> Result<(), String> {
        // TODO: Verify signature
        // TODO: Check for replay (nonce tracking)
        
        let transfer = &msg.transfer;
        
        // Skip if this is our own transfer (already applied)
        if transfer.from.node == self.local_node {
            return Ok(());
        }
        
        let mut ledger = self.ledger.write().await;
        
        // Apply transfer (MVP: trust-based, consensus comes later)
        let from_balance = ledger.get(&transfer.from).copied().unwrap_or(Credits::ZERO);
        let total_cost = transfer.amount.saturating_add(transfer.entropy_cost);
        
        if from_balance.amount >= total_cost.amount {
            ledger.insert(transfer.from.clone(), from_balance.saturating_sub(total_cost));
            
            let to_balance = ledger.get(&transfer.to).copied().unwrap_or(Credits::ZERO);
            ledger.insert(transfer.to.clone(), to_balance.saturating_add(transfer.amount));
        }
        
        Ok(())
    }

    /// Query balance from another node (for verification)
    pub async fn query_balance(&self, target: NodeId) -> Result<u64, String> {
        let request_id = rand::random();
        
        let msg = BalanceQueryMsg {
            requester: self.local_node,
            target,
            request_id,
        };
        
        let envelope = EnrMessage::BalanceQuery(msg);
        let bytes = envelope.encode().map_err(|e| e.to_string())?;
        (self.publish_fn)(CREDIT_TOPIC.to_string(), bytes)?;
        
        Ok(request_id)
    }

    /// Handle balance query from another node
    pub async fn handle_balance_query(&self, query: BalanceQueryMsg) -> Result<(), String> {
        if query.target != self.local_node {
            return Ok(()); // Not for us
        }
        
        let balance = self.local_balance().await;
        
        let response = BalanceResponseMsg {
            request_id: query.request_id,
            balance,
            as_of: Timestamp::now(),
        };
        
        let envelope = EnrMessage::BalanceResponse(response);
        let bytes = envelope.encode().map_err(|e| e.to_string())?;
        (self.publish_fn)(CREDIT_TOPIC.to_string(), bytes)?;
        
        Ok(())
    }

    /// Ensure account exists with minimum balance
    pub async fn ensure_account(&self, node: NodeId) {
        let account = AccountId::node_account(node);
        let mut ledger = self.ledger.write().await;
        ledger.entry(account).or_insert(Credits::new(INITIAL_NODE_CREDITS));
    }
}
```

### 4.4 mod.rs
```rust
//! ENR Bridge - Connects P2P Gossip to ENR Economics
//!
//! MVP Scope:
//! - Gradient broadcasting via gossipsub
//! - Credit synchronization with local ledger
//!
//! Deferred:
//! - Distributed nexus election (needs OpenRaft)
//! - Septal gates (needs network health metrics)

pub mod credits;
pub mod gradient;
pub mod messages;

pub use credits::CreditSynchronizer;
pub use gradient::GradientBroadcaster;
pub use messages::{EnrMessage, CREDIT_TOPIC, GRADIENT_TOPIC};

use univrs_enr::{Credits, NodeId, ResourceGradient};

/// Unified ENR Bridge coordinator
pub struct EnrBridge {
    pub gradient: GradientBroadcaster,
    pub credits: CreditSynchronizer,
}

impl EnrBridge {
    pub fn new<F>(local_node: NodeId, publish_fn: F) -> Self
    where
        F: Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync + Clone + 'static,
    {
        Self {
            gradient: GradientBroadcaster::new(local_node, publish_fn.clone()),
            credits: CreditSynchronizer::new(local_node, publish_fn),
        }
    }

    /// Handle incoming ENR message from gossip
    pub async fn handle_message(&self, bytes: &[u8]) -> Result<(), String> {
        let msg = EnrMessage::decode(bytes).map_err(|e| e.to_string())?;
        
        match msg {
            EnrMessage::GradientUpdate(update) => {
                self.gradient.handle_gradient(update).await
            }
            EnrMessage::CreditTransfer(transfer) => {
                self.credits.handle_transfer(transfer).await
            }
            EnrMessage::BalanceQuery(query) => {
                self.credits.handle_balance_query(query).await
            }
            EnrMessage::BalanceResponse(_response) => {
                // TODO: Handle balance response for verification
                Ok(())
            }
        }
    }

    /// Broadcast local resource gradient
    pub async fn broadcast_gradient(&self, gradient: ResourceGradient) -> Result<(), String> {
        self.gradient.broadcast_update(gradient).await
    }

    /// Transfer credits to another node
    pub async fn transfer_credits(&self, to: NodeId, amount: Credits) -> Result<(), String> {
        self.credits.transfer(to, amount).await?;
        Ok(())
    }

    /// Get local credit balance
    pub async fn local_balance(&self) -> Credits {
        self.credits.local_balance().await
    }

    /// Get network-wide gradient view
    pub async fn network_gradient(&self) -> ResourceGradient {
        self.gradient.get_network_gradient().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn mock_publish() -> (impl Fn(String, Vec<u8>) -> Result<(), String> + Clone, Arc<AtomicUsize>) {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        let f = move |_topic: String, _bytes: Vec<u8>| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        };
        (f, counter)
    }

    #[tokio::test]
    async fn test_bridge_creation() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, _) = mock_publish();
        let bridge = EnrBridge::new(node, publish);
        
        // Should have initial credits
        let balance = bridge.local_balance().await;
        assert_eq!(balance.amount, 1000);
    }

    #[tokio::test]
    async fn test_gradient_roundtrip() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, counter) = mock_publish();
        let bridge = EnrBridge::new(node, publish);
        
        let gradient = ResourceGradient {
            cpu_available: 0.5,
            memory_available: 0.6,
            gpu_available: 0.0,
            storage_available: 0.8,
            bandwidth_available: 0.9,
            credit_balance: 1000.0,
        };
        
        bridge.broadcast_gradient(gradient).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_credit_transfer() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, counter) = mock_publish();
        let bridge = EnrBridge::new(node1, publish);
        
        // Transfer 100 credits
        bridge.transfer_credits(node2, Credits::new(100)).await.unwrap();
        
        // Should have broadcast
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        
        // Balance should be reduced (100 + 2% tax = 102)
        let balance = bridge.local_balance().await;
        assert_eq!(balance.amount, 898); // 1000 - 102
    }
}
```

---

## 5. Claude Flow Swarm Commands

### Phase 0 Week 1: Core Implementation

```bash
# Day 1-2: Messages and Gradient
npx claude-flow@alpha hive-mind \
  --task "Implement enr_bridge messages and gradient modules" \
  --context "P2P ENR bridge for VUDO - connects libp2p gossipsub to ENR economics" \
  --files \
    "mycelial-network/src/enr_bridge/messages.rs:EnrMessage enum with GradientUpdate, CreditTransferMsg, BalanceQuery/Response. CBOR serialization. Topic constants." \
    "mycelial-network/src/enr_bridge/gradient.rs:GradientBroadcaster struct. broadcast_update() publishes to gossip. handle_gradient() stores updates. get_network_gradient() aggregates. Prune stale (>15s)." \
  --dependencies "univrs-enr = { path = '../../univrs-enr' }" \
  --iterations 2 \
  --success-criteria "cargo check passes, basic unit tests pass"

# Day 3-4: Credits Module  
npx claude-flow@alpha hive-mind \
  --task "Implement enr_bridge credits synchronization" \
  --context "Credit ledger MVP - local HashMap, broadcast transfers via gossip, apply entropy tax" \
  --files \
    "mycelial-network/src/enr_bridge/credits.rs:CreditSynchronizer struct. Local ledger HashMap<AccountId, Credits>. transfer() deducts + tax + broadcasts. handle_transfer() applies incoming. query_balance() for verification." \
  --imports "use univrs_enr::{Credits, CreditTransfer, NodeId, AccountId, calculate_entropy_tax};" \
  --iterations 2 \
  --success-criteria "transfer test: 1000 -> transfer 100 -> balance 898 (2% tax)"

# Day 5: Integration
npx claude-flow@alpha hive-mind \
  --task "Create EnrBridge coordinator and integration tests" \
  --context "Unified bridge tying gradient + credits together, route messages by type" \
  --files \
    "mycelial-network/src/enr_bridge/mod.rs:EnrBridge struct containing GradientBroadcaster + CreditSynchronizer. handle_message() dispatcher. Convenience methods." \
    "mycelial-network/src/enr_bridge/tests/integration.rs:Test gradient propagation (5 nodes, all receive in 15s). Test credit transfer (3 nodes, balances correct)." \
  --iterations 2 \
  --success-criteria "cargo test --all passes"
```

### Phase 0 Week 2: Gate Tests

```bash
# Integration with actual libp2p
npx claude-flow@alpha hive-mind \
  --task "Connect EnrBridge to mycelial-network gossipsub" \
  --context "Wire EnrBridge.publish_fn to actual gossipsub.publish(), subscribe to GRADIENT_TOPIC and CREDIT_TOPIC" \
  --files \
    "mycelial-network/src/lib.rs:Add pub mod enr_bridge, re-exports" \
    "mycelial-node/src/main.rs:Initialize EnrBridge, wire to swarm event loop" \
  --iterations 3 \
  --success-criteria "2 nodes exchange gradient updates over network"
```

---

## 6. Gate Test Criteria

```rust
// tests/enr_bridge_integration.rs

/// GATE TEST 1: Gradient propagates to all nodes within 15 seconds
#[tokio::test]
async fn gate_gradient_propagation() {
    let cluster = TestCluster::new(5).await;
    
    // Node 0 broadcasts gradient
    let gradient = ResourceGradient {
        cpu_available: 0.42,
        memory_available: 0.73,
        ..Default::default()
    };
    cluster.nodes[0].bridge.broadcast_gradient(gradient).await.unwrap();
    
    // Wait and verify all nodes received
    tokio::time::sleep(Duration::from_secs(15)).await;
    
    for (i, node) in cluster.nodes.iter().enumerate() {
        if i == 0 { continue; }
        let net_gradient = node.bridge.network_gradient().await;
        assert!((net_gradient.cpu_available - 0.42).abs() < 0.1,
            "Node {} didn't receive gradient", i);
    }
}

/// GATE TEST 2: Credit transfer updates all ledgers correctly
#[tokio::test]
async fn gate_credit_transfer() {
    let cluster = TestCluster::new(3).await;
    
    // Node 0 transfers 100 to Node 1
    let amount = Credits::new(100);
    cluster.nodes[0].bridge
        .transfer_credits(cluster.nodes[1].id, amount)
        .await
        .unwrap();
    
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Verify balances
    // Node 0: 1000 - 100 - 2 (tax) = 898
    let bal0 = cluster.nodes[0].bridge.local_balance().await;
    assert_eq!(bal0.amount, 898);
    
    // Node 1: 1000 + 100 = 1100
    let bal1 = cluster.nodes[1].bridge.local_balance().await;
    assert_eq!(bal1.amount, 1100);
}
```

---

## 7. Timeline

| Day | Task | Deliverable |
|-----|------|-------------|
| 1 | messages.rs | EnrMessage enum compiles |
| 2 | gradient.rs | GradientBroadcaster unit tests pass |
| 3 | credits.rs | CreditSynchronizer unit tests pass |
| 4 | mod.rs + integration | EnrBridge coordinator works |
| 5 | Wire to libp2p | Gossipsub integration |
| 6-7 | Buffer | Fix issues, polish |
| 8-10 | Gate tests | Both gate tests pass |

---

## 8. Parallel Work (Web MVP Scaffolding)

While P0 executes, start P1 scaffolding:

```bash
npx claude-flow@alpha hive-mind \
  --task "Scaffold vudo-web Vite + React + Monaco" \
  --context "Browser DOL editor - MVP is compile and run, not full IDE" \
  --subtasks \
    "npm create vite@latest vudo-web -- --template react-ts" \
    "Add @monaco-editor/react, create DOLEditor component" \
    "Create worker/compiler.worker.ts scaffold for WASM" \
    "Set up Axum API at /api/compile for cloud fallback" \
  --iterations 2
```

---

## 9. Success Criteria

**Phase 0 Complete When:**
- [ ] `cargo build -p mycelial-network` succeeds with enr_bridge
- [ ] `cargo test -p mycelial-network` all pass
- [ ] Gate test 1: Gradient propagation < 15s
- [ ] Gate test 2: Credit transfer balances correct

**Ready for Phase 3 When:**
- [ ] Phase 0 complete
- [ ] Phase 1 web editor working (parallel track)
- [ ] API endpoint for credit operations defined
