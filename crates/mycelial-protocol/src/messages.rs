//! Protocol-specific message definitions for Mycelial Economics
//!
//! This module defines all message types used in the gossipsub protocol
//! for the Mycelial Economics system: vouching, credits, governance, and resources.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Gossipsub topic names for economics protocols
pub mod topics {
    /// Topic for vouch/reputation messages
    pub const VOUCH: &str = "/mycelial/1.0.0/vouch";
    /// Topic for credit line and transfer messages
    pub const CREDIT: &str = "/mycelial/1.0.0/credit";
    /// Topic for governance proposals and votes
    pub const GOVERNANCE: &str = "/mycelial/1.0.0/governance";
    /// Topic for resource sharing metrics
    pub const RESOURCE: &str = "/mycelial/1.0.0/resource";
}

// ============================================================================
// VOUCH PROTOCOL MESSAGES
// ============================================================================

/// Messages for the vouch/reputation protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VouchMessage {
    /// Request to vouch for a peer
    VouchRequest(VouchRequest),
    /// Vouch acknowledgement
    VouchAck(VouchAck),
    /// Reputation update notification
    ReputationUpdate(ReputationUpdate),
}

/// A vouch request from one peer to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VouchRequest {
    /// Unique vouch ID
    pub id: Uuid,
    /// Peer giving the vouch
    pub voucher: String,
    /// Peer receiving the vouch
    pub vouchee: String,
    /// Stake amount (0.0 to 1.0, representing voucher's reputation commitment)
    pub stake: f64,
    /// Optional message explaining the vouch
    pub message: Option<String>,
    /// When the vouch was created
    pub timestamp: DateTime<Utc>,
    /// Expiration time for the vouch
    pub expires_at: Option<DateTime<Utc>>,
}

impl VouchRequest {
    /// Create a new vouch request
    pub fn new(voucher: String, vouchee: String, stake: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            voucher,
            vouchee,
            stake: stake.clamp(0.0, 1.0),
            message: None,
            timestamp: Utc::now(),
            expires_at: None,
        }
    }

    /// Add a message to the vouch
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set expiration time
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }
}

/// Acknowledgement of a vouch request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VouchAck {
    /// The vouch ID being acknowledged
    pub vouch_id: Uuid,
    /// Peer sending the acknowledgement
    pub from: String,
    /// Whether the vouch was accepted
    pub accepted: bool,
    /// Optional reason for rejection
    pub reason: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Reputation update notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationUpdate {
    /// Peer whose reputation changed
    pub peer_id: String,
    /// New reputation score (0.0 to 1.0)
    pub score: f64,
    /// Change from previous score
    pub delta: f64,
    /// What caused the change
    pub reason: ReputationChangeReason,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Reason for reputation change
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReputationChangeReason {
    /// Received a vouch
    VouchReceived { voucher: String, stake: f64 },
    /// Successful interaction
    SuccessfulInteraction,
    /// Failed interaction
    FailedInteraction,
    /// Vouch expired
    VouchExpired { voucher: String },
    /// Initial reputation for new peer
    Initial,
    /// Governance participation
    GovernanceParticipation,
    /// Resource contribution
    ResourceContribution,
}

// ============================================================================
// CREDIT PROTOCOL MESSAGES
// ============================================================================

/// Messages for the mutual credit protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CreditMessage {
    /// Create a new credit line
    CreateLine(CreateCreditLine),
    /// Acknowledge credit line creation
    LineAck(CreditLineAck),
    /// Transfer credits
    Transfer(CreditTransfer),
    /// Transfer acknowledgement
    TransferAck(CreditTransferAck),
    /// Credit line update notification
    LineUpdate(CreditLineUpdate),
}

/// Request to create a credit line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCreditLine {
    /// Unique credit line ID
    pub id: Uuid,
    /// Peer extending credit
    pub creditor: String,
    /// Peer receiving credit line
    pub debtor: String,
    /// Maximum credit limit
    pub limit: f64,
    /// Interest rate (0.0 = no interest)
    pub interest_rate: f64,
    /// Optional collateral description
    pub collateral: Option<String>,
    /// When the request was created
    pub timestamp: DateTime<Utc>,
}

impl CreateCreditLine {
    /// Create a new credit line request
    pub fn new(creditor: String, debtor: String, limit: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            creditor,
            debtor,
            limit,
            interest_rate: 0.0,
            collateral: None,
            timestamp: Utc::now(),
        }
    }
}

/// Acknowledgement of credit line creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditLineAck {
    /// Credit line ID
    pub line_id: Uuid,
    /// Peer sending acknowledgement
    pub from: String,
    /// Whether accepted
    pub accepted: bool,
    /// Reason for rejection
    pub reason: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Credit transfer between peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransfer {
    /// Unique transfer ID
    pub id: Uuid,
    /// Credit line ID
    pub line_id: Uuid,
    /// Peer sending credits
    pub from: String,
    /// Peer receiving credits
    pub to: String,
    /// Amount to transfer
    pub amount: f64,
    /// Optional memo
    pub memo: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl CreditTransfer {
    /// Create a new credit transfer
    pub fn new(line_id: Uuid, from: String, to: String, amount: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            line_id,
            from,
            to,
            amount,
            memo: None,
            timestamp: Utc::now(),
        }
    }

    /// Add a memo to the transfer
    pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }
}

/// Acknowledgement of credit transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransferAck {
    /// Transfer ID
    pub transfer_id: Uuid,
    /// Whether successful
    pub success: bool,
    /// New balance after transfer
    pub new_balance: Option<f64>,
    /// Error message if failed
    pub error: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Credit line update notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditLineUpdate {
    /// Credit line ID
    pub line_id: Uuid,
    /// Current balance
    pub balance: f64,
    /// Available credit
    pub available: f64,
    /// Credit limit
    pub limit: f64,
    /// Whether active
    pub active: bool,
    /// Last transaction timestamp
    pub last_transaction: DateTime<Utc>,
}

// ============================================================================
// GOVERNANCE PROTOCOL MESSAGES
// ============================================================================

/// Messages for the governance protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GovernanceMessage {
    /// Create a new proposal
    CreateProposal(CreateProposal),
    /// Cast a vote
    CastVote(CastVote),
    /// Proposal update notification
    ProposalUpdate(ProposalUpdate),
    /// Proposal executed notification
    ProposalExecuted(ProposalExecuted),
}

/// Create a new governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProposal {
    /// Unique proposal ID
    pub id: Uuid,
    /// Peer creating the proposal
    pub proposer: String,
    /// Proposal title
    pub title: String,
    /// Proposal description
    pub description: String,
    /// Type of proposal
    pub proposal_type: ProposalType,
    /// Required quorum (0.0 to 1.0)
    pub quorum: f64,
    /// Required approval threshold (0.0 to 1.0)
    pub threshold: f64,
    /// Voting deadline
    pub deadline: DateTime<Utc>,
    /// When created
    pub timestamp: DateTime<Utc>,
}

impl CreateProposal {
    /// Create a new proposal
    pub fn new(proposer: String, title: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            proposer,
            title,
            description,
            proposal_type: ProposalType::General,
            quorum: 0.5,
            threshold: 0.5,
            deadline: Utc::now() + chrono::Duration::days(7),
            timestamp: Utc::now(),
        }
    }

    /// Set proposal type
    pub fn with_type(mut self, proposal_type: ProposalType) -> Self {
        self.proposal_type = proposal_type;
        self
    }

    /// Set quorum requirement
    pub fn with_quorum(mut self, quorum: f64) -> Self {
        self.quorum = quorum.clamp(0.0, 1.0);
        self
    }

    /// Set approval threshold
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set voting deadline
    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = deadline;
        self
    }
}

/// Type of governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalType {
    /// General community proposal
    General,
    /// Change network parameters
    ParameterChange {
        parameter: String,
        old_value: String,
        new_value: String,
    },
    /// Upgrade protocol
    ProtocolUpgrade { version: String },
    /// Add/remove module
    ModuleChange { action: String, module: String },
    /// Fund allocation
    FundingRequest { amount: f64, recipient: String },
    /// Emergency action
    Emergency { action: String },
}

/// Cast a vote on a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastVote {
    /// Proposal ID
    pub proposal_id: Uuid,
    /// Voter peer ID
    pub voter: String,
    /// Vote value
    pub vote: Vote,
    /// Voting power (based on reputation)
    pub weight: f64,
    /// Optional reason
    pub reason: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl CastVote {
    /// Create a new vote
    pub fn new(proposal_id: Uuid, voter: String, vote: Vote, weight: f64) -> Self {
        Self {
            proposal_id,
            voter,
            vote,
            weight: weight.max(0.0),
            reason: None,
            timestamp: Utc::now(),
        }
    }

    /// Add a reason for the vote
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Vote value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Vote {
    /// Vote in favor
    For,
    /// Vote against
    Against,
    /// Abstain from voting
    Abstain,
}

/// Proposal status update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalUpdate {
    /// Proposal ID
    pub proposal_id: Uuid,
    /// Current status
    pub status: ProposalStatus,
    /// Total votes for
    pub votes_for: f64,
    /// Total votes against
    pub votes_against: f64,
    /// Total abstentions
    pub votes_abstain: f64,
    /// Number of unique voters
    pub voter_count: u32,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Proposal status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    /// Proposal is open for voting
    Active,
    /// Voting period ended, proposal passed
    Passed,
    /// Voting period ended, proposal rejected
    Rejected,
    /// Proposal was executed
    Executed,
    /// Proposal was cancelled
    Cancelled,
    /// Proposal expired (didn't reach quorum)
    Expired,
}

/// Proposal executed notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalExecuted {
    /// Proposal ID
    pub proposal_id: Uuid,
    /// Whether execution succeeded
    pub success: bool,
    /// Result or error message
    pub result: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

// ============================================================================
// RESOURCE PROTOCOL MESSAGES
// ============================================================================

/// Messages for the resource sharing protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResourceMessage {
    /// Report resource contribution
    Contribution(ResourceContribution),
    /// Resource metrics update
    Metrics(ResourceMetrics),
    /// Resource pool update
    PoolUpdate(ResourcePoolUpdate),
}

/// Report of resource contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContribution {
    /// Unique contribution ID
    pub id: Uuid,
    /// Contributing peer
    pub peer_id: String,
    /// Type of resource
    pub resource_type: ResourceType,
    /// Amount contributed
    pub amount: f64,
    /// Unit of measurement
    pub unit: String,
    /// Duration of contribution in seconds
    pub duration_secs: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl ResourceContribution {
    /// Create a new resource contribution report
    pub fn new(peer_id: String, resource_type: ResourceType, amount: f64, unit: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            peer_id,
            resource_type,
            amount,
            unit,
            duration_secs: 0,
            timestamp: Utc::now(),
        }
    }

    /// Set duration
    pub fn with_duration(mut self, duration_secs: u64) -> Self {
        self.duration_secs = duration_secs;
        self
    }
}

/// Type of resource
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// Network bandwidth
    Bandwidth,
    /// Storage space
    Storage,
    /// Compute cycles
    Compute,
    /// Relay/routing services
    Relay,
    /// Other resource types
    Other(String),
}

/// Resource metrics for a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// Peer ID
    pub peer_id: String,
    /// Bandwidth metrics
    pub bandwidth: BandwidthMetrics,
    /// Storage metrics
    pub storage: StorageMetrics,
    /// Compute metrics
    pub compute: ComputeMetrics,
    /// Node uptime in seconds
    pub uptime_secs: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Bandwidth metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BandwidthMetrics {
    /// Total bytes uploaded
    pub uploaded_bytes: u64,
    /// Total bytes downloaded
    pub downloaded_bytes: u64,
    /// Current upload rate (bytes/sec)
    pub upload_rate: f64,
    /// Current download rate (bytes/sec)
    pub download_rate: f64,
}

/// Storage metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageMetrics {
    /// Total storage provided (bytes)
    pub provided_bytes: u64,
    /// Storage currently used (bytes)
    pub used_bytes: u64,
    /// Available storage (bytes)
    pub available_bytes: u64,
}

/// Compute metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputeMetrics {
    /// Total tasks completed
    pub tasks_completed: u64,
    /// Average task latency (ms)
    pub average_latency_ms: f64,
    /// CPU time contributed (seconds)
    pub cpu_seconds: f64,
}

/// Resource pool update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePoolUpdate {
    /// Total bandwidth available in pool
    pub total_bandwidth: f64,
    /// Total storage available in pool
    pub total_storage: u64,
    /// Total compute available in pool
    pub total_compute: f64,
    /// Number of active contributors
    pub active_contributors: u32,
    /// Top contributors
    pub top_contributors: Vec<ContributorSummary>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Summary of a contributor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorSummary {
    /// Peer ID
    pub peer_id: String,
    /// Peer name (if known)
    pub peer_name: Option<String>,
    /// Total contribution score
    pub contribution_score: f64,
    /// Primary resource type
    pub primary_resource: ResourceType,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vouch_request_creation() {
        let vouch = VouchRequest::new("alice".to_string(), "bob".to_string(), 0.5);

        assert_eq!(vouch.voucher, "alice");
        assert_eq!(vouch.vouchee, "bob");
        assert_eq!(vouch.stake, 0.5);
        assert!(vouch.message.is_none());
    }

    #[test]
    fn test_vouch_request_with_message() {
        let vouch = VouchRequest::new("alice".to_string(), "bob".to_string(), 0.5)
            .with_message("Great collaborator!");

        assert_eq!(vouch.message, Some("Great collaborator!".to_string()));
    }

    #[test]
    fn test_stake_clamping() {
        let vouch = VouchRequest::new("alice".to_string(), "bob".to_string(), 1.5);
        assert_eq!(vouch.stake, 1.0);

        let vouch = VouchRequest::new("alice".to_string(), "bob".to_string(), -0.5);
        assert_eq!(vouch.stake, 0.0);
    }

    #[test]
    fn test_credit_line_creation() {
        let line = CreateCreditLine::new("alice".to_string(), "bob".to_string(), 100.0);

        assert_eq!(line.creditor, "alice");
        assert_eq!(line.debtor, "bob");
        assert_eq!(line.limit, 100.0);
        assert_eq!(line.interest_rate, 0.0);
    }

    #[test]
    fn test_credit_transfer() {
        let line_id = Uuid::new_v4();
        let transfer = CreditTransfer::new(line_id, "alice".to_string(), "bob".to_string(), 50.0)
            .with_memo("Payment for services");

        assert_eq!(transfer.line_id, line_id);
        assert_eq!(transfer.amount, 50.0);
        assert_eq!(transfer.memo, Some("Payment for services".to_string()));
    }

    #[test]
    fn test_proposal_creation() {
        let proposal = CreateProposal::new(
            "alice".to_string(),
            "Network Upgrade".to_string(),
            "Upgrade to v2.0".to_string(),
        )
        .with_quorum(0.6)
        .with_threshold(0.7);

        assert_eq!(proposal.proposer, "alice");
        assert_eq!(proposal.title, "Network Upgrade");
        assert_eq!(proposal.quorum, 0.6);
        assert_eq!(proposal.threshold, 0.7);
    }

    #[test]
    fn test_cast_vote() {
        let proposal_id = Uuid::new_v4();
        let vote = CastVote::new(proposal_id, "bob".to_string(), Vote::For, 0.8)
            .with_reason("I support this proposal");

        assert_eq!(vote.proposal_id, proposal_id);
        assert_eq!(vote.vote, Vote::For);
        assert_eq!(vote.weight, 0.8);
        assert!(vote.reason.is_some());
    }

    #[test]
    fn test_resource_contribution() {
        let contrib = ResourceContribution::new(
            "alice".to_string(),
            ResourceType::Bandwidth,
            1000.0,
            "Mbps".to_string(),
        )
        .with_duration(3600);

        assert_eq!(contrib.peer_id, "alice");
        assert_eq!(contrib.resource_type, ResourceType::Bandwidth);
        assert_eq!(contrib.amount, 1000.0);
        assert_eq!(contrib.duration_secs, 3600);
    }

    #[test]
    fn test_vouch_message_serialization() {
        let msg = VouchMessage::VouchRequest(VouchRequest::new(
            "alice".to_string(),
            "bob".to_string(),
            0.5,
        ));

        let json = serde_json::to_string(&msg).expect("serialization failed");
        let deserialized: VouchMessage =
            serde_json::from_str(&json).expect("deserialization failed");

        if let VouchMessage::VouchRequest(vouch) = deserialized {
            assert_eq!(vouch.voucher, "alice");
            assert_eq!(vouch.vouchee, "bob");
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_credit_message_serialization() {
        let msg = CreditMessage::CreateLine(CreateCreditLine::new(
            "alice".to_string(),
            "bob".to_string(),
            100.0,
        ));

        let json = serde_json::to_string(&msg).expect("serialization failed");
        let deserialized: CreditMessage =
            serde_json::from_str(&json).expect("deserialization failed");

        if let CreditMessage::CreateLine(line) = deserialized {
            assert_eq!(line.creditor, "alice");
            assert_eq!(line.limit, 100.0);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_governance_message_serialization() {
        let msg = GovernanceMessage::CastVote(CastVote::new(
            Uuid::new_v4(),
            "alice".to_string(),
            Vote::For,
            0.8,
        ));

        let json = serde_json::to_string(&msg).expect("serialization failed");
        let deserialized: GovernanceMessage =
            serde_json::from_str(&json).expect("deserialization failed");

        if let GovernanceMessage::CastVote(vote) = deserialized {
            assert_eq!(vote.voter, "alice");
            assert_eq!(vote.vote, Vote::For);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_resource_message_serialization() {
        let msg = ResourceMessage::Contribution(ResourceContribution::new(
            "alice".to_string(),
            ResourceType::Storage,
            1000.0,
            "GB".to_string(),
        ));

        let json = serde_json::to_string(&msg).expect("serialization failed");
        let deserialized: ResourceMessage =
            serde_json::from_str(&json).expect("deserialization failed");

        if let ResourceMessage::Contribution(contrib) = deserialized {
            assert_eq!(contrib.peer_id, "alice");
            assert_eq!(contrib.resource_type, ResourceType::Storage);
        } else {
            panic!("Wrong variant");
        }
    }
}
