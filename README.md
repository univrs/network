# Mycelial P2P Network

A peer-to-peer agent network implementing **Mycelial Economics** principles for [Univrs.io](https://univrs.io). Built with Rust, libp2p, and React.

## Overview

Mycelial creates a decentralized network where autonomous agents:

- **Discover** each other via Kademlia DHT and mDNS
- **Communicate** through gossipsub pub/sub messaging
- **Track reputation** based on contributions
- **Establish credit relationships** for mutual resource sharing
- **Orchestrate workloads** across the network
- **Visualize** everything through a real-time dashboard

## Architecture

```
                           MYCELIAL SYSTEM
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│  ┌──────────────────────────┐    ┌──────────────────────────────┐   │
│  │     React Dashboard      │    │        Orchestrator          │   │
│  │                          │    │        (port 9090)           │   │
│  │  • Live peer graph       │◄──►│  • Workload scheduling       │   │
│  │  • P2P chat              │    │  • Node health monitoring    │   │
│  │  • Reputation tracking   │    │  • Cluster resource mgmt     │   │
│  │  • Workload monitoring   │    │  • Event streaming           │   │
│  └────────────┬─────────────┘    └──────────────────────────────┘   │
│               │                                                      │
│               │ WebSocket + REST API                                 │
│               ▼                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                      P2P Node (port 8080)                     │   │
│  │                                                               │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │   │
│  │  │  Gossipsub  │  │  Kademlia   │  │       mDNS          │   │   │
│  │  │  Pub/Sub    │  │    DHT      │  │  Local Discovery    │   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘   │   │
│  │                                                               │   │
│  │  ┌─────────────────────────────────────────────────────────┐ │   │
│  │  │              TCP + Noise + Yamux Transport              │ │   │
│  │  └─────────────────────────────────────────────────────────┘ │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                      Core Layer (Rust)                        │   │
│  │  • Ed25519 Identity & DID          • Blake3 Content Hashing  │   │
│  │  • SQLite Persistence              • LRU Cache               │   │
│  │  • CRDT Conflict Resolution        • Merkle Tree Proofs      │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

## Features

### P2P Network Layer
- **Peer Discovery**: Automatic peer discovery via Kademlia DHT and local mDNS
- **Secure Messaging**: End-to-end encrypted communication with Noise protocol
- **Pub/Sub Topics**: Chat, reputation, credit transfers, and governance channels
- **Ed25519 Identity**: Cryptographic identity with DID (Decentralized Identifier) support

### Orchestrator Layer
- **Workload Management**: Schedule and monitor distributed tasks across nodes
- **Health Monitoring**: Real-time node status (Ready/NotReady) with resource metrics
- **Resource Tracking**: CPU, memory, and disk allocation monitoring
- **Event Streaming**: WebSocket-based live updates for dashboard integration

### Web Dashboard
- **Network Visualization**: Interactive force-directed graph of P2P connections
- **Real-time Chat**: Broadcast and direct messaging between peers
- **Cluster Overview**: Node status, resource usage, and workload metrics
- **Reputation Display**: Peer contribution scores and vouching relationships

## Quick Start

### Prerequisites

- **Rust 1.75+**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Node.js 18+** and **pnpm**: `npm install -g pnpm`

### Build & Run

```bash
# Clone the repository
git clone https://github.com/univrs-io/mycelial-dashboard.git
cd mycelial-dashboard

# Build all Rust components
cargo build --release

# Run tests
cargo test --workspace
```

### Start P2P Network

```bash
# Terminal 1: Bootstrap node (acts as initial peer)
cargo run --release --bin mycelial-node -- \
  --bootstrap --name "Bootstrap" --port 9000 --http-port 8080

# Terminal 2: Additional peer
cargo run --release --bin mycelial-node -- \
  --name "Alice" --connect "/ip4/127.0.0.1/tcp/9000"

# Terminal 3: Another peer
cargo run --release --bin mycelial-node -- \
  --name "Bob" --connect "/ip4/127.0.0.1/tcp/9000"
```

### Start Dashboard

```bash
cd dashboard
pnpm install
pnpm dev
# Open http://localhost:5173
```

### Optional: Start Orchestrator

```bash
# For workload management and cluster monitoring
cargo run --release --bin mycelial-orchestrator -- --port 9090
```

## Configuration

Environment variables for the dashboard (`.env`):

```bash
# P2P Network (mycelial-node)
VITE_P2P_WS_URL=ws://localhost:8080/ws
VITE_P2P_API_URL=http://localhost:8080

# Orchestrator (optional)
VITE_ORCHESTRATOR_WS_URL=ws://localhost:9090/api/v1/events
VITE_ORCHESTRATOR_API_URL=http://localhost:9090

# Development mode
VITE_USE_MOCK_DATA=false
```

## API Endpoints

### P2P Node (port 8080)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/ws` | WebSocket | Real-time P2P events |
| `/api/peers` | GET | List connected peers |
| `/api/info` | GET | Local node information |
| `/api/stats` | GET | Network statistics |
| `/health` | GET | Health check |

### Orchestrator (port 9090)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/events` | WebSocket | Real-time cluster events |
| `/api/v1/nodes` | GET | List managed nodes |
| `/api/v1/workloads` | GET/POST | Workload management |
| `/api/v1/cluster/status` | GET | Cluster health metrics |

## Mycelial Economics

This system implements the foundation for Mycelial Economics:

1. **Contribution Scoring**: Track helpful peer interactions automatically
2. **Reputation Propagation**: Gossip reputation updates through the network
3. **Mutual Credit**: Peer-to-peer credit relationships without central banking
4. **Resource Sharing**: Fair allocation based on contributions
5. **Democratic Governance**: Proposal and voting system for network policies

See the [Mycelial Economics Whitepaper](https://univrs.io/mycelial-economics) for the full framework.

## Technology Stack

| Component | Technology |
|-----------|------------|
| Core | Rust 2021, serde, thiserror |
| P2P Network | libp2p 0.54 (gossipsub, kademlia, mDNS) |
| Persistence | SQLite + sqlx + LRU cache |
| HTTP Server | Axum + tokio |
| Dashboard | React 18 + Vite + TypeScript + TailwindCSS |
| Visualization | D3.js force-directed graph |

## Project Status

- **P2P Network**: Production-ready (40+ tests passing)
- **Dashboard**: Functional (peer graph, chat, orchestrator integration)
- **Orchestrator**: Beta (workload scheduling, health monitoring)
- **Economics**: In development (reputation, credit, governance)

## Related Projects

- [libp2p](https://libp2p.io/) - Modular P2P networking stack
- [Holochain](https://holochain.org/) - Agent-centric distributed applications
- [hREA](https://github.com/h-rea/hrea) - Economic coordination on Holochain

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

We welcome contributions! Please:

1. Fork the repository
2. Create a feature branch
3. Ensure tests pass (`cargo test --workspace`)
4. Submit a pull request

---

*Built by [Univrs.io](https://univrs.io)*
