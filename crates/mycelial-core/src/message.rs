//! Message types for peer-to-peer communication

use crate::peer::PeerId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A message in the mycelial network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message identifier
    pub id: Uuid,
    /// Message type/topic
    pub message_type: MessageType,
    /// Sender peer ID
    pub sender: PeerId,
    /// Optional specific recipient (None = broadcast)
    pub recipient: Option<PeerId>,
    /// Message payload
    pub payload: Vec<u8>,
    /// When the message was created
    pub timestamp: DateTime<Utc>,
    /// Signature of the message
    pub signature: Option<Vec<u8>>,
}

/// Types of messages in the network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    /// Peer discovery and announcement
    Discovery,
    /// Content sharing (posts, media)
    Content,
    /// Reputation updates
    Reputation,
    /// Credit/economic transactions
    Credit,
    /// Governance proposals and votes
    Governance,
    /// Direct peer-to-peer message
    Direct,
    /// System/protocol messages
    System,
}

impl Message {
    /// Create a new message
    pub fn new(message_type: MessageType, sender: PeerId, payload: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type,
            sender,
            recipient: None,
            payload,
            timestamp: Utc::now(),
            signature: None,
        }
    }

    /// Create a direct message to a specific peer
    pub fn direct(sender: PeerId, recipient: PeerId, payload: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: MessageType::Direct,
            sender,
            recipient: Some(recipient),
            payload,
            timestamp: Utc::now(),
            signature: None,
        }
    }

    /// Check if message is expired (older than max_age seconds)
    pub fn is_expired(&self, max_age_secs: i64) -> bool {
        let age = Utc::now().signed_duration_since(self.timestamp);
        age.num_seconds() > max_age_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let sender = PeerId("sender".to_string());
        let msg = Message::new(
            MessageType::Content,
            sender.clone(),
            b"Hello, world!".to_vec(),
        );

        assert_eq!(msg.sender, sender);
        assert_eq!(msg.message_type, MessageType::Content);
        assert!(msg.recipient.is_none());
    }
}
