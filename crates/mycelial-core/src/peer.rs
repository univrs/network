//! Peer identity and information types

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Unique identifier for a peer in the network
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub String);

impl PeerId {
    /// Create a new peer ID from a public key
    pub fn from_public_key(key: &VerifyingKey) -> Self {
        let bytes = key.to_bytes();
        Self(bs58::encode(bytes).into_string())
    }

    /// Get the peer ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Information about a peer in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Unique peer identifier
    pub id: PeerId,
    /// Public key for verification
    pub public_key: Vec<u8>,
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
    /// Create new peer info from a signing key
    pub fn new(signing_key: &SigningKey, addresses: Vec<String>) -> Self {
        let verifying_key = signing_key.verifying_key();
        let id = PeerId::from_public_key(&verifying_key);
        let now = Utc::now();

        Self {
            id,
            public_key: verifying_key.to_bytes().to_vec(),
            addresses,
            first_seen: now,
            last_seen: now,
            name: None,
        }
    }

    /// Generate a new peer with a random keypair
    pub fn generate(addresses: Vec<String>) -> (Self, SigningKey) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let info = Self::new(&signing_key, addresses);
        (info, signing_key)
    }
}
