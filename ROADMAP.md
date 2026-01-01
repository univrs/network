# Mycelial P2P Bootstrap - Implementation Roadmap

> **Goal**: Bootstrap a P2P agent network with Web UI for observing nodes, connections, and enabling multi-chat between peers.

---

## Phase Overview

```
Phase 1: Core Foundation          ████████████████████░░  95%
Phase 2: Persistence & Server     ████████████████░░░░░░  80%
Phase 3: Node Integration         ████████████░░░░░░░░░░  60%
Phase 4: Web Dashboard            ████████████████████░░  95%
Phase 5: Polish & Testing         ██░░░░░░░░░░░░░░░░░░░░  10%
Phase 6: Mycelial Economics       ████████████████████░░  90%  [IN PROGRESS]
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

### 4.2 Components - MOSTLY COMPLETE
- [x] **PeerGraph** - Force-directed network visualization
  - [x] D3 force simulation
  - [x] Local node highlighted
  - [x] Click to select peer
  - [x] Edges show mesh connections
  - [x] Nodes colored by reputation tier
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
  - [x] Reputation score + tier visualization
  - [x] Location display
  - [x] Contribution count (contributions, interactions, vouches)
  - [x] Direct message button
  - [x] Vouch button with stake modal
- [x] **Header** - Network status bar
  - [x] Connection status indicator
  - [x] Peer count
  - [x] Credit, Govern, Resources buttons
  - [x] Join Network / New Identity button
  - [x] Theme toggle (dark/light mode)
- [x] **OnboardingPanel** - Peer creation wizard
  - [x] Multi-step onboarding flow
  - [x] Keypair generation (Web Crypto API)
  - [x] QR code for peer ID sharing
  - [x] Invite link generation
- [x] **CreditPanel** - Mutual credit management
  - [x] Credit lines tab with peer list
  - [x] Transfer tab with form
  - [x] History tab with transactions
  - [x] Create credit line modal
- [x] **GovernancePanel** - Proposal and voting
  - [x] Active/Passed/All proposal tabs
  - [x] Proposal cards with voting bars
  - [x] Vote For/Against buttons
  - [x] Create proposal modal
- [x] **ResourcePanel** - Resource sharing metrics
  - [x] Overview with pool stats
  - [x] My Resources with bandwidth/storage/compute
  - [x] Network tab with peer contributions
  - [x] Resource distribution visualization

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

## Phase 6: Mycelial Economics Bootstrap (90% - IN PROGRESS)

> **Goal**: Enable new users to easily join and participate in the regenerative economic network

### 6.1 Onboarding Flow - COMPLETE (UI)
- [x] Web-based peer creation (generate keypair in browser via Web Crypto API)
- [x] QR code component for mobile peer connection
- [x] Invite links with bootstrap node addresses
- [x] Multi-step onboarding wizard with welcome, identity, connection, and reputation steps
- [ ] First-time user tutorial/walkthrough (guided tour)

### 6.2 Reputation Seeding - COMPLETE (UI)
- [x] Initial reputation score for new peers (trust threshold with tier system)
- [x] Vouching system with stake slider and message
- [x] Reputation tier visualization (Excellent/Good/Neutral/Poor/Untrusted)
- [x] Contribution tracking display (contributions, interactions, vouches)
- [ ] Reputation decay for inactive peers (backend)
- [ ] Gossipsub integration for vouch propagation (backend)

### 6.3 Mutual Credit Foundation - COMPLETE (UI)
- [x] Credit line creation between trusted peers
- [x] Credit limit slider based on trust level
- [x] Credit transfer UI with recipient, amount, memo
- [x] Credit lines list with utilization visualization
- [x] Transaction history tab
- [ ] Credit relationship visualization in graph (backend integration)
- [ ] Gossipsub integration for credit transactions (backend)

### 6.4 Resource Sharing - COMPLETE (UI)
- [x] Bandwidth contribution tracking (upload/download rates)
- [x] Storage contribution tracking (provided/used/available)
- [x] Compute resource tracking (tasks completed, latency, CPU hours)
- [x] Network resource pool overview
- [x] Top contributors leaderboard
- [x] Resource distribution visualization
- [ ] Actual resource metering integration (backend)

### 6.5 Governance Primitives - COMPLETE (UI)
- [x] Proposal creation with title, description, duration, quorum
- [x] Vote For/Against with weight tracking
- [x] Quorum progress visualization
- [x] Active/Passed/All proposal filtering
- [x] Proposal status badges (Active/Passed/Rejected/Expired)
- [ ] Result broadcast via gossipsub (backend)
- [ ] Weighted voting based on reputation (backend)

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
- [x] New user can join network in < 2 minutes (onboarding wizard)
- [x] Reputation visible and meaningful (tier system with contribution stats)
- [x] Credit relationships can be established (credit panel UI)
- [x] Basic governance proposals work (governance panel UI)
- [x] Resource sharing metrics visible (resource panel UI)
- [ ] Backend integration for all economics features

---

*Last Updated: 2025-12-16 - Phase 6 UI complete, backend integration pending*
