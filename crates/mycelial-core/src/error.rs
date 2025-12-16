//! Comprehensive error types for the Mycelia network
//!
//! This module provides detailed error types for all operations
//! in the mycelial network.

use thiserror::Error;

/// Main error type for the Mycelia network
#[derive(Error, Debug)]
pub enum MycelialError {
    // ===== Identity & Cryptography Errors =====
    /// Invalid cryptographic signature
    #[error("Invalid signature")]
    InvalidSignature,

    /// Invalid public key format
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    /// Invalid DID format
    #[error("Invalid DID format: {0}")]
    InvalidDid(String),

    /// Key generation failed
    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),

    // ===== Peer & Network Errors =====
    /// Peer was not found
    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    /// Connection to peer failed
    #[error("Connection failed to peer {peer}: {reason}")]
    ConnectionFailed { peer: String, reason: String },

    /// Network timeout
    #[error("Network timeout after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// Maximum connections reached
    #[error("Maximum connections reached: {max}")]
    MaxConnectionsReached { max: u32 },

    /// Peer is not trusted
    #[error("Peer {peer} is not trusted (score: {score})")]
    UntrustedPeer { peer: String, score: f64 },

    // ===== Content Errors =====
    /// Content was not found
    #[error("Content not found: {0}")]
    ContentNotFound(String),

    /// Content verification failed
    #[error("Content verification failed: expected {expected}, got {actual}")]
    ContentVerificationFailed { expected: String, actual: String },

    /// Content too large
    #[error("Content too large: {size} bytes exceeds maximum {max} bytes")]
    ContentTooLarge { size: u64, max: u64 },

    /// Invalid content type
    #[error("Invalid content type: {0}")]
    InvalidContentType(String),

    // ===== Credit & Economics Errors =====
    /// Insufficient credit for operation
    #[error("Insufficient credit: required {required}, available {available}")]
    InsufficientCredit { required: f64, available: f64 },

    /// Credit relationship not found
    #[error("Credit relationship not found between {creditor} and {debtor}")]
    CreditRelationshipNotFound { creditor: String, debtor: String },

    /// Credit limit exceeded
    #[error("Credit limit exceeded: requested {requested}, limit {limit}")]
    CreditLimitExceeded { requested: f64, limit: f64 },

    /// Credit relationship is inactive
    #[error("Credit relationship is inactive")]
    InactiveCreditRelationship,

    // ===== Storage Errors =====
    /// Storage operation failed
    #[error("Storage error: {0}")]
    Storage(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Data not found in storage
    #[error("Data not found: {key}")]
    DataNotFound { key: String },

    /// Storage capacity exceeded
    #[error("Storage capacity exceeded: {used} of {capacity} bytes")]
    StorageCapacityExceeded { used: u64, capacity: u64 },

    // ===== Serialization Errors =====
    /// Serialization failed
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization failed
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),

    // ===== Module Errors =====
    /// Module not found
    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    /// Module initialization failed
    #[error("Module initialization failed for {module}: {reason}")]
    ModuleInitFailed { module: String, reason: String },

    /// Module is not running
    #[error("Module {0} is not running")]
    ModuleNotRunning(String),

    /// Invalid module state transition
    #[error("Invalid module state transition from {from:?} to {to:?}")]
    InvalidModuleStateTransition {
        from: crate::module::ModuleState,
        to: crate::module::ModuleState,
    },

    // ===== Governance Errors =====
    /// Proposal not found
    #[error("Proposal not found: {0}")]
    ProposalNotFound(String),

    /// Voting period ended
    #[error("Voting period has ended for proposal {0}")]
    VotingPeriodEnded(String),

    /// Already voted
    #[error("Already voted on proposal {0}")]
    AlreadyVoted(String),

    /// Insufficient voting power
    #[error("Insufficient voting power: {available} < {required}")]
    InsufficientVotingPower { available: f64, required: f64 },

    // ===== Configuration Errors =====
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Configuration file not found
    #[error("Configuration file not found: {0}")]
    ConfigNotFound(String),

    // ===== General Errors =====
    /// Operation was cancelled
    #[error("Operation cancelled")]
    Cancelled,

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Feature not implemented
    #[error("Feature not implemented: {0}")]
    NotImplemented(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Rate limited
    #[error("Rate limited: retry after {retry_after_secs} seconds")]
    RateLimited { retry_after_secs: u64 },
}

impl MycelialError {
    /// Check if this error is retriable
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            MycelialError::Timeout { .. }
                | MycelialError::ConnectionFailed { .. }
                | MycelialError::RateLimited { .. }
        )
    }

    /// Check if this error is a client error (bad input)
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            MycelialError::InvalidSignature
                | MycelialError::InvalidPublicKey(_)
                | MycelialError::InvalidDid(_)
                | MycelialError::InvalidMessageFormat(_)
                | MycelialError::InvalidConfig(_)
                | MycelialError::InvalidContentType(_)
                | MycelialError::ContentTooLarge { .. }
        )
    }

    /// Get an error code for this error
    pub fn error_code(&self) -> &'static str {
        match self {
            MycelialError::InvalidSignature => "INVALID_SIGNATURE",
            MycelialError::InvalidPublicKey(_) => "INVALID_PUBLIC_KEY",
            MycelialError::InvalidDid(_) => "INVALID_DID",
            MycelialError::KeyGenerationFailed(_) => "KEY_GENERATION_FAILED",
            MycelialError::PeerNotFound(_) => "PEER_NOT_FOUND",
            MycelialError::ConnectionFailed { .. } => "CONNECTION_FAILED",
            MycelialError::Timeout { .. } => "TIMEOUT",
            MycelialError::MaxConnectionsReached { .. } => "MAX_CONNECTIONS",
            MycelialError::UntrustedPeer { .. } => "UNTRUSTED_PEER",
            MycelialError::ContentNotFound(_) => "CONTENT_NOT_FOUND",
            MycelialError::ContentVerificationFailed { .. } => "CONTENT_VERIFICATION_FAILED",
            MycelialError::ContentTooLarge { .. } => "CONTENT_TOO_LARGE",
            MycelialError::InvalidContentType(_) => "INVALID_CONTENT_TYPE",
            MycelialError::InsufficientCredit { .. } => "INSUFFICIENT_CREDIT",
            MycelialError::CreditRelationshipNotFound { .. } => "CREDIT_RELATIONSHIP_NOT_FOUND",
            MycelialError::CreditLimitExceeded { .. } => "CREDIT_LIMIT_EXCEEDED",
            MycelialError::InactiveCreditRelationship => "INACTIVE_CREDIT_RELATIONSHIP",
            MycelialError::Storage(_) => "STORAGE_ERROR",
            MycelialError::Database(_) => "DATABASE_ERROR",
            MycelialError::DataNotFound { .. } => "DATA_NOT_FOUND",
            MycelialError::StorageCapacityExceeded { .. } => "STORAGE_CAPACITY_EXCEEDED",
            MycelialError::Serialization(_) => "SERIALIZATION_ERROR",
            MycelialError::Deserialization(_) => "DESERIALIZATION_ERROR",
            MycelialError::InvalidMessageFormat(_) => "INVALID_MESSAGE_FORMAT",
            MycelialError::ModuleNotFound(_) => "MODULE_NOT_FOUND",
            MycelialError::ModuleInitFailed { .. } => "MODULE_INIT_FAILED",
            MycelialError::ModuleNotRunning(_) => "MODULE_NOT_RUNNING",
            MycelialError::InvalidModuleStateTransition { .. } => "INVALID_MODULE_STATE",
            MycelialError::ProposalNotFound(_) => "PROPOSAL_NOT_FOUND",
            MycelialError::VotingPeriodEnded(_) => "VOTING_PERIOD_ENDED",
            MycelialError::AlreadyVoted(_) => "ALREADY_VOTED",
            MycelialError::InsufficientVotingPower { .. } => "INSUFFICIENT_VOTING_POWER",
            MycelialError::InvalidConfig(_) => "INVALID_CONFIG",
            MycelialError::ConfigNotFound(_) => "CONFIG_NOT_FOUND",
            MycelialError::Cancelled => "CANCELLED",
            MycelialError::Internal(_) => "INTERNAL_ERROR",
            MycelialError::NotImplemented(_) => "NOT_IMPLEMENTED",
            MycelialError::PermissionDenied(_) => "PERMISSION_DENIED",
            MycelialError::RateLimited { .. } => "RATE_LIMITED",
        }
    }
}

/// Result type alias for Mycelia operations
pub type Result<T> = std::result::Result<T, MycelialError>;

// Conversion implementations for common error types
impl From<std::io::Error> for MycelialError {
    fn from(err: std::io::Error) -> Self {
        MycelialError::Storage(err.to_string())
    }
}

impl From<serde_json::Error> for MycelialError {
    fn from(err: serde_json::Error) -> Self {
        MycelialError::Serialization(err.to_string())
    }
}

impl From<serde_cbor::Error> for MycelialError {
    fn from(err: serde_cbor::Error) -> Self {
        MycelialError::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = MycelialError::PeerNotFound("test".to_string());
        assert_eq!(err.error_code(), "PEER_NOT_FOUND");
    }

    #[test]
    fn test_is_retriable() {
        assert!(MycelialError::Timeout { duration_ms: 1000 }.is_retriable());
        assert!(!MycelialError::InvalidSignature.is_retriable());
    }

    #[test]
    fn test_is_client_error() {
        assert!(MycelialError::InvalidSignature.is_client_error());
        assert!(!MycelialError::Internal("test".to_string()).is_client_error());
    }
}
