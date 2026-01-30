# ADR-002: Meshtastic LoRa Mesh Integration

**Status**: Proposed
**Date**: 2026-01-30
**Authors**: Hive Mind Collective (hive-1769748091504)

## Context

The univrs-network P2P system currently operates over IP networks using libp2p with gossipsub, Kademlia DHT, and mDNS. To extend the network's reach to off-grid and infrastructure-independent scenarios, we propose integrating Meshtastic, a LoRa-based mesh networking protocol.

### What is Meshtastic?

Meshtastic is a decentralized wireless off-grid mesh networking protocol that:
- Operates on unlicensed ISM bands (433MHz, 868MHz, 915MHz)
- Achieves 2-10km range in open terrain with low power consumption
- Uses managed flooding for broadcast and next-hop routing for direct messages
- Encrypts payloads with AES-256 using Protocol Buffers serialization

### Why Integrate?

1. **Off-Grid Operation**: Enable P2P communication without internet infrastructure
2. **Disaster Resilience**: Maintain network connectivity during outages
3. **Extended Range**: Bridge IP-connected nodes with remote LoRa nodes
4. **Mycelial Economics**: Extend vouch, credit, and governance to LoRa mesh

## Decision

We will create a new `mycelial-meshtastic` crate that bridges Meshtastic LoRa mesh with libp2p gossipsub, enabling bidirectional message flow between the two networks.

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

## Message Flow

### LoRa → libp2p

```
1. Meshtastic device receives LoRa packet over radio
2. SerialInterface reads FromRadio protobuf (magic: 0x94C3)
3. MessageTranslator converts to univrs Message (CBOR)
4. TopicMapper determines target gossipsub topic
5. DeduplicationCache checks for duplicate (packet_id + sender)
6. NetworkHandle.publish() broadcasts to IP mesh
```

### libp2p → LoRa

```
1. NetworkEvent::MessageReceived from gossipsub subscription
2. TopicMapper checks if message should bridge to LoRa
3. DeduplicationCache marks message as seen
4. MessageTranslator converts to Meshtastic protobuf
5. HopLimitManager sets hop_limit (default: 3, max: 7)
6. SerialInterface sends ToRadio with 0x94C3 prefix
```

## Topic Mapping

| Gossipsub Topic | Meshtastic Channel | Direction | Notes |
|-----------------|-------------------|-----------|-------|
| `/mycelial/1.0.0/chat` | Primary | Bidirectional | General chat messages |
| `/mycelial/1.0.0/announce` | LongFast | LoRa → libp2p | Node announcements |
| `/mycelial/1.0.0/vouch` | Primary | Bidirectional | Reputation vouching |
| `/mycelial/1.0.0/credit` | Primary | Bidirectional | Credit transactions |
| `/mycelial/1.0.0/governance` | Primary | Bidirectional | Proposals/votes |
| `/mycelial/1.0.0/direct` | Direct | Bidirectional | Private messages |

## Crate Structure

```
crates/mycelial-meshtastic/
├── Cargo.toml
├── build.rs                    # prost-build for protobufs
├── proto/
│   └── meshtastic/            # Git submodule or vendored
│       ├── mesh.proto
│       ├── portnums.proto
│       └── ...
└── src/
    ├── lib.rs                  # Public API
    ├── error.rs                # MeshtasticError enum
    ├── interface/
    │   ├── mod.rs
    │   ├── serial.rs           # SerialInterface
    │   ├── ble.rs              # BleInterface (optional)
    │   └── tcp.rs              # TcpInterface (optional)
    ├── codec.rs                # Protobuf encoding/decoding
    ├── translator.rs           # Message translation
    ├── mapper.rs               # Topic/NodeId mapping
    ├── cache.rs                # Deduplication LRU cache
    ├── bridge.rs               # Main bridge service
    └── config.rs               # MeshtasticConfig
```

## Dependencies

```toml
[dependencies]
mycelial-core = { path = "../mycelial-core" }
mycelial-protocol = { path = "../mycelial-protocol" }
prost = "0.13"
tokio-serial = "5.4"
serialport = "4.6"
lru = "0.12"
tokio = { version = "1", features = ["sync", "time", "io-util"] }
tracing = "0.1"
thiserror = "1.0"

[build-dependencies]
prost-build = "0.13"
```

## Message Size Constraints

Meshtastic has a maximum payload of **237 bytes** (excluding protobuf overhead). To accommodate this:

1. **Compression**: Use LZ4 or similar for economics messages
2. **Message Splitting**: Large messages split into chunks with reassembly
3. **Prioritization**: Critical messages (governance votes) get priority
4. **Filtering**: Only bridge essential fields, not full message metadata

### Economics Message Compression

| Message Type | Original Size (est.) | Compressed | Fits LoRa? |
|--------------|---------------------|------------|------------|
| VouchRequest | ~150 bytes | ~80 bytes | Yes |
| CreditTransfer | ~120 bytes | ~60 bytes | Yes |
| GovernanceVote | ~100 bytes | ~50 bytes | Yes |
| Proposal (full) | ~500+ bytes | Split needed | Chunked |

## Implementation Plan

### Phase 1: Foundation (Week 1)
- [x] Research Meshtastic protocol
- [ ] Create `mycelial-meshtastic` crate
- [ ] Set up prost-build for protobuf generation
- [ ] Implement SerialInterface with tokio-serial
- [ ] Define MeshtasticError types

### Phase 2: Core Bridge (Weeks 2-3)
- [ ] Implement MessageTranslator
- [ ] Implement TopicMapper
- [ ] Implement NodeIdMapper (NodeId ↔ PeerId)
- [ ] Create DeduplicationCache
- [ ] Build MeshtasticService main loop

### Phase 3: Network Integration (Week 4)
- [ ] Integrate with NetworkService
- [ ] Add gossipsub → LoRa forwarding
- [ ] Add LoRa → gossipsub ingestion
- [ ] Implement --meshtastic CLI flag

### Phase 4: Economics Support (Week 5)
- [ ] Bridge VouchMessage protocol
- [ ] Bridge CreditMessage protocol
- [ ] Bridge GovernanceMessage protocol
- [ ] Implement message compression

### Phase 5: Testing & Docs (Week 6)
- [ ] Unit tests for protobuf conversion
- [ ] Integration tests with mock serial
- [ ] Hardware testing with real devices
- [ ] Documentation and examples

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Serial port reliability | Medium | High | Reconnection with exponential backoff |
| Message size limits | High | Medium | Compression + chunking |
| Network partition | Medium | Low | Queue messages for delayed delivery |
| Duplicate messages | High | Low | LRU deduplication cache |
| Clock drift | Low | Low | Use relative timestamps |

## Alternatives Considered

### 1. Native LoRa libp2p Transport
Create a custom libp2p transport directly over LoRa radio.

**Rejected because**:
- Would require low-level radio driver implementation
- Loses Meshtastic's proven mesh routing
- Higher development effort for similar result

### 2. MQTT Bridge
Use Meshtastic's MQTT integration to bridge via cloud.

**Rejected because**:
- Requires internet connectivity (defeats purpose)
- Adds latency and single point of failure
- Privacy concerns with cloud relay

### 3. Pure Python Bridge
Use meshtastic-python directly with pyO3 bindings.

**Rejected because**:
- Adds Python runtime dependency
- More complex deployment
- Performance overhead for async bridging

## Success Criteria

1. **Functional**: Chat messages bidirectionally flow between LoRa and IP mesh
2. **Reliable**: <1% message loss under normal conditions
3. **Latent**: <5 second end-to-end latency for bridged messages
4. **Scalable**: Support 10+ concurrent LoRa nodes per bridge
5. **Economics**: Vouch, credit, and governance work over LoRa

## References

- [Meshtastic Protocol Documentation](https://meshtastic.org/docs/overview/mesh-algo/)
- [Meshtastic Protobufs](https://meshtastic.org/docs/development/reference/protobufs/)
- [Meshtastic Python API](https://python.meshtastic.org/)
- [libp2p Gossipsub Specification](https://github.com/libp2p/specs/tree/master/pubsub/gossipsub)
- [univrs-network Architecture](./ADR-001-workspace-structure.md)
