//! TCP interface for Meshtastic device communication
//!
//! This module provides TCP socket connectivity to Meshtastic devices
//! that expose a network interface.
//!
//! # Requirements
//!
//! Enable the `tcp` feature in Cargo.toml to use this interface.

use crate::error::Result;
use async_trait::async_trait;
use bytes::Bytes;

use super::{ConnectionState, MeshtasticInterface};

/// TCP interface for connecting to Meshtastic devices over network
///
/// Some Meshtastic devices can expose a TCP socket for communication.
/// This interface connects to that socket.
pub struct TcpInterface {
    address: String,
    state: ConnectionState,
}

impl TcpInterface {
    /// Create a new TCP interface
    ///
    /// # Arguments
    ///
    /// * `address` - The address to connect to (e.g., "192.168.1.100:4403")
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            state: ConnectionState::Disconnected,
        }
    }
}

#[async_trait]
impl MeshtasticInterface for TcpInterface {
    async fn connect(&mut self) -> Result<()> {
        self.state = ConnectionState::Connecting;
        // TODO: Implement TCP connection
        self.state = ConnectionState::Connected;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.state = ConnectionState::Disconnected;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }

    async fn read_packet(&mut self) -> Result<Option<Bytes>> {
        // TODO: Implement TCP read
        Ok(None)
    }

    async fn write_packet(&mut self, _packet: &[u8]) -> Result<()> {
        // TODO: Implement TCP write
        Ok(())
    }

    fn name(&self) -> &str {
        &self.address
    }
}
