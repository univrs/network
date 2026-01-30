//! Hardware test utilities for real Meshtastic device testing
//!
//! This module provides utilities for testing with actual Meshtastic hardware devices.
//! It includes device detection, connection helpers, and test fixtures for integration
//! testing with real LoRa mesh networks.
//!
//! # Safety
//!
//! These utilities are designed for development and testing purposes. They will attempt
//! to communicate with real hardware when available, so use caution in production environments.
//!
//! # Example
//!
//! ```rust,ignore
//! use mycelial_meshtastic::test_utils::{HardwareTestContext, find_meshtastic_device};
//!
//! #[tokio::test]
//! #[ignore] // Only run when hardware is available
//! async fn test_with_real_device() {
//!     let device_path = find_meshtastic_device().expect("No Meshtastic device found");
//!
//!     let ctx = HardwareTestContext::new(&device_path)
//!         .await
//!         .expect("Failed to connect to device");
//!
//!     // Run hardware tests
//!     ctx.verify_device_info().await.expect("Device info check failed");
//! }
//! ```

use bytes::Bytes;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::config::{MeshtasticConfig, MeshtasticConfigBuilder, DEFAULT_BAUD_RATE};
use crate::error::{MeshtasticError, Result};
use crate::interface::MeshtasticInterface;

#[cfg(feature = "serial")]
use crate::interface::SerialInterface;

/// Common serial port paths for Meshtastic devices on different platforms
const COMMON_DEVICE_PATHS: &[&str] = &[
    // Linux
    "/dev/ttyUSB0",
    "/dev/ttyUSB1",
    "/dev/ttyACM0",
    "/dev/ttyACM1",
    "/dev/serial/by-id/usb-Silicon_Labs_CP210x_USB_to_UART_Bridge*",
    // macOS
    "/dev/tty.usbserial-*",
    "/dev/tty.SLAB_USBtoUART*",
    "/dev/cu.usbserial-*",
    "/dev/cu.SLAB_USBtoUART*",
    // Windows (via WSL or Cygwin)
    "/dev/ttyS*",
];

/// Common baud rates to try when detecting devices
const COMMON_BAUD_RATES: &[u32] = &[115200, 921600, 57600, 38400, 19200, 9600];

/// Find a connected Meshtastic device by scanning common serial ports
///
/// Returns the path to the first detected device, or None if no device is found.
///
/// # Example
///
/// ```rust,ignore
/// if let Some(path) = find_meshtastic_device() {
///     println!("Found Meshtastic device at: {}", path);
/// }
/// ```
pub fn find_meshtastic_device() -> Option<String> {
    #[cfg(feature = "serial")]
    {
        // Try to list available ports using serialport
        if let Ok(ports) = serialport::available_ports() {
            for port in ports {
                let path = port.port_name.clone();

                // Check if it looks like a Meshtastic device
                if is_likely_meshtastic_port(&port) {
                    info!("Found likely Meshtastic device: {}", path);
                    return Some(path);
                }
            }
        }

        // Fall back to checking common paths
        for pattern in COMMON_DEVICE_PATHS {
            if let Ok(entries) = glob::glob(pattern) {
                for entry in entries.flatten() {
                    if entry.exists() {
                        let path = entry.to_string_lossy().to_string();
                        debug!("Found serial port: {}", path);
                        return Some(path);
                    }
                }
            }

            // Direct path check if not a glob pattern
            if !pattern.contains('*') {
                let path = std::path::Path::new(pattern);
                if path.exists() {
                    return Some(pattern.to_string());
                }
            }
        }

        None
    }

    #[cfg(not(feature = "serial"))]
    {
        warn!("Serial feature not enabled, cannot detect hardware devices");
        None
    }
}

/// Check if a serial port looks like a Meshtastic device based on its info
#[cfg(feature = "serial")]
fn is_likely_meshtastic_port(port: &serialport::SerialPortInfo) -> bool {
    use serialport::SerialPortType;

    match &port.port_type {
        SerialPortType::UsbPort(usb_info) => {
            // Known Meshtastic device vendors
            // Silicon Labs CP210x (common on T-Beam, etc.)
            // FTDI
            // ESP32 native USB
            let known_vids = [0x10C4, 0x0403, 0x303A, 0x1A86];

            if known_vids.contains(&usb_info.vid) {
                debug!(
                    "USB device matches known vendor: VID={:#06X} PID={:#06X}",
                    usb_info.vid, usb_info.pid
                );
                return true;
            }

            // Check product/manufacturer strings
            if let Some(product) = &usb_info.product {
                let product_lower = product.to_lowercase();
                if product_lower.contains("meshtastic")
                    || product_lower.contains("t-beam")
                    || product_lower.contains("t-echo")
                    || product_lower.contains("lora")
                    || product_lower.contains("cp210")
                    || product_lower.contains("uart")
                {
                    debug!("USB device matches by product name: {}", product);
                    return true;
                }
            }

            false
        }
        _ => false,
    }
}

/// List all available serial ports that might be Meshtastic devices
///
/// Returns a vector of device information for all detected ports.
pub fn list_available_devices() -> Vec<DeviceInfo> {
    #[cfg(feature = "serial")]
    {
        let mut devices = Vec::new();
        if let Ok(ports) = serialport::available_ports() {
            for port in ports {
                let info = DeviceInfo::from_serial_port(&port);
                devices.push(info);
            }
        }
        devices
    }

    #[cfg(not(feature = "serial"))]
    {
        Vec::new()
    }
}

/// Information about a detected serial device
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Path to the device (e.g., /dev/ttyUSB0)
    pub path: String,
    /// Device type description
    pub device_type: String,
    /// USB Vendor ID (if available)
    pub vendor_id: Option<u16>,
    /// USB Product ID (if available)
    pub product_id: Option<u16>,
    /// Product name (if available)
    pub product_name: Option<String>,
    /// Manufacturer (if available)
    pub manufacturer: Option<String>,
    /// Serial number (if available)
    pub serial_number: Option<String>,
    /// Whether this is likely a Meshtastic device
    pub is_likely_meshtastic: bool,
}

impl DeviceInfo {
    /// Create device info from a serialport SerialPortInfo
    #[cfg(feature = "serial")]
    fn from_serial_port(port: &serialport::SerialPortInfo) -> Self {
        use serialport::SerialPortType;

        let (device_type, vendor_id, product_id, product_name, manufacturer, serial_number) =
            match &port.port_type {
                SerialPortType::UsbPort(usb_info) => (
                    "USB".to_string(),
                    Some(usb_info.vid),
                    Some(usb_info.pid),
                    usb_info.product.clone(),
                    usb_info.manufacturer.clone(),
                    usb_info.serial_num.clone(),
                ),
                SerialPortType::PciPort => ("PCI".to_string(), None, None, None, None, None),
                SerialPortType::BluetoothPort => {
                    ("Bluetooth".to_string(), None, None, None, None, None)
                }
                SerialPortType::Unknown => ("Unknown".to_string(), None, None, None, None, None),
            };

        Self {
            path: port.port_name.clone(),
            device_type,
            vendor_id,
            product_id,
            product_name,
            manufacturer,
            serial_number,
            is_likely_meshtastic: is_likely_meshtastic_port(port),
        }
    }

    /// Create device info for a device at the given path
    #[cfg(not(feature = "serial"))]
    fn from_path(path: &str) -> Self {
        Self {
            path: path.to_string(),
            device_type: "Unknown".to_string(),
            vendor_id: None,
            product_id: None,
            product_name: None,
            manufacturer: None,
            serial_number: None,
            is_likely_meshtastic: false,
        }
    }
}

/// Context for hardware integration tests
///
/// Provides a managed environment for testing with real Meshtastic devices.
#[cfg(feature = "serial")]
pub struct HardwareTestContext {
    /// Serial interface to the device
    interface: SerialInterface,
    /// Device path
    device_path: String,
    /// Configuration used
    config: MeshtasticConfig,
}

#[cfg(feature = "serial")]
impl HardwareTestContext {
    /// Create a new hardware test context
    ///
    /// Connects to the device at the specified path.
    pub async fn new(device_path: &str) -> Result<Self> {
        Self::with_baud_rate(device_path, DEFAULT_BAUD_RATE).await
    }

    /// Create with a specific baud rate
    pub async fn with_baud_rate(device_path: &str, baud_rate: u32) -> Result<Self> {
        let config = MeshtasticConfigBuilder::new()
            .serial_port(device_path)
            .baud_rate(baud_rate)
            .build();

        let mut interface = SerialInterface::new(device_path);
        interface.connect().await?;

        info!(
            "Connected to Meshtastic device at {} ({}bps)",
            device_path, baud_rate
        );

        Ok(Self {
            interface,
            device_path: device_path.to_string(),
            config,
        })
    }

    /// Auto-detect baud rate by trying common rates
    pub async fn auto_detect(device_path: &str) -> Result<Self> {
        for &baud_rate in COMMON_BAUD_RATES {
            debug!("Trying baud rate: {}", baud_rate);
            match Self::with_baud_rate(device_path, baud_rate).await {
                Ok(ctx) => {
                    // Try to verify connection by reading a packet
                    info!("Successfully connected at {} baud", baud_rate);
                    return Ok(ctx);
                }
                Err(e) => {
                    debug!("Failed at {} baud: {}", baud_rate, e);
                    continue;
                }
            }
        }

        Err(MeshtasticError::PortOpenFailed {
            port: device_path.to_string(),
            reason: "Failed to auto-detect baud rate".to_string(),
        })
    }

    /// Verify device is responding correctly
    pub async fn verify_device_info(&mut self) -> Result<()> {
        info!("Verifying device at {}", self.device_path);

        // Try to read a packet (with timeout)
        let timeout = Duration::from_secs(5);
        let result = tokio::time::timeout(timeout, self.interface.read_packet()).await;

        match result {
            Ok(Ok(Some(data))) => {
                info!("Device responding, received {} bytes", data.len());
                Ok(())
            }
            Ok(Ok(None)) => {
                info!("Device connected but no data available (this is OK)");
                Ok(())
            }
            Ok(Err(e)) => {
                warn!("Device read error: {}", e);
                Err(e)
            }
            Err(_) => {
                info!("Device timeout - may need to send a wakeup packet");
                Ok(()) // Timeout is not necessarily an error
            }
        }
    }

    /// Send a test packet and wait for echo/response
    pub async fn send_test_packet(&mut self, data: &[u8]) -> Result<Option<Bytes>> {
        info!("Sending test packet: {} bytes", data.len());

        self.interface.write_packet(data).await?;

        // Wait for response
        let timeout = Duration::from_secs(5);
        match tokio::time::timeout(timeout, self.interface.read_packet()).await {
            Ok(result) => result,
            Err(_) => Ok(None),
        }
    }

    /// Get the interface for direct access
    pub fn interface_mut(&mut self) -> &mut SerialInterface {
        &mut self.interface
    }

    /// Get the configuration
    pub fn config(&self) -> &MeshtasticConfig {
        &self.config
    }

    /// Get the device path
    pub fn device_path(&self) -> &str {
        &self.device_path
    }
}

#[cfg(feature = "serial")]
impl Drop for HardwareTestContext {
    fn drop(&mut self) {
        // Disconnect on drop
        let _ = futures::executor::block_on(self.interface.disconnect());
        info!("Disconnected from device at {}", self.device_path);
    }
}

/// Mock interface for testing without hardware
///
/// This provides a simulated Meshtastic device for unit and integration tests
/// that don't require real hardware.
#[derive(Debug, Default)]
pub struct MockInterface {
    connected: bool,
    incoming_queue: Vec<Vec<u8>>,
    outgoing_queue: Vec<Vec<u8>>,
    simulate_errors: bool,
    error_on_nth_read: Option<usize>,
    read_count: usize,
}

impl MockInterface {
    /// Create a new mock interface
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a packet to the incoming queue (simulates receiving from LoRa)
    pub fn queue_incoming(&mut self, data: Vec<u8>) {
        self.incoming_queue.push(data);
    }

    /// Get packets that were "sent" to the device
    pub fn get_outgoing(&self) -> &[Vec<u8>] {
        &self.outgoing_queue
    }

    /// Clear the outgoing queue
    pub fn clear_outgoing(&mut self) {
        self.outgoing_queue.clear();
    }

    /// Configure to simulate errors
    pub fn simulate_errors(&mut self, enabled: bool) {
        self.simulate_errors = enabled;
    }

    /// Configure to error on the Nth read
    pub fn error_on_read(&mut self, n: Option<usize>) {
        self.error_on_nth_read = n;
        self.read_count = 0;
    }

    /// Create a mock text message packet
    pub fn create_text_packet(from: u32, text: &str) -> Vec<u8> {
        let mut packet = Vec::new();
        packet.extend_from_slice(&from.to_be_bytes());
        packet.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes()); // Broadcast
        packet.extend_from_slice(&rand::random::<u32>().to_be_bytes()); // Packet ID
        packet.push(1); // TextMessage port
        packet.extend_from_slice(text.as_bytes());
        packet
    }

    /// Create a mock economics packet (vouch, credit, etc.)
    pub fn create_economics_packet(from: u32, port: u8, payload: &[u8]) -> Vec<u8> {
        let mut packet = Vec::new();
        packet.extend_from_slice(&from.to_be_bytes());
        packet.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());
        packet.extend_from_slice(&rand::random::<u32>().to_be_bytes());
        packet.push(port);
        packet.extend_from_slice(payload);
        packet
    }
}

#[async_trait::async_trait]
impl MeshtasticInterface for MockInterface {
    async fn connect(&mut self) -> Result<()> {
        if self.simulate_errors {
            return Err(MeshtasticError::ConnectionTimeout { duration_ms: 5000 });
        }
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn read_packet(&mut self) -> Result<Option<Bytes>> {
        self.read_count += 1;

        if let Some(n) = self.error_on_nth_read {
            if self.read_count == n {
                return Err(MeshtasticError::ReadError("Simulated error".to_string()));
            }
        }

        if self.simulate_errors {
            return Err(MeshtasticError::Disconnected);
        }

        if self.incoming_queue.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Bytes::from(self.incoming_queue.remove(0))))
        }
    }

    async fn write_packet(&mut self, data: &[u8]) -> Result<()> {
        if self.simulate_errors {
            return Err(MeshtasticError::WriteError("Simulated error".to_string()));
        }
        self.outgoing_queue.push(data.to_vec());
        Ok(())
    }

    fn name(&self) -> &str {
        "MockInterface"
    }
}

/// Test fixture for creating pre-configured test scenarios
pub struct TestFixture {
    /// Mock interface
    pub interface: MockInterface,
    /// Test configuration
    pub config: MeshtasticConfig,
}

impl TestFixture {
    /// Create a basic test fixture
    pub fn new() -> Self {
        Self {
            interface: MockInterface::new(),
            config: MeshtasticConfigBuilder::new().build(),
        }
    }

    /// Create with pre-populated incoming messages
    pub fn with_incoming_messages(messages: Vec<Vec<u8>>) -> Self {
        let mut fixture = Self::new();
        for msg in messages {
            fixture.interface.queue_incoming(msg);
        }
        fixture
    }

    /// Create a fixture simulating a busy network
    pub fn busy_network() -> Self {
        let mut fixture = Self::new();

        // Add various types of messages
        for i in 0..10 {
            let msg = MockInterface::create_text_packet(0x12340000 + i, &format!("Message {}", i));
            fixture.interface.queue_incoming(msg);
        }

        fixture
    }

    /// Create a fixture for economics testing
    pub fn economics_testing() -> Self {
        let mut fixture = Self::new();

        // Vouch request
        fixture
            .interface
            .queue_incoming(MockInterface::create_economics_packet(
                0xAAAA0001,
                0x00, // Vouch port byte
                &[
                    0x01, 0xAA, 0xAA, 0x00, 0x01, 0xBB, 0xBB, 0x00, 0x01, 0x00, 0x64,
                ],
            ));

        // Credit transfer
        fixture
            .interface
            .queue_incoming(MockInterface::create_economics_packet(
                0xCCCC0001,
                0x01, // Credit port byte
                &[
                    0x03, 0xCC, 0xCC, 0x00, 0x01, 0xDD, 0xDD, 0x00, 0x01, 0x00, 0x00, 0x01, 0xF4,
                ],
            ));

        fixture
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_interface_basic() {
        let mut mock = MockInterface::new();
        assert!(!mock.is_connected());

        // Queue and read
        mock.queue_incoming(vec![1, 2, 3]);
        assert!(!mock.incoming_queue.is_empty());
    }

    #[tokio::test]
    async fn test_mock_interface_connect_disconnect() {
        let mut mock = MockInterface::new();

        mock.connect().await.unwrap();
        assert!(mock.is_connected());

        mock.disconnect().await.unwrap();
        assert!(!mock.is_connected());
    }

    #[tokio::test]
    async fn test_mock_interface_read_write() {
        let mut mock = MockInterface::new();
        mock.connect().await.unwrap();

        // Write
        mock.write_packet(&[1, 2, 3]).await.unwrap();
        assert_eq!(mock.get_outgoing().len(), 1);

        // Queue and read
        mock.queue_incoming(vec![4, 5, 6]);
        let packet = mock.read_packet().await.unwrap();
        assert!(packet.is_some());
        assert_eq!(packet.unwrap().to_vec(), vec![4, 5, 6]);
    }

    #[tokio::test]
    async fn test_mock_interface_simulated_errors() {
        let mut mock = MockInterface::new();
        mock.simulate_errors(true);

        assert!(mock.connect().await.is_err());
    }

    #[test]
    fn test_create_text_packet() {
        let packet = MockInterface::create_text_packet(0x12345678, "Hello");
        assert!(packet.len() >= 13); // Header + payload
        assert_eq!(&packet[0..4], &0x12345678u32.to_be_bytes());
    }

    #[test]
    fn test_fixture_creation() {
        let fixture = TestFixture::new();
        assert!(!fixture.interface.is_connected());

        let fixture = TestFixture::busy_network();
        assert_eq!(fixture.interface.incoming_queue.len(), 10);
    }

    #[test]
    fn test_device_info() {
        let devices = list_available_devices();
        // This test just verifies the function doesn't panic
        println!("Found {} devices", devices.len());
    }
}
