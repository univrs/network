# 🍄 Mycelial P2P Bootstrap

A peer-to-peer agent network implementing **Mycelial Economics** principles for [Univrs.io](https://univrs.io). Built with Rust, libp2p, and React.

## Overview

This project creates a decentralized network where autonomous agents:
- **Discover** each other via DHT and mDNS
- **Communicate** through gossipsub pub/sub messaging  
- **Track reputation** based on contributions
- **Establish credit relationships** for mutual resource sharing
- **Visualize** the network through a real-time dashboard

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    MYCELIAL P2P BOOTSTRAP                           │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 4: Web UI Dashboard (React + WebSocket)                      │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 3: WebRTC Bridge (wasm-bindgen)                              │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 2: P2P Network (libp2p)                                      │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 1: Core Types & State (Rust)                                 │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 🚀 Building with Claude-Flow

This project is designed to be built by **8 AI agents** coordinated through claude-flow's hive-mind system.

### Prerequisites

1. **Claude Code** installed globally:
   ```bash
   npm install -g @anthropic-ai/claude-code
   ```

2. **Claude-Flow** (latest alpha):
   ```bash
   npm install -g claude-flow@alpha
   # Or use npx (recommended)
   npx claude-flow@alpha --version
   ```

3. **Rust toolchain**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup target add wasm32-unknown-unknown
   cargo install wasm-pack
   ```

4. **Node.js 18+** and **pnpm**:
   ```bash
   npm install -g pnpm
   ```

### Quick Start (Claude Code)

Open your terminal and navigate to this project:

```bash
cd mycelial-p2p-bootstrap

# Initialize claude-flow in this directory
npx claude-flow@alpha init --force

# Start the hive-mind with all 8 agents
npx claude-flow@alpha hive-mind spawn \
  "Build the Mycelial P2P Bootstrap system following CLAUDE.md specifications" \
  --agents architect,types,network,protocol,state,wasm,backend,frontend \
  --namespace mycelial \
  --claude
```

### Alternative: Step-by-Step Agent Execution

If you prefer to run agents sequentially with more control:

```bash
# 1. Initialize the workspace structure (Architect agent)
npx claude-flow@alpha swarm \
  "Execute architect-agent task from .claude-flow/tasks/architect.md" \
  --claude

# 2. Implement core types (Types agent)
npx claude-flow@alpha swarm \
  "Execute types-agent task from .claude-flow/tasks/types.md" \
  --claude

# 3. Build P2P network layer (Network agent)
npx claude-flow@alpha swarm \
  "Execute network-agent task from .claude-flow/tasks/network.md" \
  --claude

# Continue with remaining agents...
```

### Monitor Progress

```bash
# Check current hive-mind status
npx claude-flow@alpha hive-mind status

# View memory/progress for specific namespace
npx claude-flow@alpha memory query "mycelial" --namespace mycelial

# Resume if interrupted
npx claude-flow@alpha hive-mind resume <session-id>
```

---

## 📁 Project Structure

```
mycelial-p2p-bootstrap/
├── CLAUDE.md                           # Main project instructions
├── README.md                           # This file
├── Cargo.toml                          # Workspace root
├── .claude-flow/
│   ├── agents.yaml                     # Agent definitions
│   ├── hive-config.md                  # Hive-mind coordination
│   └── tasks/
│       ├── architect.md                # Workspace structure task
│       ├── types.md                    # Core types task
│       ├── network.md                  # libp2p task
│       ├── protocol.md                 # Message protocols task
│       ├── state.md                    # State management task
│       ├── wasm.md                     # Browser bridge task
│       ├── backend.md                  # WebSocket server task
│       └── frontend.md                 # React dashboard task
├── crates/
│   ├── mycelial-core/                  # Core types and traits
│   ├── mycelial-network/               # libp2p networking
│   ├── mycelial-protocol/              # Message definitions
│   ├── mycelial-state/                 # Persistence layer
│   ├── mycelial-wasm/                  # Browser WASM bridge
│   └── mycelial-node/                  # Main binary
├── dashboard/                          # React frontend
└── docs/
    └── architecture/
        └── ADR-*.md                    # Architecture decisions
```

---

## 🔧 Manual Build & Run

After agents complete their work:

### Build Rust Components

```bash
# Build all crates
cargo build --release

# Run tests
cargo test --workspace

# Build WASM module
cd crates/mycelial-wasm
wasm-pack build --target web
```

### Run P2P Nodes

```bash
# Terminal 1: Start bootstrap node
cargo run --bin mycelial-node -- --bootstrap --port 9000

# Terminal 2: Start peer node
cargo run --bin mycelial-node -- --connect /ip4/127.0.0.1/udp/9000/quic-v1

# Terminal 3: Start another peer
cargo run --bin mycelial-node -- --connect /ip4/127.0.0.1/udp/9000/quic-v1
```

### Run Dashboard

```bash
cd dashboard
pnpm install
pnpm dev
# Open http://localhost:3000
```

---

## 🌐 Agent Responsibilities

| Agent | Priority | Task |
|-------|----------|------|
| **architect-agent** | P1 | Cargo workspace structure, core traits |
| **types-agent** | P2 | PeerId, PeerInfo, Reputation, Credit types |
| **network-agent** | P3 | libp2p swarm, gossipsub, Kademlia DHT |
| **protocol-agent** | P3 | Message serialization, signatures |
| **state-agent** | P4 | SQLite persistence, state sync |
| **wasm-agent** | P5 | Browser WASM bridge, WebRTC |
| **backend-agent** | P5 | WebSocket server, REST API |
| **frontend-agent** | P6 | React dashboard, visualization |

---

## 🎯 Success Criteria

- [ ] All crates compile (`cargo check --workspace`)
- [ ] All tests pass (`cargo test --workspace`)
- [ ] Two nodes discover each other via DHT
- [ ] Messages route via gossipsub
- [ ] WASM module loads in browser
- [ ] Dashboard shows real-time peer graph
- [ ] Chat messages flow between peers
- [ ] Reputation updates propagate

---

## 🧬 Mycelial Economics Integration

This bootstrap implements the foundation for:

1. **Contribution Scoring**: Track helpful peer interactions
2. **Reputation Propagation**: Gossip reputation through network
3. **Mutual Credit**: CreditRelationship type for peer-to-peer credit
4. **Resource Sharing**: Foundation for bandwidth/storage metrics

See the [Mycelial Economics report](https://univrs.io/mycelial-economics) for full framework details.

---

## 📚 Related Projects

- [claude-flow](https://github.com/ruvnet/claude-flow) - AI orchestration platform
- [libp2p](https://libp2p.io/) - Modular P2P networking stack
- [Holochain](https://holochain.org/) - Agent-centric distributed apps
- [hREA](https://github.com/h-rea/hrea) - Economic coordination on Holochain

---

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

## 🤝 Contributing

This project welcomes contributions! To contribute:

1. Fork the repository
2. Create a feature branch
3. Run the claude-flow agents to implement your changes
4. Submit a pull request

---

*Built with 🍄 by [Univrs.io](https://univrs.io) using [Claude-Flow](https://github.com/ruvnet/claude-flow)*
