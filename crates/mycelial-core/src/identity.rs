//! Identity management with Ed25519 keys and DID support
//!
//! This module provides cryptographic identity primitives for the mycelial network,
//! using the unified `univrs-identity` crate for core types.
//!
//! ## Type Re-exports
//!
//! Core identity types are re-exported from `univrs-identity`:
//! - [`Keypair`]: Ed25519 keypair for signing
//! - [`PublicKey`]: Ed25519 public key for verification
//! - [`Signature`]: Ed25519 signature
//!
//! ## Local Types
//!
//! Mycelial-specific types:
//! - [`Did`]: Decentralized Identifier (did:key method)
//! - [`Signed<T>`]: Cryptographically signed data wrapper
//! - [`SignatureBytes`]: Legacy signature format for backward compatibility

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{MycelialError, Result};

// Re-export core identity types from univrs-identity
pub use univrs_identity::{Keypair, PublicKey, Signature};

/// Legacy signature bytes format for backward compatibility.
///
/// Use [`Signature`] from univrs-identity for new code.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SignatureBytes(pub [u8; 64]);

impl SignatureBytes {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes
    pub fn to_bytes(&self) -> [u8; 64] {
        self.0
    }

    /// Encode as hex
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Decode from hex
    pub fn from_hex(s: &str) -> crate::Result<Self> {
        let bytes =
            hex::decode(s).map_err(|e| crate::MycelialError::Serialization(e.to_string()))?;
        if bytes.len() != 64 {
            return Err(crate::MycelialError::Serialization(
                "Invalid signature length".into(),
            ));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Convert from univrs-identity Signature
    pub fn from_signature(sig: &Signature) -> Self {
        Self(sig.to_bytes())
    }

    /// Convert to univrs-identity Signature
    pub fn to_signature(&self) -> Result<Signature> {
        Signature::from_bytes(&self.0).map_err(|_| MycelialError::InvalidSignature)
    }
}

impl From<Signature> for SignatureBytes {
    fn from(sig: Signature) -> Self {
        Self(sig.to_bytes())
    }
}

impl Serialize for SignatureBytes {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for SignatureBytes {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("Invalid signature length"));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl fmt::Debug for SignatureBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SignatureBytes({}...)", &self.to_hex()[..16])
    }
}

/// A Decentralized Identifier (DID) following the did:key method
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Did(String);

impl Did {
    /// The DID method used (did:key for Ed25519)
    pub const METHOD: &'static str = "key";

    /// Multicodec prefix for Ed25519 public keys
    const ED25519_MULTICODEC: [u8; 2] = [0xed, 0x01];

    /// Create a DID from a public key
    pub fn from_public_key(public_key: &PublicKey) -> Self {
        // Prepend multicodec prefix and encode with multibase (base58btc)
        let mut bytes = Vec::with_capacity(34);
        bytes.extend_from_slice(&Self::ED25519_MULTICODEC);
        bytes.extend_from_slice(public_key.as_bytes());

        let encoded = multibase::encode(multibase::Base::Base58Btc, &bytes);
        Self(format!("did:key:{}", encoded))
    }

    /// Parse a DID string
    pub fn parse(s: &str) -> Result<Self> {
        if !s.starts_with("did:key:") {
            return Err(MycelialError::Serialization(
                "Invalid DID format: must start with 'did:key:'".into(),
            ));
        }
        Ok(Self(s.to_string()))
    }

    /// Get the full DID string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract the public key from the DID
    pub fn to_public_key(&self) -> Result<PublicKey> {
        let multibase_part = self
            .0
            .strip_prefix("did:key:")
            .ok_or_else(|| MycelialError::Serialization("Invalid DID format".into()))?;

        let (_, bytes) = multibase::decode(multibase_part)
            .map_err(|e| MycelialError::Serialization(e.to_string()))?;

        if bytes.len() != 34 || bytes[0..2] != Self::ED25519_MULTICODEC {
            return Err(MycelialError::Serialization(
                "Invalid DID key format".into(),
            ));
        }

        PublicKey::from_bytes(&bytes[2..34]).map_err(|_| MycelialError::InvalidSignature)
    }
}

impl fmt::Display for Did {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&PublicKey> for Did {
    fn from(pk: &PublicKey) -> Self {
        Self::from_public_key(pk)
    }
}

/// Extension trait for PublicKey to add DID support
pub trait PublicKeyExt {
    /// Convert to a DID (Decentralized Identifier)
    fn to_did(&self) -> Did;

    /// Verify a signature using SignatureBytes format
    fn verify_bytes(&self, message: &[u8], signature: &SignatureBytes) -> Result<()>;
}

impl PublicKeyExt for PublicKey {
    fn to_did(&self) -> Did {
        Did::from_public_key(self)
    }

    fn verify_bytes(&self, message: &[u8], signature: &SignatureBytes) -> Result<()> {
        let sig = signature.to_signature()?;
        if self.verify(message, &sig) {
            Ok(())
        } else {
            Err(MycelialError::InvalidSignature)
        }
    }
}

/// Extension trait for Keypair to add DID and SignatureBytes support
pub trait KeypairExt {
    /// Create the DID for this keypair
    fn did(&self) -> Did;

    /// Sign a message and return SignatureBytes
    fn sign_bytes(&self, message: &[u8]) -> SignatureBytes;
}

impl KeypairExt for Keypair {
    fn did(&self) -> Did {
        self.public_key().to_did()
    }

    fn sign_bytes(&self, message: &[u8]) -> SignatureBytes {
        SignatureBytes::from(self.sign(message))
    }
}

/// A signed piece of data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signed<T> {
    /// The payload data
    pub data: T,
    /// The signer's public key
    pub signer: PublicKey,
    /// The signature over the serialized data
    pub signature: SignatureBytes,
}

impl<T: Serialize> Signed<T> {
    /// Create a new signed value
    pub fn new(data: T, keypair: &Keypair) -> Result<Self> {
        let bytes =
            serde_cbor::to_vec(&data).map_err(|e| MycelialError::Serialization(e.to_string()))?;
        let signature = keypair.sign_bytes(&bytes);

        Ok(Self {
            data,
            signer: keypair.public_key(),
            signature,
        })
    }

    /// Verify the signature
    pub fn verify(&self) -> Result<()> {
        let bytes = serde_cbor::to_vec(&self.data)
            .map_err(|e| MycelialError::Serialization(e.to_string()))?;
        self.signer.verify_bytes(&bytes, &self.signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = Keypair::generate();
        let pk = kp.public_key();

        let message = b"Hello, Mycelial Network!";
        let sig = kp.sign(message);

        assert!(pk.verify(message, &sig));
        assert!(!pk.verify(b"Wrong message", &sig));
    }

    #[test]
    fn test_keypair_sign_bytes() {
        let kp = Keypair::generate();
        let pk = kp.public_key();

        let message = b"Hello, Mycelial Network!";
        let sig = kp.sign_bytes(message);

        assert!(pk.verify_bytes(message, &sig).is_ok());
        assert!(pk.verify_bytes(b"Wrong message", &sig).is_err());
    }

    #[test]
    fn test_did_roundtrip() {
        let kp = Keypair::generate();
        let did = kp.did();

        let recovered_pk = did.to_public_key().unwrap();
        assert_eq!(kp.public_key().as_bytes(), recovered_pk.as_bytes());
    }

    #[test]
    fn test_signed_data() {
        let kp = Keypair::generate();
        let data = "Important message".to_string();

        let signed = Signed::new(data.clone(), &kp).unwrap();
        assert!(signed.verify().is_ok());
        assert_eq!(signed.data, data);
    }

    #[test]
    fn test_public_key_serialization() {
        let kp = Keypair::generate();
        let pk = kp.public_key();

        let base58 = pk.to_base58();
        let recovered = PublicKey::from_base58(&base58).unwrap();

        assert_eq!(pk.as_bytes(), recovered.as_bytes());
    }

    #[test]
    fn test_signature_bytes_conversion() {
        let kp = Keypair::generate();
        let message = b"test message";

        // Sign with univrs-identity Signature
        let sig = kp.sign(message);

        // Convert to SignatureBytes
        let sig_bytes = SignatureBytes::from(sig);

        // Convert back to Signature
        let sig_restored = sig_bytes.to_signature().unwrap();

        // Both should verify
        assert!(kp.public_key().verify(message, &sig_restored));
    }
}
