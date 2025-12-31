# OpenRaft Consensus Layer for ENR Credit Ledger

## Phase 1: Distributed Consensus for Credit Synchronization

This document describes the integration of [OpenRaft](https://github.com/databendlabs/openraft) into the Mycelial Network's ENR (Entropy-Nexus-Revival) credit system, replacing the MVP's optimistic local ledger with a distributed consensus protocol.

## Overview

### Current State (Phase 0 - MVP)

The current `CreditSynchronizer` uses:
- **Local HashMap** as the ledger
- **Optimistic updates** - transfers applied immediately on sender
- **Gossipsub broadcast** - transfers propagated via gossip
- **Nonce-based replay protection** - prevents duplicate processing

```
┌─────────────────────────────────────────────────────────────────┐
│                     Phase 0: Optimistic Ledger                  │
│                                                                 │
│   Node A                    Node B                    Node C    │
│   ┌─────────┐              ┌─────────┐              ┌─────────┐│
│   │ HashMap │              │ HashMap │              │ HashMap ││
│   │ Ledger  │───gossip────▶│ Ledger  │───gossip────▶│ Ledger  ││
│   └─────────┘              └─────────┘              └─────────┘│
│                                                                 │
│   Problem: Ledgers can diverge under network partitions         │
└─────────────────────────────────────────────────────────────────┘
```

### Target State (Phase 1 - Consensus)

OpenRaft provides:
- **Replicated log** - all nodes agree on transaction order
- **Leader election** - single writer prevents conflicts
- **Snapshot support** - efficient state transfer for new nodes
- **Linearizable reads** - strong consistency guarantees

```
┌─────────────────────────────────────────────────────────────────┐
│                     Phase 1: OpenRaft Consensus                 │
│                                                                 │
│              ┌─────────────────────────────────────┐            │
│              │           Raft Cluster              │            │
│              │                                     │            │
│              │   ┌─────────┐                       │            │
│              │   │ Leader  │ ◄── write requests    │            │
│              │   │ (Node A)│                       │            │
│              │   └────┬────┘                       │            │
│              │        │ replicate                  │            │
│              │   ┌────┴────┬────────────┐         │            │
│              │   ▼         ▼            ▼         │            │
│              │ ┌─────┐  ┌─────┐    ┌─────┐       │            │
│              │ │ B   │  │ C   │    │ D   │       │            │
│              │ │Follw│  │Follw│    │Follw│       │            │
│              │ └─────┘  └─────┘    └─────┘       │            │
│              │                                     │            │
│              └─────────────────────────────────────┘            │
│                                                                 │
│   Guarantee: All nodes converge to same ledger state            │
└─────────────────────────────────────────────────────────────────┘
```

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          mycelial-network                               │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                        EnrBridge                                 │   │
│  │                                                                  │   │
│  │  ┌──────────────────┐    ┌──────────────────┐                   │   │
│  │  │ GradientBroadcst │    │ SeptalGateMgr    │  (unchanged)      │   │
│  │  └──────────────────┘    └──────────────────┘                   │   │
│  │                                                                  │   │
│  │  ┌──────────────────┐    ┌──────────────────┐                   │   │
│  │  │ DistributedElect │    │ CreditSynchronzr │◄─── Phase 1 focus │   │
│  │  └──────────────────┘    └────────┬─────────┘                   │   │
│  │                                   │                              │   │
│  └───────────────────────────────────┼──────────────────────────────┘   │
│                                      │                                  │
│  ┌───────────────────────────────────┼──────────────────────────────┐   │
│  │                         OpenRaft Layer                           │   │
│  │                                   │                              │   │
│  │  ┌────────────────────────────────▼─────────────────────────┐   │   │
│  │  │                    RaftCreditLedger                       │   │   │
│  │  │                                                           │   │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │   │
│  │  │  │ RaftNetwork │  │RaftLogStore │  │RaftStateMac │       │   │   │
│  │  │  │ (gossipsub) │  │ (sled/mem)  │  │  (credits)  │       │   │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │   │
│  │  │                                                           │   │   │
│  │  └───────────────────────────────────────────────────────────┘   │   │
│  │                                                                  │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
Credit Transfer Request
        │
        ▼
┌───────────────────┐
│ CreditSynchronizer│
│   .transfer()     │
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│ RaftCreditLedger  │
│   .propose()      │ ──── Propose to Raft
└────────┬──────────┘
         │
         ▼
┌───────────────────┐       ┌───────────────────┐
│   Raft Leader     │──────▶│   Raft Followers  │
│ (append to log)   │ replc │ (append to log)   │
└────────┬──────────┘       └────────┬──────────┘
         │                           │
         │ commit                    │ commit
         ▼                           ▼
┌───────────────────┐       ┌───────────────────┐
│  State Machine    │       │  State Machine    │
│ (apply transfer)  │       │ (apply transfer)  │
└───────────────────┘       └───────────────────┘
```

## OpenRaft Integration

### Dependencies

```toml
[dependencies]
openraft = { version = "0.10", features = ["serde", "tracing"] }
sled = "0.34"  # or use in-memory store for testing
```

### Trait Implementations

OpenRaft requires implementing three core traits:

#### 1. RaftNetwork - Transport Layer

```rust
/// Uses existing gossipsub infrastructure for Raft messages
pub struct GossipsubRaftNetwork {
    publish_fn: PublishFn,
    /// Topic for Raft protocol messages
    raft_topic: String,
}

#[async_trait]
impl RaftNetwork<CreditTypeConfig> for GossipsubRaftNetwork {
    async fn append_entries(
        &self,
        target: NodeId,
        rpc: AppendEntriesRequest<CreditTypeConfig>,
    ) -> Result<AppendEntriesResponse, RPCError> {
        // Encode and publish via gossipsub
        // Target node handles in message loop
    }

    async fn vote(
        &self,
        target: NodeId,
        rpc: VoteRequest,
    ) -> Result<VoteResponse, RPCError> {
        // Leader election messages
    }

    async fn install_snapshot(
        &self,
        target: NodeId,
        rpc: InstallSnapshotRequest,
    ) -> Result<InstallSnapshotResponse, RPCError> {
        // Snapshot transfer for catching up
    }
}
```

#### 2. RaftLogStorage - Persistent Log

```rust
/// Stores Raft log entries for durability
pub struct SledLogStorage {
    db: sled::Db,
    log_tree: sled::Tree,
    vote_tree: sled::Tree,
}

#[async_trait]
impl RaftLogStorage<CreditTypeConfig> for SledLogStorage {
    async fn append_to_log(&mut self, entries: &[Entry<CreditTypeConfig>]) -> Result<()> {
        // Persist log entries to sled
    }

    async fn delete_conflict_logs_since(&mut self, log_id: LogId) -> Result<()> {
        // Handle log conflicts during leader changes
    }

    async fn purge_logs_upto(&mut self, log_id: LogId) -> Result<()> {
        // Compact old logs after snapshot
    }

    async fn save_vote(&mut self, vote: &Vote) -> Result<()> {
        // Persist vote for crash recovery
    }

    async fn read_vote(&mut self) -> Result<Option<Vote>> {
        // Recover vote after restart
    }
}
```

#### 3. RaftStateMachine - Credit Ledger

```rust
/// The actual credit ledger as a Raft state machine
pub struct CreditStateMachine {
    /// Account balances: AccountId -> Credits
    balances: HashMap<AccountId, Credits>,
    /// Revival pool balance
    revival_pool: Credits,
    /// Last applied log ID
    last_applied: Option<LogId>,
}

#[async_trait]
impl RaftStateMachine<CreditTypeConfig> for CreditStateMachine {
    async fn apply(&mut self, entries: Vec<Entry<CreditTypeConfig>>) -> Result<Vec<Response>> {
        let mut responses = Vec::new();

        for entry in entries {
            match entry.payload {
                EntryPayload::Normal(CreditCommand::Transfer(transfer)) => {
                    // Apply transfer with entropy tax
                    let result = self.apply_transfer(&transfer);
                    responses.push(Response::Transfer(result));
                }
                EntryPayload::Normal(CreditCommand::GrantCredits(grant)) => {
                    // Initial credit grants for new nodes
                    self.apply_grant(&grant);
                    responses.push(Response::Grant);
                }
                _ => {}
            }
            self.last_applied = Some(entry.log_id);
        }

        Ok(responses)
    }

    async fn build_snapshot(&mut self) -> Result<Snapshot<CreditTypeConfig>> {
        // Serialize current balances for efficient state transfer
        let data = bincode::serialize(&self.balances)?;
        Ok(Snapshot {
            meta: SnapshotMeta { last_log_id: self.last_applied, .. },
            snapshot: Box::new(Cursor::new(data)),
        })
    }

    async fn install_snapshot(&mut self, meta: &SnapshotMeta, snapshot: Box<dyn AsyncRead>) -> Result<()> {
        // Restore state from snapshot
        let data = read_to_end(snapshot).await?;
        self.balances = bincode::deserialize(&data)?;
        self.last_applied = meta.last_log_id;
        Ok(())
    }
}
```

### Type Configuration

```rust
/// OpenRaft type configuration for credit ledger
pub struct CreditTypeConfig;

impl RaftTypeConfig for CreditTypeConfig {
    type D = CreditCommand;       // Command type
    type R = CreditResponse;      // Response type
    type NodeId = NodeId;         // Use ENR NodeId
    type Node = BasicNode;        // Node metadata
    type Entry = Entry<Self>;     // Log entry type
}

/// Commands that can be proposed to the Raft cluster
#[derive(Serialize, Deserialize)]
pub enum CreditCommand {
    Transfer(CreditTransfer),
    GrantCredits { node: NodeId, amount: Credits },
    RecordFailure { node: NodeId, reason: String },
}

/// Responses from applying commands
#[derive(Serialize, Deserialize)]
pub enum CreditResponse {
    Transfer(Result<(), TransferError>),
    Grant,
    Failure,
}
```

## Migration Strategy

### Phase 1a: Parallel Operation

Run both systems simultaneously:
1. Raft cluster for consensus
2. Gossipsub for fast reads (eventually consistent)

```rust
impl CreditSynchronizer {
    pub async fn transfer(&self, to: NodeId, amount: Credits) -> Result<CreditTransfer, TransferError> {
        // Propose to Raft cluster (blocks until committed)
        let result = self.raft_ledger.propose(CreditCommand::Transfer(transfer)).await?;

        // Also broadcast via gossipsub for fast propagation
        // (Followers can serve reads before Raft commit)
        self.publish_transfer(&transfer).await?;

        result
    }
}
```

### Phase 1b: Full Raft

After validation, remove optimistic gossip path:
- All writes through Raft
- Reads can be local (after heartbeat confirms leadership)
- Linearizable reads via `read_index` if needed

## Gossipsub Topics

New topic for Raft protocol messages:

| Topic | Purpose |
|-------|---------|
| `/vudo/enr/raft/1.0.0` | Raft AppendEntries, Vote, Snapshot |
| `/vudo/enr/credits/1.0.0` | Credit transfers (read optimization) |

## Cluster Management

### Bootstrap

```rust
/// Initialize a new Raft cluster
pub async fn bootstrap_cluster(
    nodes: Vec<NodeId>,
    initial_balances: HashMap<NodeId, Credits>,
) -> Result<RaftCreditLedger> {
    let config = Config {
        heartbeat_interval: 100,
        election_timeout_min: 300,
        election_timeout_max: 500,
        ..Default::default()
    };

    // Initialize with membership
    let membership = Membership::new(nodes.clone());

    // Create Raft instance
    let raft = Raft::new(
        local_node_id,
        config,
        network,
        log_storage,
        state_machine,
    ).await?;

    // Bootstrap if first node
    if is_bootstrap_node {
        raft.initialize(membership).await?;
    }

    Ok(RaftCreditLedger { raft })
}
```

### Dynamic Membership

```rust
/// Add a new node to the cluster
pub async fn add_node(&self, node: NodeId) -> Result<()> {
    // Propose membership change
    self.raft.change_membership(
        ChangeMembers::AddNodes(btreeset! { node }),
        false, // don't turn to learner first
    ).await?;

    // Grant initial credits
    self.raft.client_write(CreditCommand::GrantCredits {
        node,
        amount: Credits::new(INITIAL_NODE_CREDITS),
    }).await?;

    Ok(())
}

/// Remove a node from the cluster
pub async fn remove_node(&self, node: NodeId) -> Result<()> {
    self.raft.change_membership(
        ChangeMembers::RemoveNodes(btreeset! { node }),
        false,
    ).await
}
```

## Testing Strategy

### Unit Tests

```rust
#[tokio::test]
async fn test_single_node_transfer() {
    let ledger = RaftCreditLedger::new_single_node(node_id).await;
    ledger.grant_credits(node_id, Credits::new(1000)).await;

    let result = ledger.transfer(node_id, node2_id, Credits::new(100)).await;
    assert!(result.is_ok());

    let balance = ledger.get_balance(node_id).await;
    assert_eq!(balance.amount, 898); // 1000 - 100 - 2 tax
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_three_node_cluster_transfer() {
    let cluster = TestRaftCluster::spawn(3).await;
    cluster.wait_for_leader(5).await;

    // Transfer via any node (forwarded to leader)
    cluster.node(1).transfer(node0, node2, Credits::new(100)).await.unwrap();

    // All nodes should converge
    for node in cluster.nodes() {
        let balance = node.get_balance(node0).await;
        assert_eq!(balance.amount, 898);
    }
}

#[tokio::test]
async fn test_leader_failure_recovery() {
    let cluster = TestRaftCluster::spawn(3).await;
    let leader = cluster.wait_for_leader(5).await;

    // Kill leader
    cluster.kill_node(leader).await;

    // New leader should be elected
    let new_leader = cluster.wait_for_leader(10).await;
    assert_ne!(new_leader, leader);

    // Transfers should still work
    let result = cluster.node(new_leader).transfer(...).await;
    assert!(result.is_ok());
}
```

## Configuration

```rust
/// OpenRaft configuration for ENR cluster
pub struct RaftConfig {
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval: u64,
    /// Min election timeout (ms)
    pub election_timeout_min: u64,
    /// Max election timeout (ms)
    pub election_timeout_max: u64,
    /// Max entries per append request
    pub max_payload_entries: u64,
    /// Snapshot policy
    pub snapshot_policy: SnapshotPolicy,
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: 100,
            election_timeout_min: 300,
            election_timeout_max: 500,
            max_payload_entries: 100,
            snapshot_policy: SnapshotPolicy::LogsSinceLast(1000),
        }
    }
}
```

## Performance Considerations

### Latency

| Operation | Phase 0 (Gossip) | Phase 1 (Raft) |
|-----------|------------------|----------------|
| Local write | ~1ms | ~10-50ms (quorum) |
| Remote read | ~10ms (gossip) | ~1ms (local) |
| Consistency | Eventual | Strong |

### Throughput

- OpenRaft batches entries for high throughput
- Snapshot policy prevents unbounded log growth
- Read optimization via follower reads

## Implementation Roadmap

### Sprint 1: Core Infrastructure
- [ ] Add openraft dependency
- [ ] Define CreditTypeConfig and commands
- [ ] Implement in-memory log storage
- [ ] Basic single-node operation

### Sprint 2: Network Integration
- [ ] GossipsubRaftNetwork implementation
- [ ] Raft message serialization
- [ ] Leader forwarding for writes

### Sprint 3: Persistence
- [ ] Sled-based log storage
- [ ] Snapshot creation/restore
- [ ] Crash recovery tests

### Sprint 4: Cluster Operations
- [ ] Bootstrap protocol
- [ ] Dynamic membership changes
- [ ] Integration with existing EnrBridge

## References

- [OpenRaft GitHub](https://github.com/databendlabs/openraft)
- [OpenRaft Documentation](https://docs.rs/openraft)
- [Raft Paper](https://raft.github.io/raft.pdf)
- [ENR Credit System](../PHASE6_ECONOMICS.md)

---

*"Consensus is expensive, but correctness is priceless."*
