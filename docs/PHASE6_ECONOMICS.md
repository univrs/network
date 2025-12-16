# Phase 6: Mycelial Economics Bootstrap

> Enable new users to easily join and participate in the regenerative economic network

## Overview

Phase 6 transforms the Mycelial P2P network from a messaging system into a foundation for regenerative economics. The goal is to make onboarding simple while establishing trust, reputation, and mutual credit primitives.

## 6.1 Onboarding Flow

### User Journey
1. New user visits web dashboard
2. Clicks "Join Network" - keypair generated in browser
3. Receives invite link or scans QR code to connect
4. Completes brief tutorial showing network basics
5. Starts with initial reputation score

### Implementation Tasks

| Task | Priority | Complexity |
|------|----------|------------|
| Browser keypair generation (Ed25519) | High | Medium |
| QR code generation for peer addresses | High | Low |
| Invite link system with bootstrap addresses | High | Medium |
| First-time user tutorial overlay | Medium | Medium |
| Mobile-responsive onboarding | Medium | High |

### Technical Approach

```rust
// Browser-side keypair generation (WASM)
pub fn generate_identity() -> JsIdentity {
    let keypair = ed25519::Keypair::generate();
    JsIdentity {
        peer_id: PeerId::from(keypair.public()).to_string(),
        private_key: keypair.encode().to_vec(),
        did: Did::from(keypair.public()).to_string(),
    }
}
```

### Invite Link Format
```
mycelial://join?
  bootstrap=/ip4/1.2.3.4/tcp/9000/p2p/12D3KooW...
  &inviter=12D3KooW...
  &network=mainnet
```

## 6.2 Reputation System

### Score Components

| Component | Weight | Description |
|-----------|--------|-------------|
| Uptime | 20% | Time online / total time since join |
| Message Relay | 25% | Messages successfully forwarded |
| Storage Contribution | 25% | Content blocks stored for others |
| Vouches Received | 20% | Trust vouches from other peers |
| Dispute Resolution | 10% | Fair behavior in conflicts |

### Reputation Formula

```
R(t) = α × R(t-1) + β × C(t)

Where:
  R(t) = Reputation at time t
  α = 0.7 (historical weight)
  β = 0.3 (recent contribution weight)
  C(t) = Contribution score for period t
```

### Implementation Tasks

| Task | Priority | Complexity |
|------|----------|------------|
| Reputation score calculation service | High | Medium |
| Gossipsub topic for reputation updates | High | Low |
| Initial reputation for new peers (0.3) | High | Low |
| Vouching system (peer endorsements) | High | Medium |
| Reputation decay for inactive peers | Medium | Low |
| Anti-sybil measures | Medium | High |
| Dashboard reputation visualization | Medium | Medium |

### Gossipsub Message
```rust
#[derive(Serialize, Deserialize)]
pub struct ReputationUpdate {
    pub peer_id: String,
    pub score: f64,
    pub components: ReputationComponents,
    pub timestamp: i64,
    pub signature: Vec<u8>,
}
```

## 6.3 Mutual Credit Foundation

### Credit Relationships

Peers can establish bilateral credit lines based on mutual trust:

```
Alice ←──[$100 credit line]──→ Bob
         ├── Alice can owe Bob up to $100
         └── Bob can owe Alice up to $100
```

### Credit Rules

1. **Mutual Consent** - Both parties must agree to credit line
2. **Reputation Threshold** - Minimum 0.5 reputation to create credit
3. **Credit Limit Formula** - `min(A_rep, B_rep) × base_limit`
4. **Settlement Period** - Credits settle automatically after 30 days

### Implementation Tasks

| Task | Priority | Complexity |
|------|----------|------------|
| Credit relationship data model | High | Medium |
| Credit offer/accept protocol | High | Medium |
| Balance tracking (CRDT-based) | High | High |
| Credit transfer mechanism | High | Medium |
| Settlement automation | Medium | High |
| Dashboard credit visualization | Medium | Medium |
| Credit history audit trail | Low | Medium |

### Protocol Messages

```rust
pub enum CreditMessage {
    Offer {
        to: PeerId,
        limit: u64,
        terms: CreditTerms,
    },
    Accept {
        offer_id: Uuid,
    },
    Transfer {
        to: PeerId,
        amount: u64,
        memo: String,
    },
    Settle {
        relationship_id: Uuid,
    },
}
```

## 6.4 Resource Sharing

### Tracked Resources

| Resource | Unit | Measurement |
|----------|------|-------------|
| Bandwidth | MB | Data relayed for others |
| Storage | MB | Content blocks stored |
| Compute | CPU-seconds | Future: agent task execution |

### Contribution Tracking

```rust
pub struct ContributionMetrics {
    pub peer_id: PeerId,
    pub period: TimePeriod,
    pub bandwidth_mb: f64,
    pub storage_mb: f64,
    pub messages_relayed: u64,
    pub uptime_percentage: f64,
}
```

### Implementation Tasks

| Task | Priority | Complexity |
|------|----------|------------|
| Bandwidth metering | Medium | Medium |
| Storage contribution tracking | Medium | Medium |
| Contribution leaderboard | Low | Low |
| Resource sharing incentives | Low | High |

## 6.5 Governance Primitives

### Proposal System

Simple text-based proposals with voting:

```rust
pub struct Proposal {
    pub id: Uuid,
    pub author: PeerId,
    pub title: String,
    pub description: String,
    pub voting_ends: DateTime<Utc>,
    pub quorum: f64,  // e.g., 0.51 for majority
    pub votes: HashMap<PeerId, Vote>,
}

pub enum Vote {
    For,
    Against,
    Abstain,
}
```

### Voting Rules

1. **One Vote Per Peer** - Based on peer identity
2. **Reputation Weight** - Optional: votes weighted by reputation
3. **Quorum Requirement** - Minimum participation threshold
4. **Time-bound** - Proposals expire after voting period

### Implementation Tasks

| Task | Priority | Complexity |
|------|----------|------------|
| Proposal creation UI | Low | Medium |
| Vote collection via gossipsub | Low | Medium |
| Quorum calculation | Low | Low |
| Result broadcast | Low | Low |

## Database Schema Additions

```sql
-- Reputation tracking
CREATE TABLE reputation_scores (
    peer_id TEXT PRIMARY KEY,
    score REAL NOT NULL DEFAULT 0.3,
    uptime_component REAL,
    relay_component REAL,
    storage_component REAL,
    vouch_component REAL,
    updated_at INTEGER NOT NULL
);

-- Vouching system
CREATE TABLE vouches (
    voucher_id TEXT NOT NULL,
    vouchee_id TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (voucher_id, vouchee_id)
);

-- Credit relationships
CREATE TABLE credit_relationships (
    id TEXT PRIMARY KEY,
    peer_a TEXT NOT NULL,
    peer_b TEXT NOT NULL,
    limit_amount INTEGER NOT NULL,
    balance INTEGER NOT NULL DEFAULT 0,  -- positive = A owes B
    created_at INTEGER NOT NULL,
    terms_json TEXT
);

-- Credit transactions
CREATE TABLE credit_transactions (
    id TEXT PRIMARY KEY,
    relationship_id TEXT NOT NULL,
    from_peer TEXT NOT NULL,
    to_peer TEXT NOT NULL,
    amount INTEGER NOT NULL,
    memo TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (relationship_id) REFERENCES credit_relationships(id)
);

-- Governance proposals
CREATE TABLE proposals (
    id TEXT PRIMARY KEY,
    author_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    voting_ends INTEGER NOT NULL,
    quorum REAL NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    created_at INTEGER NOT NULL
);

CREATE TABLE votes (
    proposal_id TEXT NOT NULL,
    voter_id TEXT NOT NULL,
    vote TEXT NOT NULL,  -- 'for', 'against', 'abstain'
    created_at INTEGER NOT NULL,
    PRIMARY KEY (proposal_id, voter_id),
    FOREIGN KEY (proposal_id) REFERENCES proposals(id)
);
```

## Success Metrics

| Metric | Target |
|--------|--------|
| Onboarding completion rate | > 80% |
| Time to first message | < 2 minutes |
| Peer retention (7 day) | > 50% |
| Active credit relationships | > 30% of peers |
| Proposal participation | > 40% of eligible |

## Timeline Estimate

Phase 6 can be implemented incrementally:

1. **6.1 Onboarding** - Foundation for user growth
2. **6.2 Reputation** - Trust layer for economics
3. **6.3 Mutual Credit** - Core economic primitive
4. **6.4 Resource Sharing** - Sustainability model
5. **6.5 Governance** - Community self-direction

## References

- [Mutual Credit Systems](https://wiki.p2pfoundation.net/Mutual_Credit)
- [LETS (Local Exchange Trading Systems)](https://en.wikipedia.org/wiki/Local_exchange_trading_system)
- [Trustlines Network](https://trustlines.network/)
- [Grassroots Economics](https://www.grassrootseconomics.org/)
