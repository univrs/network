//! In-memory caching layer
//!
//! This module provides LRU caching for frequently accessed data like
//! peer information, messages, and credit relationships.

use lru::LruCache;
use mycelial_core::{
    credit::CreditRelationship, message::Message, peer::PeerInfo, reputation::Reputation,
};
use parking_lot::RwLock;
use std::num::NonZeroUsize;
use uuid::Uuid;

/// Generic LRU cache for frequently accessed data
pub struct MemoryCache<K, V> {
    cache: RwLock<LruCache<K, V>>,
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> MemoryCache<K, V> {
    /// Create a new cache with the given capacity
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
        Self {
            cache: RwLock::new(LruCache::new(cap)),
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Option<V> {
        self.cache.write().get(key).cloned()
    }

    /// Peek a value without updating LRU order
    pub fn peek(&self, key: &K) -> Option<V> {
        self.cache.read().peek(key).cloned()
    }

    /// Insert a value into the cache
    pub fn insert(&self, key: K, value: V) {
        self.cache.write().put(key, value);
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &K) -> Option<V> {
        self.cache.write().pop(key)
    }

    /// Check if key exists
    pub fn contains(&self, key: &K) -> bool {
        self.cache.read().contains(key)
    }

    /// Get current cache size
    pub fn len(&self) -> usize {
        self.cache.read().len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.read().is_empty()
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.cache.write().clear();
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<K> {
        self.cache.read().iter().map(|(k, _)| k.clone()).collect()
    }
}

/// Specialized cache for peer information with reputation
pub struct PeerCache {
    peers: MemoryCache<String, (PeerInfo, Reputation)>,
}

impl PeerCache {
    /// Create a new peer cache with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            peers: MemoryCache::new(capacity),
        }
    }

    /// Get peer info and reputation
    pub fn get(&self, peer_id: &str) -> Option<(PeerInfo, Reputation)> {
        self.peers.get(&peer_id.to_string())
    }

    /// Get only peer info
    pub fn get_peer_info(&self, peer_id: &str) -> Option<PeerInfo> {
        self.peers.get(&peer_id.to_string()).map(|(info, _)| info)
    }

    /// Get only reputation
    pub fn get_reputation(&self, peer_id: &str) -> Option<Reputation> {
        self.peers.get(&peer_id.to_string()).map(|(_, rep)| rep)
    }

    /// Insert or update peer
    pub fn insert(&self, peer_info: PeerInfo, reputation: Reputation) {
        self.peers
            .insert(peer_info.id.as_str().to_string(), (peer_info, reputation));
    }

    /// Update reputation for existing peer
    pub fn update_reputation(&self, peer_id: &str, reputation: Reputation) -> bool {
        if let Some((info, _)) = self.peers.get(&peer_id.to_string()) {
            self.peers.insert(peer_id.to_string(), (info, reputation));
            true
        } else {
            false
        }
    }

    /// Remove peer from cache
    pub fn remove(&self, peer_id: &str) -> Option<(PeerInfo, Reputation)> {
        self.peers.remove(&peer_id.to_string())
    }

    /// Check if peer is cached
    pub fn contains(&self, peer_id: &str) -> bool {
        self.peers.contains(&peer_id.to_string())
    }

    /// Get all cached peer IDs
    pub fn peer_ids(&self) -> Vec<String> {
        self.peers.keys()
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    /// Clear all cached peers
    pub fn clear(&self) {
        self.peers.clear();
    }

    /// Get trusted peers (reputation >= threshold)
    pub fn get_trusted(&self, threshold: f64) -> Vec<(PeerInfo, Reputation)> {
        let cache = self.peers.cache.read();
        cache
            .iter()
            .filter(|(_, (_, rep))| rep.score >= threshold)
            .map(|(_, v)| v.clone())
            .collect()
    }
}

impl Default for PeerCache {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Specialized cache for messages
pub struct MessageCache {
    messages: MemoryCache<String, Message>,
    /// Index of messages by sender
    by_sender: RwLock<std::collections::HashMap<String, Vec<String>>>,
}

impl MessageCache {
    /// Create a new message cache with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            messages: MemoryCache::new(capacity),
            by_sender: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Get a message by ID
    pub fn get(&self, id: &Uuid) -> Option<Message> {
        self.messages.get(&id.to_string())
    }

    /// Insert a message
    pub fn insert(&self, message: Message) {
        let id = message.id.to_string();
        let sender = message.sender.as_str().to_string();

        self.messages.insert(id.clone(), message);

        // Update sender index
        let mut by_sender = self.by_sender.write();
        by_sender.entry(sender).or_insert_with(Vec::new).push(id);
    }

    /// Remove a message
    pub fn remove(&self, id: &Uuid) -> Option<Message> {
        if let Some(msg) = self.messages.remove(&id.to_string()) {
            // Update sender index
            let mut by_sender = self.by_sender.write();
            if let Some(ids) = by_sender.get_mut(msg.sender.as_str()) {
                ids.retain(|i| i != &id.to_string());
            }
            Some(msg)
        } else {
            None
        }
    }

    /// Get messages from a sender
    pub fn get_from_sender(&self, sender_id: &str) -> Vec<Message> {
        let by_sender = self.by_sender.read();
        if let Some(ids) = by_sender.get(sender_id) {
            ids.iter()
                .filter_map(|id| self.messages.peek(&id.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if message exists
    pub fn contains(&self, id: &Uuid) -> bool {
        self.messages.contains(&id.to_string())
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Clear all cached messages
    pub fn clear(&self) {
        self.messages.clear();
        self.by_sender.write().clear();
    }
}

impl Default for MessageCache {
    fn default() -> Self {
        Self::new(5000)
    }
}

/// Specialized cache for credit relationships
pub struct CreditCache {
    relationships: MemoryCache<String, CreditRelationship>,
    /// Index by peer (both creditor and debtor)
    by_peer: RwLock<std::collections::HashMap<String, Vec<String>>>,
}

impl CreditCache {
    /// Create a new credit cache with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            relationships: MemoryCache::new(capacity),
            by_peer: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Generate relationship ID from peers
    fn relationship_id(creditor: &str, debtor: &str) -> String {
        format!("{}_{}", creditor, debtor)
    }

    /// Get a relationship by ID
    pub fn get(&self, id: &str) -> Option<CreditRelationship> {
        self.relationships.get(&id.to_string())
    }

    /// Get a relationship between two peers
    pub fn get_between(&self, creditor: &str, debtor: &str) -> Option<CreditRelationship> {
        let id = Self::relationship_id(creditor, debtor);
        self.relationships.get(&id)
    }

    /// Insert a relationship
    pub fn insert(&self, relationship: CreditRelationship) {
        let id =
            Self::relationship_id(relationship.creditor.as_str(), relationship.debtor.as_str());
        let creditor = relationship.creditor.as_str().to_string();
        let debtor = relationship.debtor.as_str().to_string();

        self.relationships.insert(id.clone(), relationship);

        // Update peer index
        let mut by_peer = self.by_peer.write();
        by_peer
            .entry(creditor)
            .or_insert_with(Vec::new)
            .push(id.clone());
        by_peer.entry(debtor).or_insert_with(Vec::new).push(id);
    }

    /// Remove a relationship
    pub fn remove(&self, id: &str) -> Option<CreditRelationship> {
        if let Some(rel) = self.relationships.remove(&id.to_string()) {
            // Update peer index
            let mut by_peer = self.by_peer.write();
            if let Some(ids) = by_peer.get_mut(rel.creditor.as_str()) {
                ids.retain(|i| i != id);
            }
            if let Some(ids) = by_peer.get_mut(rel.debtor.as_str()) {
                ids.retain(|i| i != id);
            }
            Some(rel)
        } else {
            None
        }
    }

    /// Get all relationships for a peer
    pub fn get_for_peer(&self, peer_id: &str) -> Vec<CreditRelationship> {
        let by_peer = self.by_peer.read();
        if let Some(ids) = by_peer.get(peer_id) {
            ids.iter()
                .filter_map(|id| self.relationships.peek(&id.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get active relationships
    pub fn get_active(&self) -> Vec<CreditRelationship> {
        let cache = self.relationships.cache.read();
        cache
            .iter()
            .filter(|(_, rel)| rel.active)
            .map(|(_, rel)| rel.clone())
            .collect()
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.relationships.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.relationships.is_empty()
    }

    /// Clear all cached relationships
    pub fn clear(&self) {
        self.relationships.clear();
        self.by_peer.write().clear();
    }
}

impl Default for CreditCache {
    fn default() -> Self {
        Self::new(500)
    }
}

/// Combined state cache for all frequently accessed data
pub struct StateCache {
    /// Peer cache
    pub peers: PeerCache,
    /// Message cache
    pub messages: MessageCache,
    /// Credit relationship cache
    pub credits: CreditCache,
}

impl StateCache {
    /// Create a new state cache with default capacities
    pub fn new() -> Self {
        Self {
            peers: PeerCache::new(1000),
            messages: MessageCache::new(5000),
            credits: CreditCache::new(500),
        }
    }

    /// Create a state cache with custom capacities
    pub fn with_capacities(peer_cap: usize, msg_cap: usize, credit_cap: usize) -> Self {
        Self {
            peers: PeerCache::new(peer_cap),
            messages: MessageCache::new(msg_cap),
            credits: CreditCache::new(credit_cap),
        }
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.peers.clear();
        self.messages.clear();
        self.credits.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            peer_count: self.peers.len(),
            message_count: self.messages.len(),
            credit_count: self.credits.len(),
        }
    }
}

impl Default for StateCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about cache usage
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub peer_count: usize,
    pub message_count: usize,
    pub credit_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use mycelial_core::message::MessageType;
    use mycelial_core::peer::PeerId;

    #[test]
    fn test_memory_cache() {
        let cache: MemoryCache<String, i32> = MemoryCache::new(10);

        cache.insert("key1".to_string(), 42);
        assert_eq!(cache.get(&"key1".to_string()), Some(42));
        assert!(cache.contains(&"key1".to_string()));

        cache.remove(&"key1".to_string());
        assert!(!cache.contains(&"key1".to_string()));
    }

    #[test]
    fn test_peer_cache() {
        let cache = PeerCache::new(10);

        let peer_info = PeerInfo {
            id: PeerId("peer1".to_string()),
            public_key: "2wMHpFAjZbL9GkXP8n3E1".to_string(), // base58 encoded
            addresses: vec![],
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            name: None,
        };
        let reputation = Reputation::new(0.8);

        cache.insert(peer_info.clone(), reputation.clone());

        assert!(cache.contains("peer1"));
        let (info, rep) = cache.get("peer1").unwrap();
        assert_eq!(info.id.as_str(), "peer1");
        assert!((rep.score - 0.8).abs() < 0.001);

        // Test trusted peers filter
        let trusted = cache.get_trusted(0.5);
        assert_eq!(trusted.len(), 1);
    }

    #[test]
    fn test_message_cache() {
        let cache = MessageCache::new(10);

        let msg = Message::new(
            MessageType::Content,
            PeerId("sender".to_string()),
            b"Hello".to_vec(),
        );
        let msg_id = msg.id;

        cache.insert(msg);

        assert!(cache.contains(&msg_id));
        let retrieved = cache.get(&msg_id).unwrap();
        assert_eq!(retrieved.sender.as_str(), "sender");

        // Test sender index
        let from_sender = cache.get_from_sender("sender");
        assert_eq!(from_sender.len(), 1);
    }

    #[test]
    fn test_credit_cache() {
        let cache = CreditCache::new(10);

        let rel = CreditRelationship::new(
            PeerId("creditor".to_string()),
            PeerId("debtor".to_string()),
            100.0,
        );

        cache.insert(rel);

        let retrieved = cache.get_between("creditor", "debtor").unwrap();
        assert_eq!(retrieved.credit_limit, 100.0);

        // Test peer index
        let for_creditor = cache.get_for_peer("creditor");
        assert_eq!(for_creditor.len(), 1);

        let for_debtor = cache.get_for_peer("debtor");
        assert_eq!(for_debtor.len(), 1);
    }

    #[test]
    fn test_state_cache() {
        let cache = StateCache::new();

        let stats = cache.stats();
        assert_eq!(stats.peer_count, 0);
        assert_eq!(stats.message_count, 0);
        assert_eq!(stats.credit_count, 0);
    }
}
