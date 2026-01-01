//! Module trait for the substrate architecture
//!
//! All functional modules (Social, Orchestration, Economics) implement this trait
//! to integrate with the Mycelia substrate layer.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{MycelialError, Result};

/// State of a module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleState {
    /// Module is initializing
    Initializing,
    /// Module is running normally
    Running,
    /// Module is paused
    Paused,
    /// Module has stopped
    Stopped,
    /// Module encountered an error
    Error,
}

/// Information about a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Unique module identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Module version
    pub version: String,
    /// Module description
    pub description: String,
    /// Topics this module subscribes to
    pub subscribed_topics: Vec<String>,
    /// Topics this module publishes to
    pub published_topics: Vec<String>,
}

/// Metrics reported by a module
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleMetrics {
    /// Number of messages processed
    pub messages_processed: u64,
    /// Number of messages published
    pub messages_published: u64,
    /// Number of errors encountered
    pub errors: u64,
    /// Average processing time in microseconds
    pub avg_processing_time_us: f64,
    /// Custom metrics
    pub custom: HashMap<String, f64>,
}

/// All modules implement this trait to integrate with the substrate
#[async_trait]
pub trait MyceliaModule: Send + Sync {
    /// Get the unique module identifier
    fn id(&self) -> &str;

    /// Get detailed module information
    fn info(&self) -> ModuleInfo;

    /// Get the topics this module subscribes to
    fn subscribed_topics(&self) -> Vec<String>;

    /// Handle an incoming message from the network
    ///
    /// # Arguments
    /// * `topic` - The gossipsub topic the message arrived on
    /// * `payload` - The raw message payload
    /// * `source` - The peer ID of the message source (optional for system messages)
    async fn handle_message(
        &mut self,
        topic: &str,
        payload: &[u8],
        source: Option<&str>,
    ) -> Result<Option<Vec<u8>>>;

    /// Periodic tick for background processing
    ///
    /// Called at regular intervals to allow modules to perform
    /// maintenance tasks, process queues, etc.
    async fn tick(&mut self) -> Result<()>;

    /// Get the current module state
    fn state(&self) -> ModuleState;

    /// Get current module metrics
    fn metrics(&self) -> ModuleMetrics;

    /// Initialize the module
    async fn initialize(&mut self) -> Result<()>;

    /// Shutdown the module gracefully
    async fn shutdown(&mut self) -> Result<()>;
}

/// Standard gossipsub topics for the Mycelia network
pub mod topics {
    /// Social content (posts, media)
    pub const CONTENT: &str = "/mycelia/1.0.0/content";
    /// Reputation updates
    pub const REPUTATION: &str = "/mycelia/1.0.0/reputation";
    /// Orchestration (scheduling, workload events)
    pub const ORCHESTRATION: &str = "/mycelia/1.0.0/orchestration";
    /// Economics (credit, transactions)
    pub const ECONOMICS: &str = "/mycelia/1.0.0/economics";
    /// Governance (proposals, votes)
    pub const GOVERNANCE: &str = "/mycelia/1.0.0/governance";
    /// Math Engine (WASM formula modules)
    pub const FORMULAS: &str = "/mycelia/1.0.0/formulas";
    /// System messages (peer discovery, health)
    pub const SYSTEM: &str = "/mycelia/1.0.0/system";
}

/// A message envelope for cross-module communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMessage {
    /// Source module ID
    pub source_module: String,
    /// Target module ID (None = broadcast)
    pub target_module: Option<String>,
    /// Message type identifier
    pub message_type: String,
    /// Serialized payload
    pub payload: Vec<u8>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Request ID for request-response patterns
    pub request_id: Option<uuid::Uuid>,
}

impl ModuleMessage {
    /// Create a new module message
    pub fn new(
        source: impl Into<String>,
        message_type: impl Into<String>,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            source_module: source.into(),
            target_module: None,
            message_type: message_type.into(),
            payload,
            timestamp: chrono::Utc::now(),
            request_id: None,
        }
    }

    /// Create a targeted message to a specific module
    pub fn to_module(
        source: impl Into<String>,
        target: impl Into<String>,
        message_type: impl Into<String>,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            source_module: source.into(),
            target_module: Some(target.into()),
            message_type: message_type.into(),
            payload,
            timestamp: chrono::Utc::now(),
            request_id: None,
        }
    }

    /// Create a request message expecting a response
    pub fn request(
        source: impl Into<String>,
        target: impl Into<String>,
        message_type: impl Into<String>,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            source_module: source.into(),
            target_module: Some(target.into()),
            message_type: message_type.into(),
            payload,
            timestamp: chrono::Utc::now(),
            request_id: Some(uuid::Uuid::new_v4()),
        }
    }

    /// Create a response to a request
    pub fn response(&self, source: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            source_module: source.into(),
            target_module: Some(self.source_module.clone()),
            message_type: format!("{}_response", self.message_type),
            payload,
            timestamp: chrono::Utc::now(),
            request_id: self.request_id,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_cbor::to_vec(self).map_err(|e| MycelialError::Serialization(e.to_string()))
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_cbor::from_slice(bytes).map_err(|e| MycelialError::Serialization(e.to_string()))
    }
}

/// Registry for managing modules
pub struct ModuleRegistry {
    modules: HashMap<String, Box<dyn MyceliaModule>>,
}

impl ModuleRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// Register a module
    pub fn register(&mut self, module: Box<dyn MyceliaModule>) {
        let id = module.id().to_string();
        self.modules.insert(id, module);
    }

    /// Get a module by ID
    pub fn get(&self, id: &str) -> Option<&dyn MyceliaModule> {
        self.modules.get(id).map(|m| m.as_ref())
    }

    /// Get a mutable reference to a module
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Box<dyn MyceliaModule>> {
        self.modules.get_mut(id)
    }

    /// List all registered module IDs
    pub fn list(&self) -> Vec<&str> {
        self.modules.keys().map(|s| s.as_str()).collect()
    }

    /// Get all modules subscribed to a topic
    pub fn modules_for_topic(&self, topic: &str) -> Vec<&str> {
        self.modules
            .iter()
            .filter(|(_, m)| m.subscribed_topics().iter().any(|t| t == topic))
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Initialize all modules
    pub async fn initialize_all(&mut self) -> Result<()> {
        for module in self.modules.values_mut() {
            module.initialize().await?;
        }
        Ok(())
    }

    /// Shutdown all modules
    pub async fn shutdown_all(&mut self) -> Result<()> {
        for module in self.modules.values_mut() {
            module.shutdown().await?;
        }
        Ok(())
    }

    /// Tick all modules
    pub async fn tick_all(&mut self) -> Result<()> {
        for module in self.modules.values_mut() {
            module.tick().await?;
        }
        Ok(())
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_message_serialization() {
        let msg = ModuleMessage::new("social", "post_created", b"test payload".to_vec());

        let bytes = msg.to_bytes().unwrap();
        let recovered = ModuleMessage::from_bytes(&bytes).unwrap();

        assert_eq!(recovered.source_module, "social");
        assert_eq!(recovered.message_type, "post_created");
        assert_eq!(recovered.payload, b"test payload");
    }

    #[test]
    fn test_module_request_response() {
        let request = ModuleMessage::request("social", "economics", "get_balance", vec![]);
        let response = request.response("economics", b"100".to_vec());

        assert_eq!(response.target_module, Some("social".to_string()));
        assert_eq!(response.request_id, request.request_id);
    }
}
