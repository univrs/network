//! Peer identity and information types
//!
//! This module uses the unified `univrs-identity` crate for cryptographic
//! identity, providing consistent identity handling across the Univrs ecosystem.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Use identity types from our identity module (which re-exports from univrs-identity)
use crate::identity::{Keypair, PublicKey};

/// Unique identifier for a peer in the network.
///
/// This is a base58-encoded Ed25519 public key, providing a human-readable
/// identifier that can be converted back to a `PublicKey` for verification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub String);

impl PeerId {
    /// Create a new peer ID from a public key
    pub fn from_public_key(key: &PublicKey) -> Self {
        Self(key.to_base58())
    }

    /// Get the peer ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get a short form of the peer ID (first 8 characters)
    pub fn short(&self) -> &str {
        &self.0[..8.min(self.0.len())]
    }

    /// Try to convert back to a public key
    pub fn to_public_key(&self) -> crate::Result<PublicKey> {
        PublicKey::from_base58(&self.0).map_err(|_| crate::MycelialError::InvalidSignature)
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&PublicKey> for PeerId {
    fn from(key: &PublicKey) -> Self {
        Self::from_public_key(key)
    }
}

impl From<PublicKey> for PeerId {
    fn from(key: PublicKey) -> Self {
        Self::from_public_key(&key)
    }
}

/// Information about a peer in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Unique peer identifier
    pub id: PeerId,
    /// Public key for verification (base58 encoded)
    pub public_key: String,
    /// Network addresses
    pub addresses: Vec<String>,
    /// When this peer was first seen
    pub first_seen: DateTime<Utc>,
    /// When this peer was last seen
    pub last_seen: DateTime<Utc>,
    /// Optional human-readable name
    pub name: Option<String>,
}

impl PeerInfo {
    /// Create new peer info from a keypair
    pub fn new(keypair: &Keypair, addresses: Vec<String>) -> Self {
        let public_key = keypair.public_key();
        let id = PeerId::from_public_key(&public_key);
        let now = Utc::now();

        Self {
            id,
            public_key: public_key.to_base58(),
            addresses,
            first_seen: now,
            last_seen: now,
            name: None,
        }
    }

    /// Create new peer info from a public key
    pub fn from_public_key(public_key: &PublicKey, addresses: Vec<String>) -> Self {
        let id = PeerId::from_public_key(public_key);
        let now = Utc::now();

        Self {
            id,
            public_key: public_key.to_base58(),
            addresses,
            first_seen: now,
            last_seen: now,
            name: None,
        }
    }

    /// Generate a new peer with a random keypair
    pub fn generate(addresses: Vec<String>) -> (Self, Keypair) {
        let keypair = Keypair::generate();
        let info = Self::new(&keypair, addresses);
        (info, keypair)
    }

    /// Get the public key for verification
    pub fn get_public_key(&self) -> crate::Result<PublicKey> {
        PublicKey::from_base58(&self.public_key).map_err(|_| crate::MycelialError::InvalidSignature)
    }

    /// Update the last_seen timestamp
    pub fn touch(&mut self) {
        self.last_seen = Utc::now();
    }

    /// Set a human-readable name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_id_from_public_key() {
        let keypair = Keypair::generate();
        let peer_id = PeerId::from_public_key(&keypair.public_key());

        // Should be able to recover the public key
        let recovered = peer_id.to_public_key().unwrap();
        assert_eq!(keypair.public_key().as_bytes(), recovered.as_bytes());
    }

    #[test]
    fn test_peer_id_short() {
        let keypair = Keypair::generate();
        let peer_id = PeerId::from(&keypair.public_key());

        let short = peer_id.short();
        assert_eq!(short.len(), 8);
        assert!(peer_id.as_str().starts_with(short));
    }

    #[test]
    fn test_peer_info_generation() {
        let (info, keypair) = PeerInfo::generate(vec!["127.0.0.1:9000".to_string()]);

        assert_eq!(info.addresses.len(), 1);
        assert!(info.name.is_none());

        // Should match the keypair
        let recovered = info.get_public_key().unwrap();
        assert_eq!(keypair.public_key().as_bytes(), recovered.as_bytes());
    }

    #[test]
    fn test_peer_info_with_name() {
        let (info, _) = PeerInfo::generate(vec![]);
        let info = info.with_name("TestNode");

        assert_eq!(info.name, Some("TestNode".to_string()));
    }

    #[test]
    fn test_peer_info_from_keypair() {
        let keypair = Keypair::generate();
        let info = PeerInfo::new(&keypair, vec!["192.168.1.1:8080".to_string()]);

        assert_eq!(info.public_key, keypair.public_key().to_base58());
        assert_eq!(info.id.as_str(), keypair.public_key().to_base58());
    }
}
