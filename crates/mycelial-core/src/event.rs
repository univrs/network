//! Event types for cross-module and network communication
//!
//! Events are the primary mechanism for communication between modules
//! and for notifying the network of state changes.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::identity::{Did, SignatureBytes};
use crate::content::ContentId;
use crate::peer::PeerId;

/// A network event that can be published and subscribed to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier
    pub id: Uuid,
    /// Event type
    pub event_type: EventType,
    /// Source peer
    pub source: PeerId,
    /// Event payload
    pub payload: EventPayload,
    /// When the event was created
    pub timestamp: DateTime<Utc>,
    /// Optional signature for verified events
    pub signature: Option<SignatureBytes>,
}

impl Event {
    /// Create a new event
    pub fn new(event_type: EventType, source: PeerId, payload: EventPayload) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            source,
            payload,
            timestamp: Utc::now(),
            signature: None,
        }
    }

    /// Create a system event
    pub fn system(source: PeerId, payload: SystemEvent) -> Self {
        Self::new(EventType::System, source, EventPayload::System(payload))
    }

    /// Create a content event
    pub fn content(source: PeerId, payload: ContentEvent) -> Self {
        Self::new(EventType::Content, source, EventPayload::Content(payload))
    }

    /// Create a reputation event
    pub fn reputation(source: PeerId, payload: ReputationEvent) -> Self {
        Self::new(EventType::Reputation, source, EventPayload::Reputation(payload))
    }

    /// Create a credit event
    pub fn credit(source: PeerId, payload: CreditEvent) -> Self {
        Self::new(EventType::Credit, source, EventPayload::Credit(payload))
    }

    /// Create a governance event
    pub fn governance(source: PeerId, payload: GovernanceEvent) -> Self {
        Self::new(EventType::Governance, source, EventPayload::Governance(payload))
    }
}

/// Types of events in the network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// System events (peer discovery, health, etc.)
    System,
    /// Content events (posts, media, etc.)
    Content,
    /// Reputation events
    Reputation,
    /// Credit/economic events
    Credit,
    /// Governance events (proposals, votes)
    Governance,
    /// Orchestration events (scheduling, workloads)
    Orchestration,
}

/// Event payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayload {
    System(SystemEvent),
    Content(ContentEvent),
    Reputation(ReputationEvent),
    Credit(CreditEvent),
    Governance(GovernanceEvent),
    Orchestration(OrchestrationEvent),
    /// Raw bytes for custom events
    Raw(Vec<u8>),
}

/// System-level events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    /// A new peer joined the network
    PeerJoined {
        peer_id: PeerId,
        addresses: Vec<String>,
    },
    /// A peer left the network
    PeerLeft {
        peer_id: PeerId,
    },
    /// Peer health heartbeat
    Heartbeat {
        peer_id: PeerId,
        uptime_secs: u64,
        connected_peers: u32,
    },
    /// Network topology changed
    TopologyChange {
        added: Vec<PeerId>,
        removed: Vec<PeerId>,
    },
}

/// Content-related events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentEvent {
    /// New content was published
    Published {
        content_id: ContentId,
        author: Did,
        content_type: String,
        size: u64,
    },
    /// Content was updated
    Updated {
        content_id: ContentId,
        new_content_id: ContentId,
        author: Did,
    },
    /// Content was deleted
    Deleted {
        content_id: ContentId,
        author: Did,
    },
    /// Content was pinned by a peer
    Pinned {
        content_id: ContentId,
        pinner: PeerId,
    },
    /// Content was unpinned
    Unpinned {
        content_id: ContentId,
        pinner: PeerId,
    },
    /// Content was requested
    Requested {
        content_id: ContentId,
        requester: PeerId,
    },
}

/// Reputation-related events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReputationEvent {
    /// Reputation score was updated
    ScoreUpdated {
        subject: Did,
        new_score: f64,
        reason: String,
    },
    /// Positive feedback was given
    PositiveFeedback {
        from: Did,
        to: Did,
        context: String,
    },
    /// Negative feedback was given
    NegativeFeedback {
        from: Did,
        to: Did,
        context: String,
    },
    /// Trust threshold was crossed
    TrustThresholdCrossed {
        subject: Did,
        crossed_above: bool,
        threshold: f64,
    },
}

/// Credit/economic events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreditEvent {
    /// Credit relationship was established
    RelationshipCreated {
        creditor: Did,
        debtor: Did,
        credit_limit: f64,
    },
    /// Credit was transferred
    Transfer {
        from: Did,
        to: Did,
        amount: f64,
        memo: Option<String>,
    },
    /// Credit limit was changed
    LimitChanged {
        creditor: Did,
        debtor: Did,
        old_limit: f64,
        new_limit: f64,
    },
    /// Relationship was terminated
    RelationshipTerminated {
        creditor: Did,
        debtor: Did,
        final_balance: f64,
    },
}

/// Governance events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GovernanceEvent {
    /// New proposal was created
    ProposalCreated {
        proposal_id: Uuid,
        proposer: Did,
        title: String,
        description: String,
        voting_ends: DateTime<Utc>,
    },
    /// Vote was cast
    VoteCast {
        proposal_id: Uuid,
        voter: Did,
        vote: Vote,
        weight: f64,
    },
    /// Proposal was executed
    ProposalExecuted {
        proposal_id: Uuid,
        result: ProposalResult,
    },
    /// Proposal was cancelled
    ProposalCancelled {
        proposal_id: Uuid,
        reason: String,
    },
}

/// Orchestration events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestrationEvent {
    /// Task was scheduled
    TaskScheduled {
        task_id: Uuid,
        scheduler: Did,
        executor: Option<Did>,
        deadline: Option<DateTime<Utc>>,
    },
    /// Task execution started
    TaskStarted {
        task_id: Uuid,
        executor: Did,
    },
    /// Task was completed
    TaskCompleted {
        task_id: Uuid,
        executor: Did,
        success: bool,
        result: Option<Vec<u8>>,
    },
    /// Workload was distributed
    WorkloadDistributed {
        workload_id: Uuid,
        coordinator: Did,
        workers: Vec<Did>,
    },
}

/// A vote on a proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    /// In favor
    Yes,
    /// Against
    No,
    /// Abstain
    Abstain,
}

/// Result of a proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalResult {
    /// Proposal passed
    Passed,
    /// Proposal rejected
    Rejected,
    /// Quorum not reached
    NoQuorum,
    /// Tie
    Tie,
}

/// Event subscription filter
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// Filter by event types
    pub event_types: Option<Vec<EventType>>,
    /// Filter by source peer
    pub source: Option<PeerId>,
    /// Filter by time range (after)
    pub after: Option<DateTime<Utc>>,
    /// Filter by time range (before)
    pub before: Option<DateTime<Utc>>,
}

impl EventFilter {
    /// Create a filter for specific event types
    pub fn for_types(types: Vec<EventType>) -> Self {
        Self {
            event_types: Some(types),
            ..Default::default()
        }
    }

    /// Create a filter for events from a specific peer
    pub fn from_peer(peer: PeerId) -> Self {
        Self {
            source: Some(peer),
            ..Default::default()
        }
    }

    /// Check if an event matches this filter
    pub fn matches(&self, event: &Event) -> bool {
        if let Some(ref types) = self.event_types {
            if !types.contains(&event.event_type) {
                return false;
            }
        }

        if let Some(ref source) = self.source {
            if &event.source != source {
                return false;
            }
        }

        if let Some(after) = self.after {
            if event.timestamp < after {
                return false;
            }
        }

        if let Some(before) = self.before {
            if event.timestamp > before {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let peer = PeerId("test-peer".to_string());
        let event = Event::system(
            peer.clone(),
            SystemEvent::PeerJoined {
                peer_id: peer.clone(),
                addresses: vec!["/ip4/127.0.0.1/tcp/4001".to_string()],
            },
        );

        assert_eq!(event.event_type, EventType::System);
        assert_eq!(event.source, peer);
    }

    #[test]
    fn test_event_filter() {
        let peer = PeerId("test-peer".to_string());
        let event = Event::content(
            peer.clone(),
            ContentEvent::Published {
                content_id: crate::content::ContentId::hash(b"test"),
                author: crate::identity::Did::parse("did:key:z6MkhaXg").unwrap(),
                content_type: "text/plain".to_string(),
                size: 100,
            },
        );

        let filter = EventFilter::for_types(vec![EventType::Content]);
        assert!(filter.matches(&event));

        let filter = EventFilter::for_types(vec![EventType::System]);
        assert!(!filter.matches(&event));
    }
}
