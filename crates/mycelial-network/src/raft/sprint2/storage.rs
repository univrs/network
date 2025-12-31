//! Raft log storage implementations

use std::collections::BTreeMap;
use std::ops::RangeBounds;
use std::sync::Arc;

use async_trait::async_trait;
use openraft::{
    Entry, EntryPayload, LogId, LogState, RaftLogReader, RaftLogStorage, SnapshotMeta,
    StorageError, StoredMembership, Vote,
};
use tokio::sync::RwLock;
use tracing::debug;

use super::types::CreditTypeConfig;

/// In-memory log storage for testing and development
pub struct MemoryLogStorage {
    /// Log entries indexed by log index
    log: BTreeMap<u64, Entry<CreditTypeConfig>>,
    /// Current vote
    vote: Option<Vote<u64>>,
    /// Committed index
    committed: Option<LogId<u64>>,
    /// Purged index
    purged: Option<LogId<u64>>,
}

impl MemoryLogStorage {
    /// Create a new empty in-memory log storage
    pub fn new() -> Self {
        Self {
            log: BTreeMap::new(),
            vote: None,
            committed: None,
            purged: None,
        }
    }
}

impl Default for MemoryLogStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RaftLogReader<CreditTypeConfig> for MemoryLogStorage {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Send + Sync>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<CreditTypeConfig>>, StorageError<u64>> {
        let entries: Vec<_> = self
            .log
            .range(range)
            .map(|(_, entry)| entry.clone())
            .collect();
        Ok(entries)
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<u64>>, StorageError<u64>> {
        Ok(self.vote.clone())
    }
}

#[async_trait]
impl RaftLogStorage<CreditTypeConfig> for MemoryLogStorage {
    type LogReader = Self;

    async fn get_log_state(&mut self) -> Result<LogState<CreditTypeConfig>, StorageError<u64>> {
        let last_log_id = self.log.iter().next_back().map(|(_, entry)| entry.log_id);
        let last_purged_log_id = self.purged;

        Ok(LogState {
            last_purged_log_id,
            last_log_id,
        })
    }

    async fn save_vote(&mut self, vote: &Vote<u64>) -> Result<(), StorageError<u64>> {
        debug!(term = vote.leader_id().term, "Saving vote");
        self.vote = Some(vote.clone());
        Ok(())
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        Self {
            log: self.log.clone(),
            vote: self.vote.clone(),
            committed: self.committed,
            purged: self.purged,
        }
    }

    async fn append<I>(&mut self, entries: I, callback: openraft::storage::LogFlushed<u64>) -> Result<(), StorageError<u64>>
    where
        I: IntoIterator<Item = Entry<CreditTypeConfig>> + Send,
    {
        for entry in entries {
            debug!(log_id = ?entry.log_id, "Appending log entry");
            self.log.insert(entry.log_id.index, entry);
        }

        // Simulate async flush completion
        callback.log_io_completed(Ok(()));

        Ok(())
    }

    async fn truncate(&mut self, log_id: LogId<u64>) -> Result<(), StorageError<u64>> {
        debug!(?log_id, "Truncating log");
        let keys_to_remove: Vec<_> = self
            .log
            .range(log_id.index..)
            .map(|(k, _)| *k)
            .collect();

        for key in keys_to_remove {
            self.log.remove(&key);
        }

        Ok(())
    }

    async fn purge(&mut self, log_id: LogId<u64>) -> Result<(), StorageError<u64>> {
        debug!(?log_id, "Purging log up to");
        let keys_to_remove: Vec<_> = self
            .log
            .range(..=log_id.index)
            .map(|(k, _)| *k)
            .collect();

        for key in keys_to_remove {
            self.log.remove(&key);
        }

        self.purged = Some(log_id);
        Ok(())
    }
}

/// Sled-based persistent log storage
#[cfg(feature = "openraft")]
pub struct SledLogStorage {
    /// Sled database
    db: sled::Db,
    /// Log entries tree
    log_tree: sled::Tree,
    /// Vote storage tree
    vote_tree: sled::Tree,
    /// Metadata tree
    meta_tree: sled::Tree,
}

#[cfg(feature = "openraft")]
impl SledLogStorage {
    /// Create or open a sled-based log storage
    pub fn new(path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        let log_tree = db.open_tree("raft_log")?;
        let vote_tree = db.open_tree("raft_vote")?;
        let meta_tree = db.open_tree("raft_meta")?;

        Ok(Self {
            db,
            log_tree,
            vote_tree,
            meta_tree,
        })
    }

    /// Create an in-memory sled storage (for testing)
    pub fn in_memory() -> Result<Self, sled::Error> {
        let config = sled::Config::new().temporary(true);
        let db = config.open()?;
        let log_tree = db.open_tree("raft_log")?;
        let vote_tree = db.open_tree("raft_vote")?;
        let meta_tree = db.open_tree("raft_meta")?;

        Ok(Self {
            db,
            log_tree,
            vote_tree,
            meta_tree,
        })
    }

    fn key_to_bytes(index: u64) -> [u8; 8] {
        index.to_be_bytes()
    }

    fn bytes_to_key(bytes: &[u8]) -> u64 {
        let arr: [u8; 8] = bytes.try_into().unwrap_or([0; 8]);
        u64::from_be_bytes(arr)
    }
}

#[cfg(feature = "openraft")]
#[async_trait]
impl RaftLogReader<CreditTypeConfig> for SledLogStorage {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Send + Sync>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<CreditTypeConfig>>, StorageError<u64>> {
        use std::ops::Bound;

        let start = match range.start_bound() {
            Bound::Included(&n) => Bound::Included(Self::key_to_bytes(n).to_vec()),
            Bound::Excluded(&n) => Bound::Excluded(Self::key_to_bytes(n).to_vec()),
            Bound::Unbounded => Bound::Unbounded,
        };

        let end = match range.end_bound() {
            Bound::Included(&n) => Bound::Included(Self::key_to_bytes(n).to_vec()),
            Bound::Excluded(&n) => Bound::Excluded(Self::key_to_bytes(n).to_vec()),
            Bound::Unbounded => Bound::Unbounded,
        };

        let mut entries = Vec::new();
        for item in self.log_tree.range((start, end)) {
            let (_, value) = item.map_err(|e| StorageError::IO {
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?;

            let entry: Entry<CreditTypeConfig> = bincode::deserialize(&value).map_err(|e| {
                StorageError::IO {
                    source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
                }
            })?;

            entries.push(entry);
        }

        Ok(entries)
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<u64>>, StorageError<u64>> {
        match self.vote_tree.get(b"vote").map_err(|e| StorageError::IO {
            source: std::io::Error::new(std::io::ErrorKind::Other, e),
        })? {
            Some(bytes) => {
                let vote: Vote<u64> = bincode::deserialize(&bytes).map_err(|e| StorageError::IO {
                    source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
                })?;
                Ok(Some(vote))
            }
            None => Ok(None),
        }
    }
}

#[cfg(feature = "openraft")]
#[async_trait]
impl RaftLogStorage<CreditTypeConfig> for SledLogStorage {
    type LogReader = Self;

    async fn get_log_state(&mut self) -> Result<LogState<CreditTypeConfig>, StorageError<u64>> {
        let last_log_id = self
            .log_tree
            .last()
            .map_err(|e| StorageError::IO {
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?
            .and_then(|(_, value)| {
                let entry: Entry<CreditTypeConfig> = bincode::deserialize(&value).ok()?;
                Some(entry.log_id)
            });

        let last_purged_log_id = self
            .meta_tree
            .get(b"purged")
            .map_err(|e| StorageError::IO {
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?
            .and_then(|bytes| bincode::deserialize(&bytes).ok());

        Ok(LogState {
            last_purged_log_id,
            last_log_id,
        })
    }

    async fn save_vote(&mut self, vote: &Vote<u64>) -> Result<(), StorageError<u64>> {
        let bytes = bincode::serialize(vote).map_err(|e| StorageError::IO {
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })?;

        self.vote_tree
            .insert(b"vote", bytes)
            .map_err(|e| StorageError::IO {
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?;

        self.vote_tree.flush().map_err(|e| StorageError::IO {
            source: std::io::Error::new(std::io::ErrorKind::Other, e),
        })?;

        Ok(())
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        // For sled, we return a reference to self
        // This is safe because sled is thread-safe
        Self {
            db: self.db.clone(),
            log_tree: self.log_tree.clone(),
            vote_tree: self.vote_tree.clone(),
            meta_tree: self.meta_tree.clone(),
        }
    }

    async fn append<I>(&mut self, entries: I, callback: openraft::storage::LogFlushed<u64>) -> Result<(), StorageError<u64>>
    where
        I: IntoIterator<Item = Entry<CreditTypeConfig>> + Send,
    {
        for entry in entries {
            let key = Self::key_to_bytes(entry.log_id.index);
            let value = bincode::serialize(&entry).map_err(|e| StorageError::IO {
                source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
            })?;

            self.log_tree.insert(key, value).map_err(|e| StorageError::IO {
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?;
        }

        self.log_tree.flush().map_err(|e| StorageError::IO {
            source: std::io::Error::new(std::io::ErrorKind::Other, e),
        })?;

        callback.log_io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogId<u64>) -> Result<(), StorageError<u64>> {
        let start_key = Self::key_to_bytes(log_id.index);

        let keys_to_remove: Vec<_> = self
            .log_tree
            .range(start_key..)
            .filter_map(|item| item.ok().map(|(k, _)| k.to_vec()))
            .collect();

        for key in keys_to_remove {
            self.log_tree.remove(key).map_err(|e| StorageError::IO {
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?;
        }

        self.log_tree.flush().map_err(|e| StorageError::IO {
            source: std::io::Error::new(std::io::ErrorKind::Other, e),
        })?;

        Ok(())
    }

    async fn purge(&mut self, log_id: LogId<u64>) -> Result<(), StorageError<u64>> {
        let end_key = Self::key_to_bytes(log_id.index + 1);

        let keys_to_remove: Vec<_> = self
            .log_tree
            .range(..end_key)
            .filter_map(|item| item.ok().map(|(k, _)| k.to_vec()))
            .collect();

        for key in keys_to_remove {
            self.log_tree.remove(key).map_err(|e| StorageError::IO {
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?;
        }

        // Save purged log id
        let bytes = bincode::serialize(&log_id).map_err(|e| StorageError::IO {
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })?;
        self.meta_tree
            .insert(b"purged", bytes)
            .map_err(|e| StorageError::IO {
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?;

        self.log_tree.flush().map_err(|e| StorageError::IO {
            source: std::io::Error::new(std::io::ErrorKind::Other, e),
        })?;
        self.meta_tree.flush().map_err(|e| StorageError::IO {
            source: std::io::Error::new(std::io::ErrorKind::Other, e),
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openraft::EntryPayload;

    use crate::raft::types::CreditCommand;

    #[tokio::test]
    async fn test_memory_storage_append_and_read() {
        let mut storage = MemoryLogStorage::new();

        let entry = Entry {
            log_id: LogId::new(openraft::LeaderId::new(1, 1), 1),
            payload: EntryPayload::Normal(CreditCommand::Noop),
        };

        // Create a callback that does nothing
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let callback = openraft::storage::LogFlushed::new(Some(tx));

        storage.append(vec![entry.clone()], callback).await.unwrap();

        let entries = storage.try_get_log_entries(0..10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].log_id.index, 1);
    }

    #[tokio::test]
    async fn test_memory_storage_vote() {
        let mut storage = MemoryLogStorage::new();

        let vote = Vote::new(1, 42);
        storage.save_vote(&vote).await.unwrap();

        let loaded = storage.read_vote().await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().leader_id().node_id, 42);
    }
}
