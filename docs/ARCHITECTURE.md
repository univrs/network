# Mycelial Network Architecture Deep Dive

> **Decentralized P2P Network Infrastructure**
> A libp2p-based peer-to-peer network with economic primitives and Byzantine fault tolerance

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [System Overview](#system-overview)
3. [Workspace Structure](#workspace-structure)
4. [Core Data Models](#core-data-models)
5. [Network Protocol Architecture](#network-protocol-architecture)
6. [Identity & Authentication](#identity--authentication)
7. [Message Formats](#message-formats)
8. [Connection Management](#connection-management)
9. [Event Handling & Pub/Sub](#event-handling--pubsub)
10. [State Management](#state-management)
11. [Economic Primitives (ENR)](#economic-primitives-enr)
12. [Distributed Systems Patterns](#distributed-systems-patterns)
13. [Configuration Management](#configuration-management)
14. [Testing Infrastructure](#testing-infrastructure)
15. [Data Flow Patterns](#data-flow-patterns)

---

## Executive Summary

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      MYCELIAL P2P NETWORK                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Gossipsub Pub/Sub   │   Kademlia DHT    │    Economic Primitives         │
│   Message Broadcast   │   Peer Discovery  │    Credits & Reputation        │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│   │   Node A    │  │   Node B    │  │   Node C    │  │   Node D    │      │
│   │             │  │             │  │             │  │             │      │
│   │  ┌───────┐  │  │  ┌───────┐  │  │  ┌───────┐  │  │  ┌───────┐  │      │
│   │  │Ed25519│  │  │  │Ed25519│  │  │  │Ed25519│  │  │  │Ed25519│  │      │
│   │  │Identity│ │  │  │Identity│ │  │  │Identity│ │  │  │Identity│ │      │
│   │  └───────┘  │  │  └───────┘  │  │  └───────┘  │  │  └───────┘  │      │
│   └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘      │
│          │                │                │                │              │
│          └────────────────┴────────────────┴────────────────┘              │
│                               │                                             │
│                    ┌──────────▼──────────┐                                 │
│                    │     libp2p Stack    │                                 │
│                    │  TCP/QUIC + Noise   │                                 │
│                    │       + Yamux       │                                 │
│                    └────────────────────┘                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Key Design Principles

| Principle | Implementation |
|-----------|----------------|
| **Decentralized** | No central authority, peer-to-peer gossip |
| **Cryptographic Identity** | Ed25519 keys with DID support |
| **Economic Fairness** | Mutual credit, reputation-based trust |
| **Byzantine Fault Tolerant** | Septal gates, distributed elections |
| **Eventually Consistent** | Vector clocks, LWW registers |

---

## System Overview

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           MYCELIAL NETWORK SYSTEM                            │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                        APPLICATION LAYER                                │ │
│  │                                                                         │ │
│  │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │ │
│  │   │  Dashboard  │  │   REST API  │  │  WebSocket  │  │    WASM     │  │ │
│  │   │   Server    │  │  Handlers   │  │   Server    │  │   Bindings  │  │ │
│  │   └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  │ │
│  │          │                │                │                │          │ │
│  │          └────────────────┴────────────────┴────────────────┘          │ │
│  │                                   │                                     │ │
│  └───────────────────────────────────┼─────────────────────────────────────┘ │
│                                      │                                       │
│  ┌───────────────────────────────────┼─────────────────────────────────────┐ │
│  │                         PROTOCOL LAYER                                  │ │
│  │                                   │                                      │ │
│  │     ┌─────────────┬───────────────┼───────────────┬─────────────┐       │ │
│  │     │             │               │               │             │       │ │
│  │     ▼             ▼               ▼               ▼             ▼       │ │
│  │ ┌───────┐   ┌──────────┐   ┌───────────┐   ┌──────────┐   ┌─────────┐  │ │
│  │ │Gossip │   │ Kademlia │   │   ENR     │   │ Economics│   │ State   │  │ │
│  │ │ sub   │   │   DHT    │   │  Bridge   │   │ Protocol │   │  Sync   │  │ │
│  │ └───┬───┘   └────┬─────┘   └─────┬─────┘   └────┬─────┘   └────┬────┘  │ │
│  │     │            │               │              │              │        │ │
│  │     └────────────┴───────────────┴──────────────┴──────────────┘        │ │
│  │                                   │                                      │ │
│  └───────────────────────────────────┼─────────────────────────────────────┘ │
│                                      │                                       │
│  ┌───────────────────────────────────┼─────────────────────────────────────┐ │
│  │                         TRANSPORT LAYER                                 │ │
│  │                                   │                                      │ │
│  │           ┌───────────────────────┴───────────────────────┐             │ │
│  │           │                                               │             │ │
│  │           ▼                                               ▼             │ │
│  │    ┌─────────────┐                                 ┌─────────────┐      │ │
│  │    │    TCP      │                                 │    QUIC     │      │ │
│  │    │ + Noise     │                                 │  + Noise    │      │ │
│  │    │ + Yamux     │                                 │             │      │ │
│  │    └─────────────┘                                 └─────────────┘      │ │
│  │                                                                         │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                         STORAGE LAYER                                  │  │
│  │                                                                        │  │
│  │   ┌─────────────────┐         ┌─────────────────┐                     │  │
│  │   │   SQLite +      │         │   LRU Cache     │                     │  │
│  │   │   sqlx async    │◀───────▶│   In-Memory     │                     │  │
│  │   └─────────────────┘         └─────────────────┘                     │  │
│  │                                                                        │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Workspace Structure

### Crate Organization

```
univrs-network/
│
├── CORE LAYER (Foundation)
│   │
│   └── crates/mycelial-core/              ─── Foundation types & traits
│       ├── src/
│       │   ├── identity.rs                ─── Ed25519, DID, PeerId
│       │   ├── peer.rs                    ─── PeerInfo, ConnectionState
│       │   ├── reputation.rs              ─── EMA-based scoring
│       │   ├── credit.rs                  ─── Mutual credit relationships
│       │   ├── message.rs                 ─── Core message types
│       │   ├── event.rs                   ─── Domain events
│       │   ├── content.rs                 ─── Content addressing (Blake3)
│       │   ├── config.rs                  ─── Configuration types
│       │   ├── error.rs                   ─── Error types
│       │   └── module.rs                  ─── MyceliaModule trait
│       └── Cargo.toml
│
├── NETWORKING LAYER
│   │
│   ├── crates/mycelial-network/           ─── P2P networking on libp2p
│   │   ├── src/
│   │   │   ├── service.rs                 ─── NetworkService + NetworkHandle
│   │   │   ├── behaviour.rs               ─── MycelialBehaviour composition
│   │   │   ├── peer_manager.rs            ─── Peer tracking & reputation
│   │   │   ├── enr_bridge.rs              ─── ENR economics integration
│   │   │   ├── partition.rs               ─── Network partition simulator
│   │   │   └── raft/                      ─── OpenRaft scaffolding
│   │   └── tests/                         ─── Integration tests
│   │
│   └── crates/mycelial-protocol/          ─── Message serialization
│       ├── src/
│       │   ├── topics.rs                  ─── Gossipsub topic definitions
│       │   ├── vouch.rs                   ─── VouchMessage
│       │   ├── credit.rs                  ─── CreditMessage
│       │   ├── governance.rs              ─── GovernanceMessage
│       │   └── resource.rs                ─── ResourceMessage
│       └── Cargo.toml
│
├── STATE MANAGEMENT
│   │
│   └── crates/mycelial-state/             ─── Persistence & sync
│       ├── src/
│       │   ├── store.rs                   ─── SqliteStore implementation
│       │   ├── cache.rs                   ─── LRU caching layer
│       │   ├── sync.rs                    ─── State synchronization
│       │   └── vector_clock.rs            ─── Causality tracking
│       └── Cargo.toml
│
├── APPLICATION LAYER
│   │
│   ├── crates/mycelial-node/              ─── Full P2P node binary
│   │   ├── src/
│   │   │   ├── main.rs                    ─── Entry point
│   │   │   ├── dashboard.rs               ─── WebSocket server
│   │   │   ├── rest.rs                    ─── REST API handlers
│   │   │   └── economics.rs               ─── Economics state manager
│   │   └── tests/integration/             ─── Integration tests
│   │
│   └── crates/mycelial-wasm/              ─── Browser WASM bindings
│       └── src/lib.rs                     ─── WASM exports
│
└── External Dependencies
    ├── univrs-identity/                   ─── Unified identity crate
    └── univrs-enr/                        ─── Economic primitives (ENR)
```

### Dependency Graph

```
                         ┌─────────────────────┐
                         │   mycelial-core     │
                         │  (Foundation types) │
                         └──────────┬──────────┘
                                    │
              ┌─────────────────────┼─────────────────────┐
              │                     │                     │
              ▼                     ▼                     ▼
   ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
   │ mycelial-network │  │ mycelial-protocol│  │  mycelial-state  │
   │    (libp2p)      │  │ (serialization)  │  │   (persistence)  │
   └────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘
            │                     │                     │
            └─────────────────────┼─────────────────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │                           │
                    ▼                           ▼
           ┌───────────────┐           ┌───────────────┐
           │ mycelial-node │           │ mycelial-wasm │
           │   (binary)    │           │   (browser)   │
           └───────────────┘           └───────────────┘
                    │
                    ▼
           ┌───────────────────────────────────────┐
           │          External Crates              │
           │                                       │
           │  ┌─────────────┐  ┌─────────────┐    │
           │  │univrs-identity│  │ univrs-enr │    │
           │  └─────────────┘  └─────────────┘    │
           │                                       │
           └───────────────────────────────────────┘
```

---

## Core Data Models

### Identity Model

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          IDENTITY MODEL                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        Ed25519 Keypair                               │   │
│  │                                                                      │   │
│  │   ┌─────────────────┐         ┌─────────────────┐                   │   │
│  │   │  Private Key    │────────▶│  Public Key     │                   │   │
│  │   │   (32 bytes)    │         │   (32 bytes)    │                   │   │
│  │   │                 │         │                 │                   │   │
│  │   │  Sign messages  │         │  Verify sigs    │                   │   │
│  │   │  Never shared   │         │  = PeerId       │                   │   │
│  │   └─────────────────┘         └────────┬────────┘                   │   │
│  │                                        │                             │   │
│  └────────────────────────────────────────┼─────────────────────────────┘   │
│                                           │                                 │
│                                           ▼                                 │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        PeerId Derivation                             │   │
│  │                                                                      │   │
│  │   PublicKey (32 bytes)                                              │   │
│  │        │                                                            │   │
│  │        ▼                                                            │   │
│  │   Base58 Encode ──────▶ "12D3KooW..." (52 chars)                   │   │
│  │        │                                                            │   │
│  │        ▼                                                            │   │
│  │   Short Form ─────────▶ "12D3KooW" (8 chars for logs)              │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     DID (Decentralized Identifier)                   │   │
│  │                                                                      │   │
│  │   Format: did:key:z<multibase-encoded-bytes>                        │   │
│  │                                                                      │   │
│  │   Construction:                                                      │   │
│  │   1. Prepend Ed25519 multicodec: [0xed, 0x01]                       │   │
│  │   2. Append 32-byte public key                                      │   │
│  │   3. Encode with multibase base58btc (prefix 'z')                   │   │
│  │                                                                      │   │
│  │   Example: did:key:z6Mkf5rGMoatrSj1f4CyvuHBeXJELe9RPdzo2PKGNCKVtZxP │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Peer Information Model

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           PEER MODEL                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                          PeerInfo                                    │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │                                                                      │   │
│  │  peer_id: String              Base58 Ed25519 public key             │   │
│  │  addresses: Vec<String>       Multiaddr format                      │   │
│  │                               "/ip4/10.0.0.1/tcp/9000"              │   │
│  │                                                                      │   │
│  │  state: ConnectionState                                             │   │
│  │    ├── Disconnected           Not connected                         │   │
│  │    ├── Connecting             Dial in progress                      │   │
│  │    ├── Connected              Active connection                     │   │
│  │    ├── Failed                 Previous attempt failed               │   │
│  │    └── Banned                 Reputation too low                    │   │
│  │                                                                      │   │
│  │  first_seen: DateTime<Utc>    When peer was discovered              │   │
│  │  last_seen: DateTime<Utc>     Last activity timestamp               │   │
│  │                                                                      │   │
│  │  agent_version: Option<String>    "mycelial/0.1.0"                  │   │
│  │  protocol_version: Option<String> "mycelial/1.0.0"                  │   │
│  │  protocols: Vec<String>       Supported protocols                   │   │
│  │                                                                      │   │
│  │  score: f64                   Reputation score [0.0, 1.0]           │   │
│  │  successful_interactions: u64 Count of successes                    │   │
│  │  failed_interactions: u64     Count of failures                     │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Connection State Machine                         │   │
│  │                                                                      │   │
│  │                        ┌──────────────┐                             │   │
│  │                        │ Disconnected │◀────────────┐               │   │
│  │                        └──────┬───────┘             │               │   │
│  │                               │ dial()              │               │   │
│  │                               ▼                     │               │   │
│  │                        ┌──────────────┐             │               │   │
│  │                 ┌─────▶│  Connecting  │─────────────┤               │   │
│  │                 │      └──────┬───────┘             │               │   │
│  │                 │             │                     │               │   │
│  │           retry │    success  │    failure         │ close         │   │
│  │                 │             ▼                     │               │   │
│  │                 │      ┌──────────────┐             │               │   │
│  │                 │      │  Connected   │─────────────┘               │   │
│  │                 │      └──────────────┘                             │   │
│  │                 │                                                   │   │
│  │          ┌──────┴───────┐         ┌──────────────┐                 │   │
│  │          │    Failed    │         │    Banned    │                 │   │
│  │          └──────────────┘         └──────────────┘                 │   │
│  │                                          ▲                          │   │
│  │                                          │ reputation < threshold   │   │
│  │                                          │                          │   │
│  │                                   (any state)                       │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Reputation Model

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        REPUTATION MODEL                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  EXPONENTIAL MOVING AVERAGE (EMA)                                           │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │   R(T) = α · R(T-1) + β · C(T)                                      │   │
│  │                                                                      │   │
│  │   Where:                                                             │   │
│  │     α = 0.4  (weight of previous score)                             │   │
│  │     β = 0.6  (weight of current contribution)                       │   │
│  │     C(T) = 1.0 for success, 0.0 for failure                        │   │
│  │     Score clamped to [0.0, 1.0]                                     │   │
│  │                                                                      │   │
│  │   Initial score: 0.5 (neutral)                                      │   │
│  │   Trust threshold: 0.4 (below = untrusted)                          │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  SCORE EVOLUTION EXAMPLE                                                    │
│                                                                             │
│  1.0 ┤                                    ╭────────────                    │
│      │                              ╭─────╯                                │
│  0.8 ┤                        ╭─────╯                                      │
│      │                  ╭─────╯                                            │
│  0.6 ┤            ╭─────╯                                                  │
│      │      ╭─────╯                                                        │
│  0.5 ┼──────╯ ←─── Initial                                                │
│      │                                                                      │
│  0.4 ┤ - - - - - - - - - - - - - - - - - - - - Trust threshold            │
│      │                                                                      │
│  0.2 ┤                                                                      │
│      │                                                                      │
│  0.0 ┼────────────────────────────────────────────────────────────────     │
│      0    5    10   15   20   25   30  Interactions                        │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Reputation struct                                │   │
│  │                                                                      │   │
│  │  score: f64                Current reputation [0.0, 1.0]            │   │
│  │  history: Vec<Snapshot>    Historical snapshots (max 100)           │   │
│  │  last_update: DateTime     When last changed                        │   │
│  │  alpha: f64                EMA weight for previous (0.4)            │   │
│  │  beta: f64                 EMA weight for current (0.6)             │   │
│  │  decay_rate: f64           Decay towards neutral (0.01)             │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Network Protocol Architecture

### libp2p Behavior Composition

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      MYCELIAL BEHAVIOUR COMPOSITION                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                        ┌─────────────────────┐                             │
│                        │  MycelialBehaviour  │                             │
│                        └──────────┬──────────┘                             │
│                                   │                                         │
│         ┌─────────────────────────┼─────────────────────────┐              │
│         │                         │                         │              │
│         ▼                         ▼                         ▼              │
│  ┌─────────────┐          ┌─────────────┐          ┌─────────────┐        │
│  │  Gossipsub  │          │  Kademlia   │          │   Identify  │        │
│  │             │          │    DHT      │          │             │        │
│  │ Pub/Sub     │          │ Peer        │          │ Protocol    │        │
│  │ Messaging   │          │ Discovery   │          │ Negotiation │        │
│  └─────────────┘          └─────────────┘          └─────────────┘        │
│         │                         │                         │              │
│         │                         │                         │              │
│         ▼                         ▼                         ▼              │
│  ┌─────────────┐          ┌─────────────┐          ┌─────────────┐        │
│  │   mDNS      │          │    DNS      │          │   Relay     │        │
│  │             │          │             │          │  (future)   │        │
│  │ LAN         │          │ Bootstrap   │          │ NAT         │        │
│  │ Discovery   │          │ Resolution  │          │ Traversal   │        │
│  └─────────────┘          └─────────────┘          └─────────────┘        │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  GOSSIPSUB CONFIGURATION                                                    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  heartbeat_interval: 1 second                                       │   │
│  │  max_message_size: 1 MB                                             │   │
│  │  validation_mode: Strict                                            │   │
│  │  mesh_n: 6         (target peers in mesh)                           │   │
│  │  mesh_n_low: 4     (minimum peers before grafting)                  │   │
│  │  mesh_n_high: 12   (maximum peers before pruning)                   │   │
│  │  gossip_lazy: 6    (peers for gossip propagation)                   │   │
│  │  gossip_factor: 0.25                                                │   │
│  │  message_id_fn: SHA256(data)  (content-based dedup)                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Transport Stack

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          TRANSPORT STACK                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                         APPLICATION DATA                                    │
│                               │                                             │
│                               ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         YAMUX                                        │   │
│  │                   Stream Multiplexing                                │   │
│  │                                                                      │   │
│  │  • Multiple logical streams per connection                          │   │
│  │  • Flow control per stream                                          │   │
│  │  • Ordered, reliable delivery                                       │   │
│  └──────────────────────────────┬──────────────────────────────────────┘   │
│                                 │                                           │
│                                 ▼                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         NOISE                                        │   │
│  │                    Encryption Layer                                  │   │
│  │                                                                      │   │
│  │  • XX handshake pattern                                             │   │
│  │  • Perfect forward secrecy                                          │   │
│  │  • Mutual authentication                                            │   │
│  │  • Ed25519 static keys                                              │   │
│  └──────────────────────────────┬──────────────────────────────────────┘   │
│                                 │                                           │
│                 ┌───────────────┴───────────────┐                          │
│                 │                               │                          │
│                 ▼                               ▼                          │
│  ┌─────────────────────────┐    ┌─────────────────────────┐               │
│  │          TCP            │    │         QUIC            │               │
│  │                         │    │                         │               │
│  │  • Reliable delivery    │    │  • UDP-based            │               │
│  │  • Connection-oriented  │    │  • Built-in encryption  │               │
│  │  • Well-supported       │    │  • Lower latency        │               │
│  │  • Default transport    │    │  • 0-RTT resume         │               │
│  └─────────────────────────┘    └─────────────────────────┘               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Gossipsub Topics

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        GOSSIPSUB TOPICS                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  MYCELIAL CORE TOPICS                                                       │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  /mycelial/1.0.0/chat           Chat messages between peers         │   │
│  │  /mycelial/1.0.0/announce       Peer discovery announcements        │   │
│  │  /mycelial/1.0.0/reputation     Reputation score updates            │   │
│  │  /mycelial/1.0.0/content        Social content (posts, media)       │   │
│  │  /mycelial/1.0.0/orchestration  Workload scheduling events          │   │
│  │  /mycelial/1.0.0/economics      Economic transactions               │   │
│  │  /mycelial/1.0.0/governance     Governance proposals/votes          │   │
│  │  /mycelial/1.0.0/system         System health & metrics             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ENR (ECONOMIC NETWORK RELAY) TOPICS                                        │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  /vudo/enr/gradient/1.0.0       Resource availability gradients     │   │
│  │  /vudo/enr/credits/1.0.0        Credit transfers & settlements      │   │
│  │  /vudo/enr/election/1.0.0       Nexus leader election               │   │
│  │  /vudo/enr/septal/1.0.0         Circuit breaker events              │   │
│  │  /vudo/enr/raft/1.0.0           Raft consensus messages             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  TOPIC SUBSCRIPTION FLOW                                                    │
│                                                                             │
│       Node                                Network                           │
│         │                                     │                             │
│         │──── Subscribe(topic) ──────────────▶│                             │
│         │                                     │                             │
│         │◀─── Mesh Join (GRAFT) ─────────────│                             │
│         │                                     │                             │
│         │◀─── Messages on topic ─────────────│                             │
│         │◀─── Messages on topic ─────────────│                             │
│         │                                     │                             │
│         │──── Publish(topic, data) ──────────▶│                             │
│         │                                     │                             │
│         │                            (gossip to mesh peers)                │
│         │                                     │                             │
│         │──── Unsubscribe(topic) ────────────▶│                             │
│         │                                     │                             │
│         │◀─── Mesh Leave (PRUNE) ────────────│                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Identity & Authentication

### Cryptographic Operations

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     CRYPTOGRAPHIC OPERATIONS                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  MESSAGE SIGNING                                                            │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │     Message                                                          │   │
│  │        │                                                             │   │
│  │        ▼                                                             │   │
│  │  ┌──────────┐     ┌──────────┐     ┌──────────┐                     │   │
│  │  │  Data    │────▶│ Ed25519  │────▶│Signature │                     │   │
│  │  │ (bytes)  │     │  Sign    │     │(64 bytes)│                     │   │
│  │  └──────────┘     └────┬─────┘     └──────────┘                     │   │
│  │                        │                                             │   │
│  │                        │                                             │   │
│  │                   Private Key                                        │   │
│  │                   (32 bytes)                                         │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  SIGNATURE VERIFICATION                                                     │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐     ┌──────────┐         │   │
│  │  │  Data    │  │Signature │  │ Public   │────▶│  Verify  │         │   │
│  │  │ (bytes)  │──│(64 bytes)│──│   Key    │     │          │         │   │
│  │  └──────────┘  └──────────┘  └──────────┘     └────┬─────┘         │   │
│  │                                                     │               │   │
│  │                                              ┌──────┴──────┐        │   │
│  │                                              │             │        │   │
│  │                                              ▼             ▼        │   │
│  │                                           Valid       Invalid      │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  CONTENT ADDRESSING (Blake3)                                                │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │     Content                                                          │   │
│  │        │                                                             │   │
│  │        ▼                                                             │   │
│  │  ┌──────────┐     ┌──────────┐     ┌──────────────────────────┐    │   │
│  │  │  Data    │────▶│  Blake3  │────▶│     Content ID           │    │   │
│  │  │ (bytes)  │     │  Hash    │     │ bafk...xyz (Base32)      │    │   │
│  │  └──────────┘     └──────────┘     └──────────────────────────┘    │   │
│  │                                                                      │   │
│  │  Properties:                                                         │   │
│  │  • Deterministic: same content → same ID                            │   │
│  │  • Collision resistant                                              │   │
│  │  • Fast parallel hashing                                            │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Message Formats

### Core Message Structure

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        MESSAGE STRUCTURE                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                          Message                                     │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │                                                                      │   │
│  │  id: UUID                    Unique message identifier              │   │
│  │                                                                      │   │
│  │  message_type: MessageType                                          │   │
│  │    ├── Discovery             Peer announcements                     │   │
│  │    ├── Content               Posts, media                           │   │
│  │    ├── Reputation            Score updates                          │   │
│  │    ├── Credit                Economic transactions                  │   │
│  │    ├── Governance            Proposals, votes                       │   │
│  │    ├── Direct                Peer-to-peer                           │   │
│  │    └── System                Protocol messages                      │   │
│  │                                                                      │   │
│  │  sender: PeerId              Base58 public key                      │   │
│  │  recipient: Option<PeerId>   None = broadcast                       │   │
│  │  payload: Vec<u8>            CBOR-encoded data                      │   │
│  │  timestamp: DateTime<Utc>    When created                           │   │
│  │  signature: Option<Vec<u8>>  Ed25519 signature (64 bytes)           │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  SERIALIZATION: CBOR (Concise Binary Object Representation)                 │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │  Benefits:                                                           │   │
│  │  • Compact binary format (smaller than JSON)                        │   │
│  │  • Schema-less (like JSON)                                          │   │
│  │  • Supports all Rust types via serde                                │   │
│  │  • Good for bandwidth-constrained P2P                               │   │
│  │                                                                      │   │
│  │  Example sizes:                                                      │   │
│  │  • Simple message: ~150 bytes (vs ~300 bytes JSON)                  │   │
│  │  • With signature: ~220 bytes (vs ~450 bytes JSON)                  │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Economics Protocol Messages

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    ECONOMICS PROTOCOL MESSAGES                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  VOUCH MESSAGE (Reputation Commitments)                                     │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  VouchMessage #[serde(tag = "type")]                                │   │
│  │                                                                      │   │
│  │  VouchRequest {                                                      │   │
│  │    voucher: PeerId,         Who is vouching                         │   │
│  │    vouchee: PeerId,         Who is being vouched for                │   │
│  │    amount: f64,             Reputation stake                        │   │
│  │    expires: DateTime,       Expiration time                         │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  VouchAck { request_id, accepted: bool }                            │   │
│  │  ReputationUpdate { peer, new_score, reason }                       │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  CREDIT MESSAGE (Mutual Credit)                                             │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  CreditMessage #[serde(tag = "type")]                               │   │
│  │                                                                      │   │
│  │  CreateLine {                                                        │   │
│  │    from: PeerId,            Credit issuer                           │   │
│  │    to: PeerId,              Credit receiver                         │   │
│  │    limit: f64,              Maximum credit                          │   │
│  │    interest_rate: f64,      Optional interest                       │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  Transfer {                                                          │   │
│  │    from: PeerId,            Sender                                  │   │
│  │    to: PeerId,              Receiver                                │   │
│  │    amount: f64,             Transfer amount                         │   │
│  │    memo: Option<String>,    Description                             │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  TransferAck { transfer_id, success, new_balance }                  │   │
│  │  LineAck { line_id, accepted: bool }                                │   │
│  │  LineUpdate { line_id, new_limit, reason }                          │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  GOVERNANCE MESSAGE                                                         │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  GovernanceMessage #[serde(tag = "type")]                           │   │
│  │                                                                      │   │
│  │  CreateProposal {                                                    │   │
│  │    proposer: PeerId,                                                │   │
│  │    title: String,                                                   │   │
│  │    description: String,                                             │   │
│  │    voting_ends: DateTime,                                           │   │
│  │    options: Vec<String>,                                            │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  CastVote { proposal_id, voter, option_index }                      │   │
│  │  ProposalUpdate { proposal_id, status }                             │   │
│  │  ProposalExecuted { proposal_id, winning_option, vote_count }       │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### ENR Message Types

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      ENR MESSAGE TYPES                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  GRADIENT UPDATE (Resource Availability)                                    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  GradientUpdate {                                                    │   │
│  │    source: NodeId,          Broadcasting node                       │   │
│  │    gradient: ResourceGradient {                                     │   │
│  │      cpu_available: f64,                                            │   │
│  │      memory_available: u64,                                         │   │
│  │      storage_available: u64,                                        │   │
│  │      bandwidth_available: u64,                                      │   │
│  │    },                                                                │   │
│  │    timestamp: Timestamp,                                            │   │
│  │    signature: Vec<u8>,      Ed25519 signature                       │   │
│  │  }                                                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ELECTION MESSAGE (Nexus Leader Election)                                   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  ElectionMessage                                                     │   │
│  │                                                                      │   │
│  │  Announcement {                                                      │   │
│  │    election_id: UUID,       Unique election ID                      │   │
│  │    initiator: NodeId,       Who started election                    │   │
│  │    region_id: String,       Geographic/logical region               │   │
│  │    timestamp: Timestamp,                                            │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  Candidacy {                                                         │   │
│  │    election_id: UUID,                                               │   │
│  │    candidate: NodeId,       Self-nomination                         │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  Vote {                                                              │   │
│  │    election_id: UUID,                                               │   │
│  │    voter: NodeId,                                                   │   │
│  │    candidate: NodeId,       Voted for                               │   │
│  │    timestamp: Timestamp,                                            │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  Result {                                                            │   │
│  │    election_id: UUID,                                               │   │
│  │    winner: NodeId,                                                  │   │
│  │    region_id: String,                                               │   │
│  │    vote_count: u32,                                                 │   │
│  │    timestamp: Timestamp,                                            │   │
│  │  }                                                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  SEPTAL MESSAGE (Circuit Breaker)                                           │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  SeptalMessage                                                       │   │
│  │                                                                      │   │
│  │  StateChange {                                                       │   │
│  │    node: NodeId,                                                    │   │
│  │    from_state: GateState,   Healthy/Degraded/Unhealthy             │   │
│  │    to_state: GateState,                                             │   │
│  │    reason: String,                                                  │   │
│  │    timestamp: Timestamp,                                            │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  HealthProbe {                                                       │   │
│  │    request_id: UUID,                                                │   │
│  │    target: NodeId,                                                  │   │
│  │    timestamp: Timestamp,                                            │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  HealthResponse {                                                    │   │
│  │    request_id: UUID,                                                │   │
│  │    node: NodeId,                                                    │   │
│  │    is_healthy: bool,                                                │   │
│  │    failure_count: u32,                                              │   │
│  │    timestamp: Timestamp,                                            │   │
│  │  }                                                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Connection Management

### Network Service Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      NETWORK SERVICE ARCHITECTURE                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       NetworkHandle                                  │   │
│  │                   (Client-facing API)                                │   │
│  │                                                                      │   │
│  │  ┌──────────────────────────────────────────────────────────────┐   │   │
│  │  │  Methods:                                                     │   │   │
│  │  │  • dial(peer_id, addr) → Result<()>                          │   │   │
│  │  │  • subscribe(topic) → Result<()>                             │   │   │
│  │  │  • unsubscribe(topic) → Result<()>                           │   │   │
│  │  │  • publish(topic, data) → Result<()>                         │   │   │
│  │  │  • put_record(key, value) → Result<()>                       │   │   │
│  │  │  • get_record(key) → Result<Option<Vec<u8>>>                 │   │   │
│  │  │  • block_peer(peer_id) → Result<()>                          │   │   │
│  │  │  • unblock_peer(peer_id) → Result<()>                        │   │   │
│  │  │  • local_peer_id() → PeerId                                  │   │   │
│  │  │  • connected_peers() → Vec<PeerId>                           │   │   │
│  │  └──────────────────────────────────────────────────────────────┘   │   │
│  │                              │                                       │   │
│  │                              │ mpsc::channel                         │   │
│  │                              ▼                                       │   │
│  └──────────────────────────────┬──────────────────────────────────────┘   │
│                                 │                                           │
│  ┌──────────────────────────────┼──────────────────────────────────────┐   │
│  │                       NetworkService                                 │   │
│  │                    (Background Task)                                 │   │
│  │                              │                                       │   │
│  │  ┌───────────────────────────┴───────────────────────────────────┐  │   │
│  │  │                                                                │  │   │
│  │  │  loop {                                                        │  │   │
│  │  │    tokio::select! {                                           │  │   │
│  │  │                                                                │  │   │
│  │  │      // Handle commands from NetworkHandle                    │  │   │
│  │  │      cmd = command_rx.recv() => { ... }                       │  │   │
│  │  │                                                                │  │   │
│  │  │      // Handle swarm events                                   │  │   │
│  │  │      event = swarm.next() => { ... }                          │  │   │
│  │  │                                                                │  │   │
│  │  │    }                                                           │  │   │
│  │  │  }                                                             │  │   │
│  │  │                                                                │  │   │
│  │  └────────────────────────────────────────────────────────────────┘  │   │
│  │                              │                                       │   │
│  │                              ▼                                       │   │
│  │  ┌────────────────────────────────────────────────────────────────┐ │   │
│  │  │                     libp2p Swarm                                │ │   │
│  │  │                                                                 │ │   │
│  │  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌──────────┐ │ │   │
│  │  │  │  Gossipsub  │ │  Kademlia   │ │  Identify   │ │   mDNS   │ │ │   │
│  │  │  └─────────────┘ └─────────────┘ └─────────────┘ └──────────┘ │ │   │
│  │  │                                                                 │ │   │
│  │  └─────────────────────────────────────────────────────────────────┘ │   │
│  │                                                                      │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  EVENT BROADCASTING                                                         │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                                                                       │  │
│  │  NetworkService ──────▶ broadcast::Sender<NetworkEvent>              │  │
│  │                                    │                                  │  │
│  │         ┌──────────────────────────┼──────────────────────────┐      │  │
│  │         │                          │                          │      │  │
│  │         ▼                          ▼                          ▼      │  │
│  │  ┌─────────────┐          ┌─────────────┐          ┌─────────────┐  │  │
│  │  │  Economics  │          │  Dashboard  │          │    REST     │  │  │
│  │  │   Handler   │          │  WebSocket  │          │   Handler   │  │  │
│  │  └─────────────┘          └─────────────┘          └─────────────┘  │  │
│  │                                                                       │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Peer Manager

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          PEER MANAGER                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        PeerManager                                   │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │                                                                      │   │
│  │  peers: RwLock<HashMap<PeerId, PeerInfo>>                           │   │
│  │  max_peers: u32                         Default: 100                │   │
│  │  trust_threshold: f64                   Default: 0.4                │   │
│  │                                                                      │   │
│  │  Methods:                                                            │   │
│  │  • add_peer(info: PeerInfo)                                         │   │
│  │  • remove_peer(peer_id: &PeerId)                                    │   │
│  │  • update_peer(peer_id, f: FnOnce(&mut PeerInfo))                   │   │
│  │  • get_peer(peer_id) → Option<PeerInfo>                             │   │
│  │  • list_peers() → Vec<PeerInfo>                                     │   │
│  │  • is_trusted(peer_id) → bool                                       │   │
│  │  • record_success(peer_id)                                          │   │
│  │  • record_failure(peer_id)                                          │   │
│  │  • ban_peer(peer_id)                                                │   │
│  │  • unban_peer(peer_id)                                              │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  PEER SCORING                                                               │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │  score = 0.3 × 0.5 + 0.7 × (success_count / total_interactions)    │   │
│  │                 │                              │                     │   │
│  │                 │                              │                     │   │
│  │          Neutral bias                  Interaction ratio            │   │
│  │          (30% weight)                    (70% weight)               │   │
│  │                                                                      │   │
│  │  New peers start with:                                              │   │
│  │  • score = 0.5 (neutral)                                            │   │
│  │  • 0 interactions                                                   │   │
│  │                                                                      │   │
│  │  Trust decision:                                                     │   │
│  │  • score >= 0.4 → trusted                                           │   │
│  │  • score < 0.4 → untrusted (limited capabilities)                   │   │
│  │  • score < 0.2 → consider banning                                   │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  NETWORK STATISTICS                                                         │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  NetworkStats {                                                      │   │
│  │    connected_peers: usize,                                          │   │
│  │    total_peers_known: usize,                                        │   │
│  │    messages_sent: u64,                                              │   │
│  │    messages_received: u64,                                          │   │
│  │    uptime_secs: u64,                                                │   │
│  │    avg_latency_ms: f64,                                             │   │
│  │  }                                                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Event Handling & Pub/Sub

### Two-Layer Event System

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       TWO-LAYER EVENT SYSTEM                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  LAYER 1: NETWORK EVENTS (from libp2p)                                      │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  NetworkEvent                                                        │   │
│  │                                                                      │   │
│  │  PeerConnected { peer_id, addresses }                               │   │
│  │  PeerDisconnected { peer_id }                                       │   │
│  │  MessageReceived { topic, data, source }                            │   │
│  │  DialStarted { peer_id }                                            │   │
│  │  DialFailed { peer_id, error }                                      │   │
│  │  DhtValueFound { key, value }                                       │   │
│  │  DhtValueNotFound { key }                                           │   │
│  │  DhtBootstrapStarted                                                │   │
│  │  DhtBootstrapCompleted                                              │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  LAYER 2: DOMAIN EVENTS (from mycelial-core)                                │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  Event {                                                             │   │
│  │    id: UUID,                                                        │   │
│  │    event_type: EventType,                                           │   │
│  │    source: PeerId,                                                  │   │
│  │    payload: EventPayload,                                           │   │
│  │    timestamp: DateTime<Utc>,                                        │   │
│  │    signature: Option<SignatureBytes>,                               │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  EventType:                                                          │   │
│  │    System        (peer join/leave, topology)                        │   │
│  │    Content       (posts, media)                                     │   │
│  │    Reputation    (score updates)                                    │   │
│  │    Credit        (relationships, transfers)                         │   │
│  │    Governance    (proposals, voting)                                │   │
│  │    Orchestration (scheduling, execution)                            │   │
│  │                                                                      │   │
│  │  EventPayload:                                                       │   │
│  │    System(SystemEvent)                                              │   │
│  │    Content(ContentEvent)                                            │   │
│  │    Reputation(ReputationEvent)                                      │   │
│  │    Credit(CreditEvent)                                              │   │
│  │    Governance(GovernanceEvent)                                      │   │
│  │    Orchestration(OrchestrationEvent)                                │   │
│  │    Raw(Vec<u8>)                                                     │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  EVENT FILTERING                                                            │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  EventFilter {                                                       │   │
│  │    event_types: Option<Vec<EventType>>,                             │   │
│  │    source: Option<PeerId>,                                          │   │
│  │    after: Option<DateTime>,                                         │   │
│  │    before: Option<DateTime>,                                        │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  filter.matches(&event) → bool                                      │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## State Management

### Storage Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       STORAGE ARCHITECTURE                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                         APPLICATION                                         │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        StateCache                                    │   │
│  │                       (LRU Layer)                                    │   │
│  │                                                                      │   │
│  │  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐       │   │
│  │  │   Peer Cache    │ │  Message Cache  │ │  Credit Cache   │       │   │
│  │  │   LRU(1000)     │ │   LRU(5000)     │ │   LRU(500)      │       │   │
│  │  └─────────────────┘ └─────────────────┘ └─────────────────┘       │   │
│  │                                                                      │   │
│  │  Benefits:                                                           │   │
│  │  • Fast reads for hot data                                          │   │
│  │  • Reduced DB queries                                               │   │
│  │  • Automatic eviction                                               │   │
│  │                                                                      │   │
│  └──────────────────────────────┬──────────────────────────────────────┘   │
│                                 │ Cache miss                               │
│                                 ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        SqliteStore                                   │   │
│  │                     (Persistent Layer)                               │   │
│  │                                                                      │   │
│  │  Features:                                                           │   │
│  │  • Async access via sqlx                                            │   │
│  │  • Compile-time query checking                                      │   │
│  │  • Connection pooling                                               │   │
│  │  • WAL mode for concurrency                                         │   │
│  │                                                                      │   │
│  │  Tables:                                                             │   │
│  │  ┌─────────────────────────────────────────────────────────────┐    │   │
│  │  │  peers (peer_id, addresses, state, first_seen, last_seen,  │    │   │
│  │  │         score, success_count, fail_count)                   │    │   │
│  │  │                                                              │    │   │
│  │  │  messages (id, type, sender, recipient, payload, timestamp, │    │   │
│  │  │           signature)                                         │    │   │
│  │  │                                                              │    │   │
│  │  │  credits (line_id, from_peer, to_peer, limit, balance,     │    │   │
│  │  │          created_at, updated_at)                            │    │   │
│  │  │                                                              │    │   │
│  │  │  reputation (peer_id, score, history_json, updated_at)     │    │   │
│  │  └─────────────────────────────────────────────────────────────┘    │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### State Synchronization

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      STATE SYNCHRONIZATION                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  VECTOR CLOCK (Causality Tracking)                                          │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │  VectorClock { entries: HashMap<PeerId, u64> }                      │   │
│  │                                                                      │   │
│  │  Node A: {A: 3, B: 2, C: 1}                                         │   │
│  │  Node B: {A: 2, B: 4, C: 1}                                         │   │
│  │                                                                      │   │
│  │  Operations:                                                         │   │
│  │  • tick(node) → increment own entry                                 │   │
│  │  • merge(other) → take max of each entry                            │   │
│  │  • compare(other) → Before | After | Concurrent                     │   │
│  │                                                                      │   │
│  │  Comparison:                                                         │   │
│  │    A < B: All entries in A ≤ entries in B, at least one <          │   │
│  │    A > B: All entries in A ≥ entries in B, at least one >          │   │
│  │    A || B: Otherwise (concurrent)                                   │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  CONFLICT RESOLUTION STRATEGIES                                             │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │  1. LAST-WRITE-WINS (LWW)                                           │   │
│  │     For peer discovery and simple properties                        │   │
│  │     Winner: Higher timestamp                                        │   │
│  │                                                                      │   │
│  │  2. GROW-ONLY COUNTERS                                              │   │
│  │     For reputation metrics (no rollback)                            │   │
│  │     Merge: Take maximum of each peer's counter                      │   │
│  │                                                                      │   │
│  │  3. APPLICATION MERGE                                                │   │
│  │     For complex data (credit lines)                                 │   │
│  │     Custom logic per data type                                      │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  SYNC FLOW                                                                  │
│                                                                             │
│       Node A                                   Node B                       │
│         │                                         │                         │
│         │──── StateUpdate(data, clock) ──────────▶│                         │
│         │                                         │                         │
│         │                              Compare clocks                       │
│         │                              Apply if newer                       │
│         │                              Merge if concurrent                  │
│         │                                         │                         │
│         │◀─── Ack(merged_clock) ──────────────────│                         │
│         │                                         │                         │
│         │        Update local clock               │                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Economic Primitives (ENR)

### ENR Bridge Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      ENR BRIDGE ARCHITECTURE                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         EnrBridge                                    │   │
│  │                 (Economic Network Relay)                             │   │
│  │                                                                      │   │
│  │  ┌─────────────────────────────────────────────────────────────┐    │   │
│  │  │                                                              │    │   │
│  │  │    ┌─────────────────┐      ┌─────────────────┐            │    │   │
│  │  │    │    Gradient     │      │     Credit      │            │    │   │
│  │  │    │   Broadcaster   │      │  Synchronizer   │            │    │   │
│  │  │    │                 │      │                 │            │    │   │
│  │  │    │ Resource avail- │      │ Mutual credit   │            │    │   │
│  │  │    │ ability signals │      │ ledger ops      │            │    │   │
│  │  │    └─────────────────┘      └─────────────────┘            │    │   │
│  │  │                                                              │    │   │
│  │  │    ┌─────────────────┐      ┌─────────────────┐            │    │   │
│  │  │    │   Distributed   │      │     Septal      │            │    │   │
│  │  │    │    Election     │      │  Gate Manager   │            │    │   │
│  │  │    │                 │      │                 │            │    │   │
│  │  │    │ Nexus leader    │      │ Circuit breaker │            │    │   │
│  │  │    │ consensus       │      │ for unhealthy   │            │    │   │
│  │  │    └─────────────────┘      └─────────────────┘            │    │   │
│  │  │                                                              │    │   │
│  │  └─────────────────────────────────────────────────────────────┘    │   │
│  │                              │                                       │   │
│  │                              ▼                                       │   │
│  │  ┌─────────────────────────────────────────────────────────────┐    │   │
│  │  │                    Message Router                            │    │   │
│  │  │                                                              │    │   │
│  │  │   Incoming:  gossipsub topic → appropriate handler          │    │   │
│  │  │   Outgoing:  handler → gossipsub publish                    │    │   │
│  │  │                                                              │    │   │
│  │  └─────────────────────────────────────────────────────────────┘    │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Mutual Credit System

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       MUTUAL CREDIT SYSTEM                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  CREDIT LINE CONCEPT                                                        │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │     Alice                                   Bob                      │   │
│  │       │                                       │                      │   │
│  │       │◀────────── Credit Line ──────────────▶│                      │   │
│  │       │           limit: 100                  │                      │   │
│  │       │                                       │                      │   │
│  │       │         Alice's view: +50             │                      │   │
│  │       │         Bob's view: -50               │                      │   │
│  │       │                                       │                      │   │
│  │  Alice can spend 50 more to Bob                                     │   │
│  │  Bob can spend 150 to Alice (50 + 100 limit)                        │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  CREDIT RELATIONSHIP                                                        │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  CreditRelationship {                                                │   │
│  │    line_id: UUID,                                                   │   │
│  │    peer_a: PeerId,                                                  │   │
│  │    peer_b: PeerId,                                                  │   │
│  │    limit: f64,              Maximum credit extension                │   │
│  │    balance: f64,            Current balance (A's perspective)       │   │
│  │    interest_rate: f64,      Optional interest                       │   │
│  │    created_at: DateTime,                                            │   │
│  │    last_transfer: DateTime,                                         │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  Invariants:                                                         │   │
│  │  • -limit ≤ balance ≤ +limit                                        │   │
│  │  • Balance from B's perspective = -balance                          │   │
│  │  • Transfers update balance atomically                              │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  TRANSFER FLOW                                                              │
│                                                                             │
│       Sender                                   Receiver                     │
│         │                                         │                         │
│         │──── Transfer(amount) ──────────────────▶│                         │
│         │     (signed, nonce)                     │                         │
│         │                                         │                         │
│         │                              Verify signature                     │
│         │                              Check balance                        │
│         │                              Apply transfer                       │
│         │                                         │                         │
│         │◀─── TransferAck(new_balance) ──────────│                         │
│         │                                         │                         │
│         │        Update local state               │                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Nexus Election

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         NEXUS ELECTION                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ELECTION PHASES                                                            │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │  Phase 1: ANNOUNCEMENT                                              │   │
│  │  ┌──────────────────────────────────────────────────────────────┐   │   │
│  │  │  • Initiator broadcasts election start                       │   │   │
│  │  │  • Includes region_id and election_id                        │   │   │
│  │  │  • Sets voting deadline                                      │   │   │
│  │  └──────────────────────────────────────────────────────────────┘   │   │
│  │                          │                                          │   │
│  │                          ▼                                          │   │
│  │  Phase 2: CANDIDACY                                                 │   │
│  │  ┌──────────────────────────────────────────────────────────────┐   │   │
│  │  │  • Eligible nodes announce candidacy                         │   │   │
│  │  │  • Must meet reputation threshold                            │   │   │
│  │  │  • Self-nomination with credentials                          │   │   │
│  │  └──────────────────────────────────────────────────────────────┘   │   │
│  │                          │                                          │   │
│  │                          ▼                                          │   │
│  │  Phase 3: VOTING                                                    │   │
│  │  ┌──────────────────────────────────────────────────────────────┐   │   │
│  │  │  • Each node casts one vote                                  │   │   │
│  │  │  • Votes are signed and timestamped                          │   │   │
│  │  │  • Votes collected until deadline                            │   │   │
│  │  └──────────────────────────────────────────────────────────────┘   │   │
│  │                          │                                          │   │
│  │                          ▼                                          │   │
│  │  Phase 4: RESULT                                                    │   │
│  │  ┌──────────────────────────────────────────────────────────────┐   │   │
│  │  │  • Tally votes (simple majority)                             │   │   │
│  │  │  • Broadcast winner                                          │   │   │
│  │  │  • Winner becomes region nexus                               │   │   │
│  │  └──────────────────────────────────────────────────────────────┘   │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  BYZANTINE TOLERANCE                                                        │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  • Votes must be signed by known peers                              │   │
│  │  • Timestamp prevents replay attacks                                │   │
│  │  • Double-voting detected and penalized                             │   │
│  │  • Quorum: > 50% of eligible voters                                 │   │
│  │  • Tie-breaker: highest reputation score                            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Septal Gate (Circuit Breaker)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        SEPTAL GATE (CIRCUIT BREAKER)                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  GATE STATE MACHINE                                                         │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │              ┌─────────────────────┐                                │   │
│  │              │      HEALTHY        │                                │   │
│  │              │                     │                                │   │
│  │              │  All requests pass  │                                │   │
│  │              │  Normal operation   │                                │   │
│  │              └──────────┬──────────┘                                │   │
│  │                         │                                            │   │
│  │              failure_count > threshold                              │   │
│  │                         │                                            │   │
│  │                         ▼                                            │   │
│  │              ┌─────────────────────┐                                │   │
│  │              │      DEGRADED       │                                │   │
│  │              │                     │                                │   │
│  │              │  Limited requests   │                                │   │
│  │              │  Health probes sent │                                │   │
│  │              └──────────┬──────────┘                                │   │
│  │                         │                                            │   │
│  │         ┌───────────────┴───────────────┐                           │   │
│  │         │                               │                           │   │
│  │   more failures                 recovery                            │   │
│  │         │                               │                           │   │
│  │         ▼                               ▼                           │   │
│  │  ┌─────────────────────┐     ┌─────────────────────┐               │   │
│  │  │     UNHEALTHY       │     │      HEALTHY        │               │   │
│  │  │                     │     │                     │               │   │
│  │  │  All requests       │     │  Back to normal     │               │   │
│  │  │  blocked            │     │                     │               │   │
│  │  │  Periodic probes    │────▶│                     │               │   │
│  │  └─────────────────────┘     └─────────────────────┘               │   │
│  │         (on recovery)                                               │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  CONFIGURATION                                                              │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  failure_threshold: 5       Failures before DEGRADED                │   │
│  │  unhealthy_threshold: 10    Failures before UNHEALTHY               │   │
│  │  probe_interval: 30s        Time between health probes              │   │
│  │  recovery_probes: 3         Successful probes to recover            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Distributed Systems Patterns

### Partition Tolerance

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       PARTITION TOLERANCE                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PARTITION SIMULATOR                                                        │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │  Normal Operation:                                                   │   │
│  │  ┌─────┐     ┌─────┐     ┌─────┐     ┌─────┐                       │   │
│  │  │  A  │◀───▶│  B  │◀───▶│  C  │◀───▶│  D  │                       │   │
│  │  └─────┘     └─────┘     └─────┘     └─────┘                       │   │
│  │                                                                      │   │
│  │  Partition Created:                                                  │   │
│  │  ┌─────┐     ┌─────┐  ║  ┌─────┐     ┌─────┐                       │   │
│  │  │  A  │◀───▶│  B  │  ║  │  C  │◀───▶│  D  │                       │   │
│  │  └─────┘     └─────┘  ║  └─────┘     └─────┘                       │   │
│  │       Group 1         ║       Group 2                               │   │
│  │                                                                      │   │
│  │  Partition Healed:                                                   │   │
│  │  ┌─────┐     ┌─────┐     ┌─────┐     ┌─────┐                       │   │
│  │  │  A  │◀───▶│  B  │◀───▶│  C  │◀───▶│  D  │                       │   │
│  │  └─────┘     └─────┘     └─────┘     └─────┘                       │   │
│  │                    State reconciliation                             │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  IMPLEMENTATION                                                             │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │  PartitionSimulator {                                                │   │
│  │    blocked_peers: HashSet<PeerId>,                                  │   │
│  │    partition_groups: Vec<HashSet<PeerId>>,                          │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  Methods:                                                            │   │
│  │  • block_peer(peer_id)                                              │   │
│  │  • unblock_peer(peer_id)                                            │   │
│  │  • create_partition(group_a, group_b)                               │   │
│  │  • heal_partition()                                                 │   │
│  │  • is_blocked(from, to) → bool                                      │   │
│  │                                                                      │   │
│  │  Used for:                                                           │   │
│  │  • Testing partition healing                                        │   │
│  │  • Validating state reconciliation                                  │   │
│  │  • Stress testing consensus                                         │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Configuration Management

### Node Configuration

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       NODE CONFIGURATION                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  NodeConfig {                                                        │   │
│  │                                                                      │   │
│  │    identity: IdentityConfig {                                       │   │
│  │      keypair_path: Option<PathBuf>,  // Load from file             │   │
│  │      node_name: Option<String>,      // Human-readable name        │   │
│  │    }                                                                 │   │
│  │                                                                      │   │
│  │    network: NetworkConfig {                                         │   │
│  │      listen_addresses: Vec<String>,  // ["/ip4/0.0.0.0/tcp/9000"] │   │
│  │      bootstrap_peers: Vec<String>,   // Initial peers to connect  │   │
│  │      enable_mdns: bool,              // Default: true              │   │
│  │      enable_kademlia: bool,          // Default: true              │   │
│  │      max_connections: u32,           // Default: 100               │   │
│  │      idle_timeout_secs: u64,         // Default: 30                │   │
│  │      enable_tcp: bool,               // Default: true              │   │
│  │      enable_quic: bool,              // Default: true              │   │
│  │                                                                      │   │
│  │      gossipsub: GossipsubConfig {                                   │   │
│  │        heartbeat_interval: Duration, // 1 second                   │   │
│  │        max_message_size: usize,      // 1 MB                       │   │
│  │        validation_mode: ValidationMode,                             │   │
│  │        mesh_n: 6,                                                   │   │
│  │        mesh_n_low: 4,                                               │   │
│  │        mesh_n_high: 12,                                             │   │
│  │      }                                                               │   │
│  │    }                                                                 │   │
│  │                                                                      │   │
│  │    storage: StorageConfig {                                         │   │
│  │      data_dir: PathBuf,              // ~/.mycelial/data           │   │
│  │      backend: StorageBackend,        // Sqlite/Memory/RocksDb      │   │
│  │      cache_size_mb: u32,             // Default: 64 MB             │   │
│  │      enable_cas: bool,               // Content-addressed storage  │   │
│  │      max_storage_gb: u64,            // 0 = unlimited              │   │
│  │    }                                                                 │   │
│  │                                                                      │   │
│  │    modules: ModulesConfig {                                         │   │
│  │      enable_economics: bool,                                        │   │
│  │      enable_governance: bool,                                       │   │
│  │      enable_orchestration: bool,                                    │   │
│  │    }                                                                 │   │
│  │                                                                      │   │
│  │    logging: LoggingConfig {                                         │   │
│  │      level: Level,                   // info, debug, trace         │   │
│  │      format: LogFormat,              // pretty, json, compact      │   │
│  │      file: Option<PathBuf>,          // Optional file output       │   │
│  │    }                                                                 │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  REPUTATION & CREDIT CONFIG                                                 │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  ReputationConfig {                                                  │   │
│  │    initial_score: 0.5,          Neutral starting point              │   │
│  │    trust_threshold: 0.4,        Below = untrusted                   │   │
│  │    alpha: 0.4,                  EMA weight for previous             │   │
│  │    beta: 0.6,                   EMA weight for current              │   │
│  │    decay_rate: 0.01,            Decay towards neutral               │   │
│  │    max_history: 100,            Historical snapshots                │   │
│  │  }                                                                   │   │
│  │                                                                      │   │
│  │  CreditConfig {                                                      │   │
│  │    default_credit_limit: 100.0,                                     │   │
│  │    max_credit_limit: 10000.0,                                       │   │
│  │    interest_rate: 0.0,                                              │   │
│  │    settlement_grace_period: 30 days,                                │   │
│  │  }                                                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Testing Infrastructure

### Test Suite Structure

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       TESTING INFRASTRUCTURE                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  TEST CATEGORIES                                                            │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │  UNIT TESTS (in-module)                                             │   │
│  │  ├── Reputation calculations                                        │   │
│  │  ├── Credit transfer logic                                          │   │
│  │  ├── Vector clock operations                                        │   │
│  │  ├── Configuration serialization                                    │   │
│  │  └── Message parsing                                                │   │
│  │                                                                      │   │
│  │  INTEGRATION TESTS (crates/mycelial-network/tests/)                 │   │
│  │  ├── partition_test.rs      Partition healing                       │   │
│  │  ├── gate_election.rs       Nexus election                          │   │
│  │  ├── gate_credits.rs        Credit transfer under Byzantine         │   │
│  │  ├── stress_election.rs     Election under load                     │   │
│  │  ├── stress_credits.rs      Transfer throughput                     │   │
│  │  ├── stress_partition.rs    Partition merge complexity              │   │
│  │  └── stress_septal.rs       Septal gate state changes               │   │
│  │                                                                      │   │
│  │  API TESTS (crates/mycelial-node/tests/integration/)                │   │
│  │  ├── rest_api.rs            HTTP endpoint testing                   │   │
│  │  ├── rest_handlers.rs       Handler logic                           │   │
│  │  ├── websocket_messages.rs  WS message format                       │   │
│  │  ├── websocket_handlers.rs  Handler state                           │   │
│  │  └── dashboard_compatibility.rs  UI integration                     │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  TEST HELPERS                                                               │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  tests/helpers/                                                      │   │
│  │  ├── cluster.rs         Multi-node test harness                     │   │
│  │  ├── node_harness.rs    Single node setup                           │   │
│  │  ├── mod.rs             Common utilities                            │   │
│  │  └── stress/            Load testing utilities                      │   │
│  │                                                                      │   │
│  │  NetworkConfig::local_test(port)                                    │   │
│  │  ├── Single TCP address                                             │   │
│  │  ├── No QUIC                                                        │   │
│  │  ├── Reduced mesh parameters                                        │   │
│  │  └── Suitable for 2-3 node tests                                    │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  STRESS TEST METRICS                                                        │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  Phase 3 Stress Testing Results:                                    │   │
│  │  • 26/26 tests passing                                              │   │
│  │  • Election stress: 50-node clusters                                │   │
│  │  • Credit stress: 1000 transfers/second                             │   │
│  │  • Partition stress: 10-way partition merge                         │   │
│  │  • Septal stress: 100 state transitions/second                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Data Flow Patterns

### P2P Message Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        P2P MESSAGE FLOW                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  OUTBOUND (Publishing)                                                      │
│                                                                             │
│  ┌────────────────────────────────────────────────────────────────────┐    │
│  │                                                                     │    │
│  │    Application                                                      │    │
│  │        │                                                            │    │
│  │        ▼                                                            │    │
│  │    NetworkHandle.publish(topic, data)                              │    │
│  │        │                                                            │    │
│  │        ▼ mpsc command                                               │    │
│  │    NetworkService                                                   │    │
│  │        │                                                            │    │
│  │        ▼                                                            │    │
│  │    Swarm.behaviour_mut().gossipsub.publish()                       │    │
│  │        │                                                            │    │
│  │        ▼                                                            │    │
│  │    libp2p transport                                                 │    │
│  │        │                                                            │    │
│  │        ▼                                                            │    │
│  │    ════════════════════════════════════════                        │    │
│  │                    NETWORK                                          │    │
│  │    ════════════════════════════════════════                        │    │
│  │                                                                     │    │
│  └────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  INBOUND (Receiving)                                                        │
│                                                                             │
│  ┌────────────────────────────────────────────────────────────────────┐    │
│  │                                                                     │    │
│  │    ════════════════════════════════════════                        │    │
│  │                    NETWORK                                          │    │
│  │    ════════════════════════════════════════                        │    │
│  │        │                                                            │    │
│  │        ▼                                                            │    │
│  │    libp2p transport                                                 │    │
│  │        │                                                            │    │
│  │        ▼                                                            │    │
│  │    Swarm event loop                                                 │    │
│  │        │                                                            │    │
│  │        ▼                                                            │    │
│  │    MycelialBehaviourEvent::Gossipsub(message)                      │    │
│  │        │                                                            │    │
│  │        ▼                                                            │    │
│  │    NetworkService event processing                                  │    │
│  │        │                                                            │    │
│  │        ▼ broadcast channel                                          │    │
│  │    NetworkEvent::MessageReceived                                    │    │
│  │        │                                                            │    │
│  │        ├───────────────┬───────────────┐                           │    │
│  │        ▼               ▼               ▼                           │    │
│  │    Economics       Dashboard       REST API                         │    │
│  │    Handler         WebSocket       Handler                          │    │
│  │                                                                     │    │
│  └────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Credit Transfer Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      CREDIT TRANSFER FLOW                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│       Sender (Alice)                           Receiver (Bob)               │
│            │                                        │                       │
│            │  1. Check local balance                │                       │
│            │     balance + amount ≤ limit           │                       │
│            │                                        │                       │
│            │  2. Create CreditTransferMsg           │                       │
│            │     { from, to, amount, nonce }        │                       │
│            │                                        │                       │
│            │  3. Sign with private key              │                       │
│            │                                        │                       │
│            │  4. Serialize (CBOR)                   │                       │
│            │                                        │                       │
│            │──── Publish to /vudo/enr/credits ─────▶│                       │
│            │                                        │                       │
│            │                           5. Receive message                   │
│            │                                        │                       │
│            │                           6. Verify signature                  │
│            │                              - Known peer?                     │
│            │                              - Valid signature?                │
│            │                                        │                       │
│            │                           7. Check nonce                       │
│            │                              - Replay prevention               │
│            │                                        │                       │
│            │                           8. Apply to local ledger             │
│            │                              - Update balance                  │
│            │                                        │                       │
│            │◀─── TransferAck(success, balance) ────│                       │
│            │                                        │                       │
│            │  9. Update local state                 │                       │
│            │                                        │                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Summary

The Mycelial Network implements a sophisticated decentralized P2P system with:

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Transport** | libp2p (TCP/QUIC + Noise + Yamux) | Secure, multiplexed connections |
| **Discovery** | Kademlia DHT + mDNS | Peer discovery across networks |
| **Messaging** | Gossipsub | Scalable pub/sub broadcasting |
| **Identity** | Ed25519 + DID | Cryptographic peer authentication |
| **Storage** | SQLite + LRU cache | Persistent state with fast reads |
| **Consistency** | Vector clocks + LWW | Eventual consistency with causality |
| **Economics** | Mutual credit + reputation | Trust-based economic system |
| **Resilience** | Septal gates + elections | Byzantine fault tolerance |

### Key Design Strengths

- **Decentralized**: No central authority or single point of failure
- **Cryptographically Secure**: Ed25519 signatures on all critical operations
- **Eventually Consistent**: Handles network partitions gracefully
- **Economically Fair**: Reputation-weighted trust and mutual credit
- **Extensible**: Modular architecture with pluggable components
- **Well Tested**: Comprehensive stress testing (26/26 tests passing)

---

*Generated: 2026-01-26*
*Version: 0.1.0*
