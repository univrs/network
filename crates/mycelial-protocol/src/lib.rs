//! Mycelial Protocol - Message serialization and protocol definitions
//!
//! This crate handles the serialization and deserialization of network messages.
//!
//! # Economics Protocol Messages
//!
//! This crate defines messages for the Mycelial Economics system:
//!
//! - [`messages::VouchMessage`] - Reputation vouching protocol
//! - [`messages::CreditMessage`] - Mutual credit protocol
//! - [`messages::GovernanceMessage`] - Governance proposals and voting
//! - [`messages::ResourceMessage`] - Resource sharing metrics
//!
//! # Gossipsub Topics
//!
//! Use [`messages::topics`] for the topic names:
//! - `/mycelial/1.0.0/vouch` - Vouch/reputation messages
//! - `/mycelial/1.0.0/credit` - Credit transactions
//! - `/mycelial/1.0.0/governance` - Governance messages
//! - `/mycelial/1.0.0/resource` - Resource metrics

pub mod codec;
pub mod messages;

// Re-export message types for convenience
pub use messages::{
    // Topics
    topics,
    BandwidthMetrics,
    CastVote,
    ComputeMetrics,
    ContributorSummary,
    CreateCreditLine,
    CreateProposal,
    CreditLineAck,
    CreditLineUpdate,
    // Credit protocol
    CreditMessage,
    CreditTransfer,
    CreditTransferAck,
    // Governance protocol
    GovernanceMessage,
    ProposalExecuted,
    ProposalStatus,
    ProposalType,
    ProposalUpdate,
    ReputationChangeReason,
    ReputationUpdate,
    ResourceContribution,
    // Resource protocol
    ResourceMessage,
    ResourceMetrics,
    ResourcePoolUpdate,
    ResourceType,
    StorageMetrics,
    Vote,
    VouchAck,
    // Vouch protocol
    VouchMessage,
    VouchRequest,
};

use mycelial_core::{Message, MycelialError, Result};

/// Serialize a message to CBOR bytes
pub fn serialize(message: &Message) -> Result<Vec<u8>> {
    serde_cbor::to_vec(message).map_err(|e| MycelialError::Serialization(e.to_string()))
}

/// Deserialize a message from CBOR bytes
pub fn deserialize(bytes: &[u8]) -> Result<Message> {
    serde_cbor::from_slice(bytes).map_err(|e| MycelialError::Serialization(e.to_string()))
}
