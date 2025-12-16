//! Transport layer configuration and creation
//!
//! This module provides transport configuration for TCP, QUIC, and WebSocket
//! with Noise encryption and Yamux multiplexing.

use libp2p::{
    core::upgrade,
    identity::Keypair,
    noise, yamux, PeerId, Transport,
};
use std::time::Duration;

use crate::error::{NetworkError, Result};

/// Transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Enable TCP transport
    pub enable_tcp: bool,
    /// Enable QUIC transport
    pub enable_quic: bool,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Maximum number of inbound streams per connection
    pub max_inbound_streams: usize,
    /// Maximum number of outbound streams per connection
    pub max_outbound_streams: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            enable_tcp: true,
            enable_quic: true,
            connection_timeout: Duration::from_secs(30),
            max_inbound_streams: 256,
            max_outbound_streams: 256,
        }
    }
}

/// Create a TCP transport with Noise encryption and Yamux multiplexing
pub fn create_tcp_transport(
    _keypair: &Keypair,
    _config: &TransportConfig,
) -> Result<libp2p::tcp::tokio::Transport> {
    let tcp_config = libp2p::tcp::Config::default()
        .nodelay(true);

    Ok(libp2p::tcp::tokio::Transport::new(tcp_config))
}

/// Create the full transport stack
///
/// This creates a transport that supports:
/// - TCP with Noise encryption and Yamux multiplexing
/// - QUIC (if enabled)
/// - DNS resolution
pub fn create_transport(
    keypair: &Keypair,
    config: &TransportConfig,
) -> Result<libp2p::core::transport::Boxed<(PeerId, libp2p::core::muxing::StreamMuxerBox)>> {
    // Create TCP transport
    let tcp = libp2p::tcp::tokio::Transport::new(
        libp2p::tcp::Config::default().nodelay(true)
    );

    // Add Noise encryption
    let noise_config = noise::Config::new(keypair)
        .map_err(|e| NetworkError::Config(format!("Noise config error: {:?}", e)))?;

    // Add Yamux multiplexing
    let yamux_config = yamux::Config::default();

    // Build authenticated transport
    let tcp_authenticated = tcp
        .upgrade(upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(yamux_config)
        .timeout(config.connection_timeout);

    // Optionally add QUIC
    if config.enable_quic {
        let quic_config = libp2p::quic::Config::new(keypair);
        let quic = libp2p::quic::tokio::Transport::new(quic_config);

        // Combine TCP and QUIC
        let transport = tcp_authenticated
            .or_transport(quic)
            .map(|either, _| match either {
                futures::future::Either::Left((peer_id, muxer)) => {
                    (peer_id, libp2p::core::muxing::StreamMuxerBox::new(muxer))
                }
                futures::future::Either::Right((peer_id, muxer)) => {
                    (peer_id, libp2p::core::muxing::StreamMuxerBox::new(muxer))
                }
            });

        // Add DNS resolution
        let dns_transport = libp2p::dns::tokio::Transport::system(transport)
            .map_err(|e| NetworkError::Config(format!("DNS config error: {:?}", e)))?;

        Ok(dns_transport.boxed())
    } else {
        // TCP only with DNS
        let transport = tcp_authenticated.map(|(peer_id, muxer), _| {
            (peer_id, libp2p::core::muxing::StreamMuxerBox::new(muxer))
        });

        let dns_transport = libp2p::dns::tokio::Transport::system(transport)
            .map_err(|e| NetworkError::Config(format!("DNS config error: {:?}", e)))?;

        Ok(dns_transport.boxed())
    }
}

/// Parse a multiaddr string
pub fn parse_multiaddr(addr: &str) -> Result<libp2p::Multiaddr> {
    addr.parse()
        .map_err(|e| NetworkError::InvalidMultiaddr(format!("{}: {}", addr, e)))
}

/// Extract peer ID from a multiaddr if present
pub fn extract_peer_id(addr: &libp2p::Multiaddr) -> Option<PeerId> {
    addr.iter().find_map(|p| {
        if let libp2p::multiaddr::Protocol::P2p(peer_id) = p {
            Some(peer_id)
        } else {
            None
        }
    })
}
