//! Deduplication cache for preventing message loops
//!
//! When bridging between Meshtastic LoRa mesh and libp2p gossipsub, there's
//! a risk of message loops where a message could be:
//!
//! 1. Sent from LoRa → libp2p
//! 2. Received by another bridge node
//! 3. Sent back to LoRa
//! 4. Received by the original node
//! 5. Sent back to libp2p... and so on
//!
//! The DeduplicationCache prevents this by tracking recently seen messages
//! using their unique identifiers, and blocking duplicates.
//!
//! # Deduplication Key
//!
//! Messages are identified by a composite key:
//! - For Meshtastic: `(sender_node_id, packet_id)`
//! - For libp2p: message UUID
//!
//! This allows detecting duplicates even when the same logical message
//! appears from different network paths.

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, trace};

use crate::config::BridgeConfig;

/// Key for deduplication cache entries
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DeduplicationKey {
    /// Source identifier (node_id for LoRa, short peer_id for libp2p)
    pub source: String,
    /// Message/packet identifier
    pub message_id: String,
}

impl DeduplicationKey {
    /// Create a key for a Meshtastic packet
    pub fn from_meshtastic(sender_node_id: u32, packet_id: u32) -> Self {
        Self {
            source: format!("lora:{:08x}", sender_node_id),
            message_id: format!("{:08x}", packet_id),
        }
    }

    /// Create a key for a libp2p message
    pub fn from_libp2p(peer_id: &str, message_id: &str) -> Self {
        Self {
            source: format!("p2p:{}", &peer_id[..peer_id.len().min(12)]),
            message_id: message_id.to_string(),
        }
    }

    /// Create a key from raw components
    pub fn new(source: impl Into<String>, message_id: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            message_id: message_id.into(),
        }
    }
}

impl std::fmt::Display for DeduplicationKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.source, self.message_id)
    }
}

/// Entry stored in the deduplication cache
#[derive(Debug, Clone)]
struct CacheEntry {
    /// When this entry was first seen
    first_seen: Instant,
    /// Number of times this message was seen
    seen_count: u32,
    /// Direction of first sighting
    direction: MessageDirection,
}

/// Direction a message was first seen traveling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageDirection {
    /// Message came from LoRa mesh
    FromLora,
    /// Message came from libp2p gossipsub
    FromLibp2p,
}

/// LRU-based deduplication cache with TTL expiration
///
/// The cache uses a combination of LRU eviction and TTL expiration to
/// manage memory while ensuring messages aren't accidentally re-bridged.
#[derive(Debug)]
pub struct DeduplicationCache {
    /// LRU cache storing seen messages
    cache: Arc<RwLock<LruCache<DeduplicationKey, CacheEntry>>>,
    /// Time-to-live for cache entries
    ttl: Duration,
    /// Statistics
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total messages checked
    pub total_checks: u64,
    /// Messages that were duplicates
    pub duplicates_blocked: u64,
    /// Messages that were new (passed through)
    pub new_messages: u64,
    /// Entries expired by TTL
    pub ttl_expirations: u64,
    /// Entries evicted by LRU
    pub lru_evictions: u64,
}

impl CacheStats {
    /// Get the duplicate rate (0.0 to 1.0)
    pub fn duplicate_rate(&self) -> f64 {
        if self.total_checks == 0 {
            0.0
        } else {
            self.duplicates_blocked as f64 / self.total_checks as f64
        }
    }

    /// Get the number of messages that passed through
    pub fn pass_through_count(&self) -> u64 {
        self.new_messages
    }
}

impl DeduplicationCache {
    /// Create a new deduplication cache with default settings
    pub fn new() -> Self {
        Self::with_capacity_and_ttl(1000, Duration::from_secs(300))
    }

    /// Create from bridge configuration
    pub fn from_config(config: &BridgeConfig) -> Self {
        Self::with_capacity_and_ttl(config.dedup_cache_size, config.dedup_ttl)
    }

    /// Create with custom capacity and TTL
    pub fn with_capacity_and_ttl(capacity: usize, ttl: Duration) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(cap))),
            ttl,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Check if a message is a duplicate
    ///
    /// Returns `true` if this message has been seen before (is a duplicate),
    /// `false` if it's new. If new, the message is automatically recorded.
    pub fn is_duplicate(&self, key: &DeduplicationKey, direction: MessageDirection) -> bool {
        let now = Instant::now();

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_checks += 1;
        }

        let mut cache = self.cache.write().unwrap();

        // Check if entry exists and is still valid
        if let Some(entry) = cache.get_mut(key) {
            // Check TTL expiration
            if now.duration_since(entry.first_seen) > self.ttl {
                // Entry expired, treat as new
                trace!(key = %key, "Cache entry expired, treating as new");
                {
                    let mut stats = self.stats.write().unwrap();
                    stats.ttl_expirations += 1;
                    stats.new_messages += 1;
                }
                // Update entry with new timestamp
                entry.first_seen = now;
                entry.seen_count = 1;
                entry.direction = direction;
                return false;
            }

            // Entry is still valid - this is a duplicate
            entry.seen_count += 1;
            debug!(
                key = %key,
                seen_count = entry.seen_count,
                "Duplicate message detected"
            );
            {
                let mut stats = self.stats.write().unwrap();
                stats.duplicates_blocked += 1;
            }
            return true;
        }

        // Not in cache - record it
        let was_full = cache.len() >= cache.cap().get();
        cache.put(
            key.clone(),
            CacheEntry {
                first_seen: now,
                seen_count: 1,
                direction,
            },
        );

        if was_full {
            let mut stats = self.stats.write().unwrap();
            stats.lru_evictions += 1;
        }

        trace!(key = %key, direction = ?direction, "New message recorded");
        {
            let mut stats = self.stats.write().unwrap();
            stats.new_messages += 1;
        }

        false
    }

    /// Check if a Meshtastic packet is a duplicate
    pub fn is_meshtastic_duplicate(&self, sender_node_id: u32, packet_id: u32) -> bool {
        let key = DeduplicationKey::from_meshtastic(sender_node_id, packet_id);
        self.is_duplicate(&key, MessageDirection::FromLora)
    }

    /// Check if a libp2p message is a duplicate
    pub fn is_libp2p_duplicate(&self, peer_id: &str, message_id: &str) -> bool {
        let key = DeduplicationKey::from_libp2p(peer_id, message_id);
        self.is_duplicate(&key, MessageDirection::FromLibp2p)
    }

    /// Mark a message as seen without checking
    ///
    /// Use this when sending a message to ensure it won't be bridged back.
    pub fn mark_seen(&self, key: &DeduplicationKey, direction: MessageDirection) {
        let mut cache = self.cache.write().unwrap();
        cache.put(
            key.clone(),
            CacheEntry {
                first_seen: Instant::now(),
                seen_count: 1,
                direction,
            },
        );
    }

    /// Mark a Meshtastic packet as seen
    pub fn mark_meshtastic_seen(&self, sender_node_id: u32, packet_id: u32) {
        let key = DeduplicationKey::from_meshtastic(sender_node_id, packet_id);
        self.mark_seen(&key, MessageDirection::FromLora);
    }

    /// Mark a libp2p message as seen
    pub fn mark_libp2p_seen(&self, peer_id: &str, message_id: &str) {
        let key = DeduplicationKey::from_libp2p(peer_id, message_id);
        self.mark_seen(&key, MessageDirection::FromLibp2p);
    }

    /// Get the number of entries in the cache
    pub fn len(&self) -> usize {
        let cache = self.cache.read().unwrap();
        cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let stats = self.stats.read().unwrap();
        stats.clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.write().unwrap();
        *stats = CacheStats::default();
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    /// Manually expire entries older than TTL
    ///
    /// This is called periodically to clean up expired entries.
    /// Returns the number of entries expired.
    ///
    /// Note: The LRU cache doesn't support iteration with removal, so actual
    /// TTL expiration happens lazily during `is_duplicate()` checks. This method
    /// is provided for API completeness but relies on LRU eviction for cleanup.
    pub fn expire_old_entries(&self) -> usize {
        // LruCache doesn't support iteration with removal, so we collect keys first
        // This is a known limitation; in production you might use a different data structure
        // For now, we rely on LRU eviction and TTL checks in is_duplicate()

        // The LRU cache will naturally evict old entries when new ones come in
        // For explicit expiration, we'd need a different approach

        0 // Actual expiration happens lazily in is_duplicate()
    }

    /// Get the configured TTL
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Get the cache capacity
    pub fn capacity(&self) -> usize {
        let cache = self.cache.read().unwrap();
        cache.cap().get()
    }
}

impl Default for DeduplicationCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DeduplicationCache {
    fn clone(&self) -> Self {
        // Create a new cache with same settings but shared data
        Self {
            cache: Arc::clone(&self.cache),
            ttl: self.ttl,
            stats: Arc::clone(&self.stats),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplication_key_creation() {
        let key1 = DeduplicationKey::from_meshtastic(0x12345678, 0xABCDEF00);
        assert!(key1.source.starts_with("lora:"));
        assert!(key1.message_id.contains("abcdef00"));

        let key2 = DeduplicationKey::from_libp2p("peer_abcdefgh12345678", "msg-12345");
        assert!(key2.source.starts_with("p2p:"));
        assert_eq!(key2.message_id, "msg-12345");
    }

    #[test]
    fn test_deduplication_key_display() {
        let key = DeduplicationKey::new("source", "message");
        assert_eq!(format!("{}", key), "source:message");
    }

    #[test]
    fn test_cache_new_message() {
        let cache = DeduplicationCache::new();

        let key = DeduplicationKey::from_meshtastic(0x12345678, 0x00000001);

        // First time should not be a duplicate
        assert!(!cache.is_duplicate(&key, MessageDirection::FromLora));
        assert_eq!(cache.len(), 1);

        let stats = cache.stats();
        assert_eq!(stats.total_checks, 1);
        assert_eq!(stats.new_messages, 1);
        assert_eq!(stats.duplicates_blocked, 0);
    }

    #[test]
    fn test_cache_duplicate_detection() {
        let cache = DeduplicationCache::new();

        let key = DeduplicationKey::from_meshtastic(0x12345678, 0x00000001);

        // First time - new
        assert!(!cache.is_duplicate(&key, MessageDirection::FromLora));

        // Second time - duplicate
        assert!(cache.is_duplicate(&key, MessageDirection::FromLora));

        // Third time - still duplicate
        assert!(cache.is_duplicate(&key, MessageDirection::FromLora));

        let stats = cache.stats();
        assert_eq!(stats.total_checks, 3);
        assert_eq!(stats.new_messages, 1);
        assert_eq!(stats.duplicates_blocked, 2);
    }

    #[test]
    fn test_cache_different_messages() {
        let cache = DeduplicationCache::new();

        let key1 = DeduplicationKey::from_meshtastic(0x12345678, 0x00000001);
        let key2 = DeduplicationKey::from_meshtastic(0x12345678, 0x00000002);
        let key3 = DeduplicationKey::from_meshtastic(0x87654321, 0x00000001);

        // All should be new
        assert!(!cache.is_duplicate(&key1, MessageDirection::FromLora));
        assert!(!cache.is_duplicate(&key2, MessageDirection::FromLora));
        assert!(!cache.is_duplicate(&key3, MessageDirection::FromLora));

        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_cache_mark_seen() {
        let cache = DeduplicationCache::new();

        let key = DeduplicationKey::from_libp2p("peer_abc", "msg-123");

        // Mark as seen before any check
        cache.mark_seen(&key, MessageDirection::FromLibp2p);

        // Now it should be detected as duplicate
        assert!(cache.is_duplicate(&key, MessageDirection::FromLibp2p));
    }

    #[test]
    fn test_cache_convenience_methods() {
        let cache = DeduplicationCache::new();

        // Meshtastic methods
        assert!(!cache.is_meshtastic_duplicate(0x12345678, 0x00000001));
        assert!(cache.is_meshtastic_duplicate(0x12345678, 0x00000001));

        // libp2p methods
        assert!(!cache.is_libp2p_duplicate("peer_abc", "msg-123"));
        assert!(cache.is_libp2p_duplicate("peer_abc", "msg-123"));
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = DeduplicationCache::with_capacity_and_ttl(3, Duration::from_secs(300));

        // Fill cache
        let key1 = DeduplicationKey::new("s1", "m1");
        let key2 = DeduplicationKey::new("s2", "m2");
        let key3 = DeduplicationKey::new("s3", "m3");

        cache.is_duplicate(&key1, MessageDirection::FromLora);
        cache.is_duplicate(&key2, MessageDirection::FromLora);
        cache.is_duplicate(&key3, MessageDirection::FromLora);

        assert_eq!(cache.len(), 3);

        // Add one more - should evict oldest (key1)
        let key4 = DeduplicationKey::new("s4", "m4");
        cache.is_duplicate(&key4, MessageDirection::FromLora);

        assert_eq!(cache.len(), 3);

        // key1 should now be seen as new (was evicted)
        assert!(!cache.is_duplicate(&key1, MessageDirection::FromLora));
    }

    #[test]
    fn test_cache_ttl_expiration() {
        // Use very short TTL for testing
        let cache = DeduplicationCache::with_capacity_and_ttl(10, Duration::from_millis(50));

        let key = DeduplicationKey::new("source", "message");

        // First check - new
        assert!(!cache.is_duplicate(&key, MessageDirection::FromLora));

        // Second check immediately - duplicate
        assert!(cache.is_duplicate(&key, MessageDirection::FromLora));

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(60));

        // Should be treated as new again
        assert!(!cache.is_duplicate(&key, MessageDirection::FromLora));
    }

    #[test]
    fn test_cache_stats() {
        let cache = DeduplicationCache::new();

        let key1 = DeduplicationKey::new("s1", "m1");
        let key2 = DeduplicationKey::new("s2", "m2");

        cache.is_duplicate(&key1, MessageDirection::FromLora);
        cache.is_duplicate(&key1, MessageDirection::FromLora);
        cache.is_duplicate(&key2, MessageDirection::FromLora);
        cache.is_duplicate(&key2, MessageDirection::FromLora);
        cache.is_duplicate(&key2, MessageDirection::FromLora);

        let stats = cache.stats();
        assert_eq!(stats.total_checks, 5);
        assert_eq!(stats.new_messages, 2);
        assert_eq!(stats.duplicates_blocked, 3);
        assert!((stats.duplicate_rate() - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_cache_reset_stats() {
        let cache = DeduplicationCache::new();

        let key = DeduplicationKey::new("s", "m");
        cache.is_duplicate(&key, MessageDirection::FromLora);
        cache.is_duplicate(&key, MessageDirection::FromLora);

        assert!(cache.stats().total_checks > 0);

        cache.reset_stats();

        assert_eq!(cache.stats().total_checks, 0);
    }

    #[test]
    fn test_cache_clear() {
        let cache = DeduplicationCache::new();

        cache.is_duplicate(
            &DeduplicationKey::new("s1", "m1"),
            MessageDirection::FromLora,
        );
        cache.is_duplicate(
            &DeduplicationKey::new("s2", "m2"),
            MessageDirection::FromLora,
        );

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_clone_shares_data() {
        let cache1 = DeduplicationCache::new();

        let key = DeduplicationKey::new("source", "message");
        cache1.is_duplicate(&key, MessageDirection::FromLora);

        // Clone shares the same underlying cache
        let cache2 = cache1.clone();

        // Second instance should see the same entry
        assert!(cache2.is_duplicate(&key, MessageDirection::FromLora));
        assert_eq!(cache1.len(), cache2.len());
    }

    #[test]
    fn test_cache_from_config() {
        let config = BridgeConfig {
            dedup_cache_size: 500,
            dedup_ttl: Duration::from_secs(120),
            ..Default::default()
        };

        let cache = DeduplicationCache::from_config(&config);

        assert_eq!(cache.capacity(), 500);
        assert_eq!(cache.ttl(), Duration::from_secs(120));
    }

    #[test]
    fn test_cache_bidirectional_dedup() {
        let cache = DeduplicationCache::new();

        // A message comes from LoRa
        cache.mark_meshtastic_seen(0x12345678, 0x00000001);

        // Same logical message should be detected as duplicate when seen from libp2p
        // This requires the bridge to use consistent keys across directions
        let key = DeduplicationKey::from_meshtastic(0x12345678, 0x00000001);
        assert!(cache.is_duplicate(&key, MessageDirection::FromLibp2p));
    }
}
