//! Serial port interface for Meshtastic devices
//!
//! This module provides async serial communication with Meshtastic devices
//! using tokio-serial. It handles packet framing with the Meshtastic protocol
//! magic number (0x94C3).

use crate::config::{DEFAULT_BAUD_RATE, DEFAULT_TIMEOUT_MS, MESHTASTIC_MAGIC};
use crate::error::{MeshtasticError, Result};
use crate::interface::{ConnectionState, MeshtasticInterface};
use async_trait::async_trait;
use bytes::{Buf, Bytes, BytesMut};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tracing::{debug, error, info, trace, warn};

/// Buffer size for reading from serial port
const READ_BUFFER_SIZE: usize = 512;

/// Minimum packet size (magic + length + minimal protobuf)
const MIN_PACKET_SIZE: usize = 4;

/// Serial interface for Meshtastic communication
///
/// This interface uses the Meshtastic serial protocol with framing:
/// - 2 bytes: Magic number (0x94C3, big-endian)
/// - 2 bytes: Packet length (big-endian)
/// - N bytes: Protobuf payload
pub struct SerialInterface {
    /// Serial port path
    port_path: PathBuf,

    /// Baud rate
    baud_rate: u32,

    /// Connection timeout
    timeout: Duration,

    /// Serial stream (when connected)
    stream: Option<SerialStream>,

    /// Current connection state
    state: ConnectionState,

    /// Read buffer for accumulating partial packets
    read_buffer: BytesMut,

    /// Interface name for logging
    name: String,
}

impl SerialInterface {
    /// Create a new serial interface
    pub fn new(port: impl AsRef<Path>) -> Self {
        let port_path = port.as_ref().to_path_buf();
        let name = format!("serial:{}", port_path.display());

        Self {
            port_path,
            baud_rate: DEFAULT_BAUD_RATE,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            stream: None,
            state: ConnectionState::Disconnected,
            read_buffer: BytesMut::with_capacity(READ_BUFFER_SIZE * 2),
            name,
        }
    }

    /// Create with custom baud rate
    pub fn with_baud_rate(mut self, baud_rate: u32) -> Self {
        self.baud_rate = baud_rate;
        self
    }

    /// Create with custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Get the port path
    pub fn port_path(&self) -> &Path {
        &self.port_path
    }

    /// Get the current connection state
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Try to parse a complete packet from the read buffer
    ///
    /// Meshtastic serial protocol:
    /// - Bytes 0-1: Magic (0x94C3)
    /// - Bytes 2-3: Length (big-endian u16)
    /// - Bytes 4+: Protobuf payload
    fn try_parse_packet(&mut self) -> Result<Option<Bytes>> {
        // Need at least magic + length
        if self.read_buffer.len() < MIN_PACKET_SIZE {
            return Ok(None);
        }

        // Check for magic number
        let magic = u16::from_be_bytes([self.read_buffer[0], self.read_buffer[1]]);

        if magic != MESHTASTIC_MAGIC {
            // Not a valid packet start - scan for magic
            if let Some(pos) = self.find_magic() {
                warn!(discarded = pos, "Discarding bytes before magic number");
                self.read_buffer.advance(pos);
            } else {
                // No magic found, clear buffer except last byte (might be partial)
                let keep = if self.read_buffer.last() == Some(&0x94) {
                    1
                } else {
                    0
                };
                let discard = self.read_buffer.len() - keep;
                if discard > 0 {
                    warn!(discarded = discard, "Discarding buffer without magic");
                    self.read_buffer.advance(discard);
                }
                return Ok(None);
            }

            // Re-check buffer size after advancing
            if self.read_buffer.len() < MIN_PACKET_SIZE {
                return Ok(None);
            }
        }

        // Read packet length
        let length = u16::from_be_bytes([self.read_buffer[2], self.read_buffer[3]]) as usize;

        // Sanity check length
        if length > READ_BUFFER_SIZE {
            warn!(length, "Packet length too large, likely corrupt");
            // Skip this magic and try to find next
            self.read_buffer.advance(2);
            return Err(MeshtasticError::InvalidPacket(format!(
                "Packet length {} exceeds maximum",
                length
            )));
        }

        // Check if we have the complete packet
        let total_size = 4 + length; // magic(2) + length(2) + payload
        if self.read_buffer.len() < total_size {
            trace!(
                have = self.read_buffer.len(),
                need = total_size,
                "Waiting for complete packet"
            );
            return Ok(None);
        }

        // Extract the packet
        let packet = self.read_buffer.split_to(total_size);
        let payload = Bytes::copy_from_slice(&packet[4..]);

        debug!(size = payload.len(), "Received complete packet");
        Ok(Some(payload))
    }

    /// Find magic number in buffer
    fn find_magic(&self) -> Option<usize> {
        for i in 0..self.read_buffer.len().saturating_sub(1) {
            if self.read_buffer[i] == 0x94 && self.read_buffer[i + 1] == 0xC3 {
                return Some(i);
            }
        }
        None
    }

    /// Frame a payload with Meshtastic protocol header
    fn frame_packet(payload: &[u8]) -> Vec<u8> {
        let length = payload.len() as u16;
        let mut packet = Vec::with_capacity(4 + payload.len());

        // Magic number (big-endian)
        packet.extend_from_slice(&MESHTASTIC_MAGIC.to_be_bytes());

        // Length (big-endian)
        packet.extend_from_slice(&length.to_be_bytes());

        // Payload
        packet.extend_from_slice(payload);

        packet
    }
}

#[async_trait]
impl MeshtasticInterface for SerialInterface {
    async fn connect(&mut self) -> Result<()> {
        if self.state == ConnectionState::Connected {
            return Ok(());
        }

        self.state = ConnectionState::Connecting;
        info!(port = %self.port_path.display(), baud = self.baud_rate, "Connecting to serial port");

        // Check if port exists
        if !self.port_path.exists() {
            self.state = ConnectionState::Disconnected;
            return Err(MeshtasticError::PortNotFound(
                self.port_path.display().to_string(),
            ));
        }

        // Open serial port
        let stream = tokio_serial::new(self.port_path.to_string_lossy(), self.baud_rate)
            .timeout(self.timeout)
            .open_native_async()
            .map_err(|e| {
                self.state = ConnectionState::Disconnected;
                MeshtasticError::PortOpenFailed {
                    port: self.port_path.display().to_string(),
                    reason: e.to_string(),
                }
            })?;

        self.stream = Some(stream);
        self.state = ConnectionState::Connected;
        self.read_buffer.clear();

        info!(port = %self.port_path.display(), "Connected to Meshtastic device");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(stream) = self.stream.take() {
            // Flush and drop
            drop(stream);
        }

        self.state = ConnectionState::Disconnected;
        self.read_buffer.clear();

        info!(port = %self.port_path.display(), "Disconnected from serial port");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected && self.stream.is_some()
    }

    async fn read_packet(&mut self) -> Result<Option<Bytes>> {
        let stream = self.stream.as_mut().ok_or(MeshtasticError::Disconnected)?;

        // First, try to parse from existing buffer
        if let Some(packet) = self.try_parse_packet()? {
            return Ok(Some(packet));
        }

        // Read more data from serial port
        let mut buf = [0u8; READ_BUFFER_SIZE];

        match stream.read(&mut buf).await {
            Ok(0) => {
                // EOF - device disconnected
                self.state = ConnectionState::Disconnected;
                Err(MeshtasticError::Disconnected)
            }
            Ok(n) => {
                trace!(bytes = n, "Read from serial port");
                self.read_buffer.extend_from_slice(&buf[..n]);

                // Try to parse again
                self.try_parse_packet()
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                // Timeout is normal, just return None
                Ok(None)
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available
                Ok(None)
            }
            Err(e) => {
                error!(error = %e, "Serial read error");
                self.state = ConnectionState::Disconnected;
                Err(MeshtasticError::ReadError(e.to_string()))
            }
        }
    }

    async fn write_packet(&mut self, payload: &[u8]) -> Result<()> {
        let stream = self.stream.as_mut().ok_or(MeshtasticError::Disconnected)?;

        let packet = Self::frame_packet(payload);
        debug!(
            size = packet.len(),
            payload_size = payload.len(),
            "Writing packet"
        );

        stream.write_all(&packet).await.map_err(|e| {
            error!(error = %e, "Serial write error");
            self.state = ConnectionState::Disconnected;
            MeshtasticError::WriteError(e.to_string())
        })?;

        stream
            .flush()
            .await
            .map_err(|e| MeshtasticError::WriteError(format!("Flush failed: {}", e)))?;

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Debug for SerialInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerialInterface")
            .field("port", &self.port_path)
            .field("baud_rate", &self.baud_rate)
            .field("state", &self.state)
            .field("buffer_len", &self.read_buffer.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_packet() {
        let payload = b"hello";
        let framed = SerialInterface::frame_packet(payload);

        // Check magic
        assert_eq!(framed[0], 0x94);
        assert_eq!(framed[1], 0xC3);

        // Check length
        let length = u16::from_be_bytes([framed[2], framed[3]]);
        assert_eq!(length, 5);

        // Check payload
        assert_eq!(&framed[4..], b"hello");
    }

    #[test]
    fn test_parse_complete_packet() {
        let mut iface = SerialInterface::new("/dev/null");

        // Add a complete packet to buffer
        let payload = b"test";
        let framed = SerialInterface::frame_packet(payload);
        iface.read_buffer.extend_from_slice(&framed);

        // Should parse successfully
        let result = iface.try_parse_packet().unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_ref(), b"test");

        // Buffer should be empty
        assert!(iface.read_buffer.is_empty());
    }

    #[test]
    fn test_parse_partial_packet() {
        let mut iface = SerialInterface::new("/dev/null");

        // Add only magic and length
        iface
            .read_buffer
            .extend_from_slice(&[0x94, 0xC3, 0x00, 0x05]);

        // Should return None (incomplete)
        let result = iface.try_parse_packet().unwrap();
        assert!(result.is_none());

        // Buffer should still have data
        assert_eq!(iface.read_buffer.len(), 4);
    }

    #[test]
    fn test_skip_garbage_before_magic() {
        let mut iface = SerialInterface::new("/dev/null");

        // Add garbage before valid packet
        iface.read_buffer.extend_from_slice(b"garbage");
        iface
            .read_buffer
            .extend_from_slice(&[0x94, 0xC3, 0x00, 0x04]);
        iface.read_buffer.extend_from_slice(b"test");

        // Should find and parse the packet
        let result = iface.try_parse_packet().unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_ref(), b"test");
    }

    #[test]
    fn test_interface_state() {
        let iface = SerialInterface::new("/dev/ttyUSB0");
        assert_eq!(iface.state(), ConnectionState::Disconnected);
        assert!(!iface.is_connected());
    }

    #[test]
    fn test_interface_name() {
        let iface = SerialInterface::new("/dev/ttyUSB0");
        assert_eq!(iface.name(), "serial:/dev/ttyUSB0");
    }
}
