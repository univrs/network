//! Identity management with Ed25519 keys and DID support
//!
//! This module provides cryptographic identity primitives for the mycelial network,
//! including key generation, signing, verification, and DID (Decentralized Identifier) support.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{MycelialError, Result};

/// A cryptographic keypair for signing and verification
#[derive(Clone)]
pub struct Keypair {
    /// The secret signing key
    signing_key: SigningKey,
}

impl Keypair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Create a keypair from a 32-byte seed
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        Self { signing_key }
    }

    /// Get the public verifying key
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.signing_key.verifying_key())
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> SignatureBytes {
        let sig = self.signing_key.sign(message);
        SignatureBytes(sig.to_bytes())
    }

    /// Get the secret key bytes (use with caution!)
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Create the DID for this keypair
    pub fn did(&self) -> Did {
        self.public_key().to_did()
    }
}

impl fmt::Debug for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Keypair")
            .field("public_key", &self.public_key())
            .finish_non_exhaustive()
    }
}

/// A public key for verification
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PublicKey(VerifyingKey);

impl PublicKey {
    /// Create from raw bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        let key = VerifyingKey::from_bytes(bytes)
            .map_err(|_| MycelialError::InvalidSignature)?;
        Ok(Self(key))
    }

    /// Get the raw bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &SignatureBytes) -> Result<()> {
        let sig = Signature::from_bytes(&signature.0);
        self.0.verify(message, &sig)
            .map_err(|_| MycelialError::InvalidSignature)
    }

    /// Convert to a DID (Decentralized Identifier)
    pub fn to_did(&self) -> Did {
        Did::from_public_key(self)
    }

    /// Encode as base58
    pub fn to_base58(&self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }

    /// Decode from base58
    pub fn from_base58(s: &str) -> Result<Self> {
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|e| MycelialError::Serialization(e.to_string()))?;

        if bytes.len() != 32 {
            return Err(MycelialError::Serialization("Invalid public key length".into()));
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Self::from_bytes(&arr)
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({})", self.to_base58())
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base58())
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.to_bytes())
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom("Invalid public key length"));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Self::from_bytes(&arr).map_err(serde::de::Error::custom)
    }
}

/// A signature as raw bytes
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
        let bytes = hex::decode(s)
            .map_err(|e| crate::MycelialError::Serialization(e.to_string()))?;
        if bytes.len() != 64 {
            return Err(crate::MycelialError::Serialization("Invalid signature length".into()));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
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
        write!(f, "Signature({}...)", &self.to_hex()[..16])
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
        bytes.extend_from_slice(&public_key.to_bytes());

        let encoded = multibase::encode(multibase::Base::Base58Btc, &bytes);
        Self(format!("did:key:{}", encoded))
    }

    /// Parse a DID string
    pub fn parse(s: &str) -> Result<Self> {
        if !s.starts_with("did:key:") {
            return Err(MycelialError::Serialization(
                "Invalid DID format: must start with 'did:key:'".into()
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
        let multibase_part = self.0.strip_prefix("did:key:")
            .ok_or_else(|| MycelialError::Serialization("Invalid DID format".into()))?;

        let (_, bytes) = multibase::decode(multibase_part)
            .map_err(|e| MycelialError::Serialization(e.to_string()))?;

        if bytes.len() != 34 || bytes[0..2] != Self::ED25519_MULTICODEC {
            return Err(MycelialError::Serialization("Invalid DID key format".into()));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes[2..34]);
        PublicKey::from_bytes(&key_bytes)
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
        let bytes = serde_cbor::to_vec(&data)
            .map_err(|e| MycelialError::Serialization(e.to_string()))?;
        let signature = keypair.sign(&bytes);

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
        self.signer.verify(&bytes, &self.signature)
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

        assert!(pk.verify(message, &sig).is_ok());
        assert!(pk.verify(b"Wrong message", &sig).is_err());
    }

    #[test]
    fn test_did_roundtrip() {
        let kp = Keypair::generate();
        let did = kp.did();

        let recovered_pk = did.to_public_key().unwrap();
        assert_eq!(kp.public_key().to_bytes(), recovered_pk.to_bytes());
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

        assert_eq!(pk.to_bytes(), recovered.to_bytes());
    }
}
