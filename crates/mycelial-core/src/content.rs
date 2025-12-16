//! Content-addressed storage types using Blake3 hashing
//!
//! This module provides types for content-addressed data, where content is
//! identified by its cryptographic hash rather than location.

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{MycelialError, Result};

/// A content identifier (CID) based on Blake3 hash
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentId([u8; 32]);

impl ContentId {
    /// Create a content ID from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Compute the content ID for some data
    pub fn hash(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self(*hash.as_bytes())
    }

    /// Get the raw bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Encode as hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Decode from hex string
    pub fn from_hex(s: &str) -> Result<Self> {
        let bytes = hex::decode(s)
            .map_err(|e| MycelialError::Serialization(e.to_string()))?;

        if bytes.len() != 32 {
            return Err(MycelialError::Serialization("Invalid content ID length".into()));
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Encode as base58
    pub fn to_base58(&self) -> String {
        bs58::encode(self.0).into_string()
    }

    /// Decode from base58
    pub fn from_base58(s: &str) -> Result<Self> {
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|e| MycelialError::Serialization(e.to_string()))?;

        if bytes.len() != 32 {
            return Err(MycelialError::Serialization("Invalid content ID length".into()));
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Verify that data matches this content ID
    pub fn verify(&self, data: &[u8]) -> bool {
        Self::hash(data) == *self
    }
}

impl fmt::Debug for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentId({})", &self.to_hex()[..16])
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base58())
    }
}

/// A piece of content with its hash
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// The content identifier
    pub id: ContentId,
    /// The raw content data
    pub data: Vec<u8>,
    /// Content type (MIME type)
    pub content_type: String,
    /// Optional metadata
    pub metadata: ContentMetadata,
}

impl Content {
    /// Create new content from raw data
    pub fn new(data: Vec<u8>, content_type: impl Into<String>) -> Self {
        let id = ContentId::hash(&data);
        Self {
            id,
            data,
            content_type: content_type.into(),
            metadata: ContentMetadata::default(),
        }
    }

    /// Create text content
    pub fn text(text: impl Into<String>) -> Self {
        let text = text.into();
        Self::new(text.into_bytes(), "text/plain")
    }

    /// Create JSON content
    pub fn json<T: Serialize>(value: &T) -> Result<Self> {
        let json = serde_json::to_vec(value)
            .map_err(|e| MycelialError::Serialization(e.to_string()))?;
        Ok(Self::new(json, "application/json"))
    }

    /// Verify content integrity
    pub fn verify(&self) -> bool {
        self.id.verify(&self.data)
    }

    /// Get content as UTF-8 string (if valid)
    pub fn as_text(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }

    /// Parse content as JSON
    pub fn parse_json<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        serde_json::from_slice(&self.data)
            .map_err(|e| MycelialError::Serialization(e.to_string()))
    }
}

/// Metadata associated with content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentMetadata {
    /// Human-readable name
    pub name: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Size in bytes
    pub size: Option<u64>,
    /// Creation timestamp
    pub created: Option<chrono::DateTime<chrono::Utc>>,
}

impl ContentMetadata {
    /// Create new metadata with a name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Default::default()
        }
    }
}

/// A Merkle tree node for content chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    /// Hash of this node
    pub hash: ContentId,
    /// Left child hash (if internal node)
    pub left: Option<ContentId>,
    /// Right child hash (if internal node)
    pub right: Option<ContentId>,
    /// Data (if leaf node)
    pub data: Option<Vec<u8>>,
}

impl MerkleNode {
    /// Create a leaf node
    pub fn leaf(data: Vec<u8>) -> Self {
        let hash = ContentId::hash(&data);
        Self {
            hash,
            left: None,
            right: None,
            data: Some(data),
        }
    }

    /// Create an internal node from two children
    pub fn internal(left: ContentId, right: ContentId) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(&left.to_bytes());
        hasher.update(&right.to_bytes());
        let hash = ContentId::from_bytes(*hasher.finalize().as_bytes());

        Self {
            hash,
            left: Some(left),
            right: Some(right),
            data: None,
        }
    }

    /// Check if this is a leaf node
    pub fn is_leaf(&self) -> bool {
        self.data.is_some()
    }
}

/// Builder for creating Merkle trees from chunked data
pub struct MerkleTreeBuilder {
    chunk_size: usize,
    leaves: Vec<MerkleNode>,
}

impl MerkleTreeBuilder {
    /// Create a new builder with the given chunk size
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            leaves: Vec::new(),
        }
    }

    /// Add data to the tree (will be chunked automatically)
    pub fn add_data(&mut self, data: &[u8]) {
        for chunk in data.chunks(self.chunk_size) {
            self.leaves.push(MerkleNode::leaf(chunk.to_vec()));
        }
    }

    /// Build the Merkle tree and return the root hash
    pub fn build(self) -> Option<ContentId> {
        if self.leaves.is_empty() {
            return None;
        }

        let mut current_level: Vec<ContentId> = self.leaves.iter().map(|n| n.hash).collect();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for pair in current_level.chunks(2) {
                match pair {
                    [left, right] => {
                        let node = MerkleNode::internal(*left, *right);
                        next_level.push(node.hash);
                    }
                    [single] => {
                        // Odd node: promote to next level
                        next_level.push(*single);
                    }
                    _ => unreachable!(),
                }
            }

            current_level = next_level;
        }

        current_level.into_iter().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_id_hash() {
        let data = b"Hello, World!";
        let id1 = ContentId::hash(data);
        let id2 = ContentId::hash(data);

        assert_eq!(id1, id2);
        assert!(id1.verify(data));
        assert!(!id1.verify(b"Different data"));
    }

    #[test]
    fn test_content_id_encoding() {
        let data = b"Test data";
        let id = ContentId::hash(data);

        // Hex roundtrip
        let hex = id.to_hex();
        let recovered = ContentId::from_hex(&hex).unwrap();
        assert_eq!(id, recovered);

        // Base58 roundtrip
        let base58 = id.to_base58();
        let recovered = ContentId::from_base58(&base58).unwrap();
        assert_eq!(id, recovered);
    }

    #[test]
    fn test_content_creation() {
        let content = Content::text("Hello, Mycelial!");
        assert!(content.verify());
        assert_eq!(content.as_text(), Some("Hello, Mycelial!"));
    }

    #[test]
    fn test_merkle_tree() {
        let mut builder = MerkleTreeBuilder::new(64);
        builder.add_data(b"This is some test data that will be chunked into multiple pieces for the Merkle tree.");

        let root = builder.build();
        assert!(root.is_some());
    }
}
