//! Message compression and chunking for LoRa payload optimization
//!
//! This module provides compression and chunking support for economics protocol
//! messages that may exceed the LoRa 237-byte payload limit.
//!
//! # Compression Strategy
//!
//! 1. **Small messages (<200 bytes)**: No compression, send as-is
//! 2. **Medium messages (200-400 bytes)**: Apply deflate compression
//! 3. **Large messages (>400 bytes)**: Compress + chunk into multiple packets
//!
//! # Chunking Protocol
//!
//! Chunked messages use the following header format:
//! - Byte 0: Chunk flags (0x80 = first chunk, 0x40 = last chunk)
//! - Bytes 1-4: Message ID (for reassembly)
//! - Byte 5: Chunk index (0-255)
//! - Byte 6: Total chunks
//! - Bytes 7+: Payload data

use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, trace, warn};

use crate::config::LORA_MAX_PAYLOAD;
use crate::error::{MeshtasticError, Result};

/// Chunk header size in bytes
const CHUNK_HEADER_SIZE: usize = 7;

/// Maximum payload per chunk after header
const CHUNK_PAYLOAD_SIZE: usize = LORA_MAX_PAYLOAD - CHUNK_HEADER_SIZE;

/// Threshold above which to apply compression
const COMPRESSION_THRESHOLD: usize = 200;

/// Threshold above which chunking is required
const CHUNKING_THRESHOLD: usize = LORA_MAX_PAYLOAD;

/// Chunk flags
const FLAG_FIRST_CHUNK: u8 = 0x80;
const FLAG_LAST_CHUNK: u8 = 0x40;
const FLAG_COMPRESSED: u8 = 0x20;

/// Message compressor for LoRa payload optimization
#[derive(Debug)]
pub struct MessageCompressor {
    /// Compression level (0-10, higher = better compression, slower)
    compression_level: u32,
}

impl MessageCompressor {
    /// Create a new message compressor with default settings
    pub fn new() -> Self {
        Self {
            compression_level: 6,
        }
    }

    /// Create with custom compression level
    pub fn with_level(compression_level: u32) -> Self {
        Self {
            compression_level: compression_level.min(10),
        }
    }

    /// Compress data using deflate algorithm
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < COMPRESSION_THRESHOLD {
            // Don't compress small payloads
            return Ok(data.to_vec());
        }

        // Use miniz_oxide for deflate compression
        let compressed = miniz_oxide::deflate::compress_to_vec(data, self.compression_level as u8);

        // Only use compression if it actually saves space
        if compressed.len() < data.len() {
            debug!(
                "Compressed {} bytes to {} bytes ({}% reduction)",
                data.len(),
                compressed.len(),
                100 - (compressed.len() * 100 / data.len())
            );
            Ok(compressed)
        } else {
            debug!(
                "Compression not beneficial ({} -> {} bytes), using original",
                data.len(),
                compressed.len()
            );
            Ok(data.to_vec())
        }
    }

    /// Decompress data
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        miniz_oxide::inflate::decompress_to_vec(data).map_err(|e| {
            MeshtasticError::CompressionFailed(format!("Decompression error: {:?}", e))
        })
    }

    /// Check if compression would be beneficial for this data
    pub fn should_compress(&self, data: &[u8]) -> bool {
        data.len() >= COMPRESSION_THRESHOLD
    }

    /// Check if data needs to be chunked
    pub fn needs_chunking(&self, data: &[u8]) -> bool {
        data.len() > CHUNKING_THRESHOLD
    }
}

impl Default for MessageCompressor {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a single chunk of a larger message
#[derive(Debug, Clone)]
pub struct MessageChunk {
    /// Unique message ID for reassembly
    pub message_id: u32,
    /// Index of this chunk (0-based)
    pub chunk_index: u8,
    /// Total number of chunks
    pub total_chunks: u8,
    /// Whether this is the first chunk
    pub is_first: bool,
    /// Whether this is the last chunk
    pub is_last: bool,
    /// Whether the payload is compressed
    pub is_compressed: bool,
    /// Chunk payload data
    pub payload: Bytes,
}

impl MessageChunk {
    /// Encode chunk to bytes for transmission
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(CHUNK_HEADER_SIZE + self.payload.len());

        // Flags byte
        let mut flags = 0u8;
        if self.is_first {
            flags |= FLAG_FIRST_CHUNK;
        }
        if self.is_last {
            flags |= FLAG_LAST_CHUNK;
        }
        if self.is_compressed {
            flags |= FLAG_COMPRESSED;
        }
        buf.put_u8(flags);

        // Message ID
        buf.put_u32(self.message_id);

        // Chunk metadata
        buf.put_u8(self.chunk_index);
        buf.put_u8(self.total_chunks);

        // Payload
        buf.put_slice(&self.payload);

        buf.freeze()
    }

    /// Decode chunk from received bytes
    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < CHUNK_HEADER_SIZE {
            return Err(MeshtasticError::InvalidPacket(
                "Chunk too short".to_string(),
            ));
        }

        let mut buf = Bytes::copy_from_slice(data);

        let flags = buf.get_u8();
        let message_id = buf.get_u32();
        let chunk_index = buf.get_u8();
        let total_chunks = buf.get_u8();

        let payload = buf.copy_to_bytes(buf.remaining());

        Ok(Self {
            message_id,
            chunk_index,
            total_chunks,
            is_first: (flags & FLAG_FIRST_CHUNK) != 0,
            is_last: (flags & FLAG_LAST_CHUNK) != 0,
            is_compressed: (flags & FLAG_COMPRESSED) != 0,
            payload,
        })
    }
}

/// Chunker for splitting large messages into LoRa-sized packets
#[derive(Debug)]
pub struct MessageChunker {
    compressor: MessageCompressor,
    /// Counter for generating message IDs
    message_counter: u32,
}

impl MessageChunker {
    /// Create a new message chunker
    pub fn new() -> Self {
        Self {
            compressor: MessageCompressor::new(),
            message_counter: rand::random(),
        }
    }

    /// Split a message into chunks for transmission
    pub fn chunk(&mut self, data: &[u8]) -> Result<Vec<MessageChunk>> {
        // Try compression first
        let compressed = self.compressor.compress(data)?;
        let is_compressed = compressed.len() < data.len();
        let payload = if is_compressed { &compressed } else { data };

        // Check if chunking is needed
        if payload.len() <= LORA_MAX_PAYLOAD {
            // Single chunk (no chunking needed)
            return Ok(vec![MessageChunk {
                message_id: self.next_message_id(),
                chunk_index: 0,
                total_chunks: 1,
                is_first: true,
                is_last: true,
                is_compressed,
                payload: Bytes::copy_from_slice(payload),
            }]);
        }

        // Calculate number of chunks needed
        let total_chunks = payload.len().div_ceil(CHUNK_PAYLOAD_SIZE);

        if total_chunks > 255 {
            return Err(MeshtasticError::MessageTooLarge {
                size: payload.len(),
                max: CHUNK_PAYLOAD_SIZE * 255,
            });
        }

        let message_id = self.next_message_id();
        let mut chunks = Vec::with_capacity(total_chunks);

        for (index, chunk_data) in payload.chunks(CHUNK_PAYLOAD_SIZE).enumerate() {
            chunks.push(MessageChunk {
                message_id,
                chunk_index: index as u8,
                total_chunks: total_chunks as u8,
                is_first: index == 0,
                is_last: index == total_chunks - 1,
                is_compressed,
                payload: Bytes::copy_from_slice(chunk_data),
            });
        }

        debug!(
            "Split {} byte message into {} chunks (compressed: {})",
            data.len(),
            chunks.len(),
            is_compressed
        );

        Ok(chunks)
    }

    fn next_message_id(&mut self) -> u32 {
        self.message_counter = self.message_counter.wrapping_add(1);
        self.message_counter
    }
}

impl Default for MessageChunker {
    fn default() -> Self {
        Self::new()
    }
}

/// Reassembly buffer entry
#[derive(Debug)]
struct ReassemblyEntry {
    /// Received chunks (indexed by chunk_index)
    chunks: HashMap<u8, Bytes>,
    /// Total chunks expected
    total_chunks: u8,
    /// Whether payload is compressed
    is_compressed: bool,
    /// When the first chunk was received
    created_at: Instant,
}

/// Reassembler for combining chunks back into complete messages
#[derive(Debug)]
pub struct MessageReassembler {
    /// Pending reassembly entries keyed by message_id
    pending: HashMap<u32, ReassemblyEntry>,
    /// Timeout for incomplete messages
    timeout: Duration,
    /// Compressor for decompression
    compressor: MessageCompressor,
}

impl MessageReassembler {
    /// Create a new reassembler
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            timeout: Duration::from_secs(30),
            compressor: MessageCompressor::new(),
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            pending: HashMap::new(),
            timeout,
            compressor: MessageCompressor::new(),
        }
    }

    /// Add a chunk to the reassembly buffer
    ///
    /// Returns `Some(data)` if the message is complete, `None` otherwise
    pub fn add_chunk(&mut self, chunk: MessageChunk) -> Result<Option<Vec<u8>>> {
        // Clean up expired entries
        self.expire_old_entries();

        // Single-chunk message - return immediately
        if chunk.total_chunks == 1 && chunk.is_first && chunk.is_last {
            let data = chunk.payload.to_vec();
            return if chunk.is_compressed {
                self.compressor.decompress(&data).map(Some)
            } else {
                Ok(Some(data))
            };
        }

        // Multi-chunk message
        let entry = self
            .pending
            .entry(chunk.message_id)
            .or_insert_with(|| ReassemblyEntry {
                chunks: HashMap::new(),
                total_chunks: chunk.total_chunks,
                is_compressed: chunk.is_compressed,
                created_at: Instant::now(),
            });

        // Store the chunk
        entry.chunks.insert(chunk.chunk_index, chunk.payload);

        trace!(
            "Received chunk {}/{} for message {}",
            chunk.chunk_index + 1,
            entry.total_chunks,
            chunk.message_id
        );

        // Check if complete
        if entry.chunks.len() == entry.total_chunks as usize {
            // Reassemble in order
            let mut complete = Vec::new();
            for i in 0..entry.total_chunks {
                if let Some(chunk_data) = entry.chunks.get(&i) {
                    complete.extend_from_slice(chunk_data);
                } else {
                    warn!("Missing chunk {} for message {}", i, chunk.message_id);
                    return Ok(None);
                }
            }

            let is_compressed = entry.is_compressed;

            // Remove from pending
            self.pending.remove(&chunk.message_id);

            debug!(
                "Reassembled message {} ({} bytes, compressed: {})",
                chunk.message_id,
                complete.len(),
                is_compressed
            );

            // Decompress if needed
            if is_compressed {
                self.compressor.decompress(&complete).map(Some)
            } else {
                Ok(Some(complete))
            }
        } else {
            Ok(None)
        }
    }

    /// Expire old incomplete messages
    fn expire_old_entries(&mut self) {
        let now = Instant::now();
        self.pending.retain(|msg_id, entry| {
            let keep = now.duration_since(entry.created_at) < self.timeout;
            if !keep {
                warn!(
                    "Expiring incomplete message {} ({}/{} chunks received)",
                    msg_id,
                    entry.chunks.len(),
                    entry.total_chunks
                );
            }
            keep
        });
    }

    /// Get the number of pending incomplete messages
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

impl Default for MessageReassembler {
    fn default() -> Self {
        Self::new()
    }
}

/// Economics message wrapper that handles compression/chunking transparently
#[derive(Debug)]
pub struct EconomicsMessageCodec {
    chunker: MessageChunker,
    reassembler: MessageReassembler,
}

impl EconomicsMessageCodec {
    /// Create a new economics message codec
    pub fn new() -> Self {
        Self {
            chunker: MessageChunker::new(),
            reassembler: MessageReassembler::new(),
        }
    }

    /// Encode a message, applying compression and chunking as needed
    pub fn encode(&mut self, data: &[u8]) -> Result<Vec<Bytes>> {
        let chunks = self.chunker.chunk(data)?;
        Ok(chunks.into_iter().map(|c| c.encode()).collect())
    }

    /// Decode a received packet
    ///
    /// Returns `Some(data)` if a complete message is ready, `None` if waiting for more chunks
    pub fn decode(&mut self, packet: &[u8]) -> Result<Option<Vec<u8>>> {
        let chunk = MessageChunk::decode(packet)?;
        self.reassembler.add_chunk(chunk)
    }

    /// Get pending reassembly count
    pub fn pending_count(&self) -> usize {
        self.reassembler.pending_count()
    }
}

impl Default for EconomicsMessageCodec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressor_small_data() {
        let compressor = MessageCompressor::new();
        let data = b"small";

        let compressed = compressor.compress(data).unwrap();
        assert_eq!(compressed, data); // No compression for small data
    }

    #[test]
    fn test_compressor_large_data() {
        let compressor = MessageCompressor::new();
        // Create compressible data (repeated pattern)
        let data: Vec<u8> = (0u32..500).map(|i| (i % 10) as u8).collect();

        let compressed = compressor.compress(&data).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_chunk_encode_decode() {
        let chunk = MessageChunk {
            message_id: 12345,
            chunk_index: 2,
            total_chunks: 5,
            is_first: false,
            is_last: false,
            is_compressed: true,
            payload: Bytes::from(vec![1, 2, 3, 4, 5]),
        };

        let encoded = chunk.encode();
        let decoded = MessageChunk::decode(&encoded).unwrap();

        assert_eq!(decoded.message_id, 12345);
        assert_eq!(decoded.chunk_index, 2);
        assert_eq!(decoded.total_chunks, 5);
        assert!(!decoded.is_first);
        assert!(!decoded.is_last);
        assert!(decoded.is_compressed);
        assert_eq!(decoded.payload, Bytes::from(vec![1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_chunker_small_message() {
        let mut chunker = MessageChunker::new();
        let data = vec![1, 2, 3, 4, 5];

        let chunks = chunker.chunk(&data).unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].is_first);
        assert!(chunks[0].is_last);
    }

    #[test]
    fn test_chunker_large_message() {
        let mut chunker = MessageChunker::new();
        // Create a message larger than LORA_MAX_PAYLOAD
        let data: Vec<u8> = (0u32..500).map(|i| (i % 256) as u8).collect();

        let chunks = chunker.chunk(&data).unwrap();
        assert!(chunks.len() > 1);
        assert!(chunks[0].is_first);
        assert!(!chunks[0].is_last);
        assert!(!chunks.last().unwrap().is_first);
        assert!(chunks.last().unwrap().is_last);
    }

    #[test]
    fn test_reassembler_single_chunk() {
        let mut reassembler = MessageReassembler::new();

        let chunk = MessageChunk {
            message_id: 1,
            chunk_index: 0,
            total_chunks: 1,
            is_first: true,
            is_last: true,
            is_compressed: false,
            payload: Bytes::from(vec![1, 2, 3]),
        };

        let result = reassembler.add_chunk(chunk).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_reassembler_multi_chunk() {
        let mut reassembler = MessageReassembler::new();

        // Chunk 0
        let chunk0 = MessageChunk {
            message_id: 42,
            chunk_index: 0,
            total_chunks: 3,
            is_first: true,
            is_last: false,
            is_compressed: false,
            payload: Bytes::from(vec![1, 2, 3]),
        };

        // Chunk 1
        let chunk1 = MessageChunk {
            message_id: 42,
            chunk_index: 1,
            total_chunks: 3,
            is_first: false,
            is_last: false,
            is_compressed: false,
            payload: Bytes::from(vec![4, 5, 6]),
        };

        // Chunk 2
        let chunk2 = MessageChunk {
            message_id: 42,
            chunk_index: 2,
            total_chunks: 3,
            is_first: false,
            is_last: true,
            is_compressed: false,
            payload: Bytes::from(vec![7, 8, 9]),
        };

        // Add out of order
        assert!(reassembler.add_chunk(chunk1).unwrap().is_none());
        assert!(reassembler.add_chunk(chunk0).unwrap().is_none());

        let result = reassembler.add_chunk(chunk2).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_economics_codec_roundtrip() {
        let mut encoder = EconomicsMessageCodec::new();
        let mut decoder = EconomicsMessageCodec::new();

        let original_data = b"Hello from economics protocol!".to_vec();

        let encoded = encoder.encode(&original_data).unwrap();
        assert_eq!(encoded.len(), 1); // Should be single packet

        let decoded = decoder.decode(&encoded[0]).unwrap();
        assert!(decoded.is_some());
        assert_eq!(decoded.unwrap(), original_data);
    }

    #[test]
    fn test_economics_codec_large_message() {
        let mut encoder = EconomicsMessageCodec::new();
        let mut decoder = EconomicsMessageCodec::new();

        // Create a large governance proposal-like message
        let original_data: Vec<u8> = (0u32..1000).map(|i| (i % 256) as u8).collect();

        let encoded = encoder.encode(&original_data).unwrap();
        assert!(encoded.len() > 1); // Should require multiple chunks

        // Decode all chunks
        let mut result = None;
        for packet in encoded {
            result = decoder.decode(&packet).unwrap();
        }

        assert!(result.is_some());
        assert_eq!(result.unwrap(), original_data);
    }
}
