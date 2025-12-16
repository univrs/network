# Mycelial P2P Bootstrap System

> **See [ROADMAP.md](./ROADMAP.md) for implementation progress tracking**

## Project Overview

A **production-ready Peer-to-Peer agent network** implementing Mycelial Economics principles for Univrs.io. This system enables autonomous agents to discover, connect, and coordinate resources using biological network patterns.

**Current Status**: MVP functional (~70% complete)
- 3+ nodes can discover each other and exchange messages
- Web dashboard shows live peer graph and chat
- 40 passing tests across workspace

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    MYCELIAL P2P BOOTSTRAP                           │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 4: Web Dashboard (React + WebSocket)              [WORKING]  │
│    • Real-time peer visualization (D3 force graph)                  │
│    • P2P Chat with local echo                                       │
│    • Connection status indicator                                    │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 3: HTTP/WebSocket Server (Axum)                   [WORKING]  │
│    • WebSocket at /ws for real-time events                          │
│    • REST: /api/peers, /api/info, /api/stats, /health               │
│    • Bridge P2P events to browser clients                           │
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
| HTTP Server | Axum + tokio | Complete |
| Dashboard | React 18 + Vite + TailwindCSS | 60% |
| WASM Bridge | wasm-bindgen | Deferred |

## Cargo Workspace Structure

```
mycelial-p2p-bootstrap/
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
│   ├── mycelial-protocol/     # Message protocols (scaffolded)
│   │   └── src/lib.rs         # CBOR serialization helpers
│   │
│   ├── mycelial-wasm/         # Browser bridge (deferred)
│   │   └── src/lib.rs
│   │
│   └── mycelial-node/         # Main binary (400 LOC)
│       └── src/
│           ├── main.rs        # CLI, event loop, graceful shutdown
│           └── server/
│               ├── mod.rs     # Axum router with CORS
│               ├── websocket.rs # WebSocket handler, local echo
│               └── messages.rs  # WsMessage, ClientMessage types
│
├── dashboard/                  # React frontend
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       ├── App.tsx
│       ├── types.ts           # TypeScript matching Rust structs
│       ├── components/
│       │   ├── PeerGraph.tsx  # D3 force-directed graph
│       │   ├── ChatPanel.tsx  # Message list + send input
│       │   └── ReputationCard.tsx # Peer details sidebar
│       └── hooks/
│           └── useP2P.ts      # WebSocket + REST hook
│
└── docs/
    └── architecture/
        └── ADR-001-workspace-structure.md
```

## Build & Run Commands

```bash
# Build all crates
cargo build --release

# Run tests
cargo test --workspace

# Start bootstrap node
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

## Gossipsub Topics

| Topic | Purpose | Status |
|-------|---------|--------|
| `/mycelial/1.0.0/chat` | Broadcast chat messages | Working |
| `/mycelial/1.0.0/direct` | Direct messages (not delivered back to sender) | Working |
| `/mycelial/1.0.0/reputation` | Reputation score updates | Planned |
| `/mycelial/1.0.0/credit` | Mutual credit transactions | Planned |

## WebSocket Protocol

### Server -> Client Messages
```typescript
type WsMessage =
  | { type: "peers_list", peers: PeerListEntry[] }
  | { type: "peer_joined", peer_id: string, peer_info: object }
  | { type: "peer_left", peer_id: string }
  | { type: "chat_message", id: string, from: string, from_name: string, content: string, timestamp: number }
  | { type: "stats", peer_count: number, message_count: number, uptime_seconds: number }
```

### Client -> Server Messages
```typescript
type ClientMessage =
  | { type: "send_chat", content: string, to?: string }
  | { type: "get_peers" }
  | { type: "get_stats" }
  | { type: "subscribe", topic: string }
```

## Key Implementation Details

### Local Echo for Chat
Gossipsub doesn't deliver messages back to the sender. The WebSocket handler broadcasts a local echo to all connected clients after successful publish:

```rust
// crates/mycelial-node/src/server/websocket.rs:128
let echo_msg = WsMessage::ChatMessage {
    id: message_id,
    from: state.local_peer_id.to_string(),
    from_name: state.node_name.clone(),
    to: to.clone(),
    content: content.clone(),
    timestamp,
};
state.event_tx.send(echo_msg)?;
```

### Gossipsub Mesh Configuration
Optimized for small networks (1-10 peers):

```rust
// crates/mycelial-network/src/service.rs
mesh_n: 2,           // Target peers in mesh
mesh_n_low: 1,       // Min before grafting
mesh_n_high: 4,      // Max before pruning
mesh_outbound_min: 0 // Allow zero outbound (small network)
```

### Auto Port Selection
When `--port` is not specified, the node finds an available port:

```rust
// crates/mycelial-node/src/main.rs
let port = args.port.unwrap_or_else(|| {
    (9001..9100).find(|p| TcpListener::bind(("0.0.0.0", *p)).is_ok())
        .unwrap_or(9001)
});
```

## Claude Flow Integration

This project supports AI-assisted development with claude-flow for multi-agent coordination.

### Running with Claude Flow
```bash
# Initialize swarm for development tasks
npx claude-flow@alpha swarm "Fix bug in peer discovery"

# Task orchestration
npx claude-flow@alpha task orchestrate "Implement reputation system"
```

### Agent Types for This Project
| Agent | Use Case |
|-------|----------|
| `coder` | Implement Rust/TypeScript features |
| `tester` | Write and run tests |
| `reviewer` | Code review and quality checks |
| `architect` | Design decisions, ADRs |
| `researcher` | Explore libp2p docs, patterns |

### Memory Namespaces
```
mycelial-p2p/architecture  # Design decisions
mycelial-p2p/bugs          # Known issues
mycelial-p2p/progress      # Implementation status
```

## Next Steps (Phase 6: Economics Bootstrap)

See [ROADMAP.md](./ROADMAP.md#phase-6-mycelial-economics-bootstrap-0---next-phase) for details.

1. **Onboarding Flow** - Web-based peer creation, QR codes, invite links
2. **Reputation Seeding** - Initial scores, vouching system, contribution tracking
3. **Mutual Credit** - Credit line creation, transfer UI, visualization
4. **Resource Sharing** - Bandwidth/storage contribution metrics
5. **Governance** - Proposal creation, voting, quorum decisions

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
```

**Test Summary**: 40 tests passing
- mycelial-core: 23 tests
- mycelial-network: 4 tests
- mycelial-state: 13 tests

## Troubleshooting

### Dashboard not connecting
1. Check WebSocket URL in `dashboard/src/hooks/useP2P.ts`
2. Ensure node is running with `--http-port 8080`
3. Check browser console for CORS errors

### Peers not discovering each other
1. Ensure mDNS is working (same local network)
2. Check Kademlia bootstrap with `--connect` flag
3. Look for "Mesh peer added" in logs

### Messages not appearing
1. Check gossipsub mesh formation in logs
2. Verify topic subscription on both nodes
3. Local echo should show sender's own messages
