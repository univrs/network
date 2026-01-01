//! Network service unit tests
//!
//! These tests verify the behavior of the NetworkService and NetworkHandle.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::config::NetworkConfig;
    use crate::error::NetworkError;
    use libp2p::PeerId;
    use tokio::sync::mpsc;

    // ============ NetworkHandle Tests ============

    #[tokio::test]
    async fn test_mock_network_handle_creation() {
        let (handle, _rx) = NetworkHandle::mock();
        // Handle should have a valid peer ID
        assert!(!handle.local_peer_id().to_string().is_empty());
    }

    #[tokio::test]
    async fn test_mock_network_handle_with_peer_id() {
        let peer_id = PeerId::random();
        let (handle, _rx) = NetworkHandle::mock_with_peer_id(peer_id);
        assert_eq!(handle.local_peer_id(), peer_id);
    }

    #[tokio::test]
    async fn test_network_handle_subscribe() {
        let (handle, mut rx) = NetworkHandle::mock();

        handle.subscribe("test-topic").await.unwrap();

        match rx.recv().await.unwrap() {
            NetworkCommand::Subscribe { topic } => {
                assert_eq!(topic, "test-topic");
            }
            _ => panic!("Expected Subscribe command"),
        }
    }

    #[tokio::test]
    async fn test_network_handle_unsubscribe() {
        let (handle, mut rx) = NetworkHandle::mock();

        handle.unsubscribe("test-topic").await.unwrap();

        match rx.recv().await.unwrap() {
            NetworkCommand::Unsubscribe { topic } => {
                assert_eq!(topic, "test-topic");
            }
            _ => panic!("Expected Unsubscribe command"),
        }
    }

    #[tokio::test]
    async fn test_network_handle_publish() {
        let (handle, mut rx) = NetworkHandle::mock();
        let data = b"test data".to_vec();

        handle.publish("test-topic", data.clone()).await.unwrap();

        match rx.recv().await.unwrap() {
            NetworkCommand::Publish { topic, data: d } => {
                assert_eq!(topic, "test-topic");
                assert_eq!(d, data);
            }
            _ => panic!("Expected Publish command"),
        }
    }

    #[tokio::test]
    async fn test_network_handle_dial() {
        use libp2p::Multiaddr;

        let (handle, mut rx) = NetworkHandle::mock();
        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/9000".parse().unwrap();

        handle.dial(addr.clone()).await.unwrap();

        match rx.recv().await.unwrap() {
            NetworkCommand::Dial { address } => {
                assert_eq!(address, addr);
            }
            _ => panic!("Expected Dial command"),
        }
    }

    #[tokio::test]
    async fn test_network_handle_disconnect() {
        let (handle, mut rx) = NetworkHandle::mock();
        let peer_id = PeerId::random();

        handle.disconnect(peer_id).await.unwrap();

        match rx.recv().await.unwrap() {
            NetworkCommand::Disconnect { peer_id: pid } => {
                assert_eq!(pid, peer_id);
            }
            _ => panic!("Expected Disconnect command"),
        }
    }

    #[tokio::test]
    async fn test_network_handle_put_record() {
        let (handle, mut rx) = NetworkHandle::mock();
        let key = b"test-key".to_vec();
        let value = b"test-value".to_vec();

        handle.put_record(key.clone(), value.clone()).await.unwrap();

        match rx.recv().await.unwrap() {
            NetworkCommand::PutRecord { key: k, value: v } => {
                assert_eq!(k, key);
                assert_eq!(v, value);
            }
            _ => panic!("Expected PutRecord command"),
        }
    }

    #[tokio::test]
    async fn test_network_handle_get_record() {
        let (handle, mut rx) = NetworkHandle::mock();
        let key = b"test-key".to_vec();

        handle.get_record(key.clone()).await.unwrap();

        match rx.recv().await.unwrap() {
            NetworkCommand::GetRecord { key: k } => {
                assert_eq!(k, key);
            }
            _ => panic!("Expected GetRecord command"),
        }
    }

    #[tokio::test]
    async fn test_network_handle_shutdown() {
        let (handle, mut rx) = NetworkHandle::mock();

        handle.shutdown().await.unwrap();

        match rx.recv().await.unwrap() {
            NetworkCommand::Shutdown => {}
            _ => panic!("Expected Shutdown command"),
        }
    }

    #[tokio::test]
    async fn test_network_handle_error_on_closed_channel() {
        let (handle, rx) = NetworkHandle::mock();
        drop(rx); // Close the channel

        let result = handle.subscribe("topic").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_network_handle_clone() {
        let (handle1, mut rx) = NetworkHandle::mock();
        let handle2 = handle1.clone();

        // Both handles should send to same channel
        handle1.subscribe("topic1").await.unwrap();
        handle2.subscribe("topic2").await.unwrap();

        // Verify both commands received
        let cmd1 = rx.recv().await.unwrap();
        let cmd2 = rx.recv().await.unwrap();

        match (cmd1, cmd2) {
            (NetworkCommand::Subscribe { topic: t1 }, NetworkCommand::Subscribe { topic: t2 }) => {
                assert_eq!(t1, "topic1");
                assert_eq!(t2, "topic2");
            }
            _ => panic!("Expected two Subscribe commands"),
        }
    }

    // ============ NetworkConfig Tests ============

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();

        assert!(config.enable_mdns);
        assert!(config.enable_kademlia);
        assert!(config.enable_tcp);
        assert!(config.enable_quic);
        assert_eq!(config.max_message_size, 1024 * 1024);
    }

    #[test]
    fn test_network_config_local_test() {
        let config = NetworkConfig::local_test(5000);

        assert!(config
            .listen_addresses
            .contains(&"/ip4/127.0.0.1/tcp/5000".to_string()));
        assert!(config.enable_mdns); // mDNS enabled for local discovery
        assert!(!config.enable_quic); // QUIC disabled for simpler testing
    }

    #[test]
    fn test_network_config_bootstrap_peers() {
        let mut config = NetworkConfig::default();
        config
            .bootstrap_peers
            .push("/ip4/1.2.3.4/tcp/9000/p2p/12D3KooWTest".to_string());

        assert!(!config.bootstrap_peers.is_empty());
    }

    // ============ NetworkCommand Tests ============

    #[test]
    fn test_network_command_debug() {
        let cmd = NetworkCommand::Subscribe {
            topic: "test".to_string(),
        };
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Subscribe"));
    }

    // ============ Address Filtering Tests ============

    #[test]
    fn test_routable_address_localhost() {
        use libp2p::Multiaddr;

        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/9000".parse().unwrap();
        assert!(is_routable_address(&addr));
    }

    #[test]
    fn test_routable_address_docker_bridge_blocked() {
        use libp2p::Multiaddr;

        let addr: Multiaddr = "/ip4/172.17.0.1/tcp/9000".parse().unwrap();
        assert!(!is_routable_address(&addr));
    }

    #[test]
    fn test_routable_address_wsl_magic_blocked() {
        use libp2p::Multiaddr;

        let addr: Multiaddr = "/ip4/10.255.255.254/tcp/9000".parse().unwrap();
        assert!(!is_routable_address(&addr));
    }

    #[test]
    fn test_routable_address_wsl_internal_blocked() {
        use libp2p::Multiaddr;

        // WSL internal bridges
        let addr1: Multiaddr = "/ip4/172.28.0.1/tcp/9000".parse().unwrap();
        let addr2: Multiaddr = "/ip4/172.29.0.1/tcp/9000".parse().unwrap();

        assert!(!is_routable_address(&addr1));
        assert!(!is_routable_address(&addr2));
    }

    #[test]
    fn test_routable_address_private_allowed() {
        use libp2p::Multiaddr;

        // Standard private ranges should be allowed
        let addr1: Multiaddr = "/ip4/192.168.1.1/tcp/9000".parse().unwrap();
        let addr2: Multiaddr = "/ip4/10.0.0.1/tcp/9000".parse().unwrap();

        assert!(is_routable_address(&addr1));
        assert!(is_routable_address(&addr2));
    }

    #[test]
    fn test_routable_address_public_allowed() {
        use libp2p::Multiaddr;

        let addr: Multiaddr = "/ip4/8.8.8.8/tcp/9000".parse().unwrap();
        assert!(is_routable_address(&addr));
    }

    #[test]
    fn test_routable_address_ipv6_allowed() {
        use libp2p::Multiaddr;

        let addr: Multiaddr = "/ip6/::1/tcp/9000".parse().unwrap();
        assert!(is_routable_address(&addr));
    }
}
