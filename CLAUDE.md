# Mycelial P2P Bootstrap System

> **See [ROADMAP.md](./ROADMAP.md) for implementation progress tracking**
> **See [docs/SESSION_2025-12-18.md](./docs/SESSION_2025-12-18.md) for latest session notes**

## Project Overview

A **production-ready Peer-to-Peer agent network** implementing Mycelial Economics principles for Univrs.io. This system enables autonomous agents to discover, connect, and coordinate resources using biological network patterns.

**Current Status**: Phase 7 - Orchestrator Integration (~70% overall)
- P2P Network: Fully functional (3+ nodes discover and chat)
- Web Dashboard: P2P features + Orchestrator integration
- Economics UI: Complete (vouch, credit, governance, resources)
- Backend Integration: In progress (WebSocket bridge complete)
- 40+ passing tests across workspace

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    MYCELIAL P2P BOOTSTRAP                           │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 5: React Dashboard                                [WORKING]  │
│    • P2P: PeerGraph, ChatPanel, ReputationCard                      │
│    • Orchestrator: NodeStatus, WorkloadList, ClusterOverview        │
│    • Economics: CreditPanel, GovernancePanel, ResourcePanel         │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 4: Dashboard Hooks                                [WORKING]  │
│    • useP2P → ws://localhost:8080/ws (P2P Node)                     │
│    • useOrchestrator → ws://localhost:9090/api/v1/events            │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 3: HTTP/WebSocket Servers                         [WORKING]  │
│    • P2P Node: Axum server on port 8080 (/ws, /api/*)               │
│    • Orchestrator: API on port 9090 (/api/v1/*)                     │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 2: P2P Network (libp2p)                           [WORKING]  │
│    • gossipsub for pub/sub messaging                                │
│    • Kademlia DHT for peer discovery                                │
│    • mDNS for local network discovery                               │
│    • TCP + Noise + Yamux transport                                  │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 1: Core Types & State (Rust)                      [WORKING]  │
│    • Identity (Ed25519, DID, Signed<T>)                             │
│    • Content addressing (Blake3, Merkle trees)                      │
│    • SQLite persistence with LRU cache                              │
│    • CRDT conflict resolution                                       │
└─────────────────────────────────────────────────────────────────────┘
```

## Technology Stack

| Component | Technology | Status |
|-----------|------------|--------|
| Core Types | Rust, serde, thiserror | Complete |
| P2P Network | libp2p 0.54 (gossipsub, kademlia, mdns) | Complete |
| State Store | SQLite + sqlx + LRU cache | Complete |
| P2P HTTP Server | Axum + tokio (port 8080) | Complete |
| Orchestrator API | REST + WebSocket (port 9090) | Complete |
| Dashboard | React 18 + Vite + TailwindCSS | 90% |
| WASM Bridge | wasm-bindgen | Deferred |

## Cargo Workspace Structure

```
univrs-network/                # Formerly mycelial-dashboard
├── Cargo.toml                 # Workspace root
├── CLAUDE.md                  # AI context (this file)
├── ROADMAP.md                 # Implementation progress
├── crates/
│   ├── mycelial-core/         # Core types (3,000 LOC, 23 tests)
│   │   └── src/
│   │       ├── lib.rs         # Module exports
│   │       ├── identity.rs    # Keypair, PublicKey, Did, Signed<T>
│   │       ├── content.rs     # ContentId, MerkleNode, MerkleTreeBuilder
│   │       ├── module.rs      # MyceliaModule trait, ModuleRegistry
│   │       ├── event.rs       # Event, EventType, EventFilter
│   │       ├── config.rs      # NodeConfig, NetworkConfig, StorageConfig
│   │       ├── message.rs     # Message, MessageType, generic messages
│   │       └── error.rs       # MycelialError (30+ variants)
│   │
│   ├── mycelial-network/      # libp2p networking (1,400 LOC, 4 tests)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── service.rs     # NetworkService (655 LOC)
│   │       └── behaviour.rs   # MyceliaBehaviour composite
│   │
│   ├── mycelial-state/        # Persistence (1,500 LOC, 13 tests)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── store.rs       # SqliteStore with sqlx
│   │       ├── cache.rs       # LRU cache for hot data
│   │       └── sync.rs        # Vector clocks, CRDT resolution
│   │
│   ├── mycelial-protocol/     # Message protocols
│   │   └── src/lib.rs         # Economics messages (Vouch, Credit, etc.)
│   │
│   ├── mycelial-wasm/         # Browser bridge (deferred)
│   │   └── src/lib.rs
│   │
│   └── mycelial-node/         # Main binary (400 LOC)
│       └── src/
│           ├── main.rs        # CLI, event loop, graceful shutdown
│           └── server/
│               ├── mod.rs     # Axum router with CORS
│               ├── websocket.rs # WebSocket handler, economics protocols
│               └── messages.rs  # WsMessage, ClientMessage types
│
├── dashboard/                  # React frontend
│   ├── package.json
│   ├── vite.config.ts
│   ├── .env                   # Environment configuration
│   └── src/
│       ├── App.tsx
│       ├── types.ts           # TypeScript matching Rust structs
│       ├── components/
│       │   ├── PeerGraph.tsx      # D3 force-directed graph
│       │   ├── ChatPanel.tsx      # Message list + send input
│       │   ├── ReputationCard.tsx # Peer details sidebar
│       │   ├── OnboardingPanel.tsx # New user wizard
│       │   ├── CreditPanel.tsx    # Mutual credit management
│       │   ├── GovernancePanel.tsx # Proposals and voting
│       │   ├── ResourcePanel.tsx  # Resource sharing metrics
│       │   ├── NodeStatus.tsx     # Orchestrator node health
│       │   ├── WorkloadList.tsx   # Orchestrator workloads
│       │   └── ClusterOverview.tsx # Orchestrator cluster metrics
│       └── hooks/
│           ├── useP2P.ts          # P2P WebSocket + REST (port 8080)
│           └── useOrchestrator.ts # Orchestrator API (port 9090)
│
├── .claude-flow/              # AI agent coordination
│   ├── agents.yaml            # Agent definitions
│   └── tasks/                 # Task specifications
│
└── docs/
    ├── architecture/
    │   └── ADR-001-workspace-structure.md
    └── SESSION_2025-12-18.md  # Latest session notes
```

## Build & Run Commands

```bash
# Build all crates
cargo build --release

# Run tests
cargo test --workspace

# Start bootstrap node (P2P on port 9000, HTTP on port 8080)
cargo run --release --bin mycelial-node -- \
  --bootstrap --name "Bootstrap" --port 9000 --http-port 8080

# Start peer node (auto port selection)
cargo run --release --bin mycelial-node -- \
  --name "Alice" --connect "/ip4/127.0.0.1/tcp/9000"

# Start another peer
cargo run --release --bin mycelial-node -- \
  --name "Bob" --connect "/ip4/127.0.0.1/tcp/9000"

# Start dashboard (separate terminal)
cd dashboard && pnpm install && pnpm dev
```

## Environment Configuration

The dashboard connects to two separate services:

```bash
# dashboard/.env

# P2P Network (mycelial-node on port 8080)
VITE_P2P_WS_URL=ws://localhost:8080/ws
VITE_P2P_API_URL=http://localhost:8080

# Orchestrator (on port 9090)
VITE_ORCHESTRATOR_WS_URL=ws://localhost:9090/api/v1/events
VITE_ORCHESTRATOR_API_URL=http://localhost:9090

# Disable mock data to use real APIs
VITE_USE_MOCK_DATA=false
```

## Gossipsub Topics

| Topic | Purpose | Status |
|-------|---------|--------|
| `/mycelial/1.0.0/chat` | Broadcast chat messages | Working |
| `/mycelial/1.0.0/direct` | Direct messages | Working |
| `/mycelial/1.0.0/vouch` | Reputation vouching | UI Complete |
| `/mycelial/1.0.0/credit` | Mutual credit transactions | UI Complete |
| `/mycelial/1.0.0/governance` | Proposals and votes | UI Complete |
| `/mycelial/1.0.0/resource` | Resource sharing metrics | UI Complete |

## WebSocket Protocol

### P2P Node (port 8080) - Server -> Client
```typescript
type WsMessage =
  | { type: "peers_list", peers: PeerListEntry[] }
  | { type: "peer_joined", peer_id: string, peer_info: object }
  | { type: "peer_left", peer_id: string }
  | { type: "chat_message", id: string, from: string, from_name: string, content: string, timestamp: number }
  | { type: "stats", peer_count: number, message_count: number, uptime_seconds: number }
  | { type: "vouch_request", id: string, voucher: string, vouchee: string, weight: number, timestamp: number }
  | { type: "vouch_ack", id: string, request_id: string, accepted: boolean, timestamp: number }
  | { type: "credit_line", id: string, creditor: string, debtor: string, limit: number, balance: number, timestamp: number }
  | { type: "credit_transfer", id: string, from: string, to: string, amount: number, memo?: string, timestamp: number }
  | { type: "proposal", id: string, proposer: string, title: string, description: string, ... }
  | { type: "vote_cast", id: string, proposal_id: string, voter: string, vote: string, weight: number, timestamp: number }
  | { type: "resource_contribution", id: string, peer_id: string, resource_type: string, amount: number, unit: string, timestamp: number }
```

### P2P Node (port 8080) - Client -> Server
```typescript
type ClientMessage =
  | { type: "send_chat", content: string, to?: string }
  | { type: "get_peers" }
  | { type: "get_stats" }
  | { type: "subscribe", topic: string }
  | { type: "send_vouch", vouchee: string, weight: number, message?: string }
  | { type: "respond_vouch", request_id: string, accept: boolean }
  | { type: "create_credit_line", debtor: string, limit: number }
  | { type: "transfer_credit", to: string, amount: number, memo?: string }
  | { type: "create_proposal", title: string, description: string, proposal_type: string }
  | { type: "cast_vote", proposal_id: string, vote: string }
  | { type: "report_resource", resource_type: string, amount: number, unit: string }
```

### Orchestrator (port 9090) - REST API
```
GET  /api/v1/nodes                    # List all nodes
GET  /api/v1/workloads                # List all workloads
GET  /api/v1/cluster/status           # Cluster metrics
POST /api/v1/workloads/:id/cancel     # Cancel workload
POST /api/v1/workloads/:id/retry      # Retry workload
```

## Key Implementation Details

### Dual Service Architecture
The dashboard connects to two independent services:
1. **P2P Node** (port 8080): Peer discovery, chat, economics protocols
2. **Orchestrator** (port 9090): Workload scheduling, node health, cluster metrics

### API Format Handling
The orchestrator returns nodes in this format:
```typescript
interface ApiNode {
  id: string;
  address: string;
  status: 'Ready' | 'NotReady';
  resources_capacity: { cpu_cores, memory_mb, disk_mb };
  resources_allocatable: { cpu_cores, memory_mb, disk_mb };
}
```

Usage is calculated as: `(capacity - allocatable) / capacity * 100`

### Local Echo for Chat
Gossipsub doesn't deliver messages back to the sender. The WebSocket handler broadcasts a local echo to all connected clients after successful publish:
```rust
// crates/mycelial-node/src/server/websocket.rs:128
let echo_msg = WsMessage::ChatMessage { ... };
state.event_tx.send(echo_msg)?;
```

### Gossipsub Mesh Configuration
Optimized for small networks (1-10 peers):
```rust
mesh_n: 2,           // Target peers in mesh
mesh_n_low: 1,       // Min before grafting
mesh_n_high: 4,      // Max before pruning
mesh_outbound_min: 0 // Allow zero outbound (small network)
```

## Testing

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p mycelial-core
cargo test -p mycelial-network
cargo test -p mycelial-state

# Run with logging
RUST_LOG=debug cargo test --workspace -- --nocapture

# TypeScript check
cd dashboard && npx tsc --noEmit
```

**Test Summary**: 40+ tests passing
- mycelial-core: 23 tests
- mycelial-network: 4 tests
- mycelial-state: 13 tests

## Next Steps (Phase 7 Remaining)

### Sprint 3: Backend Economics Integration
- [ ] Wire vouch requests through gossipsub to all peers
- [ ] Wire credit transfers through gossipsub
- [ ] Wire governance proposals/votes through gossipsub
- [ ] Wire resource contributions through gossipsub

### Sprint 4: Orchestrator Economics Integration
- [ ] Reputation-weighted workload scheduling
- [ ] Credit-based resource allocation
- [ ] Governance proposals for cluster policies

## Troubleshooting

### Dashboard not connecting to P2P
1. Check P2P node is running with `--http-port 8080`
2. Verify `VITE_P2P_WS_URL=ws://localhost:8080/ws` in `.env`
3. Check browser console for WebSocket errors

### Dashboard not connecting to orchestrator
1. Check orchestrator is running on port 9090
2. Verify `VITE_ORCHESTRATOR_API_URL=http://localhost:9090` in `.env`
3. Set `VITE_USE_MOCK_DATA=true` to use mock data as fallback

### Peers not discovering each other
1. Ensure mDNS is working (same local network)
2. Check Kademlia bootstrap with `--connect` flag
3. Look for "Mesh peer added" in logs

### Messages not appearing
1. Check gossipsub mesh formation in logs
2. Verify topic subscription on both nodes
3. Local echo should show sender's own messages

---

## MANDATORY Pre-PR Checks

**CRITICAL**: Before creating ANY Pull Request, you MUST run these checks and fix all issues:

```bash
# Run the pre-PR check script
./scripts/pre-pr-check.sh

# Or manually:
# 1. Format check (MUST pass)
cargo fmt --all -- --check
# If fails: cargo fmt --all

# 2. Clippy lint (MUST pass with zero warnings)
cargo clippy --all-targets --all-features -- -D warnings

# 3. Build verification
cargo build --all-features

# 4. Unit tests
cargo test --all-features --lib
```

DO NOT create a PR if any of these checks fail. Fix the issues first.

Common fixes:
- `cargo fmt --all` - Auto-fix formatting
- Remove unused imports
- Prefix unused variables with `_`
- Address dead code warnings

## important-instruction-reminders

- Do what has been asked; nothing more, nothing less
- NEVER create files unless absolutely necessary
- ALWAYS prefer editing existing files to creating new ones
- NEVER proactively create documentation files unless explicitly requested
- Never save working files to the root folder - use appropriate subdirectories
- ALWAYS run `/pre-pr` before creating any Pull Request
