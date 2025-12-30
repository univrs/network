//! ENR Message Types for P2P Transport
//!
//! Defines the message envelope and types exchanged over gossipsub
//! for gradient broadcasting and credit synchronization.

use serde::{Deserialize, Serialize};
use univrs_enr::{
    core::{Credits, CreditTransfer, NodeId, Timestamp},
    nexus::ResourceGradient,
};

/// Gossipsub topic for gradient updates
pub const GRADIENT_TOPIC: &str = "/vudo/enr/gradient/1.0.0";

/// Gossipsub topic for credit operations
pub const CREDIT_TOPIC: &str = "/vudo/enr/credits/1.0.0";

/// Envelope for all ENR messages over gossip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnrMessage {
    /// Resource gradient broadcast from a node
    GradientUpdate(GradientUpdate),
    /// Credit transfer announcement
    CreditTransfer(CreditTransferMsg),
    /// Balance query request (for verification)
    BalanceQuery(BalanceQueryMsg),
    /// Balance query response
    BalanceResponse(BalanceResponseMsg),
}

/// Gradient update broadcast by a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientUpdate {
    /// Source node publishing the gradient
    pub source: NodeId,
    /// Current resource availability
    pub gradient: ResourceGradient,
    /// When this gradient was measured
    pub timestamp: Timestamp,
    /// Ed25519 signature over (source || gradient || timestamp)
    pub signature: Vec<u8>,
}

/// Credit transfer announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransferMsg {
    /// The transfer details
    pub transfer: CreditTransfer,
    /// Unique nonce to prevent replay
    pub nonce: u64,
    /// Ed25519 signature from sender
    pub signature: Vec<u8>,
}

/// Balance query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceQueryMsg {
    /// Node making the request
    pub requester: NodeId,
    /// Node being queried
    pub target: NodeId,
    /// Unique request ID for correlation
    pub request_id: u64,
}

/// Balance query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponseMsg {
    /// Correlates to BalanceQueryMsg.request_id
    pub request_id: u64,
    /// Current balance of the queried account
    pub balance: Credits,
    /// Timestamp of the balance snapshot
    pub as_of: Timestamp,
}

impl EnrMessage {
    /// Serialize message to CBOR bytes
    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        serde_cbor::to_vec(self).map_err(EncodeError::Cbor)
    }

    /// Deserialize message from CBOR bytes
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        serde_cbor::from_slice(bytes).map_err(DecodeError::Cbor)
    }

    /// Get the topic this message should be published to
    pub fn topic(&self) -> &'static str {
        match self {
            EnrMessage::GradientUpdate(_) => GRADIENT_TOPIC,
            EnrMessage::CreditTransfer(_) |
            EnrMessage::BalanceQuery(_) |
            EnrMessage::BalanceResponse(_) => CREDIT_TOPIC,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("CBOR encoding error: {0}")]
    Cbor(#[from] serde_cbor::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("CBOR decoding error: {0}")]
    Cbor(#[from] serde_cbor::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_roundtrip() {
        let node = NodeId::from_bytes([1u8; 32]);
        let msg = EnrMessage::GradientUpdate(GradientUpdate {
            source: node,
            gradient: ResourceGradient::zero(),
            timestamp: Timestamp::now(),
            signature: vec![],
        });

        let bytes = msg.encode().unwrap();
        let decoded = EnrMessage::decode(&bytes).unwrap();

        match decoded {
            EnrMessage::GradientUpdate(update) => {
                assert_eq!(update.source, node);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_topic_routing() {
        let node = NodeId::from_bytes([1u8; 32]);
        
        let gradient_msg = EnrMessage::GradientUpdate(GradientUpdate {
            source: node,
            gradient: ResourceGradient::zero(),
            timestamp: Timestamp::now(),
            signature: vec![],
        });
        assert_eq!(gradient_msg.topic(), GRADIENT_TOPIC);

        let balance_msg = EnrMessage::BalanceQuery(BalanceQueryMsg {
            requester: node,
            target: node,
            request_id: 1,
        });
        assert_eq!(balance_msg.topic(), CREDIT_TOPIC);
    }
}
