//! Wave 5: Septal (Cross-Shard) Stress Tests
//!
//! Tests for cross-region/shard communication:
//! - test_septal_gradient_10_nodes: Gradient aggregation across 10 nodes
//! - test_septal_routing_efficiency: Routing path efficiency
//! - test_septal_load_distribution: Load balancing across shards
//! - test_septal_convergence_time: Time to reach consistent state
//! - test_septal_failure_recovery: Recovery from shard failure

use std::time::{Duration, Instant};
use tokio::time::timeout;
use univrs_enr::nexus::ResourceGradient;

use crate::helpers::TestCluster;

/// Test gradient aggregation across 10 nodes
/// Expected: Network gradient converges within 30 seconds
#[tokio::test]
async fn test_septal_gradient_10_nodes() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(10)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(2, 30)
        .await
        .expect("Mesh formation timeout");

    // Each node broadcasts its unique gradient
    for i in 0..cluster.node_count() {
        let gradient = ResourceGradient {
            cpu_available: 0.1 * (i as f64 + 1.0),
            memory_available: 0.5,
            ..Default::default()
        };
        cluster
            .node(i)
            .enr_bridge
            .broadcast_gradient(gradient)
            .await
            .expect("Failed to broadcast");

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Wait for aggregation
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Check network gradients
    let mut gradient_values = Vec::new();
    for i in 0..cluster.node_count() {
        let grad = cluster.node(i).enr_bridge.network_gradient().await;
        gradient_values.push(grad.cpu_available);
    }

    println!("Network gradient CPU values: {:?}", gradient_values);

    // Verify some aggregation occurred
    let non_zero_count = gradient_values.iter().filter(|&&v| v > 0.0).count();
    assert!(
        non_zero_count >= 5,
        "Expected gradient propagation to at least 50% of nodes"
    );

    cluster.shutdown().await;
}

/// Test routing efficiency across mesh
/// Expected: Messages reach destination efficiently
#[tokio::test]
async fn test_septal_routing_efficiency() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(10)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(2, 30)
        .await
        .expect("Mesh formation timeout");

    // Measure time for gradient to propagate from node 0 to node 9
    let start = Instant::now();

    let test_gradient = ResourceGradient {
        cpu_available: 0.99,
        memory_available: 0.99,
        ..Default::default()
    };

    cluster
        .node(0)
        .enr_bridge
        .broadcast_gradient(test_gradient)
        .await
        .expect("Failed to broadcast");

    // Wait for propagation to far node
    let result = timeout(Duration::from_secs(15), async {
        loop {
            let grad = cluster.node(9).enr_bridge.network_gradient().await;
            if (grad.cpu_available - 0.99).abs() < 0.01 {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    let propagation_time = start.elapsed();

    assert!(result.is_ok(), "Gradient did not reach far node");
    println!(
        "Routing efficiency: gradient reached node 9 in {:?}",
        propagation_time
    );

    // Should be reasonably fast (gossipsub is O(log n) hops)
    assert!(
        propagation_time.as_secs() < 10,
        "Routing too slow: {:?}",
        propagation_time
    );

    cluster.shutdown().await;
}

/// Test load distribution across cluster
/// Expected: Work is distributed across available nodes
#[tokio::test]
async fn test_septal_load_distribution() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(5)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(1, 15)
        .await
        .expect("Mesh formation timeout");

    // Simulate load by having each node report different resource levels
    for i in 0..cluster.node_count() {
        let gradient = ResourceGradient {
            cpu_available: (1.0 - (i as f64 * 0.15)),   // Decreasing CPU
            memory_available: (0.3 + (i as f64 * 0.1)), // Increasing memory
            bandwidth_available: 0.8,
            ..Default::default()
        };
        cluster
            .node(i)
            .enr_bridge
            .broadcast_gradient(gradient)
            .await
            .expect("Failed to broadcast");
    }

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check that nodes have visibility into network resources
    for i in 0..cluster.node_count() {
        let grad = cluster.node(i).enr_bridge.network_gradient().await;
        println!(
            "Node {} sees network: CPU={:.2}, Memory={:.2}",
            i, grad.cpu_available, grad.memory_available
        );
    }

    // Verify active node tracking
    let active_count = cluster.node(0).enr_bridge.active_node_count().await;
    println!("Active node count: {}", active_count);

    cluster.shutdown().await;
}

/// Test convergence time for consistent network state
/// Expected: Network reaches consistent state within 45 seconds
#[tokio::test]
async fn test_septal_convergence_time() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(8)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(2, 30)
        .await
        .expect("Mesh formation timeout");

    let start = Instant::now();

    // All nodes broadcast same gradient simultaneously
    let test_gradient = ResourceGradient {
        cpu_available: 0.42,
        memory_available: 0.42,
        ..Default::default()
    };

    for i in 0..cluster.node_count() {
        let _ = cluster
            .node(i)
            .enr_bridge
            .broadcast_gradient(test_gradient)
            .await;
    }

    // Wait for convergence (all nodes see similar values)
    let result = timeout(Duration::from_secs(45), async {
        loop {
            let mut values = Vec::new();
            for i in 0..cluster.node_count() {
                let grad = cluster.node(i).enr_bridge.network_gradient().await;
                values.push(grad.cpu_available);
            }

            // Check if all values are similar (within 0.1)
            let converged = values.iter().all(|&v| (v - 0.42).abs() < 0.1);
            if converged {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await;

    let convergence_time = start.elapsed();

    assert!(result.is_ok(), "Network did not converge");
    println!("Network converged in {:?}", convergence_time);

    cluster.shutdown().await;
}

/// Test recovery from simulated shard failure
/// Expected: Remaining nodes continue operating
#[tokio::test]
async fn test_septal_failure_recovery() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(6)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(2, 30)
        .await
        .expect("Mesh formation timeout");

    // Simulate failure by isolating a group (shard)
    // Isolate nodes 4 and 5 as a "failed shard"
    cluster
        .isolate_node(4)
        .await
        .expect("Failed to isolate node 4");
    cluster
        .isolate_node(5)
        .await
        .expect("Failed to isolate node 5");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Remaining nodes should still be able to communicate
    let test_gradient = ResourceGradient {
        cpu_available: 0.55,
        memory_available: 0.66,
        ..Default::default()
    };

    cluster
        .node(0)
        .enr_bridge
        .broadcast_gradient(test_gradient)
        .await
        .expect("Failed to broadcast");

    // Wait for propagation among healthy nodes
    let result = timeout(Duration::from_secs(15), async {
        loop {
            let mut received_count = 0;
            for i in 0..4 {
                // Only check healthy nodes
                let grad = cluster.node(i).enr_bridge.network_gradient().await;
                if (grad.cpu_available - 0.55).abs() < 0.01 {
                    received_count += 1;
                }
            }
            if received_count >= 3 {
                return received_count;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await;

    assert!(
        result.is_ok(),
        "Healthy nodes should continue communicating"
    );
    println!(
        "After shard failure, {} healthy nodes received gradient",
        result.unwrap()
    );

    // Rejoin failed nodes
    cluster
        .rejoin_node(4)
        .await
        .expect("Failed to rejoin node 4");
    cluster
        .rejoin_node(5)
        .await
        .expect("Failed to rejoin node 5");

    tokio::time::sleep(Duration::from_secs(3)).await;

    println!("Failed shard rejoined");

    cluster.shutdown().await;
}
