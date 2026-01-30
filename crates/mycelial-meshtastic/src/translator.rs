//! Message translation between Meshtastic and Mycelial formats
//!
//! This module provides bidirectional translation between:
//! - Meshtastic protobufs (via LoRa mesh network)
//! - Mycelial CBOR messages (via libp2p gossipsub)
//!
//! # Message Format Differences
//!
//! | Meshtastic | Mycelial |
//! |------------|----------|
//! | Protobuf (prost) | CBOR (serde_cbor) |
//! | NodeId (u32) | PeerId (base58 string) |
//! | packet_id (u32) | Uuid |
//! | channel (u8 index) | Topic (string) |
//! | Max 237 bytes | Unlimited |
//!
//! # Compression
//!
//! For economics messages (vouch, credit, governance), compression is applied
//! to fit within LoRa payload limits. The bridge uses a compact binary format
//! for these critical messages.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use chrono::{DateTime, TimeZone, Utc};
use mycelial_core::{Message, MessageType, PeerId};
use mycelial_protocol::{
    CastVote, CreateCreditLine, CreateProposal, CreditLineAck, CreditLineUpdate, CreditMessage,
    CreditTransfer, CreditTransferAck, GovernanceMessage, ProposalExecuted, ProposalStatus,
    ProposalType, ProposalUpdate, ReputationChangeReason, ReputationUpdate, ResourceContribution,
    ResourceMessage, ResourceMetrics, ResourcePoolUpdate, ResourceType, Vote, VouchAck,
    VouchMessage, VouchRequest,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};
use uuid::Uuid;

use crate::config::LORA_MAX_PAYLOAD;
use crate::error::{MeshtasticError, Result};
use crate::mapper::NodeIdMapper;

/// Port numbers for Meshtastic data payloads
/// Based on Meshtastic PortNum enum from portnums.proto
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshtasticPort {
    /// Unknown/invalid port
    Unknown = 0,
    /// Text message port (UTF-8 encoded strings)
    TextMessage = 1,
    /// Remote hardware control
    RemoteHardware = 2,
    /// Position data
    Position = 3,
    /// Node info (user data)
    NodeInfo = 4,
    /// Routing protocol messages
    Routing = 5,
    /// Admin messages
    Admin = 6,
    /// Telemetry data
    Telemetry = 67,
    /// Private application ports start here
    PrivateApp = 256,
    /// Mycelial vouch protocol
    MycelialVouch = 512,
    /// Mycelial credit protocol
    MycelialCredit = 513,
    /// Mycelial governance protocol
    MycelialGovernance = 514,
    /// Mycelial resource protocol
    MycelialResource = 515,
}

impl From<u32> for MeshtasticPort {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::TextMessage,
            2 => Self::RemoteHardware,
            3 => Self::Position,
            4 => Self::NodeInfo,
            5 => Self::Routing,
            6 => Self::Admin,
            67 => Self::Telemetry,
            256 => Self::PrivateApp,
            512 => Self::MycelialVouch,
            513 => Self::MycelialCredit,
            514 => Self::MycelialGovernance,
            515 => Self::MycelialResource,
            _ => Self::Unknown,
        }
    }
}

impl From<MeshtasticPort> for u32 {
    fn from(port: MeshtasticPort) -> Self {
        port as u32
    }
}

/// A decoded Meshtastic packet ready for translation
#[derive(Debug, Clone)]
pub struct MeshtasticPacket {
    /// Source node ID (Meshtastic 4-byte identifier)
    pub from: u32,
    /// Destination node ID (0xFFFFFFFF for broadcast)
    pub to: u32,
    /// Unique packet identifier for deduplication
    pub packet_id: u32,
    /// Channel index (0-7)
    pub channel: u8,
    /// Port number indicating payload type
    pub port_num: MeshtasticPort,
    /// Raw payload data
    pub payload: Bytes,
    /// Hop limit for mesh propagation
    pub hop_limit: u8,
    /// Whether this is a broadcast message
    pub want_ack: bool,
    /// Timestamp (if available)
    pub rx_time: Option<DateTime<Utc>>,
}

impl MeshtasticPacket {
    /// Check if this is a broadcast message
    pub fn is_broadcast(&self) -> bool {
        self.to == 0xFFFFFFFF
    }

    /// Create a new packet for sending
    pub fn new_outgoing(
        from: u32,
        to: u32,
        port_num: MeshtasticPort,
        payload: Bytes,
        hop_limit: u8,
    ) -> Self {
        Self {
            from,
            to,
            packet_id: rand::random(),
            channel: 0,
            port_num,
            payload,
            hop_limit,
            want_ack: false,
            rx_time: Some(Utc::now()),
        }
    }
}

/// Bidirectional message translator between Meshtastic and Mycelial formats
#[derive(Debug)]
pub struct MessageTranslator {
    /// Node ID mapper for converting between Meshtastic NodeId and libp2p PeerId
    node_mapper: NodeIdMapper,
    /// Whether to compress economics messages
    enable_compression: bool,
}

impl MessageTranslator {
    /// Create a new message translator
    pub fn new(node_mapper: NodeIdMapper) -> Self {
        Self {
            node_mapper,
            enable_compression: true,
        }
    }

    /// Create with compression setting
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.enable_compression = enabled;
        self
    }

    /// Translate a Meshtastic packet to a Mycelial Message
    ///
    /// This is the LoRa → libp2p direction
    pub fn meshtastic_to_mycelial(&self, packet: &MeshtasticPacket) -> Result<Message> {
        let sender_peer_id = self.node_mapper.node_to_peer(packet.from)?;

        let recipient = if packet.is_broadcast() {
            None
        } else {
            Some(self.node_mapper.node_to_peer(packet.to)?)
        };

        let (message_type, payload) = self.translate_payload_to_mycelial(packet)?;

        Ok(Message {
            id: Uuid::from_u128(packet.packet_id as u128),
            message_type,
            sender: sender_peer_id,
            recipient,
            payload,
            timestamp: packet.rx_time.unwrap_or_else(Utc::now),
            signature: None,
        })
    }

    /// Translate a Mycelial Message to a Meshtastic packet
    ///
    /// This is the libp2p → LoRa direction
    pub fn mycelial_to_meshtastic(
        &self,
        message: &Message,
        hop_limit: u8,
    ) -> Result<MeshtasticPacket> {
        let from = self.node_mapper.peer_to_node(&message.sender)?;

        let to = match &message.recipient {
            Some(peer_id) => self.node_mapper.peer_to_node(peer_id)?,
            None => 0xFFFFFFFF, // Broadcast
        };

        let (port_num, payload) = self.translate_payload_to_meshtastic(message)?;

        // Check payload size
        if payload.len() > LORA_MAX_PAYLOAD {
            return Err(MeshtasticError::MessageTooLarge {
                size: payload.len(),
                max: LORA_MAX_PAYLOAD,
            });
        }

        Ok(MeshtasticPacket {
            from,
            to,
            packet_id: message.id.as_u128() as u32,
            channel: 0, // Default channel, will be set by TopicMapper
            port_num,
            payload,
            hop_limit,
            want_ack: false,
            rx_time: Some(message.timestamp),
        })
    }

    /// Translate Meshtastic payload to Mycelial format
    fn translate_payload_to_mycelial(
        &self,
        packet: &MeshtasticPacket,
    ) -> Result<(MessageType, Vec<u8>)> {
        match packet.port_num {
            MeshtasticPort::TextMessage => {
                // Simple text message - treat as content
                let text = String::from_utf8_lossy(&packet.payload);
                debug!(text = %text, "Translating text message from LoRa");
                Ok((MessageType::Content, packet.payload.to_vec()))
            }
            MeshtasticPort::MycelialVouch => {
                let vouch_msg = self.decode_vouch_message(&packet.payload)?;
                let payload = serde_cbor::to_vec(&vouch_msg)
                    .map_err(|e| MeshtasticError::TranslationFailed(e.to_string()))?;
                Ok((MessageType::Reputation, payload))
            }
            MeshtasticPort::MycelialCredit => {
                let credit_msg = self.decode_credit_message(&packet.payload)?;
                let payload = serde_cbor::to_vec(&credit_msg)
                    .map_err(|e| MeshtasticError::TranslationFailed(e.to_string()))?;
                Ok((MessageType::Credit, payload))
            }
            MeshtasticPort::MycelialGovernance => {
                let gov_msg = self.decode_governance_message(&packet.payload)?;
                let payload = serde_cbor::to_vec(&gov_msg)
                    .map_err(|e| MeshtasticError::TranslationFailed(e.to_string()))?;
                Ok((MessageType::Governance, payload))
            }
            MeshtasticPort::MycelialResource => {
                let res_msg = self.decode_resource_message(&packet.payload)?;
                let payload = serde_cbor::to_vec(&res_msg)
                    .map_err(|e| MeshtasticError::TranslationFailed(e.to_string()))?;
                Ok((MessageType::System, payload))
            }
            _ => {
                warn!(port = ?packet.port_num, "Unknown Meshtastic port, treating as raw payload");
                Ok((MessageType::System, packet.payload.to_vec()))
            }
        }
    }

    /// Translate Mycelial payload to Meshtastic format
    fn translate_payload_to_meshtastic(
        &self,
        message: &Message,
    ) -> Result<(MeshtasticPort, Bytes)> {
        match message.message_type {
            MessageType::Content | MessageType::Discovery => {
                // Text/content messages go as TextMessage
                Ok((
                    MeshtasticPort::TextMessage,
                    Bytes::from(message.payload.clone()),
                ))
            }
            MessageType::Direct => {
                // Direct messages are text
                Ok((
                    MeshtasticPort::TextMessage,
                    Bytes::from(message.payload.clone()),
                ))
            }
            MessageType::Reputation => {
                // Try to decode as VouchMessage
                let vouch_msg: VouchMessage = serde_cbor::from_slice(&message.payload)
                    .map_err(|e| MeshtasticError::TranslationFailed(e.to_string()))?;
                let payload = self.encode_vouch_message(&vouch_msg)?;
                Ok((MeshtasticPort::MycelialVouch, payload))
            }
            MessageType::Credit => {
                let credit_msg: CreditMessage = serde_cbor::from_slice(&message.payload)
                    .map_err(|e| MeshtasticError::TranslationFailed(e.to_string()))?;
                let payload = self.encode_credit_message(&credit_msg)?;
                Ok((MeshtasticPort::MycelialCredit, payload))
            }
            MessageType::Governance => {
                let gov_msg: GovernanceMessage = serde_cbor::from_slice(&message.payload)
                    .map_err(|e| MeshtasticError::TranslationFailed(e.to_string()))?;
                let payload = self.encode_governance_message(&gov_msg)?;
                Ok((MeshtasticPort::MycelialGovernance, payload))
            }
            MessageType::System => {
                // System messages as private app port
                Ok((
                    MeshtasticPort::PrivateApp,
                    Bytes::from(message.payload.clone()),
                ))
            }
        }
    }

    // ========================================================================
    // Compact Binary Encoding for Economics Messages
    // ========================================================================
    //
    // These methods encode economics messages in a compact binary format
    // that fits within LoRa payload limits (237 bytes). The format uses
    // variable-length encoding and omits optional fields when not present.

    /// Encode a VouchMessage to compact binary format
    fn encode_vouch_message(&self, msg: &VouchMessage) -> Result<Bytes> {
        let mut buf = BytesMut::with_capacity(128);

        match msg {
            VouchMessage::VouchRequest(req) => {
                buf.put_u8(0x01); // Type marker
                self.encode_vouch_request(&mut buf, req)?;
            }
            VouchMessage::VouchAck(ack) => {
                buf.put_u8(0x02);
                self.encode_vouch_ack(&mut buf, ack)?;
            }
            VouchMessage::ReputationUpdate(update) => {
                buf.put_u8(0x03);
                // Reputation updates are informational, encode minimally
                buf.put_slice(update.peer_id.as_bytes());
                buf.put_u8(0x00); // Null terminator
                buf.put_f32((update.score * 100.0) as f32); // Score as percentage
            }
        }

        Ok(buf.freeze())
    }

    fn encode_vouch_request(&self, buf: &mut BytesMut, req: &VouchRequest) -> Result<()> {
        // UUID as 16 bytes
        buf.put_slice(req.id.as_bytes());
        // Voucher (truncated to 32 bytes max)
        let voucher = &req.voucher[..req.voucher.len().min(32)];
        buf.put_u8(voucher.len() as u8);
        buf.put_slice(voucher.as_bytes());
        // Vouchee (truncated to 32 bytes max)
        let vouchee = &req.vouchee[..req.vouchee.len().min(32)];
        buf.put_u8(vouchee.len() as u8);
        buf.put_slice(vouchee.as_bytes());
        // Stake as u8 percentage (0-100)
        buf.put_u8((req.stake * 100.0) as u8);
        // Timestamp as Unix seconds (4 bytes is enough until 2106)
        buf.put_u32(req.timestamp.timestamp() as u32);
        Ok(())
    }

    fn encode_vouch_ack(&self, buf: &mut BytesMut, ack: &VouchAck) -> Result<()> {
        buf.put_slice(ack.vouch_id.as_bytes());
        let from = &ack.from[..ack.from.len().min(32)];
        buf.put_u8(from.len() as u8);
        buf.put_slice(from.as_bytes());
        buf.put_u8(if ack.accepted { 1 } else { 0 });
        buf.put_u32(ack.timestamp.timestamp() as u32);
        Ok(())
    }

    /// Decode a VouchMessage from compact binary format
    fn decode_vouch_message(&self, data: &[u8]) -> Result<VouchMessage> {
        if data.is_empty() {
            return Err(MeshtasticError::TranslationFailed(
                "Empty vouch message".to_string(),
            ));
        }

        let mut buf = Bytes::copy_from_slice(data);
        let msg_type = buf.get_u8();

        match msg_type {
            0x01 => {
                let req = self.decode_vouch_request(&mut buf)?;
                Ok(VouchMessage::VouchRequest(req))
            }
            0x02 => {
                let ack = self.decode_vouch_ack(&mut buf)?;
                Ok(VouchMessage::VouchAck(ack))
            }
            0x03 => {
                let update = self.decode_reputation_update(&mut buf)?;
                Ok(VouchMessage::ReputationUpdate(update))
            }
            _ => Err(MeshtasticError::TranslationFailed(format!(
                "Unknown vouch message type: 0x{:02X}",
                msg_type
            ))),
        }
    }

    fn decode_reputation_update(&self, buf: &mut Bytes) -> Result<ReputationUpdate> {
        let peer_id = self.decode_null_terminated_string(buf)?;
        let score = buf.get_f32() as f64 / 100.0; // Decode from percentage

        Ok(ReputationUpdate {
            peer_id,
            score,
            delta: 0.0,
            reason: ReputationChangeReason::Initial,
            timestamp: Utc::now(),
        })
    }

    fn decode_null_terminated_string(&self, buf: &mut Bytes) -> Result<String> {
        let mut bytes = Vec::new();
        while buf.has_remaining() {
            let b = buf.get_u8();
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    fn decode_vouch_request(&self, buf: &mut Bytes) -> Result<VouchRequest> {
        if buf.remaining() < 16 {
            return Err(MeshtasticError::TranslationFailed(
                "Vouch request too short".to_string(),
            ));
        }

        let mut uuid_bytes = [0u8; 16];
        buf.copy_to_slice(&mut uuid_bytes);
        let id = Uuid::from_bytes(uuid_bytes);

        let voucher_len = buf.get_u8() as usize;
        if buf.remaining() < voucher_len {
            return Err(MeshtasticError::TranslationFailed(
                "Invalid voucher length".to_string(),
            ));
        }
        let voucher = String::from_utf8_lossy(&buf.copy_to_bytes(voucher_len)).to_string();

        let vouchee_len = buf.get_u8() as usize;
        if buf.remaining() < vouchee_len {
            return Err(MeshtasticError::TranslationFailed(
                "Invalid vouchee length".to_string(),
            ));
        }
        let vouchee = String::from_utf8_lossy(&buf.copy_to_bytes(vouchee_len)).to_string();

        let stake = buf.get_u8() as f64 / 100.0;
        let timestamp_secs = buf.get_u32() as i64;
        let timestamp = Utc
            .timestamp_opt(timestamp_secs, 0)
            .single()
            .unwrap_or_else(Utc::now);

        Ok(VouchRequest {
            id,
            voucher,
            vouchee,
            stake,
            message: None,
            timestamp,
            expires_at: None,
        })
    }

    fn decode_vouch_ack(&self, buf: &mut Bytes) -> Result<VouchAck> {
        if buf.remaining() < 16 {
            return Err(MeshtasticError::TranslationFailed(
                "Vouch ack too short".to_string(),
            ));
        }

        let mut uuid_bytes = [0u8; 16];
        buf.copy_to_slice(&mut uuid_bytes);
        let vouch_id = Uuid::from_bytes(uuid_bytes);

        let from_len = buf.get_u8() as usize;
        let from = String::from_utf8_lossy(&buf.copy_to_bytes(from_len)).to_string();

        let accepted = buf.get_u8() != 0;
        let timestamp_secs = buf.get_u32() as i64;
        let timestamp = Utc
            .timestamp_opt(timestamp_secs, 0)
            .single()
            .unwrap_or_else(Utc::now);

        Ok(VouchAck {
            vouch_id,
            from,
            accepted,
            reason: None,
            timestamp,
        })
    }

    /// Encode a CreditMessage to compact binary format
    fn encode_credit_message(&self, msg: &CreditMessage) -> Result<Bytes> {
        let mut buf = BytesMut::with_capacity(128);

        match msg {
            CreditMessage::CreateLine(line) => {
                buf.put_u8(0x01);
                buf.put_slice(line.id.as_bytes());
                self.encode_short_string(&mut buf, &line.creditor);
                self.encode_short_string(&mut buf, &line.debtor);
                buf.put_f32(line.limit as f32);
                buf.put_u32(line.timestamp.timestamp() as u32);
            }
            CreditMessage::LineAck(ack) => {
                buf.put_u8(0x02);
                buf.put_slice(ack.line_id.as_bytes());
                self.encode_short_string(&mut buf, &ack.from);
                buf.put_u8(if ack.accepted { 1 } else { 0 });
            }
            CreditMessage::Transfer(transfer) => {
                buf.put_u8(0x03);
                buf.put_slice(transfer.id.as_bytes());
                buf.put_slice(transfer.line_id.as_bytes());
                self.encode_short_string(&mut buf, &transfer.from);
                self.encode_short_string(&mut buf, &transfer.to);
                buf.put_f32(transfer.amount as f32);
            }
            CreditMessage::TransferAck(ack) => {
                buf.put_u8(0x04);
                buf.put_slice(ack.transfer_id.as_bytes());
                buf.put_u8(if ack.success { 1 } else { 0 });
                if let Some(balance) = ack.new_balance {
                    buf.put_f32(balance as f32);
                }
            }
            CreditMessage::LineUpdate(update) => {
                buf.put_u8(0x05);
                buf.put_slice(update.line_id.as_bytes());
                buf.put_f32(update.balance as f32);
                buf.put_f32(update.available as f32);
            }
        }

        Ok(buf.freeze())
    }

    /// Decode a CreditMessage from compact binary format
    fn decode_credit_message(&self, data: &[u8]) -> Result<CreditMessage> {
        if data.is_empty() {
            return Err(MeshtasticError::TranslationFailed(
                "Empty credit message".to_string(),
            ));
        }

        let mut buf = Bytes::copy_from_slice(data);
        let msg_type = buf.get_u8();

        match msg_type {
            0x01 => {
                // CreateLine
                let mut uuid_bytes = [0u8; 16];
                buf.copy_to_slice(&mut uuid_bytes);
                let id = Uuid::from_bytes(uuid_bytes);
                let creditor = self.decode_short_string(&mut buf)?;
                let debtor = self.decode_short_string(&mut buf)?;
                let limit = buf.get_f32() as f64;
                let timestamp_secs = buf.get_u32() as i64;
                let timestamp = Utc
                    .timestamp_opt(timestamp_secs, 0)
                    .single()
                    .unwrap_or_else(Utc::now);

                Ok(CreditMessage::CreateLine(CreateCreditLine {
                    id,
                    creditor,
                    debtor,
                    limit,
                    interest_rate: 0.0,
                    collateral: None,
                    timestamp,
                }))
            }
            0x02 => {
                // LineAck
                let mut uuid_bytes = [0u8; 16];
                buf.copy_to_slice(&mut uuid_bytes);
                let line_id = Uuid::from_bytes(uuid_bytes);
                let from = self.decode_short_string(&mut buf)?;
                let accepted = buf.get_u8() != 0;

                Ok(CreditMessage::LineAck(CreditLineAck {
                    line_id,
                    from,
                    accepted,
                    reason: None,
                    timestamp: Utc::now(),
                }))
            }
            0x03 => {
                // Transfer
                let mut id_bytes = [0u8; 16];
                buf.copy_to_slice(&mut id_bytes);
                let id = Uuid::from_bytes(id_bytes);

                let mut line_id_bytes = [0u8; 16];
                buf.copy_to_slice(&mut line_id_bytes);
                let line_id = Uuid::from_bytes(line_id_bytes);

                let from = self.decode_short_string(&mut buf)?;
                let to = self.decode_short_string(&mut buf)?;
                let amount = buf.get_f32() as f64;

                Ok(CreditMessage::Transfer(CreditTransfer {
                    id,
                    line_id,
                    from,
                    to,
                    amount,
                    memo: None,
                    timestamp: Utc::now(),
                }))
            }
            0x04 => {
                // TransferAck
                let mut uuid_bytes = [0u8; 16];
                buf.copy_to_slice(&mut uuid_bytes);
                let transfer_id = Uuid::from_bytes(uuid_bytes);
                let success = buf.get_u8() != 0;
                let new_balance = if buf.has_remaining() {
                    Some(buf.get_f32() as f64)
                } else {
                    None
                };

                Ok(CreditMessage::TransferAck(CreditTransferAck {
                    transfer_id,
                    success,
                    new_balance,
                    error: None,
                    timestamp: Utc::now(),
                }))
            }
            0x05 => {
                // LineUpdate
                let mut uuid_bytes = [0u8; 16];
                buf.copy_to_slice(&mut uuid_bytes);
                let line_id = Uuid::from_bytes(uuid_bytes);
                let balance = buf.get_f32() as f64;
                let available = buf.get_f32() as f64;

                Ok(CreditMessage::LineUpdate(CreditLineUpdate {
                    line_id,
                    balance,
                    available,
                    limit: available + balance.abs(), // Estimate limit from available + used
                    active: true,
                    last_transaction: Utc::now(),
                }))
            }
            _ => Err(MeshtasticError::TranslationFailed(format!(
                "Unknown credit message type: 0x{:02X}",
                msg_type
            ))),
        }
    }

    /// Encode a GovernanceMessage to compact binary format
    fn encode_governance_message(&self, msg: &GovernanceMessage) -> Result<Bytes> {
        let mut buf = BytesMut::with_capacity(200);

        match msg {
            GovernanceMessage::CreateProposal(proposal) => {
                buf.put_u8(0x01);
                buf.put_slice(proposal.id.as_bytes());
                self.encode_short_string(&mut buf, &proposal.proposer);
                // Title truncated to 64 chars for LoRa
                self.encode_short_string(&mut buf, &proposal.title[..proposal.title.len().min(64)]);
                buf.put_u32(proposal.deadline.timestamp() as u32);
            }
            GovernanceMessage::CastVote(vote) => {
                buf.put_u8(0x02);
                buf.put_slice(vote.proposal_id.as_bytes());
                self.encode_short_string(&mut buf, &vote.voter);
                buf.put_u8(match vote.vote {
                    Vote::For => 1,
                    Vote::Against => 2,
                    Vote::Abstain => 0,
                });
                buf.put_u8((vote.weight * 100.0) as u8);
            }
            GovernanceMessage::ProposalUpdate(update) => {
                buf.put_u8(0x03);
                buf.put_slice(update.proposal_id.as_bytes());
                buf.put_f32(update.votes_for as f32);
                buf.put_f32(update.votes_against as f32);
                buf.put_u16(update.voter_count as u16);
            }
            GovernanceMessage::ProposalExecuted(exec) => {
                buf.put_u8(0x04);
                buf.put_slice(exec.proposal_id.as_bytes());
                buf.put_u8(if exec.success { 1 } else { 0 });
            }
        }

        Ok(buf.freeze())
    }

    /// Decode a GovernanceMessage from compact binary format
    fn decode_governance_message(&self, data: &[u8]) -> Result<GovernanceMessage> {
        if data.is_empty() {
            return Err(MeshtasticError::TranslationFailed(
                "Empty governance message".to_string(),
            ));
        }

        let mut buf = Bytes::copy_from_slice(data);
        let msg_type = buf.get_u8();

        match msg_type {
            0x01 => {
                // CreateProposal
                let mut uuid_bytes = [0u8; 16];
                buf.copy_to_slice(&mut uuid_bytes);
                let id = Uuid::from_bytes(uuid_bytes);

                let proposer = self.decode_short_string(&mut buf)?;
                let title = self.decode_short_string(&mut buf)?;
                let deadline_secs = buf.get_u32() as i64;
                let deadline = Utc
                    .timestamp_opt(deadline_secs, 0)
                    .single()
                    .unwrap_or_else(|| Utc::now() + chrono::Duration::days(7));

                Ok(GovernanceMessage::CreateProposal(CreateProposal {
                    id,
                    proposer,
                    title,
                    description: "[Compressed - full description on IP network]".to_string(),
                    proposal_type: ProposalType::General,
                    quorum: 0.5,
                    threshold: 0.5,
                    deadline,
                    timestamp: Utc::now(),
                }))
            }
            0x02 => {
                // CastVote - most common over LoRa
                let mut uuid_bytes = [0u8; 16];
                buf.copy_to_slice(&mut uuid_bytes);
                let proposal_id = Uuid::from_bytes(uuid_bytes);

                let voter = self.decode_short_string(&mut buf)?;
                let vote_val = buf.get_u8();
                let vote = match vote_val {
                    1 => Vote::For,
                    2 => Vote::Against,
                    _ => Vote::Abstain,
                };
                let weight = buf.get_u8() as f64 / 100.0;

                Ok(GovernanceMessage::CastVote(CastVote {
                    proposal_id,
                    voter,
                    vote,
                    weight,
                    reason: None,
                    timestamp: Utc::now(),
                }))
            }
            0x03 => {
                // ProposalUpdate
                let mut uuid_bytes = [0u8; 16];
                buf.copy_to_slice(&mut uuid_bytes);
                let proposal_id = Uuid::from_bytes(uuid_bytes);

                let votes_for = buf.get_f32() as f64;
                let votes_against = buf.get_f32() as f64;
                let voter_count = buf.get_u16() as u32;

                Ok(GovernanceMessage::ProposalUpdate(ProposalUpdate {
                    proposal_id,
                    status: ProposalStatus::Active,
                    votes_for,
                    votes_against,
                    votes_abstain: 0.0,
                    voter_count,
                    timestamp: Utc::now(),
                }))
            }
            0x04 => {
                // ProposalExecuted
                let mut uuid_bytes = [0u8; 16];
                buf.copy_to_slice(&mut uuid_bytes);
                let proposal_id = Uuid::from_bytes(uuid_bytes);
                let success = buf.get_u8() != 0;

                Ok(GovernanceMessage::ProposalExecuted(ProposalExecuted {
                    proposal_id,
                    success,
                    result: if success {
                        "Executed via LoRa".to_string()
                    } else {
                        "Failed via LoRa".to_string()
                    },
                    timestamp: Utc::now(),
                }))
            }
            _ => Err(MeshtasticError::TranslationFailed(format!(
                "Unknown governance message type: 0x{:02X}",
                msg_type
            ))),
        }
    }

    /// Encode a ResourceMessage to compact binary format
    fn encode_resource_message(&self, msg: &ResourceMessage) -> Result<Bytes> {
        let mut buf = BytesMut::with_capacity(64);

        match msg {
            ResourceMessage::Contribution(contrib) => {
                buf.put_u8(0x01);
                buf.put_slice(contrib.id.as_bytes());
                self.encode_short_string(&mut buf, &contrib.peer_id);
                buf.put_u8(match contrib.resource_type {
                    ResourceType::Bandwidth => 0,
                    ResourceType::Storage => 1,
                    ResourceType::Compute => 2,
                    ResourceType::Relay => 3,
                    ResourceType::Other(_) => 4,
                });
                buf.put_f32(contrib.amount as f32);
            }
            ResourceMessage::Metrics(metrics) => {
                buf.put_u8(0x02);
                self.encode_short_string(&mut buf, &metrics.peer_id);
                buf.put_u64(metrics.uptime_secs);
            }
            ResourceMessage::PoolUpdate(update) => {
                buf.put_u8(0x03);
                buf.put_u32(update.active_contributors);
                buf.put_f32(update.total_bandwidth as f32);
            }
        }

        Ok(buf.freeze())
    }

    /// Decode a ResourceMessage from compact binary format
    fn decode_resource_message(&self, data: &[u8]) -> Result<ResourceMessage> {
        if data.is_empty() {
            return Err(MeshtasticError::TranslationFailed(
                "Empty resource message".to_string(),
            ));
        }

        let mut buf = Bytes::copy_from_slice(data);
        let msg_type = buf.get_u8();

        match msg_type {
            0x01 => {
                // Contribution
                let mut uuid_bytes = [0u8; 16];
                buf.copy_to_slice(&mut uuid_bytes);
                let id = Uuid::from_bytes(uuid_bytes);

                let peer_id = self.decode_short_string(&mut buf)?;
                let res_type_byte = buf.get_u8();
                let resource_type = match res_type_byte {
                    0 => ResourceType::Bandwidth,
                    1 => ResourceType::Storage,
                    2 => ResourceType::Compute,
                    3 => ResourceType::Relay,
                    _ => ResourceType::Other("unknown".to_string()),
                };
                let amount = buf.get_f32() as f64;

                Ok(ResourceMessage::Contribution(ResourceContribution {
                    id,
                    peer_id,
                    resource_type,
                    amount,
                    unit: "units".to_string(),
                    duration_secs: 0,
                    timestamp: Utc::now(),
                }))
            }
            0x02 => {
                // Metrics
                use mycelial_protocol::{BandwidthMetrics, ComputeMetrics, StorageMetrics};

                let peer_id = self.decode_short_string(&mut buf)?;
                let uptime_secs = buf.get_u64();

                Ok(ResourceMessage::Metrics(ResourceMetrics {
                    peer_id,
                    bandwidth: BandwidthMetrics::default(),
                    storage: StorageMetrics::default(),
                    compute: ComputeMetrics::default(),
                    uptime_secs,
                    timestamp: Utc::now(),
                }))
            }
            0x03 => {
                // PoolUpdate
                let active_contributors = buf.get_u32();
                let total_bandwidth = buf.get_f32() as f64;

                Ok(ResourceMessage::PoolUpdate(ResourcePoolUpdate {
                    total_bandwidth,
                    total_storage: 0,
                    total_compute: 0.0,
                    active_contributors,
                    top_contributors: Vec::new(),
                    timestamp: Utc::now(),
                }))
            }
            _ => Err(MeshtasticError::TranslationFailed(format!(
                "Unknown resource message type: 0x{:02X}",
                msg_type
            ))),
        }
    }

    // Helper methods for short string encoding
    fn encode_short_string(&self, buf: &mut BytesMut, s: &str) {
        let truncated = &s[..s.len().min(32)];
        buf.put_u8(truncated.len() as u8);
        buf.put_slice(truncated.as_bytes());
    }

    fn decode_short_string(&self, buf: &mut Bytes) -> Result<String> {
        let len = buf.get_u8() as usize;
        if buf.remaining() < len {
            return Err(MeshtasticError::TranslationFailed(
                "String length exceeds buffer".to_string(),
            ));
        }
        let bytes = buf.copy_to_bytes(len);
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }
}

impl Default for MessageTranslator {
    fn default() -> Self {
        Self::new(NodeIdMapper::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meshtastic_port_conversion() {
        assert_eq!(MeshtasticPort::from(1), MeshtasticPort::TextMessage);
        assert_eq!(MeshtasticPort::from(512), MeshtasticPort::MycelialVouch);
        assert_eq!(u32::from(MeshtasticPort::MycelialCredit), 513);
    }

    #[test]
    fn test_packet_is_broadcast() {
        let packet = MeshtasticPacket::new_outgoing(
            0x12345678,
            0xFFFFFFFF,
            MeshtasticPort::TextMessage,
            Bytes::from("Hello LoRa!"),
            3,
        );
        assert!(packet.is_broadcast());

        let direct_packet = MeshtasticPacket::new_outgoing(
            0x12345678,
            0x87654321,
            MeshtasticPort::TextMessage,
            Bytes::from("Hello!"),
            3,
        );
        assert!(!direct_packet.is_broadcast());
    }

    #[test]
    fn test_vouch_request_encoding_roundtrip() {
        let translator = MessageTranslator::default();

        let original = VouchMessage::VouchRequest(VouchRequest::new(
            "alice".to_string(),
            "bob".to_string(),
            0.75,
        ));

        let encoded = translator.encode_vouch_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);

        let decoded = translator.decode_vouch_message(&encoded).unwrap();

        if let (VouchMessage::VouchRequest(orig), VouchMessage::VouchRequest(dec)) =
            (&original, &decoded)
        {
            assert_eq!(orig.voucher, dec.voucher);
            assert_eq!(orig.vouchee, dec.vouchee);
            assert!((orig.stake - dec.stake).abs() < 0.01); // Allow small float error
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_credit_transfer_encoding_roundtrip() {
        let translator = MessageTranslator::default();
        let line_id = Uuid::new_v4();

        let original = CreditMessage::Transfer(CreditTransfer::new(
            line_id,
            "alice".to_string(),
            "bob".to_string(),
            50.0,
        ));

        let encoded = translator.encode_credit_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);

        let decoded = translator.decode_credit_message(&encoded).unwrap();

        if let (CreditMessage::Transfer(orig), CreditMessage::Transfer(dec)) = (&original, &decoded)
        {
            assert_eq!(orig.from, dec.from);
            assert_eq!(orig.to, dec.to);
            assert!((orig.amount - dec.amount).abs() < 0.01);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_governance_vote_encoding_roundtrip() {
        let translator = MessageTranslator::default();
        let proposal_id = Uuid::new_v4();

        let original = GovernanceMessage::CastVote(CastVote::new(
            proposal_id,
            "alice".to_string(),
            Vote::For,
            0.85,
        ));

        let encoded = translator.encode_governance_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);

        let decoded = translator.decode_governance_message(&encoded).unwrap();

        if let (GovernanceMessage::CastVote(orig), GovernanceMessage::CastVote(dec)) =
            (&original, &decoded)
        {
            assert_eq!(orig.proposal_id, dec.proposal_id);
            assert_eq!(orig.voter, dec.voter);
            assert_eq!(orig.vote, dec.vote);
            assert!((orig.weight - dec.weight).abs() < 0.01);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_resource_contribution_encoding() {
        let translator = MessageTranslator::default();

        let original = ResourceMessage::Contribution(ResourceContribution::new(
            "alice".to_string(),
            ResourceType::Bandwidth,
            1000.0,
            "Mbps".to_string(),
        ));

        let encoded = translator.encode_resource_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);
    }

    // ========================================================================
    // Phase 4: Comprehensive Economics Protocol Tests
    // ========================================================================

    #[test]
    fn test_vouch_ack_encoding_roundtrip() {
        let translator = MessageTranslator::default();

        let original = VouchMessage::VouchAck(VouchAck {
            vouch_id: Uuid::new_v4(),
            from: "charlie".to_string(),
            accepted: true,
            reason: Some("Trusted peer".to_string()),
            timestamp: Utc::now(),
        });

        let encoded = translator.encode_vouch_message(&original).unwrap();
        assert!(
            encoded.len() < LORA_MAX_PAYLOAD,
            "VouchAck should fit in LoRa payload"
        );

        let decoded = translator.decode_vouch_message(&encoded).unwrap();

        if let (VouchMessage::VouchAck(orig), VouchMessage::VouchAck(dec)) = (&original, &decoded) {
            assert_eq!(orig.vouch_id, dec.vouch_id);
            assert_eq!(orig.from, dec.from);
            assert_eq!(orig.accepted, dec.accepted);
        } else {
            panic!("Wrong variant after decode");
        }
    }

    #[test]
    fn test_reputation_update_encoding_roundtrip() {
        let translator = MessageTranslator::default();

        let original = VouchMessage::ReputationUpdate(ReputationUpdate {
            peer_id: "alice".to_string(),
            score: 0.85,
            delta: 0.05,
            reason: ReputationChangeReason::VouchReceived {
                voucher: "bob".to_string(),
                stake: 0.5,
            },
            timestamp: Utc::now(),
        });

        let encoded = translator.encode_vouch_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);

        let decoded = translator.decode_vouch_message(&encoded).unwrap();

        if let (VouchMessage::ReputationUpdate(orig), VouchMessage::ReputationUpdate(dec)) =
            (&original, &decoded)
        {
            assert_eq!(orig.peer_id, dec.peer_id);
            assert!((orig.score - dec.score).abs() < 0.02); // Allow for float encoding precision
        } else {
            panic!("Wrong variant after decode");
        }
    }

    #[test]
    fn test_create_credit_line_encoding_roundtrip() {
        let translator = MessageTranslator::default();

        let original = CreditMessage::CreateLine(CreateCreditLine::new(
            "alice".to_string(),
            "bob".to_string(),
            1000.0,
        ));

        let encoded = translator.encode_credit_message(&original).unwrap();
        assert!(
            encoded.len() < LORA_MAX_PAYLOAD,
            "CreateCreditLine should fit in LoRa payload"
        );

        let decoded = translator.decode_credit_message(&encoded).unwrap();

        if let (CreditMessage::CreateLine(orig), CreditMessage::CreateLine(dec)) =
            (&original, &decoded)
        {
            assert_eq!(orig.creditor, dec.creditor);
            assert_eq!(orig.debtor, dec.debtor);
            assert!((orig.limit - dec.limit).abs() < 0.01);
        } else {
            panic!("Wrong variant after decode");
        }
    }

    #[test]
    fn test_credit_line_ack_encoding_roundtrip() {
        let translator = MessageTranslator::default();

        let original = CreditMessage::LineAck(CreditLineAck {
            line_id: Uuid::new_v4(),
            from: "bob".to_string(),
            accepted: true,
            reason: None,
            timestamp: Utc::now(),
        });

        let encoded = translator.encode_credit_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);

        let decoded = translator.decode_credit_message(&encoded).unwrap();

        if let (CreditMessage::LineAck(orig), CreditMessage::LineAck(dec)) = (&original, &decoded) {
            assert_eq!(orig.line_id, dec.line_id);
            assert_eq!(orig.from, dec.from);
            assert_eq!(orig.accepted, dec.accepted);
        } else {
            panic!("Wrong variant after decode");
        }
    }

    #[test]
    fn test_credit_transfer_ack_encoding_roundtrip() {
        let translator = MessageTranslator::default();

        let original = CreditMessage::TransferAck(CreditTransferAck {
            transfer_id: Uuid::new_v4(),
            success: true,
            new_balance: Some(450.0),
            error: None,
            timestamp: Utc::now(),
        });

        let encoded = translator.encode_credit_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);

        let decoded = translator.decode_credit_message(&encoded).unwrap();

        if let (CreditMessage::TransferAck(orig), CreditMessage::TransferAck(dec)) =
            (&original, &decoded)
        {
            assert_eq!(orig.transfer_id, dec.transfer_id);
            assert_eq!(orig.success, dec.success);
            if let (Some(orig_bal), Some(dec_bal)) = (orig.new_balance, dec.new_balance) {
                assert!((orig_bal - dec_bal).abs() < 0.01);
            }
        } else {
            panic!("Wrong variant after decode");
        }
    }

    #[test]
    fn test_credit_line_update_encoding_roundtrip() {
        let translator = MessageTranslator::default();

        let original = CreditMessage::LineUpdate(CreditLineUpdate {
            line_id: Uuid::new_v4(),
            balance: -50.0,
            available: 950.0,
            limit: 1000.0,
            active: true,
            last_transaction: Utc::now(),
        });

        let encoded = translator.encode_credit_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);

        let decoded = translator.decode_credit_message(&encoded).unwrap();

        if let (CreditMessage::LineUpdate(orig), CreditMessage::LineUpdate(dec)) =
            (&original, &decoded)
        {
            assert_eq!(orig.line_id, dec.line_id);
            assert!((orig.balance - dec.balance).abs() < 0.01);
            assert!((orig.available - dec.available).abs() < 0.01);
        } else {
            panic!("Wrong variant after decode");
        }
    }

    #[test]
    fn test_create_proposal_encoding_roundtrip() {
        let translator = MessageTranslator::default();

        let original = GovernanceMessage::CreateProposal(CreateProposal::new(
            "alice".to_string(),
            "Network Upgrade v2".to_string(),
            "Upgrade to protocol version 2.0".to_string(),
        ));

        let encoded = translator.encode_governance_message(&original).unwrap();
        assert!(
            encoded.len() < LORA_MAX_PAYLOAD,
            "CreateProposal should fit in LoRa payload"
        );

        let decoded = translator.decode_governance_message(&encoded).unwrap();

        if let (GovernanceMessage::CreateProposal(orig), GovernanceMessage::CreateProposal(dec)) =
            (&original, &decoded)
        {
            assert_eq!(orig.id, dec.id);
            assert_eq!(orig.proposer, dec.proposer);
            assert_eq!(orig.title, dec.title);
        } else {
            panic!("Wrong variant after decode");
        }
    }

    #[test]
    fn test_proposal_update_encoding_roundtrip() {
        let translator = MessageTranslator::default();

        let original = GovernanceMessage::ProposalUpdate(ProposalUpdate {
            proposal_id: Uuid::new_v4(),
            status: ProposalStatus::Active,
            votes_for: 75.5,
            votes_against: 24.5,
            votes_abstain: 10.0,
            voter_count: 100,
            timestamp: Utc::now(),
        });

        let encoded = translator.encode_governance_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);

        let decoded = translator.decode_governance_message(&encoded).unwrap();

        if let (GovernanceMessage::ProposalUpdate(orig), GovernanceMessage::ProposalUpdate(dec)) =
            (&original, &decoded)
        {
            assert_eq!(orig.proposal_id, dec.proposal_id);
            assert!((orig.votes_for - dec.votes_for).abs() < 0.1);
            assert!((orig.votes_against - dec.votes_against).abs() < 0.1);
            assert_eq!(orig.voter_count, dec.voter_count);
        } else {
            panic!("Wrong variant after decode");
        }
    }

    #[test]
    fn test_proposal_executed_encoding_roundtrip() {
        let translator = MessageTranslator::default();

        let original = GovernanceMessage::ProposalExecuted(ProposalExecuted {
            proposal_id: Uuid::new_v4(),
            success: true,
            result: "Upgrade applied successfully".to_string(),
            timestamp: Utc::now(),
        });

        let encoded = translator.encode_governance_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);

        let decoded = translator.decode_governance_message(&encoded).unwrap();

        if let (
            GovernanceMessage::ProposalExecuted(orig),
            GovernanceMessage::ProposalExecuted(dec),
        ) = (&original, &decoded)
        {
            assert_eq!(orig.proposal_id, dec.proposal_id);
            assert_eq!(orig.success, dec.success);
        } else {
            panic!("Wrong variant after decode");
        }
    }

    #[test]
    fn test_governance_vote_all_variants() {
        let translator = MessageTranslator::default();
        let proposal_id = Uuid::new_v4();

        // Test For vote
        let for_vote = GovernanceMessage::CastVote(CastVote::new(
            proposal_id,
            "alice".to_string(),
            Vote::For,
            0.8,
        ));
        let encoded = translator.encode_governance_message(&for_vote).unwrap();
        let decoded = translator.decode_governance_message(&encoded).unwrap();
        if let GovernanceMessage::CastVote(v) = decoded {
            assert_eq!(v.vote, Vote::For);
        }

        // Test Against vote
        let against_vote = GovernanceMessage::CastVote(CastVote::new(
            proposal_id,
            "bob".to_string(),
            Vote::Against,
            0.6,
        ));
        let encoded = translator.encode_governance_message(&against_vote).unwrap();
        let decoded = translator.decode_governance_message(&encoded).unwrap();
        if let GovernanceMessage::CastVote(v) = decoded {
            assert_eq!(v.vote, Vote::Against);
        }

        // Test Abstain vote
        let abstain_vote = GovernanceMessage::CastVote(CastVote::new(
            proposal_id,
            "charlie".to_string(),
            Vote::Abstain,
            0.4,
        ));
        let encoded = translator.encode_governance_message(&abstain_vote).unwrap();
        let decoded = translator.decode_governance_message(&encoded).unwrap();
        if let GovernanceMessage::CastVote(v) = decoded {
            assert_eq!(v.vote, Vote::Abstain);
        }
    }

    #[test]
    fn test_resource_contribution_roundtrip() {
        let translator = MessageTranslator::default();

        let original = ResourceMessage::Contribution(ResourceContribution::new(
            "alice".to_string(),
            ResourceType::Storage,
            500.0,
            "GB".to_string(),
        ));

        let encoded = translator.encode_resource_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);

        let decoded = translator.decode_resource_message(&encoded).unwrap();

        if let (ResourceMessage::Contribution(orig), ResourceMessage::Contribution(dec)) =
            (&original, &decoded)
        {
            assert_eq!(orig.peer_id, dec.peer_id);
            assert_eq!(orig.resource_type, dec.resource_type);
            assert!((orig.amount - dec.amount).abs() < 0.01);
        } else {
            panic!("Wrong variant after decode");
        }
    }

    #[test]
    fn test_resource_metrics_encoding_roundtrip() {
        use mycelial_protocol::{BandwidthMetrics, ComputeMetrics, StorageMetrics};

        let translator = MessageTranslator::default();

        let original = ResourceMessage::Metrics(ResourceMetrics {
            peer_id: "alice".to_string(),
            bandwidth: BandwidthMetrics::default(),
            storage: StorageMetrics::default(),
            compute: ComputeMetrics::default(),
            uptime_secs: 86400,
            timestamp: Utc::now(),
        });

        let encoded = translator.encode_resource_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);

        let decoded = translator.decode_resource_message(&encoded).unwrap();

        if let (ResourceMessage::Metrics(orig), ResourceMessage::Metrics(dec)) =
            (&original, &decoded)
        {
            assert_eq!(orig.peer_id, dec.peer_id);
            assert_eq!(orig.uptime_secs, dec.uptime_secs);
        } else {
            panic!("Wrong variant after decode");
        }
    }

    #[test]
    fn test_resource_pool_update_encoding_roundtrip() {
        let translator = MessageTranslator::default();

        let original = ResourceMessage::PoolUpdate(ResourcePoolUpdate {
            total_bandwidth: 10000.0,
            total_storage: 1000000,
            total_compute: 500.0,
            active_contributors: 50,
            top_contributors: Vec::new(),
            timestamp: Utc::now(),
        });

        let encoded = translator.encode_resource_message(&original).unwrap();
        assert!(encoded.len() < LORA_MAX_PAYLOAD);

        let decoded = translator.decode_resource_message(&encoded).unwrap();

        if let (ResourceMessage::PoolUpdate(orig), ResourceMessage::PoolUpdate(dec)) =
            (&original, &decoded)
        {
            assert_eq!(orig.active_contributors, dec.active_contributors);
            assert!((orig.total_bandwidth - dec.total_bandwidth).abs() < 0.1);
        } else {
            panic!("Wrong variant after decode");
        }
    }

    #[test]
    fn test_all_resource_types() {
        let translator = MessageTranslator::default();

        let types = vec![
            ResourceType::Bandwidth,
            ResourceType::Storage,
            ResourceType::Compute,
            ResourceType::Relay,
            ResourceType::Other("custom".to_string()),
        ];

        for res_type in types {
            let msg = ResourceMessage::Contribution(ResourceContribution::new(
                "peer".to_string(),
                res_type.clone(),
                100.0,
                "units".to_string(),
            ));

            let encoded = translator.encode_resource_message(&msg).unwrap();
            let decoded = translator.decode_resource_message(&encoded).unwrap();

            if let ResourceMessage::Contribution(contrib) = decoded {
                match (&res_type, &contrib.resource_type) {
                    (ResourceType::Other(_), ResourceType::Other(_)) => {} // Both are Other variant
                    _ => assert_eq!(res_type, contrib.resource_type),
                }
            } else {
                panic!("Wrong variant");
            }
        }
    }

    #[test]
    fn test_message_size_within_lora_limits() {
        let translator = MessageTranslator::default();

        // Test maximum-length peer IDs (32 chars)
        let long_peer_id = "a".repeat(32);

        let vouch = VouchMessage::VouchRequest(VouchRequest::new(
            long_peer_id.clone(),
            long_peer_id.clone(),
            0.5,
        ));
        let encoded = translator.encode_vouch_message(&vouch).unwrap();
        assert!(
            encoded.len() <= LORA_MAX_PAYLOAD,
            "VouchRequest with max-length IDs: {} bytes > {} limit",
            encoded.len(),
            LORA_MAX_PAYLOAD
        );

        let credit = CreditMessage::CreateLine(CreateCreditLine::new(
            long_peer_id.clone(),
            long_peer_id.clone(),
            99999.0,
        ));
        let encoded = translator.encode_credit_message(&credit).unwrap();
        assert!(
            encoded.len() <= LORA_MAX_PAYLOAD,
            "CreateCreditLine with max-length IDs: {} bytes > {} limit",
            encoded.len(),
            LORA_MAX_PAYLOAD
        );
    }
}
