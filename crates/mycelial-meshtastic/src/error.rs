//! Error types for Meshtastic bridge operations
//!
//! This module provides comprehensive error handling for all Meshtastic
//! bridge operations including serial communication, protocol translation,
//! and message bridging.

use thiserror::Error;

/// Main error type for Meshtastic bridge operations
#[derive(Error, Debug)]
pub enum MeshtasticError {
    // ===== Serial/Interface Errors =====
    /// Serial port not found
    #[error("Serial port not found: {0}")]
    PortNotFound(String),

    /// Serial port open failed
    #[error("Failed to open serial port {port}: {reason}")]
    PortOpenFailed {
        /// Port path
        port: String,
        /// Failure reason
        reason: String,
    },

    /// Serial read error
    #[error("Serial read error: {0}")]
    ReadError(String),

    /// Serial write error
    #[error("Serial write error: {0}")]
    WriteError(String),

    /// Serial port disconnected
    #[error("Serial port disconnected")]
    Disconnected,

    /// Connection timeout
    #[error("Connection timeout after {duration_ms}ms")]
    ConnectionTimeout {
        /// Timeout duration in milliseconds
        duration_ms: u64,
    },

    // ===== Protocol Errors =====
    /// Invalid magic number in packet
    #[error("Invalid magic number: expected 0x94C3, got 0x{got:04X}")]
    InvalidMagic {
        /// The received magic number
        got: u16,
    },

    /// Protobuf decode error
    #[error("Protobuf decode error: {0}")]
    ProtobufDecode(String),

    /// Protobuf encode error
    #[error("Protobuf encode error: {0}")]
    ProtobufEncode(String),

    /// Invalid packet format
    #[error("Invalid packet format: {0}")]
    InvalidPacket(String),

    /// Unknown port number
    #[error("Unknown Meshtastic port number: {0}")]
    UnknownPort(u32),

    // ===== Message Translation Errors =====
    /// Message too large for LoRa (max 237 bytes)
    #[error("Message too large: {size} bytes exceeds LoRa maximum of {max} bytes")]
    MessageTooLarge {
        /// Actual message size
        size: usize,
        /// Maximum allowed size
        max: usize,
    },

    /// Failed to translate message
    #[error("Message translation failed: {0}")]
    TranslationFailed(String),

    /// Unsupported message type for LoRa bridge
    #[error("Unsupported message type for LoRa bridge: {0}")]
    UnsupportedMessageType(String),

    /// Channel mapping not found
    #[error("No channel mapping for topic: {0}")]
    NoChannelMapping(String),

    // ===== Compression Errors =====
    /// Compression or decompression failed
    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    /// Message chunk reassembly failed
    #[error("Chunk reassembly failed: {0}")]
    ReassemblyFailed(String),

    // ===== Bridge Errors =====
    /// Bridge not running
    #[error("Meshtastic bridge is not running")]
    BridgeNotRunning,

    /// Bridge already running
    #[error("Meshtastic bridge is already running")]
    BridgeAlreadyRunning,

    /// Duplicate message detected
    #[error("Duplicate message detected: packet_id={packet_id}")]
    DuplicateMessage {
        /// The duplicate packet ID
        packet_id: u32,
    },

    /// Hop limit exceeded
    #[error("Hop limit exceeded: {hops} > {max_hops}")]
    HopLimitExceeded {
        /// Current hop count
        hops: u8,
        /// Maximum allowed hops
        max_hops: u8,
    },

    // ===== Node/Identity Errors =====
    /// Unknown node ID
    #[error("Unknown node ID: {0}")]
    UnknownNode(u32),

    /// Invalid node ID format
    #[error("Invalid node ID format: {0}")]
    InvalidNodeId(String),

    /// Node ID mapping failed
    #[error("Failed to map node ID {node_id} to PeerId: {reason}")]
    NodeMappingFailed {
        /// Meshtastic node ID
        node_id: u32,
        /// Mapping failure reason
        reason: String,
    },

    // ===== Configuration Errors =====
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Missing required configuration
    #[error("Missing required configuration: {0}")]
    MissingConfig(String),

    // ===== General Errors =====
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Channel send error
    #[error("Channel send error: {0}")]
    ChannelError(String),

    /// Channel closed
    #[error("Channel closed")]
    ChannelClosed,

    /// IO error wrapper
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl MeshtasticError {
    /// Check if this error is recoverable/retriable
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            MeshtasticError::ConnectionTimeout { .. }
                | MeshtasticError::Disconnected
                | MeshtasticError::ReadError(_)
                | MeshtasticError::WriteError(_)
        )
    }

    /// Check if this is a protocol error (bad data from device)
    pub fn is_protocol_error(&self) -> bool {
        matches!(
            self,
            MeshtasticError::InvalidMagic { .. }
                | MeshtasticError::ProtobufDecode(_)
                | MeshtasticError::InvalidPacket(_)
                | MeshtasticError::UnknownPort(_)
        )
    }

    /// Get an error code for logging/metrics
    pub fn error_code(&self) -> &'static str {
        match self {
            MeshtasticError::PortNotFound(_) => "PORT_NOT_FOUND",
            MeshtasticError::PortOpenFailed { .. } => "PORT_OPEN_FAILED",
            MeshtasticError::ReadError(_) => "READ_ERROR",
            MeshtasticError::WriteError(_) => "WRITE_ERROR",
            MeshtasticError::Disconnected => "DISCONNECTED",
            MeshtasticError::ConnectionTimeout { .. } => "CONNECTION_TIMEOUT",
            MeshtasticError::InvalidMagic { .. } => "INVALID_MAGIC",
            MeshtasticError::ProtobufDecode(_) => "PROTOBUF_DECODE",
            MeshtasticError::ProtobufEncode(_) => "PROTOBUF_ENCODE",
            MeshtasticError::InvalidPacket(_) => "INVALID_PACKET",
            MeshtasticError::UnknownPort(_) => "UNKNOWN_PORT",
            MeshtasticError::MessageTooLarge { .. } => "MESSAGE_TOO_LARGE",
            MeshtasticError::TranslationFailed(_) => "TRANSLATION_FAILED",
            MeshtasticError::UnsupportedMessageType(_) => "UNSUPPORTED_MESSAGE",
            MeshtasticError::NoChannelMapping(_) => "NO_CHANNEL_MAPPING",
            MeshtasticError::CompressionFailed(_) => "COMPRESSION_FAILED",
            MeshtasticError::ReassemblyFailed(_) => "REASSEMBLY_FAILED",
            MeshtasticError::BridgeNotRunning => "BRIDGE_NOT_RUNNING",
            MeshtasticError::BridgeAlreadyRunning => "BRIDGE_ALREADY_RUNNING",
            MeshtasticError::DuplicateMessage { .. } => "DUPLICATE_MESSAGE",
            MeshtasticError::HopLimitExceeded { .. } => "HOP_LIMIT_EXCEEDED",
            MeshtasticError::UnknownNode(_) => "UNKNOWN_NODE",
            MeshtasticError::InvalidNodeId(_) => "INVALID_NODE_ID",
            MeshtasticError::NodeMappingFailed { .. } => "NODE_MAPPING_FAILED",
            MeshtasticError::InvalidConfig(_) => "INVALID_CONFIG",
            MeshtasticError::MissingConfig(_) => "MISSING_CONFIG",
            MeshtasticError::Internal(_) => "INTERNAL_ERROR",
            MeshtasticError::ChannelError(_) => "CHANNEL_ERROR",
            MeshtasticError::ChannelClosed => "CHANNEL_CLOSED",
            MeshtasticError::Io(_) => "IO_ERROR",
        }
    }
}

/// Result type alias for Meshtastic operations
pub type Result<T> = std::result::Result<T, MeshtasticError>;

// Conversion from prost decode error
impl From<prost::DecodeError> for MeshtasticError {
    fn from(err: prost::DecodeError) -> Self {
        MeshtasticError::ProtobufDecode(err.to_string())
    }
}

// Conversion from prost encode error
impl From<prost::EncodeError> for MeshtasticError {
    fn from(err: prost::EncodeError) -> Self {
        MeshtasticError::ProtobufEncode(err.to_string())
    }
}

// Conversion from serialport error (only when serial feature is enabled)
#[cfg(feature = "serial")]
impl From<serialport::Error> for MeshtasticError {
    fn from(err: serialport::Error) -> Self {
        match err.kind {
            serialport::ErrorKind::NoDevice => MeshtasticError::PortNotFound(err.description),
            serialport::ErrorKind::Io(kind) => {
                MeshtasticError::Io(std::io::Error::new(kind, err.description))
            }
            _ => MeshtasticError::PortOpenFailed {
                port: String::new(),
                reason: err.description,
            },
        }
    }
}

// Conversion from tokio mpsc send error
impl<T> From<tokio::sync::mpsc::error::SendError<T>> for MeshtasticError {
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        MeshtasticError::ChannelError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = MeshtasticError::PortNotFound("/dev/ttyUSB0".to_string());
        assert_eq!(err.error_code(), "PORT_NOT_FOUND");
    }

    #[test]
    fn test_is_retriable() {
        assert!(MeshtasticError::Disconnected.is_retriable());
        assert!(MeshtasticError::ConnectionTimeout { duration_ms: 5000 }.is_retriable());
        assert!(!MeshtasticError::InvalidMagic { got: 0x1234 }.is_retriable());
    }

    #[test]
    fn test_is_protocol_error() {
        assert!(MeshtasticError::InvalidMagic { got: 0x1234 }.is_protocol_error());
        assert!(MeshtasticError::ProtobufDecode("test".to_string()).is_protocol_error());
        assert!(!MeshtasticError::Disconnected.is_protocol_error());
    }

    #[test]
    fn test_message_too_large() {
        let err = MeshtasticError::MessageTooLarge {
            size: 300,
            max: 237,
        };
        assert!(err.to_string().contains("300"));
        assert!(err.to_string().contains("237"));
    }
}
