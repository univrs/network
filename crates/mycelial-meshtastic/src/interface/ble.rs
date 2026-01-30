//! Bluetooth Low Energy interface for Meshtastic device communication
//!
//! This module provides BLE connectivity to Meshtastic devices.
//!
//! # Requirements
//!
//! Enable the `ble` feature in Cargo.toml to use this interface.
//!
//! On Linux, you'll also need:
//! ```bash
//! apt install libdbus-1-dev
//! ```

use crate::error::Result;
use async_trait::async_trait;
use bytes::Bytes;

use super::{ConnectionState, MeshtasticInterface};

/// BLE interface for connecting to Meshtastic devices over Bluetooth
///
/// Meshtastic devices expose a BLE GATT service for communication.
/// This interface uses that service for packet exchange.
pub struct BleInterface {
    device_name: String,
    state: ConnectionState,
}

impl BleInterface {
    /// Create a new BLE interface
    ///
    /// # Arguments
    ///
    /// * `device_name` - The Bluetooth device name or address
    pub fn new(device_name: impl Into<String>) -> Self {
        Self {
            device_name: device_name.into(),
            state: ConnectionState::Disconnected,
        }
    }
}

#[async_trait]
impl MeshtasticInterface for BleInterface {
    async fn connect(&mut self) -> Result<()> {
        self.state = ConnectionState::Connecting;
        // TODO: Implement BLE connection
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
        // TODO: Implement BLE read
        Ok(None)
    }

    async fn write_packet(&mut self, _packet: &[u8]) -> Result<()> {
        // TODO: Implement BLE write
        Ok(())
    }

    fn name(&self) -> &str {
        &self.device_name
    }
}
