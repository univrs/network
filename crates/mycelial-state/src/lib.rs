//! Mycelial State - Persistence and state management
//!
//! This crate provides storage backends and state management for the mycelial network.
//!
//! ## Components
//!
//! - **storage**: SQLite-based persistence with sqlx
//! - **cache**: LRU in-memory caching for peers, messages, and credit relationships
//! - **sync**: State synchronization with vector clocks and CRDT-style merge strategies
//! - **error**: State-specific error types
//!
//! ## Example
//!
//! ```ignore
//! use mycelial_state::{SqliteStore, StateCache, StateSync};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize storage
//!     let store = SqliteStore::new("mycelial.db").await?;
//!
//!     // Initialize cache
//!     let cache = Arc::new(StateCache::new());
//!
//!     // Initialize sync manager
//!     let sync = StateSync::new("local_peer_id".to_string(), cache.clone());
//!
//!     Ok(())
//! }
//! ```

pub mod cache;
pub mod error;
pub mod storage;
pub mod sync;

// Re-exports for convenience
pub use cache::{CacheStats, CreditCache, MemoryCache, MessageCache, PeerCache, StateCache};
pub use error::{Result, StateError};
pub use storage::SqliteStore;
pub use sync::{PeerInfoUpdate, StateSync, StateUpdate, VectorClock};
