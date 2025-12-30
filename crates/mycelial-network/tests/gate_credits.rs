//! Phase 0 Gate Test: Credit Transfer
//!
//! Tests that credit transfers work correctly with the 2% entropy tax:
//! - Sender: 1000 - 100 - 2 (tax) = 898
//! - Receiver: 1000 + 100 = 1100

mod helpers;

use std::time::Duration;
use tokio::time::timeout;
use univrs_enr::core::{Credits, NodeId};

use helpers::TestCluster;
use mycelial_network::enr_bridge::INITIAL_NODE_CREDITS;

/// Test credit transfer between two nodes with 2% tax
///
/// Setup:
/// - Spawn 3 nodes (each starts with 1000 credits)
/// - Wait for mesh formation
/// - Node 0 transfers 100 credits to Node 1
/// - Verify: Node 0 balance = 898 (1000 - 100 - 2 tax)
/// - Verify: Node 1 balance = 1100 (1000 + 100)
///
/// Run with: cargo test --test gate_credits -- --ignored
#[tokio::test]
#[ignore = "Integration test - requires clean network environment"]
async fn test_credit_transfer_with_tax() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=debug,gate_credits=debug")
        .try_init();

    // Spawn a 3-node cluster
    let cluster = TestCluster::spawn(3)
        .await
        .expect("Failed to spawn cluster");

    // Wait for mesh formation (min 1 peer for star topology)
    cluster
        .wait_for_mesh(1, 10)
        .await
        .expect("Mesh formation timeout");

    // Verify initial balances
    let sender_initial = cluster.node(0).balance().await;
    let receiver_initial = cluster.node(1).balance().await;

    assert_eq!(
        sender_initial, INITIAL_NODE_CREDITS,
        "Sender should start with {} credits",
        INITIAL_NODE_CREDITS
    );
    assert_eq!(
        receiver_initial, INITIAL_NODE_CREDITS,
        "Receiver should start with {} credits",
        INITIAL_NODE_CREDITS
    );

    // Get receiver's NodeId from their bridge
    let receiver_peer_id = cluster.node(1).handle.local_peer_id();
    let peer_id_bytes = receiver_peer_id.to_bytes();
    let mut node_id_bytes = [0u8; 32];
    let len = peer_id_bytes.len().min(32);
    node_id_bytes[..len].copy_from_slice(&peer_id_bytes[..len]);
    let receiver_node_id = NodeId::from_bytes(node_id_bytes);

    // Transfer 100 credits from Node 0 to Node 1
    let transfer_amount = Credits::new(100);
    cluster
        .node(0)
        .enr_bridge
        .transfer_credits(receiver_node_id, transfer_amount)
        .await
        .expect("Transfer failed");

    // Verify sender balance immediately (deducted optimistically)
    let sender_after = cluster.node(0).balance().await;
    assert_eq!(
        sender_after, 898,
        "Sender balance should be 898 (1000 - 100 - 2 tax), got {}",
        sender_after
    );

    // Wait for receiver to get the transfer message (max 10 seconds)
    let receive_result = timeout(Duration::from_secs(10), async {
        loop {
            let balance = cluster.node(1).balance().await;
            if balance == 1100 {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    assert!(
        receive_result.is_ok(),
        "Receiver did not receive credits within 10 seconds"
    );

    // Final verification
    let sender_final = cluster.node(0).balance().await;
    let receiver_final = cluster.node(1).balance().await;

    assert_eq!(
        sender_final, 898,
        "Final sender balance should be 898, got {}",
        sender_final
    );
    assert_eq!(
        receiver_final, 1100,
        "Final receiver balance should be 1100, got {}",
        receiver_final
    );

    // Third node should be unaffected
    let observer_balance = cluster.node(2).balance().await;
    assert_eq!(
        observer_balance, INITIAL_NODE_CREDITS,
        "Observer balance should remain at {}, got {}",
        INITIAL_NODE_CREDITS, observer_balance
    );

    cluster.shutdown().await;
}

/// Test that self-transfer is rejected
#[tokio::test]
#[ignore = "Integration test - requires clean network environment"]
async fn test_self_transfer_rejected() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(2)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(1, 10)
        .await
        .expect("Mesh formation timeout");

    // Get node 0's own NodeId
    let self_peer_id = cluster.node(0).handle.local_peer_id();
    let peer_id_bytes = self_peer_id.to_bytes();
    let mut node_id_bytes = [0u8; 32];
    let len = peer_id_bytes.len().min(32);
    node_id_bytes[..len].copy_from_slice(&peer_id_bytes[..len]);
    let self_node_id = NodeId::from_bytes(node_id_bytes);

    // Attempt self-transfer
    let result = cluster
        .node(0)
        .enr_bridge
        .transfer_credits(self_node_id, Credits::new(100))
        .await;

    assert!(
        result.is_err(),
        "Self-transfer should be rejected"
    );

    // Balance should be unchanged
    let balance = cluster.node(0).balance().await;
    assert_eq!(
        balance, INITIAL_NODE_CREDITS,
        "Balance should be unchanged after rejected self-transfer"
    );

    cluster.shutdown().await;
}

/// Test that transfer of more than balance is rejected
#[tokio::test]
#[ignore = "Integration test - requires clean network environment"]
async fn test_insufficient_balance_rejected() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(2)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(1, 10)
        .await
        .expect("Mesh formation timeout");

    // Get receiver's NodeId
    let receiver_peer_id = cluster.node(1).handle.local_peer_id();
    let peer_id_bytes = receiver_peer_id.to_bytes();
    let mut node_id_bytes = [0u8; 32];
    let len = peer_id_bytes.len().min(32);
    node_id_bytes[..len].copy_from_slice(&peer_id_bytes[..len]);
    let receiver_node_id = NodeId::from_bytes(node_id_bytes);

    // Attempt to transfer more than balance (initial is 1000)
    let result = cluster
        .node(0)
        .enr_bridge
        .transfer_credits(receiver_node_id, Credits::new(2000))
        .await;

    assert!(
        result.is_err(),
        "Transfer exceeding balance should be rejected"
    );

    // Balance should be unchanged
    let balance = cluster.node(0).balance().await;
    assert_eq!(
        balance, INITIAL_NODE_CREDITS,
        "Balance should be unchanged after rejected transfer"
    );

    cluster.shutdown().await;
}

/// Test multiple sequential transfers
#[tokio::test]
#[ignore = "Integration test - requires clean network environment"]
async fn test_multiple_transfers() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(3)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(1, 10)
        .await
        .expect("Mesh formation timeout");

    // Get NodeIds
    let node1_peer_id = cluster.node(1).handle.local_peer_id();
    let peer_id_bytes = node1_peer_id.to_bytes();
    let mut node_id_bytes = [0u8; 32];
    let len = peer_id_bytes.len().min(32);
    node_id_bytes[..len].copy_from_slice(&peer_id_bytes[..len]);
    let node1_id = NodeId::from_bytes(node_id_bytes);

    let node2_peer_id = cluster.node(2).handle.local_peer_id();
    let peer_id_bytes = node2_peer_id.to_bytes();
    let mut node_id_bytes = [0u8; 32];
    let len = peer_id_bytes.len().min(32);
    node_id_bytes[..len].copy_from_slice(&peer_id_bytes[..len]);
    let node2_id = NodeId::from_bytes(node_id_bytes);

    // Transfer 1: Node 0 -> Node 1 (100 credits)
    cluster
        .node(0)
        .enr_bridge
        .transfer_credits(node1_id, Credits::new(100))
        .await
        .expect("Transfer 1 failed");

    // Transfer 2: Node 0 -> Node 2 (200 credits)
    cluster
        .node(0)
        .enr_bridge
        .transfer_credits(node2_id, Credits::new(200))
        .await
        .expect("Transfer 2 failed");

    // Node 0 balance: 1000 - 100 - 2 - 200 - 4 = 694
    let node0_balance = cluster.node(0).balance().await;
    assert_eq!(
        node0_balance, 694,
        "Node 0 balance after 2 transfers should be 694, got {}",
        node0_balance
    );

    // Wait for receivers
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Note: In a real network test, we'd verify the receivers got the credits
    // For now, just verify sender's balance is correctly deducted

    cluster.shutdown().await;
}
