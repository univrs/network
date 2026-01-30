# mycelial-meshtastic

Meshtastic LoRa Mesh Bridge for the Mycelial P2P Network

[![Crates.io](https://img.shields.io/crates/v/mycelial-meshtastic.svg)](https://crates.io/crates/mycelial-meshtastic)
[![Documentation](https://docs.rs/mycelial-meshtastic/badge.svg)](https://docs.rs/mycelial-meshtastic)
[![License](https://img.shields.io/crates/l/mycelial-meshtastic.svg)](LICENSE)

This crate provides a bidirectional bridge between [Meshtastic](https://meshtastic.org/) LoRa mesh networks and the mycelial libp2p gossipsub network, enabling off-grid P2P communication through low-power, long-range radio.

## Features

- **Bidirectional Bridging**: Messages flow seamlessly between LoRa mesh and IP network
- **Economics Protocol Support**: Full support for vouch, credit, governance, and resource protocols over LoRa
- **Automatic Compression**: Messages are automatically compressed to fit LoRa's 237-byte payload limit
- **Message Chunking**: Large messages (like governance proposals) are automatically split and reassembled
- **Deduplication**: Smart deduplication prevents message loops between networks
- **Multiple Interfaces**: Serial, BLE, and TCP connections supported

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    MESHTASTIC-LIBP2P BRIDGE                             │
├─────────────────────────────────────────────────────────────────────────┤
│  Layer 4: Application Integration                                       │
│    • CLI flag: --meshtastic /dev/ttyUSB0                                │
│    • Dashboard: LoRa peer visualization                                 │
│    • Economics: Vouch/Credit over LoRa                                  │
├─────────────────────────────────────────────────────────────────────────┤
│  Layer 3: Network Integration                                           │
│    • MeshtasticBridge (NetworkService integration)                      │
│    • Gossipsub forwarding → LoRa broadcast                              │
│    • LoRa ingestion → Gossipsub publish                                 │
├─────────────────────────────────────────────────────────────────────────┤
│  Layer 2: Protocol Translation                                          │
│    • MessageTranslator (Meshtastic ↔ univrs)                            │
│    • TopicMapper (channels ↔ gossipsub topics)                          │
│    • NodeIdMapper (NodeId ↔ PeerId)                                     │
│    • DeduplicationCache (prevent message loops)                         │
├─────────────────────────────────────────────────────────────────────────┤
│  Layer 1: Physical Bridge                                               │
│    • SerialInterface (async tokio-serial)                               │
│    • ProtobufCodec (prost for ToRadio/FromRadio)                        │
│    • Reconnection with exponential backoff                              │
└─────────────────────────────────────────────────────────────────────────┘
          │                                           │
          ▼                                           ▼
┌─────────────────────┐                 ┌─────────────────────────────────┐
│  Meshtastic Device  │                 │       libp2p Gossipsub          │
│  (T-Beam, T-Echo)   │                 │  (univrs-network P2P mesh)      │
│                     │                 │                                 │
│  LoRa Radio Module  │◄───────────────►│  TCP/QUIC + Noise + Yamux       │
│  Serial/BLE/TCP     │    Bridge       │  Kademlia DHT + mDNS            │
└─────────────────────┘                 └─────────────────────────────────┘
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
mycelial-meshtastic = "0.1"

# Enable serial port support (requires libudev-dev on Linux)
mycelial-meshtastic = { version = "0.1", features = ["serial"] }

# Enable all interfaces
mycelial-meshtastic = { version = "0.1", features = ["full"] }
```

### Linux Prerequisites

For the `serial` feature on Linux, install the required system libraries:

```bash
# Debian/Ubuntu
sudo apt install libudev-dev pkg-config

# Fedora/RHEL
sudo dnf install systemd-devel pkg-config

# Arch Linux
sudo pacman -S systemd-libs pkgconf
```

## Quick Start

### Basic Bridge Setup

```rust
use mycelial_meshtastic::{
    MeshtasticBridge, MeshtasticConfigBuilder, PublishCallback,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration
    let config = MeshtasticConfigBuilder::new()
        .serial_port("/dev/ttyUSB0")
        .max_hops(3)
        .build();

    // Create publish callback for gossipsub
    let publish_callback: PublishCallback = Arc::new(|topic, data| {
        println!("Publishing to {}: {} bytes", topic, data.len());
        Ok(())
    });

    // Create the bridge (requires `serial` feature)
    #[cfg(feature = "serial")]
    {
        use mycelial_meshtastic::SerialInterface;

        let interface = SerialInterface::new("/dev/ttyUSB0");
        let (bridge, handle) = MeshtasticBridge::new(interface, &config, publish_callback);

        // Run the bridge in a background task
        let bridge_task = tokio::spawn(async move {
            bridge.run().await
        });

        // Use the handle to interact with the bridge
        let stats = handle.stats().await?;
        println!("Bridge stats: {:?}", stats);

        // Shutdown when done
        handle.shutdown().await?;
        bridge_task.await??;
    }

    Ok(())
}
```

### Forwarding Messages to LoRa

```rust
use mycelial_meshtastic::GossipsubMessage;

// Forward a gossipsub message to the LoRa mesh
let msg = GossipsubMessage {
    topic: "/mycelial/1.0.0/chat".to_string(),
    source: Some("QmMyPeerId123".to_string()),
    data: b"Hello from the IP network!".to_vec(),
    message_id: "msg-123".to_string(),
};

handle.forward_to_lora(msg).await?;
```

### Handling Economics Protocols

The bridge automatically handles economics protocol messages with compression:

```rust
use mycelial_meshtastic::{EconomicsMessageCodec, LORA_MAX_PAYLOAD};

let mut codec = EconomicsMessageCodec::new();

// Large governance proposal (800 bytes)
let proposal_data = vec![0u8; 800];

// Automatically compressed and chunked for LoRa
let packets = codec.encode(&proposal_data)?;
println!("Split into {} LoRa packets", packets.len());

// Reassemble on receive
let mut decoder = EconomicsMessageCodec::new();
for packet in packets {
    if let Some(complete) = decoder.decode(&packet)? {
        assert_eq!(complete, proposal_data);
    }
}
```

## Topic Mapping

Messages are automatically mapped between gossipsub topics and Meshtastic channels:

| Gossipsub Topic | Meshtastic Channel | Direction | Notes |
|-----------------|-------------------|-----------|-------|
| `/mycelial/1.0.0/chat` | Primary | Bidirectional | General chat messages |
| `/mycelial/1.0.0/announce` | LongFast | LoRa → libp2p | Node announcements |
| `/mycelial/1.0.0/vouch` | Primary | Bidirectional | Reputation vouching |
| `/mycelial/1.0.0/credit` | Primary | Bidirectional | Credit transactions |
| `/mycelial/1.0.0/governance` | Primary | Bidirectional | Proposals/votes |
| `/mycelial/1.0.0/direct` | Direct | Bidirectional | Private messages |

## Hardware Testing

### Detecting Devices

```rust
use mycelial_meshtastic::test_utils::{find_meshtastic_device, list_available_devices};

// Auto-detect connected Meshtastic device
if let Some(path) = find_meshtastic_device() {
    println!("Found Meshtastic device at: {}", path);
}

// List all available serial devices
for device in list_available_devices() {
    println!("{}: {} (Meshtastic: {})",
        device.path,
        device.device_type,
        device.is_likely_meshtastic
    );
}
```

### Hardware Test Context

```rust
use mycelial_meshtastic::test_utils::HardwareTestContext;

#[tokio::test]
#[ignore] // Only run when hardware is available
async fn test_with_real_device() {
    let device_path = "/dev/ttyUSB0";

    let mut ctx = HardwareTestContext::new(device_path)
        .await
        .expect("Failed to connect");

    // Verify device is responding
    ctx.verify_device_info().await.expect("Device check failed");

    // Send test packet
    let response = ctx.send_test_packet(b"Hello LoRa!").await?;
}
```

## Configuration

### Builder Pattern

```rust
use mycelial_meshtastic::MeshtasticConfigBuilder;
use std::time::Duration;

let config = MeshtasticConfigBuilder::new()
    // Serial port settings
    .serial_port("/dev/ttyUSB0")
    .baud_rate(115200)
    .timeout(Duration::from_secs(5))

    // Bridge settings
    .max_hops(3)                              // Max mesh hops (1-7)
    .dedup_capacity(10000)                    // Dedup cache size
    .dedup_ttl(Duration::from_secs(300))      // Cache TTL

    // Reconnection settings
    .reconnect_delay(Duration::from_secs(2))
    .max_reconnect_attempts(10)

    .build();
```

### Channel Configuration

```rust
use mycelial_meshtastic::{ChannelConfig, BridgeDirection, MessagePriority};

let channel = ChannelConfig {
    name: "MyChannel".to_string(),
    index: 0,
    psk: Some(vec![0x01, 0x02, 0x03]),  // Pre-shared key
    direction: BridgeDirection::Bidirectional,
    priority: MessagePriority::High,
};
```

## Message Size Constraints

Meshtastic has a maximum payload of **237 bytes**. The bridge handles this automatically:

| Message Type | Original Size | Compressed | Fits LoRa? |
|--------------|--------------|------------|------------|
| VouchRequest | ~150 bytes | ~80 bytes | Yes |
| CreditTransfer | ~120 bytes | ~60 bytes | Yes |
| GovernanceVote | ~100 bytes | ~50 bytes | Yes |
| Proposal (full) | ~500+ bytes | Split needed | Chunked |

## Error Handling

The crate provides comprehensive error types:

```rust
use mycelial_meshtastic::MeshtasticError;

match result {
    Err(MeshtasticError::Disconnected) => {
        // Handle disconnection, will auto-reconnect
    }
    Err(MeshtasticError::MessageTooLarge { size, max }) => {
        // Message was too large even after compression
        println!("Message {} bytes exceeds max {} bytes", size, max);
    }
    Err(e) if e.is_retriable() => {
        // Transient error, retry is appropriate
    }
    Err(e) if e.is_protocol_error() => {
        // Bad data from device
    }
    _ => {}
}
```

## Testing

### Unit Tests

```bash
# Run all tests
cargo test -p mycelial-meshtastic

# Run with logging
RUST_LOG=debug cargo test -p mycelial-meshtastic -- --nocapture

# Run specific test
cargo test -p mycelial-meshtastic test_bridge_creation
```

### Integration Tests

```bash
# Run integration tests
cargo test -p mycelial-meshtastic --test integration_tests

# Run with serial feature
cargo test -p mycelial-meshtastic --test integration_tests --features serial
```

### Hardware Tests (Ignored by Default)

```bash
# Run hardware tests (requires connected Meshtastic device)
cargo test -p mycelial-meshtastic --features serial -- --ignored
```

## Supported Hardware

This bridge works with any Meshtastic-compatible device:

- **LILYGO T-Beam** - Popular ESP32 + LoRa + GPS
- **LILYGO T-Echo** - nRF52840 + LoRa + E-ink display
- **Heltec LoRa 32** - ESP32 + LoRa + OLED
- **RAK WisBlock** - Modular LoRa development kit
- **DIY builds** - Any ESP32/nRF52 with LoRa module

## Feature Flags

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `serial` | Serial port interface | `tokio-serial`, `serialport` |
| `ble` | Bluetooth Low Energy interface | `btleplug` |
| `tcp` | TCP interface for networked devices | (none) |
| `full` | All interfaces enabled | All above |

## Protocol Documentation

- [Meshtastic Protocol Documentation](https://meshtastic.org/docs/overview/mesh-algo/)
- [Meshtastic Protobufs](https://meshtastic.org/docs/development/reference/protobufs/)
- [libp2p Gossipsub Specification](https://github.com/libp2p/specs/tree/master/pubsub/gossipsub)
- [univrs-network Architecture](../docs/architecture/ADR-001-workspace-structure.md)
- [Meshtastic Integration ADR](../../docs/architecture/ADR-002-meshtastic-integration.md)

## License

This project is licensed under the MIT License - see the [LICENSE](../../LICENSE) file for details.

## Contributing

Contributions are welcome! Please see our [Contributing Guide](../../CONTRIBUTING.md) for details.

## Acknowledgments

- [Meshtastic](https://meshtastic.org/) - The LoRa mesh networking protocol
- [libp2p](https://libp2p.io/) - The modular networking stack
- [prost](https://github.com/tokio-rs/prost) - Protocol Buffers for Rust
