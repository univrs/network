# ENR UI Panels - Manual Testing Guide

> **Testing guide for Phase 4 ENR Bridge UI components**

This document explains how to manually test the four ENR (Economic Network Resource) dashboard panels:
- **GradientPanel** - Resource availability visualization
- **ElectionPanel** - Nexus coordinator elections
- **SeptalPanel** - Circuit breaker status
- **EnrCreditPanel** - ENR credit balances and transfers

---

## Prerequisites

- Rust toolchain (1.70+)
- Node.js (18+) with pnpm
- Two terminal windows minimum

## Quick Start

### 1. Build and Start the P2P Node

```bash
# Build the project
cargo build --release

# Start bootstrap node with HTTP server enabled
cargo run --release --bin mycelial-node -- \
  --bootstrap \
  --name "Bootstrap" \
  --port 9000 \
  --http-port 8080
```

Expected output:
```
INFO  Starting Mycelial node: Bootstrap
INFO  Peer ID: 12D3KooW...
INFO  Listening on /ip4/0.0.0.0/tcp/9000
INFO  HTTP server listening on 0.0.0.0:8080
INFO  ENR Bridge initialized
INFO  Subscribed to topic: /mycelial/1.0.0/chat
INFO  Subscribed to topic: /vudo/enr/gradient/1.0.0
INFO  Subscribed to topic: /vudo/enr/credits/1.0.0
INFO  Subscribed to topic: /vudo/enr/election/1.0.0
INFO  Subscribed to topic: /vudo/enr/septal/1.0.0
```

### 2. Start Additional Peers (Recommended)

For meaningful ENR data, start 2-3 additional peers:

**Terminal 2:**
```bash
cargo run --release --bin mycelial-node -- \
  --name "Alice" \
  --connect "/ip4/127.0.0.1/tcp/9000"
```

**Terminal 3:**
```bash
cargo run --release --bin mycelial-node -- \
  --name "Bob" \
  --connect "/ip4/127.0.0.1/tcp/9000"
```

### 3. Start the Dashboard

```bash
cd dashboard
pnpm install
pnpm dev
```

Open http://localhost:5173 in your browser.

---

## Testing Each Panel

### Accessing the ENR Panels

The ENR panels are accessible via buttons in the header, separated by a vertical divider from the core P2P panels:

| Button | Panel | Description |
|--------|-------|-------------|
| **Gradients** | GradientPanel | Resource availability gauges |
| **Elections** | ElectionPanel | Nexus coordinator voting |
| **Septal** | SeptalPanel | Circuit breaker status |
| **ENR** | EnrCreditPanel | Credit balances & transfers |

---

## Test Case 1: GradientPanel

### Empty State Test

1. Click the **Gradients** button in the header
2. **Expected**: Panel opens with message "No gradient updates received yet"
3. **Expected**: Subtitle explains "Gradients will appear as nodes broadcast their resource availability"

### With Data Test

When nodes are broadcasting gradients, you should see:

1. **Network Aggregate Section**:
   - Four circular gauges for CPU, Memory, Bandwidth, Storage
   - Each gauge shows 0-100% with color coding:
     - Green (>=70%): Good availability
     - Yellow (40-70%): Moderate availability
     - Red (<40%): Low availability
   - Node count indicator (e.g., "3 nodes reporting")

2. **Per-Node Resources Section**:
   - List of nodes with shortened IDs
   - Four horizontal progress bars per node
   - Septal state badge (closed/half_open/open)
   - Timestamp showing last update

### Verification Checklist

- [ ] Panel opens and closes correctly (X button)
- [ ] Empty state displays when no data
- [ ] Circular gauges render without errors
- [ ] Color coding reflects availability levels
- [ ] Node list scrolls when many nodes present

---

## Test Case 2: ElectionPanel

### Empty State Test

1. Click the **Elections** button in the header
2. **Expected**: Panel opens with message "No elections in progress"
3. **Expected**: Subtitle explains "Elections are initiated when a region needs a new coordinator"

### With Data Test

When an election is active, you should see:

1. **Active Elections Section**:
   - Pulsing indicator for active elections
   - Election card with region ID and election number
   - Status badge (announced/voting/completed)
   - Expandable details showing:
     - Candidate count and vote count
     - Candidate cards with qualifications (uptime, reputation, CPU, memory)
     - Winner indicator for completed elections

2. **Recent Elections Section**:
   - List of completed elections
   - Click to expand and see results

### Verification Checklist

- [ ] Panel opens and closes correctly
- [ ] Empty state displays when no elections
- [ ] Active elections auto-expand
- [ ] Candidate cards show all qualification metrics
- [ ] Winner badge displays for completed elections
- [ ] Votes count updates correctly

---

## Test Case 3: SeptalPanel

### Empty State Test

1. Click the **Septal** button in the header
2. **Expected**: Panel opens with message "No septal gate data available"
3. **Expected**: Subtitle explains "Circuit breaker states will appear as nodes report health status"

### With Data Test

When nodes report septal state, you should see:

1. **Network Health Overview**:
   - Circular health percentage gauge
   - State counts (Closed/Half-Open/Open)
   - Total nodes and total failures
   - Stacked bar showing state distribution

2. **Node Groups** (ordered by priority):
   - **Tripped Circuits (Open)**: Red pulsing indicator, nodes needing attention
   - **Testing Recovery (Half-Open)**: Yellow indicator, recovering nodes
   - **Healthy Nodes (Closed)**: Green, operating normally

3. **Per-Node Cards**:
   - Node ID with state icon
   - Health status (Healthy/Unhealthy)
   - Failure count
   - Balance display
   - Failure warning banner when count > 0

### Verification Checklist

- [ ] Panel opens and closes correctly
- [ ] Empty state displays when no data
- [ ] Health gauge shows correct percentage
- [ ] State color coding is correct (green/yellow/red)
- [ ] Tripped circuits appear first (most urgent)
- [ ] Failure warnings display for affected nodes

---

## Test Case 4: EnrCreditPanel

### Empty State Test

1. Click the **ENR** button in the header
2. **Expected**: Panel opens with message "No ENR credit data available"
3. **Expected**: Subtitle explains "Credit balances and transfers will appear as nodes participate in the network"

### With Data Test

When credit data is available, you should see:

1. **Network Credit Summary**:
   - Total Balance across all nodes
   - Average Balance per node
   - Total Transferred amount
   - Tax Collected (entropy tax)
   - Balance range indicator (min to max)

2. **Node Balances Leaderboard**:
   - Ranked list of nodes by balance
   - Top 3 highlighted with gold/silver/bronze styling
   - Septal state indicator per node
   - Last updated timestamp

3. **Recent Transfers**:
   - Transfer cards with from/to addresses
   - Amount with +/- indicator for local node
   - Tax amount when applicable
   - Transaction nonce
   - Incoming transfers highlighted in cyan
   - Outgoing transfers highlighted in purple

### Verification Checklist

- [ ] Panel opens and closes correctly
- [ ] Empty state displays when no data
- [ ] Statistics calculate correctly
- [ ] Leaderboard ranks nodes by balance
- [ ] Top 3 nodes have special styling
- [ ] Transfers show correct direction indicators
- [ ] Tax amounts display when > 0

---

## Simulating ENR Messages (Development)

To test with synthetic data, you can send WebSocket messages directly. Open browser DevTools console:

```javascript
// Get the WebSocket connection
const ws = new WebSocket('ws://localhost:8080/ws');

// Wait for connection
ws.onopen = () => {
  // Simulate a gradient update
  ws.send(JSON.stringify({
    type: 'gradient_update',
    source: 'test-node-001',
    cpuAvailable: 0.75,
    memoryAvailable: 0.60,
    bandwidthAvailable: 0.85,
    storageAvailable: 0.45,
    timestamp: Date.now()
  }));
};
```

**Note**: The backend must support these message types for simulation to work. Check `crates/mycelial-node/src/server/websocket.rs` for supported message handlers.

---

## Troubleshooting

### Panels show empty state but nodes are connected

1. **Check ENR Bridge initialization**: Look for "ENR Bridge initialized" in node logs
2. **Verify gossipsub subscriptions**: Logs should show `/vudo/enr/*` topic subscriptions
3. **Check WebSocket connection**: Dashboard should show "Connected" in header
4. **Inspect network tab**: Look for WebSocket messages with ENR types

### Panel data not updating

1. **Verify multiple nodes running**: ENR data flows between peers
2. **Check mesh formation**: Look for "Mesh peer added" in logs
3. **Wait for broadcast interval**: Gradient updates occur on a timer (default 30s)

### TypeScript errors in console

1. **Check types match**: Ensure `dashboard/src/types.ts` matches Rust message format
2. **Verify field names**: Server uses `snake_case`, client expects `camelCase`

---

## WebSocket Message Format Reference

### Server -> Dashboard Messages

| Type | Fields | Description |
|------|--------|-------------|
| `gradient_update` | source, cpuAvailable, memoryAvailable, bandwidthAvailable, storageAvailable, timestamp | Resource availability |
| `enr_balance_update` | nodeId, balance, timestamp | Node balance change |
| `enr_credit_transfer` | from, to, amount, tax, nonce, timestamp | Credit transfer |
| `election_announcement` | electionId, initiator, regionId, timestamp | New election started |
| `election_candidacy` | electionId, candidate, uptime, cpuAvailable, memoryAvailable, reputation, timestamp | Candidate joined |
| `election_vote` | electionId, voter, candidate, timestamp | Vote cast |
| `election_result` | electionId, winner, regionId, voteCount, timestamp | Election completed |
| `septal_state_change` | nodeId, fromState, toState, reason, timestamp | Circuit breaker change |
| `septal_health_status` | nodeId, isHealthy, failureCount, timestamp | Health probe result |

---

## Success Criteria

After completing manual testing, verify:

- [ ] All four panels open without errors
- [ ] Empty states display correctly
- [ ] Panels close via X button and clicking outside
- [ ] Data renders correctly when available
- [ ] Color coding and visualizations are accurate
- [ ] Scrolling works for long lists
- [ ] No console errors during normal operation

---

*Last Updated: 2025-12-31 - Phase 4 ENR UI Testing Guide*
