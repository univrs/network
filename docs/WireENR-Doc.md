# ENR Bridge Wiring Documentation

> **Complete integration guide for ENR economics across all system layers**

This document explains how ENR (Economic Network Resource) primitives are wired through the univrs-network stack, from Rust gossipsub to React dashboard.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         ENR WIRING ARCHITECTURE                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  PHASE 4: UI Components (React)                        [COMPLETE]   │   │
│  │    • GradientPanel - visualize resource availability               │   │
│  │    • ElectionPanel - nexus election tracking                       │   │
│  │    • SeptalPanel - circuit breaker status                          │   │
│  │    • EnrCreditPanel - ENR balance & transfers                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  PHASE 3: Dashboard Hooks (TypeScript)                 [COMPLETE]   │   │
│  │    • useP2P.ts - WebSocket message handlers for ENR               │   │
│  │    • useEconomicsAPI.ts - REST API polling + state                 │   │
│  │    • types.ts - TypeScript interfaces for all ENR types            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  PHASE 1: WebSocket Server (Rust)                      [COMPLETE]   │   │
│  │    • messages.rs - WsMessage enum with ENR variants                │   │
│  │    • websocket.rs - broadcast ENR events to dashboard              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  PHASE 0: ENR Bridge (Rust)                            [COMPLETE]   │   │
│  │    • gradient.rs - GradientBroadcaster via gossipsub               │   │
│  │    • credits.rs - CreditSynchronizer with local ledger             │   │
│  │    • nexus.rs - DistributedElection for hub selection              │   │
│  │    • septal.rs - SeptalGateManager (circuit breakers)              │   │
│  │    • messages.rs - CBOR-encoded EnrMessage envelope                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  univrs-enr (External Crate)                           [READY]      │   │
│  │    • Credits, CreditTransfer, NodeId, AccountId                    │   │
│  │    • ResourceGradient, NexusRole, NexusTopology                    │   │
│  │    • SeptalGate, SeptalGateState                                   │   │
│  │    • calculate_entropy_tax()                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 0: ENR Bridge Core (Rust)

**Location:** `crates/mycelial-network/src/enr_bridge/`

### Gossipsub Topics

| Topic | Purpose | Message Type |
|-------|---------|--------------|
| `/vudo/enr/gradient/1.0.0` | Resource availability broadcasts | `GradientUpdate` |
| `/vudo/enr/credits/1.0.0` | Credit transfers & balance queries | `CreditTransferMsg`, `BalanceQuery` |
| `/vudo/enr/election/1.0.0` | Nexus hub elections | `ElectionMessage` |
| `/vudo/enr/septal/1.0.0` | Circuit breaker state changes | `SeptalMessage` |

### Message Envelope (`messages.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnrMessage {
    GradientUpdate(GradientUpdate),
    CreditTransfer(CreditTransferMsg),
    BalanceQuery(BalanceQueryMsg),
    BalanceResponse(BalanceResponseMsg),
    Election(ElectionMessage),
    Septal(SeptalMessage),
}
```

### Key Components

| Module | Struct | Purpose |
|--------|--------|---------|
| `gradient.rs` | `GradientBroadcaster` | Broadcast/receive resource availability |
| `credits.rs` | `CreditSynchronizer` | Local ledger + transfer gossip |
| `nexus.rs` | `DistributedElection` | Hub election state machine |
| `septal.rs` | `SeptalGateManager` | Circuit breaker per-peer |
| `mod.rs` | `EnrBridge` | Unified coordinator |

### Integration Point: `EnrBridge`

```rust
// crates/mycelial-network/src/enr_bridge/mod.rs

pub struct EnrBridge {
    pub gradient: GradientBroadcaster,
    pub credits: CreditSynchronizer,
    pub election: DistributedElection,
    pub septal: SeptalGateManager,
}

impl EnrBridge {
    // Route incoming gossip messages
    pub async fn handle_message(&self, bytes: &[u8]) -> Result<(), HandleError>;

    // Broadcast local gradient
    pub async fn broadcast_gradient(&self, gradient: ResourceGradient);

    // Transfer credits with entropy tax
    pub async fn transfer_credits(&self, to: NodeId, amount: Credits);

    // Circuit breaker controls
    pub async fn record_peer_failure(&self, peer: NodeId, reason: &str);
    pub async fn allows_traffic(&self, peer: &NodeId) -> bool;
}
```

---

## Phase 1: WebSocket Server (Rust)

**Location:** `crates/mycelial-node/src/server/`

### WsMessage Variants (`messages.rs`)

ENR events are forwarded to dashboard clients via these WebSocket message types:

```rust
// crates/mycelial-node/src/server/messages.rs

pub enum WsMessage {
    // ... existing chat/peer messages ...

    // ENR Bridge Messages
    GradientUpdate {
        source: String,
        cpu_available: f64,
        memory_available: f64,
        bandwidth_available: f64,
        storage_available: f64,
        timestamp: i64,
    },

    EnrCreditTransfer {
        from: String,
        to: String,
        amount: u64,
        tax: u64,
        nonce: u64,
        timestamp: i64,
    },

    EnrBalanceUpdate {
        node_id: String,
        balance: u64,
        timestamp: i64,
    },

    ElectionAnnouncement {
        election_id: u64,
        initiator: String,
        region_id: String,
        timestamp: i64,
    },

    ElectionCandidacy {
        election_id: u64,
        candidate: String,
        uptime: u64,
        cpu_available: f64,
        memory_available: f64,
        reputation: f64,
        timestamp: i64,
    },

    ElectionVote {
        election_id: u64,
        voter: String,
        candidate: String,
        timestamp: i64,
    },

    ElectionResult {
        election_id: u64,
        winner: String,
        region_id: String,
        vote_count: u32,
        timestamp: i64,
    },

    SeptalStateChange {
        node_id: String,
        from_state: String,  // "closed" | "open" | "half_open"
        to_state: String,
        reason: String,
        timestamp: i64,
    },

    SeptalHealthStatus {
        node_id: String,
        is_healthy: bool,
        failure_count: u32,
        timestamp: i64,
    },
}
```

### Event Broadcasting (`websocket.rs`)

ENR events are broadcast to all connected WebSocket clients:

```rust
// When ENR bridge receives a gossip message, forward to dashboard
fn forward_enr_event(bridge_event: EnrEvent, event_tx: &broadcast::Sender<WsMessage>) {
    let ws_msg = match bridge_event {
        EnrEvent::GradientUpdate(g) => WsMessage::GradientUpdate { ... },
        EnrEvent::CreditTransfer(t) => WsMessage::EnrCreditTransfer { ... },
        EnrEvent::Election(e) => match e { ... },
        EnrEvent::Septal(s) => match s { ... },
    };
    let _ = event_tx.send(ws_msg);
}
```

---

## Phase 2: Per-Peer State Tracking

**Location:** `crates/mycelial-node/src/server/rest.rs` (planned)

### REST API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/economics` | GET | Economics summary |
| `/api/economics/credit-lines` | GET | All credit lines |
| `/api/economics/proposals` | GET | Governance proposals |
| `/api/economics/vouches` | GET | Vouch requests |
| `/api/economics/resources` | GET | Resource contributions |
| `/api/economics/peer/:id` | GET | Per-peer economics |

### Response Formats

```json
// GET /api/economics
{
  "total_credit_lines": 5,
  "total_credit_limit": 5000,
  "total_credit_balance": 1250,
  "active_proposals": 2,
  "total_vouches": 3,
  "total_resource_contributions": 8,
  "enr_total_balance": 10000,
  "active_elections": 0
}

// GET /api/economics/peer/:id
{
  "peer_id": "12D3KooW...",
  "enr_balance": 1000,
  "gradient": {
    "cpu_available": 0.75,
    "memory_available": 0.60,
    "bandwidth_available": 0.90,
    "storage_available": 0.50
  },
  "septal_state": "closed",
  "septal_healthy": true,
  "failure_count": 0
}
```

---

## Phase 3: Dashboard Integration (TypeScript)

**Location:** `dashboard/src/`

### TypeScript Types (`types.ts`)

```typescript
// dashboard/src/types.ts

// Resource gradient from a node
export interface GradientUpdate {
  source: string;
  cpuAvailable: number;
  memoryAvailable: number;
  bandwidthAvailable: number;
  storageAvailable: number;
  timestamp: number;
}

// ENR credit transfer (separate from mutual credit)
export interface EnrCreditTransfer {
  from: string;
  to: string;
  amount: number;
  tax: number;
  nonce: number;
  timestamp: number;
}

// Septal gate states
export type SeptalState = 'closed' | 'open' | 'half_open';

// Per-node ENR state
export interface NodeEnrState {
  nodeId: string;
  balance: number;
  gradient?: GradientUpdate;
  septalState: SeptalState;
  septalHealthy: boolean;
  failureCount: number;
  lastUpdated: number;
}

// Election tracking
export interface Election {
  id: number;
  regionId: string;
  initiator: string;
  status: 'announced' | 'voting' | 'completed';
  candidates: ElectionCandidacy[];
  votes: ElectionVote[];
  winner?: string;
  startedAt: number;
  completedAt?: number;
}
```

### WebSocket Hook (`useP2P.ts`)

```typescript
// dashboard/src/hooks/useP2P.ts

interface P2PState {
  // ... existing fields ...

  // ENR Bridge state
  gradients: Map<string, GradientUpdate>;
  enrTransfers: EnrCreditTransfer[];
  nodeEnrStates: Map<string, NodeEnrState>;
  elections: Map<number, Election>;
}

// Message handlers added for 9 ENR message types:
case 'gradient_update': { ... }
case 'enr_credit_transfer': { ... }
case 'enr_balance_update': { ... }
case 'election_announcement': { ... }
case 'election_candidacy': { ... }
case 'election_vote': { ... }
case 'election_result': { ... }
case 'septal_state_change': { ... }
case 'septal_health_status': { ... }
```

### Economics API Hook (`useEconomicsAPI.ts`)

```typescript
// dashboard/src/hooks/useEconomicsAPI.ts

export interface EconomicsState {
  creditLines: CreditLine[];
  proposals: Proposal[];
  vouches: VouchRequest[];
  resourceContributions: ResourceContribution[];
  nodeEnrStates: Map<string, NodeEnrState>;
  gradients: Map<string, GradientUpdate>;
  elections: Map<number, Election>;
  summary: EconomicsSummary | null;
}

export function useEconomicsAPI(options: UseEconomicsAPIOptions = {}) {
  // REST API polling with configurable interval
  // Mock data fallback for development
  // Methods for updating state from WebSocket
  return {
    creditLines, proposals, vouches, resourceContributions,
    nodeEnrStates, gradients, elections, summary,
    loading, error,
    fetchPeerEconomics, refreshData, clearError,
    updateNodeEnrState, updateGradient, updateElection,
  };
}
```

---

## Phase 4: ENR UI Components (Planned)

**Location:** `dashboard/src/components/` (to be created)

### Component Mapping

| Component | Data Source | Purpose |
|-----------|-------------|---------|
| `GradientPanel.tsx` | `gradients` Map | Network-wide resource heatmap |
| `ElectionPanel.tsx` | `elections` Map | Live election progress |
| `SeptalPanel.tsx` | `nodeEnrStates` Map | Circuit breaker dashboard |
| `EnrCreditPanel.tsx` | `enrTransfers[]` | ENR balance & history |

### GradientPanel Design

```typescript
// dashboard/src/components/GradientPanel.tsx (planned)

interface GradientPanelProps {
  gradients: Map<string, GradientUpdate>;
  nodeEnrStates: Map<string, NodeEnrState>;
}

export function GradientPanel({ gradients, nodeEnrStates }: GradientPanelProps) {
  // Aggregate network gradient
  const networkGradient = useMemo(() => {
    const values = Array.from(gradients.values());
    if (values.length === 0) return null;
    return {
      cpu: values.reduce((acc, g) => acc + g.cpuAvailable, 0) / values.length,
      memory: values.reduce((acc, g) => acc + g.memoryAvailable, 0) / values.length,
      // ...
    };
  }, [gradients]);

  return (
    <Card>
      <CardHeader>Network Resources</CardHeader>
      <CardContent>
        {/* Resource bars, node list, etc. */}
      </CardContent>
    </Card>
  );
}
```

### ElectionPanel Design

```typescript
// dashboard/src/components/ElectionPanel.tsx (planned)

interface ElectionPanelProps {
  elections: Map<number, Election>;
}

export function ElectionPanel({ elections }: ElectionPanelProps) {
  const activeElections = Array.from(elections.values())
    .filter(e => e.status !== 'completed');

  return (
    <Card>
      <CardHeader>Nexus Elections</CardHeader>
      <CardContent>
        {activeElections.map(election => (
          <ElectionCard key={election.id} election={election} />
        ))}
      </CardContent>
    </Card>
  );
}
```

---

## Data Flow Summary

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           ENR DATA FLOW                                      │
└──────────────────────────────────────────────────────────────────────────────┘

1. GRADIENT BROADCAST
   Node A measures resources
         │
         ▼
   GradientBroadcaster.broadcast_update()
         │
         ▼
   gossipsub.publish("/vudo/enr/gradient/1.0.0", cbor_bytes)
         │
         ▼
   Node B,C,D receive via gossipsub subscription
         │
         ▼
   EnrBridge.handle_message() → GradientBroadcaster.handle_gradient()
         │
         ▼
   event_tx.send(WsMessage::GradientUpdate { ... })
         │
         ▼
   Dashboard WebSocket receives JSON
         │
         ▼
   useP2P hook: setState(s => { gradients.set(source, update) })
         │
         ▼
   GradientPanel re-renders with new data


2. CREDIT TRANSFER
   Node A initiates transfer to Node B
         │
         ▼
   CreditSynchronizer.transfer(to, amount)
     • Debit local ledger (amount + entropy_tax)
     • Broadcast CreditTransferMsg
         │
         ▼
   gossipsub.publish("/vudo/enr/credits/1.0.0", cbor_bytes)
         │
         ▼
   All nodes receive and apply to local ledger
         │
         ▼
   Dashboard receives EnrCreditTransfer via WebSocket
         │
         ▼
   useP2P hook updates enrTransfers[] and nodeEnrStates


3. NEXUS ELECTION
   Node triggers election for region
         │
         ▼
   DistributedElection.trigger_election(region_id)
         │
         ▼
   ElectionAnnouncement broadcast
         │
         ▼
   Candidates submit ElectionCandidacy
         │
         ▼
   Voters send ElectionVote
         │
         ▼
   Winner determined → ElectionResult broadcast
         │
         ▼
   Dashboard tracks full lifecycle via elections Map


4. SEPTAL GATE (CIRCUIT BREAKER)
   Node A experiences failures from Node B
         │
         ▼
   SeptalGateManager.record_failure(peer, reason)
     • Increment failure count
     • If threshold exceeded → gate opens
         │
         ▼
   SeptalStateMsg broadcast
         │
         ▼
   Dashboard updates nodeEnrStates with septal status
```

---

## Field Mapping Reference

### Rust → WebSocket → TypeScript

| Rust (messages.rs) | WebSocket JSON | TypeScript |
|--------------------|----------------|------------|
| `source: NodeId` | `source: string` | `source: string` |
| `cpu_available: f64` | `cpu_available: number` | `cpuAvailable: number` |
| `memory_available: f64` | `memory_available: number` | `memoryAvailable: number` |
| `election_id: u64` | `election_id: number` | `electionId: number` |
| `from_state: SeptalGateState` | `from_state: string` | `fromState: SeptalState` |

### Snake_case → CamelCase Handling

The TypeScript handlers normalize field names:

```typescript
case 'gradient_update': {
  const data = (message.data || message) as Record<string, unknown>;
  const gradient: GradientUpdate = {
    source: data.source as string,
    // Handle both snake_case (from Rust) and camelCase
    cpuAvailable: (data.cpu_available ?? data.cpuAvailable) as number,
    memoryAvailable: (data.memory_available ?? data.memoryAvailable) as number,
    // ...
  };
}
```

---

## Testing Integration

### Rust Unit Tests

```bash
# Test ENR bridge in isolation
cargo test -p mycelial-network enr_bridge

# Test WebSocket message serialization
cargo test -p mycelial-node messages
```

### TypeScript Tests

```bash
cd dashboard
pnpm test        # Run all tests
pnpm test:watch  # Watch mode
```

### Integration Tests

```bash
# Start P2P node
cargo run --bin mycelial-node -- --bootstrap --http-port 8080

# In another terminal, start dashboard
cd dashboard && pnpm dev

# Verify WebSocket connection in browser console
# Check for gradient_update, election_*, septal_* messages
```

---

## Related PRs

| PR | Phase | Description |
|----|-------|-------------|
| #8 | Phase 1 | Wire ENR messages to dashboard WebSocket |
| #9 | Phase 2 | Per-peer state tracking with REST API |
| #11 | Phase 3 | Dashboard integration (hooks + types) |
| TBD | Phase 4 | ENR UI components |

---

## Future Work

### Phase 5: Consensus Integration
- Replace local ledger with OpenRaft consensus
- Multi-node credit verification
- Distributed state machine for elections

### Phase 6: Advanced Features
- Gradient-based workload routing
- Credit-backed resource reservations
- Automatic nexus failover
- Septal gate recovery automation
