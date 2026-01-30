//! Interface implementations for Meshtastic device communication
//!
//! This module provides different interface implementations for connecting
//! to Meshtastic devices:
//!
//! - [`serial::SerialInterface`] - Serial port communication (requires `serial` feature)
//! - [`tcp::TcpInterface`] - TCP connection (requires `tcp` feature)
//! - [`ble::BleInterface`] - Bluetooth LE (requires `ble` feature)
//!
//! # Feature Requirements
//!
//! - `serial`: Requires `libudev-dev` and `pkg-config` on Linux
//!   ```bash
//!   # Ubuntu/Debian
//!   apt install libudev-dev pkg-config
//!   ```
//!
//! - `ble`: Requires BlueZ development files on Linux
//!   ```bash
//!   apt install libdbus-1-dev
//!   ```

#[cfg(feature = "serial")]
mod serial;

#[cfg(feature = "serial")]
pub use serial::SerialInterface;

#[cfg(feature = "tcp")]
mod tcp;
#[cfg(feature = "tcp")]
pub use tcp::TcpInterface;

#[cfg(feature = "ble")]
mod ble;
#[cfg(feature = "ble")]
pub use ble::BleInterface;

use crate::error::Result;
use async_trait::async_trait;
use bytes::Bytes;

/// Trait for Meshtastic device interfaces
///
/// This trait abstracts over different connection methods (serial, TCP, BLE)
/// providing a unified API for reading and writing packets.
#[async_trait]
pub trait MeshtasticInterface: Send + Sync {
    /// Connect to the Meshtastic device
    async fn connect(&mut self) -> Result<()>;

    /// Disconnect from the device
    async fn disconnect(&mut self) -> Result<()>;

    /// Check if currently connected
    fn is_connected(&self) -> bool;

    /// Read a packet from the device
    ///
    /// Returns `None` if no complete packet is available yet.
    /// Returns `Err` on connection/read errors.
    async fn read_packet(&mut self) -> Result<Option<Bytes>>;

    /// Write a packet to the device
    async fn write_packet(&mut self, packet: &[u8]) -> Result<()>;

    /// Get the interface name (for logging)
    fn name(&self) -> &str;
}

/// Connection state for interfaces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connection in progress
    Connecting,
    /// Successfully connected
    Connected,
    /// Connection lost, may reconnect
    Reconnecting,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "disconnected"),
            ConnectionState::Connecting => write!(f, "connecting"),
            ConnectionState::Connected => write!(f, "connected"),
            ConnectionState::Reconnecting => write!(f, "reconnecting"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_state_display() {
        assert_eq!(ConnectionState::Connected.to_string(), "connected");
        assert_eq!(ConnectionState::Disconnected.to_string(), "disconnected");
    }
}
