//! Raft configuration options

/// Configuration for the Raft consensus layer
#[derive(Debug, Clone)]
pub struct RaftConfig {
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval: u64,
    /// Minimum election timeout in milliseconds
    pub election_timeout_min: u64,
    /// Maximum election timeout in milliseconds
    pub election_timeout_max: u64,
    /// Maximum entries per append request
    pub max_payload_entries: u64,
    /// Enable heartbeat (set false for testing)
    pub enable_heartbeat: bool,
    /// Enable leader election (set false for testing)
    pub enable_elect: bool,
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: 100,
            election_timeout_min: 300,
            election_timeout_max: 500,
            max_payload_entries: 100,
            enable_heartbeat: true,
            enable_elect: true,
        }
    }
}

impl RaftConfig {
    /// Create a configuration optimized for testing
    pub fn for_testing() -> Self {
        Self {
            heartbeat_interval: 50,
            election_timeout_min: 150,
            election_timeout_max: 300,
            max_payload_entries: 10,
            enable_heartbeat: true,
            enable_elect: true,
        }
    }

    /// Create a configuration optimized for low-latency networks
    pub fn low_latency() -> Self {
        Self {
            heartbeat_interval: 50,
            election_timeout_min: 150,
            election_timeout_max: 300,
            max_payload_entries: 200,
            enable_heartbeat: true,
            enable_elect: true,
        }
    }

    /// Create a configuration optimized for high-latency networks
    pub fn high_latency() -> Self {
        Self {
            heartbeat_interval: 500,
            election_timeout_min: 1500,
            election_timeout_max: 3000,
            max_payload_entries: 50,
            enable_heartbeat: true,
            enable_elect: true,
        }
    }
}
