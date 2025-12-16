# Mycelial P2P Bootstrap - Implementation Roadmap

> **Goal**: Bootstrap a P2P agent network with Web UI for observing nodes, connections, and enabling multi-chat between peers.

---

## Phase Overview

```
Phase 1: Core Foundation          ████████████████████░░  95%
Phase 2: Persistence & Server     ████████████████░░░░░░  80%
Phase 3: Node Integration         ████████████░░░░░░░░░░  60%
Phase 4: Web Dashboard            ████████░░░░░░░░░░░░░░  40%
Phase 5: Polish & Testing         ██░░░░░░░░░░░░░░░░░░░░  10%
Phase 6: Mycelial Economics       ░░░░░░░░░░░░░░░░░░░░░░   0%  [NEXT]
```

---

## Phase 1: Core Foundation (95% Complete)

### 1.1 Core Types - COMPLETE
- [x] Identity module (Keypair, PublicKey, SignatureBytes, Did, Signed<T>)
- [x] Content module (ContentId, Content, MerkleNode, MerkleTreeBuilder)
- [x] Module system (MyceliaModule trait, ModuleState, ModuleRegistry)
- [x] Event system (Event, EventType, EventFilter, all payloads)
- [x] Configuration (NodeConfig, NetworkConfig, StorageConfig)
- [x] Error handling (MycelialError with 30+ variants)
- [x] All tests passing (23 tests + 1 doc test)

### 1.2 Network Layer - COMPLETE
- [x] Swarm configuration (TCP + Noise + Yamux)
- [x] Kademlia DHT for peer discovery
- [x] Gossipsub for pub/sub messaging (optimized for small networks)
- [x] mDNS for local network discovery
- [x] Connection lifecycle handlers
- [x] NetworkService with event broadcasting (655 LOC)
- [x] Mesh peer tracking and debugging helpers
- [x] Integration test: two nodes discover each other

### 1.3 Protocol Layer - PARTIAL
- [x] Basic message types in mycelial-core
- [x] CBOR serialization helpers
- [ ] Announce protocol (peer announcements with signatures)
- [ ] Chat protocol (structured message types)
- [ ] Reputation protocol (queries, responses, updates)
- [ ] Credit protocol (offers, accepts, transfers)

*Note: Generic Message type works for MVP; specialized protocols can be added later*

---

## Phase 2: Persistence & Server (80% Complete)

### 2.1 State Management - COMPLETE
- [x] SQLite schema (peers, messages, credit_relationships tables)
- [x] SqliteStore with sqlx async queries
- [x] LRU cache for hot peer data
- [x] State sync via gossipsub
- [x] CRDT-style conflict resolution with vector clocks
- [x] 13 passing tests

### 2.2 WebSocket Backend - COMPLETE
- [x] Axum router with CORS
- [x] WebSocket endpoint at `/ws`
- [x] Server->Client messages (PeerJoined, PeerLeft, ChatMessage)
- [x] Client->Server messages (SendChat, GetPeers)
- [x] REST endpoints (/api/peers, /api/info, /api/stats, /health)
- [x] Bridge P2P events to WebSocket clients
- [x] Local echo for chat messages (gossipsub fix)

---

## Phase 3: Node Integration (60% Complete)

### 3.1 Main Binary - COMPLETE
- [x] CLI argument parsing (clap)
- [x] Keypair generation/loading
- [x] Initialize NetworkService
- [x] Initialize StateStore
- [x] Start WebSocket server
- [x] Event loop bridging P2P <-> WebSocket
- [x] Graceful shutdown handling
- [x] Bootstrap mode with auto port selection

### 3.2 Bootstrap Testing - PARTIAL
- [x] Start bootstrap node
- [x] Connect second peer
- [x] Verify peer discovery via mDNS and Kademlia
- [x] Send chat message
- [x] Verify message delivery
- [x] WebSocket receives events
- [ ] Multi-peer stress testing (10+ nodes)
- [ ] Network partition recovery testing

**Working Commands:**
```bash
# Terminal 1: Bootstrap
cargo run --release --bin mycelial-node -- --bootstrap --name "Bootstrap" --port 9000 --http-port 8080

# Terminal 2: Peer (auto port selection)
cargo run --release --bin mycelial-node -- --name "Alice" --connect "/ip4/127.0.0.1/tcp/9000"

# Terminal 3: Another Peer
cargo run --release --bin mycelial-node -- --name "Bob" --connect "/ip4/127.0.0.1/tcp/9000"
```

---

## Phase 4: Web Dashboard (40% Complete)

### 4.1 Frontend Setup - COMPLETE
- [x] Vite + React 18 + TypeScript
- [x] TailwindCSS with dark theme
- [x] Type definitions matching Rust structs
- [x] WebSocket connection hook (useP2P)
- [x] REST API fallback for peer list

### 4.2 Components - PARTIAL
- [x] **PeerGraph** - Force-directed network visualization
  - [x] D3 force simulation
  - [x] Local node highlighted
  - [x] Click to select peer
  - [x] Edges show mesh connections
  - [ ] Nodes colored by reputation (red->yellow->green)
  - [ ] Animate on message flow
- [x] **ChatPanel** - P2P messaging interface
  - [x] Message list with timestamps
  - [x] Send messages via WebSocket
  - [x] Auto-scroll to latest
  - [x] Optimistic UI for sent messages
  - [ ] Sender reputation badges
  - [ ] Direct message vs broadcast toggle
- [x] **ReputationCard** - Peer details sidebar
  - [x] Display name + peer ID
  - [x] Basic peer info display
  - [ ] Reputation score + tier visualization
  - [ ] Location display (if shared)
  - [ ] Contribution count
  - [ ] Direct message button
- [ ] **Header** - Network status bar (not implemented)

### 4.3 Real-time Features - PARTIAL
- [x] Live peer updates via WebSocket
- [x] Connection status indicator
- [x] Reconnection with backoff
- [ ] Peer join/leave animations
- [ ] Message delivery indicators
- [ ] Reputation change notifications

### 4.4 Configuration - NEEDS FIX
- [ ] Environment variables for WebSocket URL
- [ ] Production build configuration
- [ ] Docker support

---

## Phase 5: Polish & Testing (10% Complete)

### 5.1 WASM Browser Bridge - DEFERRED
- [ ] wasm-bindgen exports
- [ ] WebRTC signaling
- [ ] Browser-to-browser P2P

*Note: Dashboard works via WebSocket to any node - WASM bridge is optional*

### 5.2 Documentation - PARTIAL
- [x] README with quick start
- [x] CLAUDE.md with project context
- [x] ADR-001 workspace structure
- [ ] Architecture diagram
- [ ] API documentation
- [ ] Deployment guide
- [ ] Contributing guidelines

### 5.3 Testing - PARTIAL
- [x] Unit tests per crate (40 tests passing)
- [ ] Integration tests (multi-node scenarios)
- [ ] E2E tests (Playwright for dashboard)
- [ ] Load testing (100+ peers)

### 5.4 Repository Cleanup - PENDING
- [ ] Remove tracked database files from git
- [ ] Clean up generated artifacts
- [ ] Standardize environment configuration

---

## Phase 6: Mycelial Economics Bootstrap (0% - NEXT PHASE)

> **Goal**: Enable new users to easily join and participate in the regenerative economic network

### 6.1 Onboarding Flow
- [ ] Web-based peer creation (generate keypair in browser or via CLI)
- [ ] QR code for mobile peer connection
- [ ] Invite links with bootstrap node addresses
- [ ] First-time user tutorial/walkthrough

### 6.2 Reputation Seeding
- [ ] Initial reputation score for new peers (trust threshold)
- [ ] Vouching system (existing peers can vouch for new peers)
- [ ] Reputation decay for inactive peers
- [ ] Contribution tracking (relayed messages, uptime)

### 6.3 Mutual Credit Foundation
- [ ] Credit line creation between trusted peers
- [ ] Credit limit based on mutual reputation
- [ ] Simple credit transfer UI
- [ ] Credit relationship visualization in graph

### 6.4 Resource Sharing
- [ ] Bandwidth contribution tracking
- [ ] Storage contribution for content addressing
- [ ] Compute resource sharing (future: agent tasks)

### 6.5 Governance Primitives
- [ ] Proposal creation (text-based)
- [ ] Simple voting mechanism
- [ ] Quorum-based decision making
- [ ] Result broadcast via gossipsub

---

## Quick Reference

### Build Commands
```bash
# Build all crates
cargo build --release

# Run tests
cargo test --workspace

# Start bootstrap node
cargo run --release --bin mycelial-node -- --bootstrap --name "Bootstrap" --port 9000 --http-port 8080

# Start peer node
cargo run --release --bin mycelial-node -- --name "Alice" --connect "/ip4/127.0.0.1/tcp/9000"

# Start dashboard
cd dashboard && pnpm install && pnpm dev
```

### Crate Structure
```
crates/
  mycelial-core/      # Core types, identity, content addressing (23 tests)
  mycelial-network/   # libp2p networking, gossipsub, kademlia (4 tests)
  mycelial-state/     # SQLite persistence, caching (13 tests)
  mycelial-protocol/  # Message serialization (scaffolded)
  mycelial-wasm/      # Browser bridge (deferred)
  mycelial-node/      # Main binary with WebSocket server
dashboard/            # React + Vite + TailwindCSS
```

---

## Success Criteria

### MVP (Current Target)
- [x] 3+ nodes discover each other automatically
- [x] Chat messages route correctly (broadcast)
- [x] Dashboard shows live peer graph
- [ ] Reputation updates propagate through network
- [x] System survives node disconnect/reconnect
- [x] Sub-second message latency on local network

### Economics Bootstrap (Phase 6)
- [ ] New user can join network in < 2 minutes
- [ ] Reputation visible and meaningful
- [ ] Credit relationships can be established
- [ ] Basic governance proposals work

---

*Last Updated: 2024-12-16 - Phase 3/4 functional, Phase 6 planning*
