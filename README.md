# Mycelial P2P Network

> [!CAUTION]
> This project is a research demonstrator. It is in early development and may change significantly. Using permissive Univrs tools in your repository requires careful attention to security considerations and careful human supervision, and even then things can still go wrong. Use it with caution, and at your own risk. See [Disclaimer](#disclaimer).

A peer-to-peer agent network implementing **Mycelial Economics** principles for [Univrs.io](https://univrs.io). Built with Rust, libp2p, and React.

**Latest: v0.8.0** - Meshtastic LoRa mesh bridge integration

## What's New in v0.8.0

### Meshtastic LoRa Bridge
- Complete bridge between Meshtastic LoRa mesh and libp2p gossipsub (~8,000 LOC)
- All economics protocols (vouch, credit, governance, resource) work over radio
- 116 tests covering translation, mapping, compression, and deduplication
- Serial, TCP, and BLE device interfaces

### Since v0.7.0
- **ENR Bridge UI**: Gradients, elections, septal gates, and ENR credit panels
- **Stress Testing**: 26+ stress tests for network reliability
- **Full Cluster Testing**: Automated orchestrator + P2P test scripts
- **Architecture Documentation**: Comprehensive ADR documents

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
│  │  • ENR Bridge panels     │    │  • ENR credit system         │   │
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
│               ▲                                                      │
│               │ Meshtastic Bridge                                    │
│               ▼                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                   Meshtastic LoRa Bridge                      │   │
│  │                                                               │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │   │
│  │  │ Translator  │  │ TopicMapper │  │  DeduplicationCache │   │   │
│  │  │ proto↔CBOR  │  │ topic↔chan  │  │   LRU + TTL         │   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘   │   │
│  │                                                               │   │
│  │  Serial/TCP/BLE Interface → LoRa Radio (2-10km range)        │   │
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

### Meshtastic LoRa Bridge (NEW in v1.0.0)
- **Long-Range Mesh**: Bridge to Meshtastic LoRa devices (2-10km range)
- **Bidirectional Forwarding**: gossipsub ↔ LoRa message translation
- **Economics Over Radio**: Vouch, credit, governance, and resource protocols over LoRa
- **Compression & Chunking**: Automatic compression and message splitting for 237-byte LoRa payloads
- **Deduplication**: LRU + TTL cache prevents message loops between networks
- **Multiple Interfaces**: Serial, TCP, and BLE device connectivity

### Orchestrator Layer
- **Workload Management**: Schedule and monitor distributed tasks across nodes
- **Health Monitoring**: Real-time node status (Ready/NotReady) with resource metrics
- **Resource Tracking**: CPU, memory, and disk allocation monitoring
- **Event Streaming**: WebSocket-based live updates for dashboard integration
- **ENR Credit System**: Energy-based resource credits for workload allocation

### Web Dashboard
- **Network Visualization**: Interactive force-directed graph of P2P connections
- **Real-time Chat**: Broadcast and direct messaging between peers
- **Cluster Overview**: Node status, resource usage, and workload metrics
- **Reputation Display**: Peer contribution scores and vouching relationships
- **ENR Bridge Panels**: Gradients, elections, septal gates, and ENR credits

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

### Optional: Enable Meshtastic Bridge

#### Hardware Requirements

- **Meshtastic Device**: Any supported device (T-Beam, T-Echo, Heltec, RAK, etc.)
- **LoRa Frequency**: Must match your region (e.g., 915MHz US, 868MHz EU, 433MHz Asia)
- **Firmware**: Meshtastic firmware 2.0+ recommended
- **Connection**: USB serial, TCP network, or Bluetooth LE

#### System Dependencies (Linux)

For serial port access:
```bash
# Ubuntu/Debian
sudo apt install libudev-dev pkg-config

# Add user to dialout group for serial port access
sudo usermod -a -G dialout $USER
# Log out and back in for group changes to take effect
```

#### Build with Meshtastic Support

```bash
# Serial interface (USB connection)
cargo build --release --features meshtastic-serial

# TCP interface (network-connected devices)
cargo build --release --features meshtastic-tcp

# BLE interface (Bluetooth Low Energy)
cargo build --release --features meshtastic-ble

# All interfaces
cargo build --release --features meshtastic-full
```

#### Run with LoRa Bridge

**Serial (USB):**
```bash
cargo run --release --bin mycelial-node --features meshtastic-serial -- \
  --bootstrap --name "LoRa Bridge" --port 9000 --http-port 8080 \
  --meshtastic /dev/ttyUSB0
```

**TCP (Network-connected device):**
```bash
cargo run --release --bin mycelial-node --features meshtastic-tcp -- \
  --bootstrap --name "LoRa Bridge" --port 9000 --http-port 8080 \
  --meshtastic tcp://192.168.1.100:4403
```

**BLE (Bluetooth):**
```bash
cargo run --release --bin mycelial-node --features meshtastic-ble -- \
  --bootstrap --name "LoRa Bridge" --port 9000 --http-port 8080 \
  --meshtastic ble://MyMeshtastic
```

#### Meshtastic Device Configuration

Configure your Meshtastic device for optimal bridge performance:

```bash
# Using Meshtastic Python CLI
pip install meshtastic

# Set device name
meshtastic --set lora.region US

# Enable serial output
meshtastic --set serial.enabled true
meshtastic --set serial.echo true

# Increase hop limit for mesh
meshtastic --set lora.hop_limit 7

# Optional: Increase transmit power (check local regulations)
meshtastic --set lora.tx_power 20
```

Or via the Meshtastic mobile app: Settings → Radio Configuration → LoRa → Set region and hop limit.

#### What Gets Bridged

The bridge automatically translates and forwards:

- **Chat Messages**: `/chat` gossipsub ↔ Meshtastic TEXT_MESSAGE_APP
- **Reputation/Vouches**: `/reputation` ↔ PRIVATE_APP (channel 1)
- **Credit Transfers**: `/credit` ↔ PRIVATE_APP (channel 2)
- **Governance Proposals**: `/governance` ↔ PRIVATE_APP (channel 3)
- **Resource Sharing**: `/resources` ↔ PRIVATE_APP (channel 4)

Messages are automatically:
- **Compressed** using miniz_oxide (typically 60-80% size reduction)
- **Chunked** if needed (>237 bytes split across multiple LoRa packets)
- **Deduplicated** via LRU cache to prevent loops
- **Mapped** between libp2p PeerIds and Meshtastic node IDs

#### Range & Performance

- **Urban**: 500m - 2km (buildings, obstacles)
- **Suburban**: 2km - 5km (moderate line-of-sight)
- **Rural/Open**: 5km - 10km+ (clear line-of-sight)
- **Mountain/Hilltop**: 20km+ possible with elevated nodes

**Latency**: 1-5 seconds typical (LoRa airtime + mesh hops)  
**Throughput**: ~1-3 messages/second (depends on spreading factor and congestion)

#### Troubleshooting

**Device not detected:**
```bash
# List serial ports
ls -la /dev/tty* | grep USB

# Check permissions
groups  # Should include 'dialout'

# Test with Meshtastic CLI
meshtastic --info
```

**No messages forwarding:**
- Verify device is on the same Meshtastic channel/region
- Check bridge logs for translation errors: `RUST_LOG=mycelial_meshtastic=debug`
- Ensure hop_limit > 0 on device for mesh routing
- Verify libp2p node is publishing to the correct topics

**High message loss:**
- Reduce transmit frequency (LoRa channels have duty cycle limits)
- Increase spreading factor for better range at cost of speed
- Check for local interference on your LoRa frequency

The bridge automatically forwards messages between the libp2p network and Meshtastic LoRa mesh, enabling long-range (2-10km) radio communication for the Mycelial Economics protocols.

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
| Meshtastic Bridge | tokio-serial, miniz_oxide, async-trait |
| Persistence | SQLite + sqlx + LRU cache |
| HTTP Server | Axum + tokio |
| Dashboard | React 18 + Vite + TypeScript + TailwindCSS |
| Visualization | D3.js force-directed graph |

## Project Status

- **P2P Network**: Production-ready (150+ tests passing)
- **Meshtastic Bridge**: Complete (116 tests - serial, TCP, BLE interfaces)
- **Dashboard**: Functional (peer graph, chat, ENR panels, orchestrator)
- **Orchestrator**: Beta (workload scheduling, health monitoring, ENR credits)
- **Economics**: Complete (reputation, credit, governance, resource sharing)

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

## Disclaimer

> [!IMPORTANT]
> **This is an experimental system. _We break things frequently_.**

- Not accepting contributions yet (but we plan to!)
- No stability guarantees
- Pin commits if you need consistency
- This is a learning resource, not production software
- **No support provided** - See [SUPPORT.md](SUPPORT.md)
---

*Built by [Univrs.io](https://univrs.io)*
