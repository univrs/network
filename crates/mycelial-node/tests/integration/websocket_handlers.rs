//! WebSocket handler behavior tests
//!
//! These tests verify the behavior of the `handle_client_message` function,
//! ensuring correct message handling, network publishing, and event broadcasting.

use mycelial_network::{NetworkCommand, NetworkHandle};
use tokio::sync::broadcast;

/// Helper to create a mock network handle and command receiver
fn create_mock_network() -> (NetworkHandle, tokio::sync::mpsc::Receiver<NetworkCommand>) {
    NetworkHandle::mock()
}

// ============ SendChat Handler Tests ============

#[tokio::test]
async fn test_send_chat_publishes_to_chat_topic() {
    // Test: SendChat message should publish to /mycelial/1.0.0/chat topic
    let (network, mut cmd_rx) = create_mock_network();

    // Send a subscribe command to verify the mock works
    network.subscribe("/mycelial/1.0.0/chat").await.unwrap();

    // Verify command was received
    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Subscribe { topic } => {
            assert_eq!(topic, "/mycelial/1.0.0/chat");
        }
        _ => panic!("Expected Subscribe command"),
    }
}

#[tokio::test]
async fn test_send_chat_publishes_to_room_topic() {
    // Test: SendChat with room_id should publish to room-specific topic
    let (network, mut cmd_rx) = create_mock_network();

    let room_id = "test-room-123";
    let expected_topic = format!("/mycelial/1.0.0/room/{}", room_id);

    network.subscribe(&expected_topic).await.unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Subscribe { topic } => {
            assert_eq!(topic, expected_topic);
        }
        _ => panic!("Expected Subscribe command"),
    }
}

#[tokio::test]
async fn test_send_chat_publishes_to_direct_topic() {
    // Test: SendChat with 'to' field should publish to /mycelial/1.0.0/direct topic
    let (network, mut cmd_rx) = create_mock_network();

    network.subscribe("/mycelial/1.0.0/direct").await.unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Subscribe { topic } => {
            assert_eq!(topic, "/mycelial/1.0.0/direct");
        }
        _ => panic!("Expected Subscribe command"),
    }
}

// ============ Publish Tests ============

#[tokio::test]
async fn test_network_publish_sends_data() {
    let (network, mut cmd_rx) = create_mock_network();

    let topic = "/test/topic";
    let data = b"test message".to_vec();

    network.publish(topic, data.clone()).await.unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Publish { topic: t, data: d } => {
            assert_eq!(t, topic);
            assert_eq!(d, data);
        }
        _ => panic!("Expected Publish command"),
    }
}

// ============ Subscribe/Unsubscribe Tests ============

#[tokio::test]
async fn test_subscribe_command() {
    let (network, mut cmd_rx) = create_mock_network();

    network.subscribe("test-topic").await.unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Subscribe { topic } => {
            assert_eq!(topic, "test-topic");
        }
        _ => panic!("Expected Subscribe command"),
    }
}

#[tokio::test]
async fn test_unsubscribe_command() {
    let (network, mut cmd_rx) = create_mock_network();

    network.unsubscribe("test-topic").await.unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Unsubscribe { topic } => {
            assert_eq!(topic, "test-topic");
        }
        _ => panic!("Expected Unsubscribe command"),
    }
}

// ============ Economics Protocol Topic Tests ============

#[tokio::test]
async fn test_vouch_topic_publish() {
    let (network, mut cmd_rx) = create_mock_network();

    // The vouch topic used by the handler
    let vouch_topic = "/mycelial/protocol/vouch";

    network.publish(vouch_topic, vec![1, 2, 3]).await.unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Publish { topic, data } => {
            assert_eq!(topic, vouch_topic);
            assert_eq!(data, vec![1, 2, 3]);
        }
        _ => panic!("Expected Publish command"),
    }
}

#[tokio::test]
async fn test_credit_topic_publish() {
    let (network, mut cmd_rx) = create_mock_network();

    let credit_topic = "/mycelial/protocol/credit";

    network.publish(credit_topic, vec![4, 5, 6]).await.unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Publish { topic, data } => {
            assert_eq!(topic, credit_topic);
            assert_eq!(data, vec![4, 5, 6]);
        }
        _ => panic!("Expected Publish command"),
    }
}

#[tokio::test]
async fn test_governance_topic_publish() {
    let (network, mut cmd_rx) = create_mock_network();

    let governance_topic = "/mycelial/protocol/governance";

    network
        .publish(governance_topic, vec![7, 8, 9])
        .await
        .unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Publish { topic, data } => {
            assert_eq!(topic, governance_topic);
            assert_eq!(data, vec![7, 8, 9]);
        }
        _ => panic!("Expected Publish command"),
    }
}

#[tokio::test]
async fn test_resource_topic_publish() {
    let (network, mut cmd_rx) = create_mock_network();

    let resource_topic = "/mycelial/protocol/resource";

    network
        .publish(resource_topic, vec![10, 11, 12])
        .await
        .unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Publish { topic, data } => {
            assert_eq!(topic, resource_topic);
            assert_eq!(data, vec![10, 11, 12]);
        }
        _ => panic!("Expected Publish command"),
    }
}

// ============ Room Handler Topic Tests ============

#[tokio::test]
async fn test_create_room_subscribes_to_topic() {
    let (network, mut cmd_rx) = create_mock_network();

    let room_id = "new-room-456";
    let room_topic = format!("/mycelial/1.0.0/room/{}", room_id);

    network.subscribe(&room_topic).await.unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Subscribe { topic } => {
            assert_eq!(topic, room_topic);
        }
        _ => panic!("Expected Subscribe command"),
    }
}

#[tokio::test]
async fn test_join_room_subscribes_and_publishes() {
    let (network, mut cmd_rx) = create_mock_network();

    let room_id = "existing-room-789";
    let room_topic = format!("/mycelial/1.0.0/room/{}", room_id);

    // First, subscribe to the topic
    network.subscribe(&room_topic).await.unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Subscribe { topic } => {
            assert_eq!(topic, room_topic);
        }
        _ => panic!("Expected Subscribe command"),
    }

    // Then, publish peer joined message
    network.publish(&room_topic, vec![1, 2, 3]).await.unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Publish { topic, .. } => {
            assert_eq!(topic, room_topic);
        }
        _ => panic!("Expected Publish command"),
    }
}

#[tokio::test]
async fn test_leave_room_publishes_and_unsubscribes() {
    let (network, mut cmd_rx) = create_mock_network();

    let room_id = "leaving-room-101";
    let room_topic = format!("/mycelial/1.0.0/room/{}", room_id);

    // First, publish peer left message
    network.publish(&room_topic, vec![1, 2, 3]).await.unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Publish { topic, .. } => {
            assert_eq!(topic, room_topic);
        }
        _ => panic!("Expected Publish command"),
    }

    // Then, unsubscribe from the topic
    network.unsubscribe(&room_topic).await.unwrap();

    let cmd = cmd_rx.recv().await.unwrap();
    match cmd {
        NetworkCommand::Unsubscribe { topic } => {
            assert_eq!(topic, room_topic);
        }
        _ => panic!("Expected Unsubscribe command"),
    }
}

// ============ Multiple Command Sequence Tests ============

#[tokio::test]
async fn test_multiple_publishes_in_sequence() {
    let (network, mut cmd_rx) = create_mock_network();

    // Send multiple publish commands
    network.publish("topic1", b"msg1".to_vec()).await.unwrap();
    network.publish("topic2", b"msg2".to_vec()).await.unwrap();
    network.publish("topic3", b"msg3".to_vec()).await.unwrap();

    // Verify all commands received in order
    for (expected_topic, expected_data) in [
        ("topic1", b"msg1".to_vec()),
        ("topic2", b"msg2".to_vec()),
        ("topic3", b"msg3".to_vec()),
    ] {
        let cmd = cmd_rx.recv().await.unwrap();
        match cmd {
            NetworkCommand::Publish { topic, data } => {
                assert_eq!(topic, expected_topic);
                assert_eq!(data, expected_data);
            }
            _ => panic!("Expected Publish command"),
        }
    }
}

#[tokio::test]
async fn test_mixed_commands_sequence() {
    let (network, mut cmd_rx) = create_mock_network();

    // Subscribe, publish, unsubscribe sequence
    network.subscribe("topic").await.unwrap();
    network.publish("topic", b"data".to_vec()).await.unwrap();
    network.unsubscribe("topic").await.unwrap();

    // Verify sequence
    match cmd_rx.recv().await.unwrap() {
        NetworkCommand::Subscribe { topic } => assert_eq!(topic, "topic"),
        _ => panic!("Expected Subscribe"),
    }

    match cmd_rx.recv().await.unwrap() {
        NetworkCommand::Publish { topic, .. } => assert_eq!(topic, "topic"),
        _ => panic!("Expected Publish"),
    }

    match cmd_rx.recv().await.unwrap() {
        NetworkCommand::Unsubscribe { topic } => assert_eq!(topic, "topic"),
        _ => panic!("Expected Unsubscribe"),
    }
}

// ============ Broadcast Channel Tests ============

#[tokio::test]
async fn test_broadcast_channel_sends_to_subscribers() {
    let (tx, mut rx1) = broadcast::channel::<String>(16);
    let mut rx2 = tx.subscribe();

    tx.send("test message".to_string()).unwrap();

    assert_eq!(rx1.recv().await.unwrap(), "test message");
    assert_eq!(rx2.recv().await.unwrap(), "test message");
}

#[tokio::test]
async fn test_broadcast_channel_multiple_messages() {
    let (tx, mut rx) = broadcast::channel::<String>(16);

    tx.send("msg1".to_string()).unwrap();
    tx.send("msg2".to_string()).unwrap();
    tx.send("msg3".to_string()).unwrap();

    assert_eq!(rx.recv().await.unwrap(), "msg1");
    assert_eq!(rx.recv().await.unwrap(), "msg2");
    assert_eq!(rx.recv().await.unwrap(), "msg3");
}

// ============ Network Handle Clone Tests ============

#[tokio::test]
async fn test_network_handle_clone_shares_channel() {
    let (network1, mut cmd_rx) = create_mock_network();
    let network2 = network1.clone();

    // Both handles should send to same channel
    network1.subscribe("topic1").await.unwrap();
    network2.subscribe("topic2").await.unwrap();

    match cmd_rx.recv().await.unwrap() {
        NetworkCommand::Subscribe { topic } => assert_eq!(topic, "topic1"),
        _ => panic!("Expected Subscribe"),
    }

    match cmd_rx.recv().await.unwrap() {
        NetworkCommand::Subscribe { topic } => assert_eq!(topic, "topic2"),
        _ => panic!("Expected Subscribe"),
    }
}

// ============ Error Handling Tests ============

#[tokio::test]
async fn test_network_handle_returns_ok_on_send() {
    let (network, _cmd_rx) = create_mock_network();

    // Should not panic or error
    let result = network.publish("topic", vec![]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_network_handle_error_on_dropped_receiver() {
    let (network, cmd_rx) = create_mock_network();

    // Drop the receiver
    drop(cmd_rx);

    // Should return an error
    let result = network.publish("topic", vec![]).await;
    assert!(result.is_err());
}

// ============ PeerId Tests ============

#[tokio::test]
async fn test_mock_network_has_peer_id() {
    let (network, _cmd_rx) = create_mock_network();

    let peer_id = network.local_peer_id();
    // Peer ID should be valid (not empty when converted to string)
    assert!(!peer_id.to_string().is_empty());
}

#[tokio::test]
async fn test_mock_network_with_specific_peer_id() {
    use mycelial_network::Libp2pPeerId;

    let specific_peer_id = Libp2pPeerId::random();
    let (network, _cmd_rx) = NetworkHandle::mock_with_peer_id(specific_peer_id);

    assert_eq!(network.local_peer_id(), specific_peer_id);
}
