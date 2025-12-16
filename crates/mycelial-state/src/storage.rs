//! SQLite storage backend implementation
//!
//! This module provides persistent storage for peers, messages, and credit
//! relationships using SQLite with sqlx.

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use mycelial_core::{
    credit::CreditRelationship,
    message::{Message, MessageType},
    peer::{PeerId, PeerInfo},
    reputation::{Reputation, ReputationSnapshot},
    Result as CoreResult, StateStore,
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions},
    Row,
};
use std::str::FromStr;
use tracing::{debug, info};
use uuid::Uuid;

use crate::error::{Result, StateError};

/// SQLite-based storage backend
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    /// Create a new SQLite store with the given database path
    ///
    /// # Arguments
    /// * `path` - Path to the SQLite database file (use ":memory:" for in-memory)
    pub async fn new(path: &str) -> Result<Self> {
        info!("Initializing SQLite store at: {}", path);

        let options = SqliteConnectOptions::from_str(path)
            .map_err(|e| StateError::Connection(e.to_string()))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| StateError::Connection(e.to_string()))?;

        let store = Self { pool };
        store.run_migrations().await?;

        info!("SQLite store initialized successfully");
        Ok(store)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        debug!("Running database migrations");

        // Run the initial schema
        sqlx::query(include_str!("../migrations/001_initial.sql"))
            .execute(&self.pool)
            .await
            .map_err(|e| StateError::Migration(e.to_string()))?;

        debug!("Migrations completed successfully");
        Ok(())
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ========== Peer Operations ==========

    /// Store or update a peer
    pub async fn upsert_peer(&self, info: &PeerInfo, reputation: Option<&Reputation>) -> Result<()> {
        let peer_id = info.id.as_str();
        let public_key = &info.public_key;
        let addresses_json = serde_json::to_string(&info.addresses)?;
        let first_seen = info.first_seen.timestamp();
        let last_seen = info.last_seen.timestamp();
        let display_name = info.name.as_deref();

        let (reputation_score, successful, failed, history_json) = match reputation {
            Some(rep) => (
                rep.score,
                rep.successful_interactions as i64,
                rep.failed_interactions as i64,
                serde_json::to_string(&rep.history)?,
            ),
            None => (0.5, 0i64, 0i64, "[]".to_string()),
        };

        sqlx::query(
            r#"
            INSERT INTO peers (
                peer_id, public_key, display_name, addresses_json,
                reputation_score, successful_interactions, failed_interactions,
                reputation_history_json, first_seen, last_seen
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(peer_id) DO UPDATE SET
                public_key = excluded.public_key,
                display_name = COALESCE(excluded.display_name, peers.display_name),
                addresses_json = excluded.addresses_json,
                reputation_score = excluded.reputation_score,
                successful_interactions = excluded.successful_interactions,
                failed_interactions = excluded.failed_interactions,
                reputation_history_json = excluded.reputation_history_json,
                last_seen = excluded.last_seen,
                updated_at = strftime('%s', 'now')
            "#,
        )
        .bind(peer_id)
        .bind(public_key)
        .bind(display_name)
        .bind(&addresses_json)
        .bind(reputation_score)
        .bind(successful)
        .bind(failed)
        .bind(&history_json)
        .bind(first_seen)
        .bind(last_seen)
        .execute(&self.pool)
        .await?;

        debug!("Upserted peer: {}", peer_id);
        Ok(())
    }

    /// Get a peer by ID
    pub async fn get_peer(&self, peer_id: &str) -> Result<Option<(PeerInfo, Reputation)>> {
        let row = sqlx::query(
            r#"
            SELECT peer_id, public_key, display_name, addresses_json, location_json,
                   reputation_score, successful_interactions, failed_interactions,
                   reputation_history_json, first_seen, last_seen
            FROM peers WHERE peer_id = ?
            "#,
        )
        .bind(peer_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let peer_info = self.row_to_peer_info(&row)?;
                let reputation = self.row_to_reputation(&row)?;
                Ok(Some((peer_info, reputation)))
            }
            None => Ok(None),
        }
    }

    /// List all peers
    pub async fn list_peers(&self) -> Result<Vec<(PeerInfo, Reputation)>> {
        let rows = sqlx::query(
            r#"
            SELECT peer_id, public_key, display_name, addresses_json, location_json,
                   reputation_score, successful_interactions, failed_interactions,
                   reputation_history_json, first_seen, last_seen
            FROM peers ORDER BY last_seen DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let peer_info = self.row_to_peer_info(&row)?;
            let reputation = self.row_to_reputation(&row)?;
            results.push((peer_info, reputation));
        }

        Ok(results)
    }

    /// List peers with reputation above threshold
    pub async fn list_trusted_peers(&self, threshold: f64) -> Result<Vec<(PeerInfo, Reputation)>> {
        let rows = sqlx::query(
            r#"
            SELECT peer_id, public_key, display_name, addresses_json, location_json,
                   reputation_score, successful_interactions, failed_interactions,
                   reputation_history_json, first_seen, last_seen
            FROM peers WHERE reputation_score >= ? ORDER BY reputation_score DESC
            "#,
        )
        .bind(threshold)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let peer_info = self.row_to_peer_info(&row)?;
            let reputation = self.row_to_reputation(&row)?;
            results.push((peer_info, reputation));
        }

        Ok(results)
    }

    /// Update peer reputation
    pub async fn update_peer_reputation(&self, peer_id: &str, reputation: &Reputation) -> Result<()> {
        let history_json = serde_json::to_string(&reputation.history)?;

        let result = sqlx::query(
            r#"
            UPDATE peers SET
                reputation_score = ?,
                successful_interactions = ?,
                failed_interactions = ?,
                reputation_history_json = ?,
                updated_at = strftime('%s', 'now')
            WHERE peer_id = ?
            "#,
        )
        .bind(reputation.score)
        .bind(reputation.successful_interactions as i64)
        .bind(reputation.failed_interactions as i64)
        .bind(&history_json)
        .bind(peer_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StateError::NotFound {
                entity: "peer".to_string(),
                id: peer_id.to_string(),
            });
        }

        debug!("Updated reputation for peer: {}", peer_id);
        Ok(())
    }

    /// Update peer last seen timestamp
    pub async fn touch_peer(&self, peer_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            UPDATE peers SET last_seen = ?, updated_at = strftime('%s', 'now')
            WHERE peer_id = ?
            "#,
        )
        .bind(now)
        .bind(peer_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a peer
    pub async fn delete_peer(&self, peer_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM peers WHERE peer_id = ?")
            .bind(peer_id)
            .execute(&self.pool)
            .await?;

        debug!("Deleted peer: {}", peer_id);
        Ok(())
    }

    /// Count peers
    pub async fn count_peers(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM peers")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }

    // Helper to convert row to PeerInfo
    fn row_to_peer_info(&self, row: &sqlx::sqlite::SqliteRow) -> Result<PeerInfo> {
        let peer_id: String = row.get("peer_id");
        let public_key: Vec<u8> = row.get("public_key");
        let display_name: Option<String> = row.get("display_name");
        let addresses_json: String = row.get("addresses_json");
        let first_seen: i64 = row.get("first_seen");
        let last_seen: i64 = row.get("last_seen");

        let addresses: Vec<String> = serde_json::from_str(&addresses_json)
            .map_err(|e| StateError::Deserialization(e.to_string()))?;

        Ok(PeerInfo {
            id: PeerId(peer_id),
            public_key,
            addresses,
            first_seen: Utc.timestamp_opt(first_seen, 0).single().unwrap_or_else(Utc::now),
            last_seen: Utc.timestamp_opt(last_seen, 0).single().unwrap_or_else(Utc::now),
            name: display_name,
        })
    }

    // Helper to convert row to Reputation
    fn row_to_reputation(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Reputation> {
        let score: f64 = row.get("reputation_score");
        let successful: i64 = row.get("successful_interactions");
        let failed: i64 = row.get("failed_interactions");
        let history_json: String = row.get("reputation_history_json");

        let history: Vec<ReputationSnapshot> = serde_json::from_str(&history_json)
            .map_err(|e| StateError::Deserialization(e.to_string()))?;

        Ok(Reputation {
            score,
            successful_interactions: successful as u64,
            failed_interactions: failed as u64,
            last_updated: Utc::now(),
            history,
        })
    }

    // ========== Message Operations ==========

    /// Store a message
    pub async fn store_message(&self, message: &Message) -> Result<()> {
        let id = message.id.to_string();
        let message_type = format!("{:?}", message.message_type);
        let sender = message.sender.as_str();
        let recipient = message.recipient.as_ref().map(|p| p.as_str().to_string());
        let timestamp = message.timestamp.timestamp();

        sqlx::query(
            r#"
            INSERT INTO messages (id, message_type, sender_peer_id, recipient_peer_id, payload, signature, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO NOTHING
            "#,
        )
        .bind(&id)
        .bind(&message_type)
        .bind(sender)
        .bind(&recipient)
        .bind(&message.payload)
        .bind(&message.signature)
        .bind(timestamp)
        .execute(&self.pool)
        .await?;

        debug!("Stored message: {}", id);
        Ok(())
    }

    /// Get a message by ID
    pub async fn get_message(&self, id: &Uuid) -> Result<Option<Message>> {
        let row = sqlx::query(
            r#"
            SELECT id, message_type, sender_peer_id, recipient_peer_id, payload, signature, timestamp
            FROM messages WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_message(&row)?)),
            None => Ok(None),
        }
    }

    /// List messages from a sender
    pub async fn list_messages_from(&self, peer_id: &str, limit: i64) -> Result<Vec<Message>> {
        let rows = sqlx::query(
            r#"
            SELECT id, message_type, sender_peer_id, recipient_peer_id, payload, signature, timestamp
            FROM messages WHERE sender_peer_id = ?
            ORDER BY timestamp DESC LIMIT ?
            "#,
        )
        .bind(peer_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            results.push(self.row_to_message(&row)?);
        }

        Ok(results)
    }

    /// List recent messages
    pub async fn list_recent_messages(&self, limit: i64) -> Result<Vec<Message>> {
        let rows = sqlx::query(
            r#"
            SELECT id, message_type, sender_peer_id, recipient_peer_id, payload, signature, timestamp
            FROM messages ORDER BY timestamp DESC LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            results.push(self.row_to_message(&row)?);
        }

        Ok(results)
    }

    /// List messages by type
    pub async fn list_messages_by_type(&self, message_type: &MessageType, limit: i64) -> Result<Vec<Message>> {
        let type_str = format!("{:?}", message_type);

        let rows = sqlx::query(
            r#"
            SELECT id, message_type, sender_peer_id, recipient_peer_id, payload, signature, timestamp
            FROM messages WHERE message_type = ?
            ORDER BY timestamp DESC LIMIT ?
            "#,
        )
        .bind(&type_str)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            results.push(self.row_to_message(&row)?);
        }

        Ok(results)
    }

    /// Delete old messages
    pub async fn prune_messages(&self, older_than_secs: i64) -> Result<u64> {
        let cutoff = Utc::now().timestamp() - older_than_secs;

        let result = sqlx::query("DELETE FROM messages WHERE timestamp < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        let deleted = result.rows_affected();
        if deleted > 0 {
            info!("Pruned {} old messages", deleted);
        }

        Ok(deleted)
    }

    // Helper to convert row to Message
    fn row_to_message(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Message> {
        let id: String = row.get("id");
        let message_type_str: String = row.get("message_type");
        let sender: String = row.get("sender_peer_id");
        let recipient: Option<String> = row.get("recipient_peer_id");
        let payload: Vec<u8> = row.get("payload");
        let signature: Option<Vec<u8>> = row.get("signature");
        let timestamp: i64 = row.get("timestamp");

        let message_type = match message_type_str.as_str() {
            "Discovery" => MessageType::Discovery,
            "Content" => MessageType::Content,
            "Reputation" => MessageType::Reputation,
            "Credit" => MessageType::Credit,
            "Governance" => MessageType::Governance,
            "Direct" => MessageType::Direct,
            "System" => MessageType::System,
            _ => MessageType::System,
        };

        Ok(Message {
            id: Uuid::parse_str(&id).map_err(|e| StateError::Deserialization(e.to_string()))?,
            message_type,
            sender: PeerId(sender),
            recipient: recipient.map(PeerId),
            payload,
            timestamp: Utc.timestamp_opt(timestamp, 0).single().unwrap_or_else(Utc::now),
            signature,
        })
    }

    // ========== Credit Relationship Operations ==========

    /// Store or update a credit relationship
    pub async fn upsert_credit_relationship(&self, rel: &CreditRelationship) -> Result<String> {
        let id = format!("{}_{}", rel.creditor.as_str(), rel.debtor.as_str());
        let creditor = rel.creditor.as_str();
        let debtor = rel.debtor.as_str();
        let established = rel.established.timestamp();
        let last_transaction = rel.last_transaction.timestamp();
        let active = if rel.active { 1 } else { 0 };

        sqlx::query(
            r#"
            INSERT INTO credit_relationships (
                id, creditor_peer_id, debtor_peer_id, credit_limit, balance,
                active, established, last_transaction
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(creditor_peer_id, debtor_peer_id) DO UPDATE SET
                credit_limit = excluded.credit_limit,
                balance = excluded.balance,
                active = excluded.active,
                last_transaction = excluded.last_transaction,
                updated_at = strftime('%s', 'now')
            "#,
        )
        .bind(&id)
        .bind(creditor)
        .bind(debtor)
        .bind(rel.credit_limit)
        .bind(rel.balance)
        .bind(active)
        .bind(established)
        .bind(last_transaction)
        .execute(&self.pool)
        .await?;

        debug!("Upserted credit relationship: {}", id);
        Ok(id)
    }

    /// Get a credit relationship by ID
    pub async fn get_credit_relationship(&self, id: &str) -> Result<Option<CreditRelationship>> {
        let row = sqlx::query(
            r#"
            SELECT id, creditor_peer_id, debtor_peer_id, credit_limit, balance,
                   active, established, last_transaction
            FROM credit_relationships WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_credit_relationship(&row)?)),
            None => Ok(None),
        }
    }

    /// Get credit relationship between two peers
    pub async fn get_credit_relationship_between(
        &self,
        creditor: &str,
        debtor: &str,
    ) -> Result<Option<CreditRelationship>> {
        let row = sqlx::query(
            r#"
            SELECT id, creditor_peer_id, debtor_peer_id, credit_limit, balance,
                   active, established, last_transaction
            FROM credit_relationships
            WHERE creditor_peer_id = ? AND debtor_peer_id = ?
            "#,
        )
        .bind(creditor)
        .bind(debtor)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_credit_relationship(&row)?)),
            None => Ok(None),
        }
    }

    /// List all credit relationships for a peer (as creditor or debtor)
    pub async fn list_credit_relationships_for(&self, peer_id: &str) -> Result<Vec<CreditRelationship>> {
        let rows = sqlx::query(
            r#"
            SELECT id, creditor_peer_id, debtor_peer_id, credit_limit, balance,
                   active, established, last_transaction
            FROM credit_relationships
            WHERE creditor_peer_id = ? OR debtor_peer_id = ?
            ORDER BY last_transaction DESC
            "#,
        )
        .bind(peer_id)
        .bind(peer_id)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            results.push(self.row_to_credit_relationship(&row)?);
        }

        Ok(results)
    }

    /// List all active credit relationships
    pub async fn list_active_credit_relationships(&self) -> Result<Vec<CreditRelationship>> {
        let rows = sqlx::query(
            r#"
            SELECT id, creditor_peer_id, debtor_peer_id, credit_limit, balance,
                   active, established, last_transaction
            FROM credit_relationships WHERE active = 1
            ORDER BY last_transaction DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            results.push(self.row_to_credit_relationship(&row)?);
        }

        Ok(results)
    }

    /// Record a credit transaction
    pub async fn record_credit_transaction(
        &self,
        relationship_id: &str,
        amount: f64,
        balance_after: f64,
        description: Option<&str>,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO credit_transactions (id, relationship_id, amount, balance_after, description, timestamp)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(relationship_id)
        .bind(amount)
        .bind(balance_after)
        .bind(description)
        .bind(timestamp)
        .execute(&self.pool)
        .await?;

        debug!("Recorded credit transaction: {}", id);
        Ok(())
    }

    // Helper to convert row to CreditRelationship
    fn row_to_credit_relationship(&self, row: &sqlx::sqlite::SqliteRow) -> Result<CreditRelationship> {
        let creditor: String = row.get("creditor_peer_id");
        let debtor: String = row.get("debtor_peer_id");
        let credit_limit: f64 = row.get("credit_limit");
        let balance: f64 = row.get("balance");
        let active: i32 = row.get("active");
        let established: i64 = row.get("established");
        let last_transaction: i64 = row.get("last_transaction");

        Ok(CreditRelationship {
            creditor: PeerId(creditor),
            debtor: PeerId(debtor),
            credit_limit,
            balance,
            active: active != 0,
            established: Utc.timestamp_opt(established, 0).single().unwrap_or_else(Utc::now),
            last_transaction: Utc.timestamp_opt(last_transaction, 0).single().unwrap_or_else(Utc::now),
        })
    }

    // ========== State Sync Operations ==========

    /// Store a sync key-value pair
    pub async fn set_sync_value(&self, key: &str, value: &[u8]) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO state_sync (key, value, version)
            VALUES (?, ?, 1)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                version = state_sync.version + 1,
                updated_at = strftime('%s', 'now')
            "#,
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a sync value
    pub async fn get_sync_value(&self, key: &str) -> Result<Option<(Vec<u8>, i64)>> {
        let row = sqlx::query("SELECT value, version FROM state_sync WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let value: Vec<u8> = row.get("value");
                let version: i64 = row.get("version");
                Ok(Some((value, version)))
            }
            None => Ok(None),
        }
    }

    /// Delete a sync key
    pub async fn delete_sync_value(&self, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM state_sync WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

// Implement the core StateStore trait
#[async_trait]
impl StateStore for SqliteStore {
    async fn store_peer(&self, info: &PeerInfo) -> CoreResult<()> {
        self.upsert_peer(info, None)
            .await
            .map_err(|e| mycelial_core::MycelialError::Storage(e.to_string()))
    }

    async fn get_peer(&self, id: &PeerId) -> CoreResult<Option<PeerInfo>> {
        match self.get_peer(id.as_str()).await {
            Ok(Some((info, _))) => Ok(Some(info)),
            Ok(None) => Ok(None),
            Err(e) => Err(mycelial_core::MycelialError::Storage(e.to_string())),
        }
    }

    async fn list_peers(&self) -> CoreResult<Vec<PeerInfo>> {
        match self.list_peers().await {
            Ok(peers) => Ok(peers.into_iter().map(|(info, _)| info).collect()),
            Err(e) => Err(mycelial_core::MycelialError::Storage(e.to_string())),
        }
    }

    async fn update_reputation(&self, id: &PeerId, reputation: &Reputation) -> CoreResult<()> {
        self.update_peer_reputation(id.as_str(), reputation)
            .await
            .map_err(|e| mycelial_core::MycelialError::Storage(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_store() -> SqliteStore {
        SqliteStore::new(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_peer_crud() {
        let store = create_test_store().await;

        // Create peer info
        let peer_id = PeerId("test_peer_123".to_string());
        let peer_info = PeerInfo {
            id: peer_id.clone(),
            public_key: vec![1, 2, 3, 4],
            addresses: vec!["/ip4/127.0.0.1/tcp/4001".to_string()],
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            name: Some("Test Peer".to_string()),
        };

        let reputation = Reputation::new(0.75);

        // Store peer
        store.upsert_peer(&peer_info, Some(&reputation)).await.unwrap();

        // Retrieve peer
        let (retrieved, rep) = store.get_peer("test_peer_123").await.unwrap().unwrap();
        assert_eq!(retrieved.id.as_str(), "test_peer_123");
        assert_eq!(retrieved.name, Some("Test Peer".to_string()));
        assert!((rep.score - 0.75).abs() < 0.001);

        // List peers
        let peers = store.list_peers().await.unwrap();
        assert_eq!(peers.len(), 1);

        // Delete peer
        store.delete_peer("test_peer_123").await.unwrap();
        assert!(store.get_peer("test_peer_123").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_message_crud() {
        let store = create_test_store().await;

        // First create the sender peer (foreign key requirement)
        let sender = PeerId("sender_peer".to_string());
        let sender_info = PeerInfo {
            id: sender.clone(),
            public_key: vec![1, 2, 3],
            addresses: vec![],
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            name: Some("Sender".to_string()),
        };
        store.upsert_peer(&sender_info, None).await.unwrap();

        // Create message
        let message = Message::new(MessageType::Content, sender.clone(), b"Hello, world!".to_vec());
        let msg_id = message.id;

        // Store message
        store.store_message(&message).await.unwrap();

        // Retrieve message
        let retrieved = store.get_message(&msg_id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, msg_id);
        assert_eq!(retrieved.sender.as_str(), "sender_peer");
        assert_eq!(retrieved.payload, b"Hello, world!");

        // List messages from sender
        let messages = store.list_messages_from("sender_peer", 10).await.unwrap();
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn test_credit_relationship_crud() {
        let store = create_test_store().await;

        // First create the peer records (foreign key requirement)
        let creditor = PeerId("creditor_peer".to_string());
        let debtor = PeerId("debtor_peer".to_string());

        let creditor_info = PeerInfo {
            id: creditor.clone(),
            public_key: vec![1, 2, 3],
            addresses: vec![],
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            name: Some("Creditor".to_string()),
        };
        store.upsert_peer(&creditor_info, None).await.unwrap();

        let debtor_info = PeerInfo {
            id: debtor.clone(),
            public_key: vec![4, 5, 6],
            addresses: vec![],
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            name: Some("Debtor".to_string()),
        };
        store.upsert_peer(&debtor_info, None).await.unwrap();

        // Create credit relationship
        let rel = CreditRelationship::new(creditor.clone(), debtor.clone(), 100.0);

        // Store relationship
        let rel_id = store.upsert_credit_relationship(&rel).await.unwrap();

        // Retrieve relationship
        let retrieved = store.get_credit_relationship(&rel_id).await.unwrap().unwrap();
        assert_eq!(retrieved.creditor.as_str(), "creditor_peer");
        assert_eq!(retrieved.debtor.as_str(), "debtor_peer");
        assert_eq!(retrieved.credit_limit, 100.0);
        assert_eq!(retrieved.balance, 0.0);

        // Get between peers
        let retrieved = store
            .get_credit_relationship_between("creditor_peer", "debtor_peer")
            .await
            .unwrap()
            .unwrap();
        assert!(retrieved.active);

        // List for peer
        let rels = store.list_credit_relationships_for("creditor_peer").await.unwrap();
        assert_eq!(rels.len(), 1);
    }

    #[tokio::test]
    async fn test_sync_values() {
        let store = create_test_store().await;

        // Set value
        store.set_sync_value("test_key", b"test_value").await.unwrap();

        // Get value
        let (value, version) = store.get_sync_value("test_key").await.unwrap().unwrap();
        assert_eq!(value, b"test_value");
        assert_eq!(version, 1);

        // Update value
        store.set_sync_value("test_key", b"updated_value").await.unwrap();
        let (value, version) = store.get_sync_value("test_key").await.unwrap().unwrap();
        assert_eq!(value, b"updated_value");
        assert_eq!(version, 2);

        // Delete value
        store.delete_sync_value("test_key").await.unwrap();
        assert!(store.get_sync_value("test_key").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_trusted_peers() {
        let store = create_test_store().await;

        // Create peers with different reputation scores
        for i in 0..5 {
            let peer_id = PeerId(format!("peer_{}", i));
            let peer_info = PeerInfo {
                id: peer_id.clone(),
                public_key: vec![i as u8],
                addresses: vec![],
                first_seen: Utc::now(),
                last_seen: Utc::now(),
                name: None,
            };
            let reputation = Reputation::new(0.2 + (i as f64 * 0.15)); // 0.2, 0.35, 0.5, 0.65, 0.8

            store.upsert_peer(&peer_info, Some(&reputation)).await.unwrap();
        }

        // Get trusted peers (threshold 0.5)
        let trusted = store.list_trusted_peers(0.5).await.unwrap();
        assert_eq!(trusted.len(), 3); // peer_2, peer_3, peer_4
    }
}
