# Testing Roadmap for Univrs Network

> Comprehensive testing strategy for DOL-to-DOL language evolution

## Current Test Status

| Crate | Tests | Coverage | Status |
|-------|-------|----------|--------|
| mycelial-core | 30 | 95% | ✅ Well-tested |
| mycelial-protocol | 12 | 85% | ✅ Well-tested |
| mycelial-state | 13 | 80% | ✅ Well-tested |
| mycelial-network | 11 | 30% | ⚠️ Gaps in service/behaviour |
| mycelial-node | 0 | 0% | 🔴 Critical gaps |
| mycelial-wasm | 0 | 0% | ⏸️ Deferred |
| dashboard | 0 | 0% | 🔴 No test infrastructure |

**Total: 66 unit tests passing**

---

## Critical Testing Gaps

### Tier 1: Core Infrastructure (Must Fix)

#### 1. WebSocket Handler Tests (`mycelial-node/src/server/websocket.rs`)
**Priority: CRITICAL** | **LOC: 600** | **Tests: 0**

Missing test coverage for:
- [ ] Chat message handling with local echo
- [ ] Direct message routing
- [ ] Room creation, joining, leaving
- [ ] Vouch request/acknowledgment workflow
- [ ] Credit line creation and transfers
- [ ] Governance proposal creation and voting
- [ ] Resource contribution reporting
- [ ] Error handling for malformed messages

#### 2. Network Service Tests (`mycelial-network/src/service.rs`)
**Priority: CRITICAL** | **LOC: 655** | **Tests: 0**

Missing test coverage for:
- [ ] NetworkService initialization
- [ ] Command handling (publish, subscribe, dial)
- [ ] Gossipsub mesh formation
- [ ] Kademlia DHT operations
- [ ] Peer connection lifecycle
- [ ] Message delivery verification

#### 3. REST API Tests (`mycelial-node/src/server/rest.rs`)
**Priority: HIGH** | **LOC: 80** | **Tests: 0**

Missing test coverage for:
- [ ] GET /api/peers - list all peers
- [ ] GET /api/peers/{id} - get specific peer
- [ ] GET /api/stats - network statistics
- [ ] Error responses for invalid requests

---

## Integration Test Structure

### Proposed Directory Layout

```
crates/
├── mycelial-node/
│   └── tests/
│       ├── integration/
│       │   ├── mod.rs
│       │   ├── websocket_test.rs    # WebSocket protocol tests
│       │   ├── rest_api_test.rs     # REST endpoint tests
│       │   └── economics_test.rs    # Full economics flow tests
│       └── e2e/
│           ├── mod.rs
│           ├── multi_node_test.rs   # Multi-node network tests
│           ├── room_chat_test.rs    # Room functionality tests
│           └── dol_protocol_test.rs # DOL-to-DOL messaging tests
│
├── mycelial-network/
│   └── tests/
│       ├── integration/
│       │   ├── mod.rs
│       │   ├── gossipsub_test.rs    # Gossipsub mesh tests
│       │   ├── kademlia_test.rs     # DHT operation tests
│       │   └── discovery_test.rs    # mDNS/peer discovery tests
│
dashboard/
├── src/
│   └── __tests__/
│       ├── hooks/
│       │   ├── useP2P.test.ts
│       │   └── useOrchestrator.test.ts
│       └── components/
│           ├── ChatPanel.test.tsx
│           ├── PeerGraph.test.tsx
│           └── ConversationSidebar.test.tsx
├── e2e/
│   ├── chat.spec.ts
│   ├── rooms.spec.ts
│   └── economics.spec.ts
```

---

## DOL-to-DOL Evolution Testing

### What is DOL?

DOL (Decentralized Object Language) represents the evolution of inter-agent communication in the Mycelial network. Testing for DOL evolution requires:

1. **Protocol Versioning Tests**
   - Backward compatibility between protocol versions
   - Message format evolution (v1.0.0 → v2.0.0)
   - Graceful degradation for unknown message types

2. **Language Interoperability Tests**
   - Rust ↔ TypeScript message serialization
   - WASM bridge message passing
   - Cross-language type consistency

3. **Semantic Messaging Tests**
   - Intent-based message routing
   - Context preservation across hops
   - Semantic equivalence validation

### DOL Test Categories

#### Category A: Protocol Evolution
```rust
#[test]
fn test_protocol_version_compatibility() {
    // v1.0.0 message should be parseable by v1.1.0 handler
    let v1_msg = r#"{"type":"chat","content":"hello"}"#;
    let v1_1_handler = MessageHandler::new(Version::V1_1);
    assert!(v1_1_handler.can_parse(v1_msg));
}

#[test]
fn test_unknown_field_tolerance() {
    // New fields should be ignored by old parsers
    let future_msg = r#"{"type":"chat","content":"hello","neural_embedding":[0.1,0.2]}"#;
    let current_handler = MessageHandler::new(Version::V1_0);
    assert!(current_handler.can_parse(future_msg));
}
```

#### Category B: Cross-Language Consistency
```rust
#[test]
fn test_rust_typescript_message_parity() {
    let rust_msg = ChatMessage::new("hello");
    let json = serde_json::to_string(&rust_msg).unwrap();

    // Verify JSON matches TypeScript interface
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["type"], "chat_message");
    assert_eq!(parsed["content"], "hello");
    assert!(parsed["timestamp"].is_number());
}
```

#### Category C: Semantic Routing
```rust
#[test]
fn test_intent_based_routing() {
    // Message with intent should route to appropriate handler
    let msg = Message::with_intent(
        Intent::Query,
        Content::ResourceRequest { resource_type: "compute" }
    );

    let router = IntentRouter::new();
    let handler = router.route(&msg);

    assert_eq!(handler.name(), "ResourceHandler");
}
```

---

## E2E Test Scenarios

### Scenario 1: Multi-Node Chat Flow
```
Given: 3 nodes (Bootstrap, Alice, Bob) are connected
When: Alice sends a broadcast message "Hello network"
Then: Bob receives the message within 2 seconds
And: Bootstrap receives the message within 2 seconds
And: Alice sees her own message (local echo)
```

### Scenario 2: Room-Based Communication
```
Given: 3 nodes are connected
When: Alice creates room "Engineering"
And: Bob joins room "Engineering"
And: Alice sends "Sprint planning" to the room
Then: Bob receives the message in the room
And: Carol (not in room) does NOT receive the message
```

### Scenario 3: Economics Protocol Flow
```
Given: Alice and Bob are connected peers
When: Alice sends a vouch request to Bob (weight: 0.8)
Then: Bob receives the vouch request
When: Bob accepts the vouch
Then: Alice receives the vouch acknowledgment
And: Alice's reputation with Bob increases
```

### Scenario 4: Credit Transfer Flow
```
Given: Alice has created a credit line with Bob (limit: 1000)
When: Alice transfers 100 credits to Bob
Then: Bob's balance increases by 100
And: Alice's balance decreases by 100
And: Both peers receive transfer confirmation
```

### Scenario 5: Governance Flow
```
Given: 5 nodes are connected with established reputation
When: Alice creates proposal "Increase credit limits"
Then: All nodes receive the proposal
When: Bob, Carol, Dave vote "yes"
And: Eve votes "no"
Then: Proposal status updates to "passed" (3/4 majority)
```

---

## Test Implementation Plan

### Phase 1: Core Unit Tests (Week 1-2)

1. **WebSocket Handler Tests**
   ```rust
   // crates/mycelial-node/src/server/websocket.rs
   #[cfg(test)]
   mod tests {
       use super::*;
       use tokio::sync::mpsc;

       #[tokio::test]
       async fn test_send_chat_broadcast() {
           let (tx, mut rx) = mpsc::channel(100);
           let state = create_test_state(tx);

           let msg = ClientMessage::SendChat {
               content: "hello".to_string(),
               to: None,
               room_id: None,
           };

           handle_message(msg, &state).await.unwrap();

           let published = rx.recv().await.unwrap();
           assert!(matches!(published, WsMessage::ChatMessage { .. }));
       }

       #[tokio::test]
       async fn test_create_room() {
           let (tx, mut rx) = mpsc::channel(100);
           let state = create_test_state(tx);

           let msg = ClientMessage::CreateRoom {
               name: "TestRoom".to_string(),
               description: None,
           };

           handle_message(msg, &state).await.unwrap();

           let response = rx.recv().await.unwrap();
           assert!(matches!(response, WsMessage::RoomJoined { .. }));
       }
   }
   ```

2. **REST API Tests**
   ```rust
   // crates/mycelial-node/tests/rest_api_test.rs
   use axum_test::TestServer;

   #[tokio::test]
   async fn test_get_peers() {
       let app = create_test_app().await;
       let server = TestServer::new(app).unwrap();

       let response = server.get("/api/peers").await;

       response.assert_status_ok();
       let peers: Vec<PeerInfo> = response.json();
       assert!(!peers.is_empty());
   }
   ```

### Phase 2: Integration Tests (Week 3-4)

1. **Multi-Node Network Tests**
   ```rust
   // crates/mycelial-network/tests/integration/multi_node_test.rs
   #[tokio::test]
   async fn test_three_node_mesh() {
       let bootstrap = spawn_node(NodeConfig::bootstrap(9000)).await;
       let alice = spawn_node(NodeConfig::peer("Alice", 9000)).await;
       let bob = spawn_node(NodeConfig::peer("Bob", 9000)).await;

       // Wait for mesh formation
       tokio::time::sleep(Duration::from_secs(3)).await;

       // Verify connectivity
       assert_eq!(bootstrap.peer_count(), 2);
       assert!(alice.is_connected_to(&bob.peer_id()));
   }
   ```

2. **Gossipsub Message Delivery Tests**
   ```rust
   #[tokio::test]
   async fn test_broadcast_delivery() {
       let network = TestNetwork::new(3).await;

       network.nodes[0].publish("chat", b"hello").await;

       // All nodes should receive
       for node in &network.nodes[1..] {
           let msg = node.recv_timeout(Duration::from_secs(2)).await;
           assert_eq!(msg.data, b"hello");
       }
   }
   ```

### Phase 3: E2E Tests (Week 5-6)

1. **Dashboard Test Setup**
   ```typescript
   // dashboard/vitest.config.ts
   import { defineConfig } from 'vitest/config';
   import react from '@vitejs/plugin-react';

   export default defineConfig({
     plugins: [react()],
     test: {
       environment: 'jsdom',
       setupFiles: ['./src/__tests__/setup.ts'],
       include: ['src/**/*.test.{ts,tsx}'],
     },
   });
   ```

2. **useP2P Hook Tests**
   ```typescript
   // dashboard/src/__tests__/hooks/useP2P.test.ts
   import { renderHook, act } from '@testing-library/react';
   import { useP2P } from '../../hooks/useP2P';
   import { MockWebSocket } from '../mocks/websocket';

   describe('useP2P', () => {
     it('connects to WebSocket on mount', async () => {
       const mockWs = new MockWebSocket();
       const { result } = renderHook(() => useP2P());

       await act(async () => {
         mockWs.open();
       });

       expect(result.current.connected).toBe(true);
     });

     it('receives and stores chat messages', async () => {
       const mockWs = new MockWebSocket();
       const { result } = renderHook(() => useP2P());

       await act(async () => {
         mockWs.receive({
           type: 'chat_message',
           content: 'hello',
           from: 'peer-123',
         });
       });

       expect(result.current.messages).toHaveLength(1);
       expect(result.current.messages[0].content).toBe('hello');
     });
   });
   ```

3. **Playwright E2E Tests**
   ```typescript
   // dashboard/e2e/chat.spec.ts
   import { test, expect } from '@playwright/test';

   test('can send and receive chat messages', async ({ page }) => {
     await page.goto('http://localhost:5173');

     // Wait for connection
     await expect(page.locator('[data-testid="connection-status"]'))
       .toHaveText('Connected');

     // Send message
     await page.fill('[data-testid="chat-input"]', 'Hello world');
     await page.click('[data-testid="send-button"]');

     // Verify message appears
     await expect(page.locator('[data-testid="message-list"]'))
       .toContainText('Hello world');
   });
   ```

---

## Test Dependencies

### Rust
```toml
# Cargo.toml [dev-dependencies]
tokio-test = "0.4"
axum-test = "0.2"
mockall = "0.11"
proptest = "1.0"
wiremock = "0.5"
tempfile = "3.0"
```

### Dashboard
```json
{
  "devDependencies": {
    "vitest": "^1.0.0",
    "@testing-library/react": "^14.0.0",
    "@testing-library/user-event": "^14.0.0",
    "@playwright/test": "^1.40.0",
    "msw": "^2.0.0"
  }
}
```

---

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Unit test count | 66 | 150+ |
| Integration tests | 0 | 30+ |
| E2E tests | 0 | 20+ |
| Code coverage | ~40% | 80%+ |
| WebSocket handler coverage | 0% | 90%+ |
| Network service coverage | 0% | 80%+ |

---

## DOL Alignment Checklist

For each new feature, ensure tests cover:

- [ ] Message serialization/deserialization parity (Rust ↔ TypeScript)
- [ ] Protocol version compatibility
- [ ] Graceful handling of unknown message types
- [ ] Cross-node message delivery verification
- [ ] Error propagation and recovery
- [ ] State consistency after failures
- [ ] Performance under load (10+ nodes)

---

## Next Steps

1. **Immediate**: Add `#[cfg(test)]` module to `websocket.rs` with 10+ handler tests
2. **Short-term**: Set up Vitest in dashboard with mock WebSocket
3. **Medium-term**: Create integration test harness for multi-node scenarios
4. **Long-term**: Implement Playwright E2E suite with full user journey coverage

---

*Generated: 2024-12-27*
*Aligns with: DOL Protocol v1.0.0, Mycelial Economics Phase 7*
