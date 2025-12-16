//! Mycelial Protocol - Message serialization and protocol definitions
//!
//! This crate handles the serialization and deserialization of network messages.

pub mod codec;
pub mod messages;

use mycelial_core::{Message, MycelialError, Result};

/// Serialize a message to CBOR bytes
pub fn serialize(message: &Message) -> Result<Vec<u8>> {
    serde_cbor::to_vec(message)
        .map_err(|e| MycelialError::Serialization(e.to_string()))
}

/// Deserialize a message from CBOR bytes
pub fn deserialize(bytes: &[u8]) -> Result<Message> {
    serde_cbor::from_slice(bytes)
        .map_err(|e| MycelialError::Serialization(e.to_string()))
}
