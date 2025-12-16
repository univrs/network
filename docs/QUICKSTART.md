# Mycelial P2P Bootstrap - Quick Start Guide

Get started with the Mycelial P2P network in under 5 minutes.

## Prerequisites

- **Rust** (1.70+): Install via [rustup](https://rustup.rs/)
- **Node.js** (18+): For the dashboard
- **pnpm**: `npm install -g pnpm`

## 1. Build the Project

```bash
cd mycelial-p2p-bootstrap

# Build all Rust crates
cargo build --release
```

## 2. Start a Bootstrap Node

The bootstrap node is the first peer in your network. Other peers connect to it to discover each other.

```bash
cargo run --release --bin mycelial-node -- \
  --bootstrap \
  --name "Bootstrap" \
  --port 9000 \
  --http-port 8080
```

You should see output like:
```
INFO  Starting Mycelial node: Bootstrap
INFO  Peer ID: 12D3KooW...
INFO  Listening on /ip4/0.0.0.0/tcp/9000
INFO  HTTP server listening on 0.0.0.0:8080
INFO  Subscribed to topic: /mycelial/1.0.0/chat
```

## 3. Start Additional Peers

Open new terminal windows and start more peers. They will auto-discover the bootstrap node.

**Peer Alice:**
```bash
cargo run --release --bin mycelial-node -- \
  --name "Alice" \
  --connect "/ip4/127.0.0.1/tcp/9000"
```

**Peer Bob:**
```bash
cargo run --release --bin mycelial-node -- \
  --name "Bob" \
  --connect "/ip4/127.0.0.1/tcp/9000"
```

Watch the logs - you should see peer discovery messages:
```
INFO  Peer discovered via Kademlia: 12D3KooW...
INFO  Mesh peer added: 12D3KooW...
```

## 4. Start the Dashboard

```bash
cd dashboard
pnpm install
pnpm dev
```

Open http://localhost:5173 in your browser.

You'll see:
- **Peer Graph**: Visual network of connected peers
- **Chat Panel**: Send messages to the network
- **Connection Status**: Green indicator when connected

## 5. Send Messages

In the dashboard chat panel, type a message and press Enter. Messages are broadcast to all connected peers via gossipsub.

You can also watch the node logs to see message flow:
```
INFO  Received chat message from 12D3KooW...: "Hello network!"
INFO  Chat message published successfully
```

## Configuration

### Environment Variables (Dashboard)

Create `dashboard/.env.local`:
```bash
VITE_WS_URL=ws://localhost:8080/ws
VITE_API_URL=http://localhost:8080
```

### CLI Arguments (Node)

| Argument | Description | Default |
|----------|-------------|---------|
| `--name` | Display name for this peer | `Peer-{id}` |
| `--port` | P2P listening port | Auto-select 9001-9100 |
| `--http-port` | HTTP/WebSocket server port | None (disabled) |
| `--bootstrap` | Run as bootstrap node | false |
| `--connect` | Multiaddr to connect to | None |

## Network Topology

```
                    ┌─────────────┐
                    │  Bootstrap  │ :9000
                    │   (Hub)     │
                    └──────┬──────┘
                           │
           ┌───────────────┼───────────────┐
           │               │               │
    ┌──────┴──────┐ ┌──────┴──────┐ ┌──────┴──────┐
    │   Alice     │ │    Bob      │ │   Charlie   │
    │   :9001     │ │   :9002     │ │   :9003     │
    └─────────────┘ └─────────────┘ └─────────────┘
```

All peers discover each other through:
1. **Kademlia DHT** - Distributed peer discovery
2. **mDNS** - Local network discovery (same LAN)
3. **Gossipsub** - Message routing mesh

## Troubleshooting

### "Connection refused" when connecting to bootstrap
- Ensure the bootstrap node is running
- Check the port matches (`--port 9000`)
- Verify firewall isn't blocking TCP 9000

### Dashboard shows "Disconnected"
- Ensure node is running with `--http-port 8080`
- Check browser console for WebSocket errors
- Verify CORS allows your origin

### Messages not appearing on other peers
- Check logs for "Mesh peer added" - peers need mesh connections
- Wait a few seconds for gossipsub mesh to form
- Ensure both peers are subscribed to `/mycelial/1.0.0/chat`

## Next Steps

- Read [CLAUDE.md](../CLAUDE.md) for architecture details
- Check [ROADMAP.md](../ROADMAP.md) for implementation progress
- Explore the codebase:
  - `crates/mycelial-core/` - Core types and traits
  - `crates/mycelial-network/` - libp2p networking
  - `crates/mycelial-state/` - SQLite persistence
  - `dashboard/` - React web UI
