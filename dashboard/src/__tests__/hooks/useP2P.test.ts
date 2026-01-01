/**
 * useP2P Hook Tests
 *
 * Comprehensive tests for the P2P WebSocket hook including:
 * - Connection management
 * - Message handling
 * - Peer management
 * - Chat functionality
 * - Room management
 * - Economics protocols (vouch, credit, governance, resources)
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useP2P } from '@/hooks/useP2P';
import { mockWebSocketServer } from '../mocks/websocket';

describe('useP2P Hook', () => {
  beforeEach(() => {
    mockWebSocketServer.reset();
    mockWebSocketServer.autoConnect = true;
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('Connection Management', () => {
    it('connects to WebSocket on mount', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      // Wait for connection
      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      expect(result.current.connected).toBe(true);
      expect(mockWebSocketServer.instances.length).toBe(1);
    });

    it('starts with disconnected state', () => {
      mockWebSocketServer.autoConnect = false;
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      expect(result.current.connected).toBe(false);
    });

    it('provides disconnect function', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      expect(result.current.connected).toBe(true);

      act(() => {
        result.current.disconnect();
      });

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      expect(result.current.connected).toBe(false);
    });

    it('provides resetConnection function', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        result.current.resetConnection();
      });

      await act(async () => {
        await vi.advanceTimersByTimeAsync(200);
      });

      // Should have created a new connection
      expect(mockWebSocketServer.instances.length).toBeGreaterThan(1);
    });

    it('initializes with community conversation', () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      expect(result.current.activeConversationId).toBe('community');
      expect(result.current.conversations.has('community')).toBe(true);
      expect(result.current.conversations.get('community')?.type).toBe('community');
    });
  });

  describe('Peer Management', () => {
    it('handles peers_list message', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        mockWebSocketServer.sendPeersList([
          { id: '12D3KooWAlice', name: 'Alice', reputation: 0.9 },
          { id: '12D3KooWBob', name: 'Bob', reputation: 0.7 },
        ]);
      });

      expect(result.current.peers.size).toBe(2);
      expect(result.current.peers.get('12D3KooWAlice')?.name).toBe('Alice');
      expect(result.current.peers.get('12D3KooWBob')?.reputation).toBe(0.7);
    });

    it('handles peer_joined message', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        mockWebSocketServer.sendPeerJoined({ peer_id: '12D3KooWCarol', name: 'Carol' });
      });

      expect(result.current.peers.has('12D3KooWCarol')).toBe(true);
    });

    it('handles peer_left message', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      // First add a peer
      act(() => {
        mockWebSocketServer.sendPeersList([{ id: '12D3KooWAlice', name: 'Alice' }]);
      });

      expect(result.current.peers.has('12D3KooWAlice')).toBe(true);

      // Then remove them
      act(() => {
        mockWebSocketServer.sendPeerLeft('12D3KooWAlice');
      });

      expect(result.current.peers.has('12D3KooWAlice')).toBe(false);
    });

    it('generates graph data from peers', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        mockWebSocketServer.sendPeersList([
          { id: '12D3KooWAlice', name: 'Alice' },
          { id: '12D3KooWBob', name: 'Bob' },
          { id: '12D3KooWCarol', name: 'Carol' },
        ]);
      });

      const graphData = result.current.graphData();

      expect(graphData.nodes.length).toBe(3);
      // With 3 nodes, we should have 3 links in a mesh (3 choose 2 = 3)
      expect(graphData.links.length).toBe(3);
    });
  });

  describe('Chat Functionality', () => {
    it('handles chat_message from server', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        mockWebSocketServer.sendChatMessage({
          from: '12D3KooWAlice',
          from_name: 'Alice',
          content: 'Hello world!',
        });
      });

      expect(result.current.messages.length).toBe(1);
      expect(result.current.messages[0].content).toBe('Hello world!');
      expect(result.current.messages[0].from_name).toBe('Alice');
    });

    it('sends chat message via WebSocket', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        result.current.sendChat('Hello from test!');
      });

      // Check message was sent
      expect(mockWebSocketServer.receivedMessages.length).toBe(1);
      expect(mockWebSocketServer.receivedMessages[0]).toMatchObject({
        type: 'send_chat',
        content: 'Hello from test!',
      });

      // Check optimistic update added message to state
      expect(result.current.messages.length).toBe(1);
    });

    it('sends direct message to specific peer', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        result.current.sendChat('Private message', '12D3KooWBob');
      });

      expect(mockWebSocketServer.receivedMessages[0]).toMatchObject({
        type: 'send_chat',
        content: 'Private message',
        to: '12D3KooWBob',
      });
    });

    it('sends message to room', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        result.current.sendChat('Room message', undefined, 'room-123');
      });

      expect(mockWebSocketServer.receivedMessages[0]).toMatchObject({
        type: 'send_chat',
        content: 'Room message',
        room_id: 'room-123',
      });
    });

    it('filters messages by active conversation', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      // Simulate local peer ID
      act(() => {
        mockWebSocketServer.broadcast({
          type: 'peers_list',
          peers: [{ id: 'local-peer', name: 'Local' }],
        });
      });

      // Add community message
      act(() => {
        mockWebSocketServer.sendChatMessage({
          from: '12D3KooWAlice',
          content: 'Community message',
        });
      });

      // Add room message
      act(() => {
        mockWebSocketServer.sendChatMessage({
          from: '12D3KooWBob',
          content: 'Room message',
          room_id: 'room-123',
        });
      });

      // Should only see community messages when on community
      const communityMessages = result.current.getActiveMessages();
      expect(communityMessages.length).toBe(1);
      expect(communityMessages[0].content).toBe('Community message');
    });
  });

  describe('Conversation Management', () => {
    it('switches active conversation', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      expect(result.current.activeConversationId).toBe('community');

      act(() => {
        result.current.setActiveConversation('room:test-room');
      });

      expect(result.current.activeConversationId).toBe('room:test-room');
    });

    it('starts DM conversation with peer', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      // Add a peer
      act(() => {
        mockWebSocketServer.sendPeersList([{ id: '12D3KooWAlice', name: 'Alice' }]);
      });

      act(() => {
        result.current.startDMConversation('12D3KooWAlice');
      });

      // Should have created a DM conversation and switched to it
      expect(result.current.activeConversationId).toContain('dm:');
      expect(result.current.conversations.size).toBe(2); // community + DM
    });

    it('clears unread count when switching to conversation', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      // Receive a room message while on community
      act(() => {
        mockWebSocketServer.sendRoomJoined({
          id: 'room-123',
          name: 'Test Room',
          created_by: '12D3KooWAlice',
        });
      });

      act(() => {
        mockWebSocketServer.sendChatMessage({
          from: '12D3KooWAlice',
          content: 'Room message',
          room_id: 'room-123',
        });
      });

      const roomConv = result.current.conversations.get('room:room-123');
      expect(roomConv?.unreadCount).toBeGreaterThanOrEqual(0);

      // Switch to the room
      act(() => {
        result.current.setActiveConversation('room:room-123');
      });

      const updatedRoomConv = result.current.conversations.get('room:room-123');
      expect(updatedRoomConv?.unreadCount).toBe(0);
    });
  });

  describe('Room Management', () => {
    it('handles room_joined message', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        mockWebSocketServer.sendRoomJoined({
          id: 'room-123',
          name: 'Engineering',
          description: 'Engineering team',
          created_by: '12D3KooWAlice',
        });
      });

      expect(result.current.rooms.has('room-123')).toBe(true);
      expect(result.current.rooms.get('room-123')?.name).toBe('Engineering');
      expect(result.current.conversations.has('room:room-123')).toBe(true);
    });

    it('sends create room command', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        result.current.createRoom('New Room', 'A test room');
      });

      expect(mockWebSocketServer.receivedMessages.length).toBe(1);
      expect(mockWebSocketServer.receivedMessages[0]).toMatchObject({
        type: 'create_room',
        room_name: 'New Room',
        description: 'A test room',
      });
    });

    it('sends join room command', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        result.current.joinRoom('room-456');
      });

      expect(mockWebSocketServer.receivedMessages[0]).toMatchObject({
        type: 'join_room',
        room_id: 'room-456',
      });
    });

    it('sends leave room command and updates state', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      // First join a room
      act(() => {
        mockWebSocketServer.sendRoomJoined({
          id: 'room-123',
          name: 'Test Room',
          created_by: '12D3KooWAlice',
        });
      });

      expect(result.current.rooms.has('room-123')).toBe(true);

      // Then leave it
      act(() => {
        result.current.leaveRoom('room-123');
      });

      expect(mockWebSocketServer.receivedMessages[0]).toMatchObject({
        type: 'leave_room',
        room_id: 'room-123',
      });

      // Optimistic update should remove room
      expect(result.current.rooms.has('room-123')).toBe(false);
      expect(result.current.conversations.has('room:room-123')).toBe(false);
    });
  });

  describe('Economics: Vouching', () => {
    it('handles vouch_request message', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        mockWebSocketServer.sendVouchRequest({
          id: 'vouch-123',
          fromPeerId: '12D3KooWAlice',
          toPeerId: '12D3KooWBob',
          stake: 0.8,
          timestamp: Date.now(),
        });
      });

      expect(result.current.vouches.length).toBe(1);
      expect(result.current.vouches[0].stake).toBe(0.8);
    });

    it('sends vouch request', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      const vouchRequest = {
        fromPeerId: 'local-peer',
        toPeerId: '12D3KooWBob',
        stake: 0.9,
        timestamp: Date.now(),
      };

      act(() => {
        result.current.sendVouch(vouchRequest);
      });

      expect(mockWebSocketServer.receivedMessages[0]).toMatchObject({
        type: 'send_vouch',
      });

      // Optimistic update
      expect(result.current.vouches.length).toBe(1);
    });
  });

  describe('Economics: Credit', () => {
    it('handles credit_line message', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        mockWebSocketServer.sendCreditLine({
          id: 'cl-123',
          creditor: '12D3KooWAlice',
          debtor: '12D3KooWBob',
          limit: 1000,
          balance: 250,
        });
      });

      expect(result.current.creditLines.length).toBe(1);
      expect(result.current.creditLines[0].limit).toBe(1000);
    });

    it('sends credit line creation', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        result.current.sendCreditLine('12D3KooWBob', 500);
      });

      expect(mockWebSocketServer.receivedMessages[0]).toMatchObject({
        type: 'send_credit_line',
      });

      // Optimistic update
      expect(result.current.creditLines.length).toBe(1);
      expect(result.current.creditLines[0].limit).toBe(500);
    });

    it('sends credit transfer', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        result.current.sendCreditTransfer('12D3KooWBob', 100, 'Payment for services');
      });

      expect(mockWebSocketServer.receivedMessages[0]).toMatchObject({
        type: 'send_credit_transfer',
      });

      // Optimistic update
      expect(result.current.creditTransfers.length).toBe(1);
      expect(result.current.creditTransfers[0].amount).toBe(100);
    });
  });

  describe('Economics: Governance', () => {
    it('handles proposal message', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        mockWebSocketServer.sendProposal({
          id: 'prop-123',
          proposer: '12D3KooWAlice',
          title: 'Increase limits',
          description: 'Proposal to increase credit limits',
        });
      });

      expect(result.current.proposals.length).toBe(1);
      expect(result.current.proposals[0].title).toBe('Increase limits');
    });

    it('sends proposal creation', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        result.current.sendProposal('New Feature', 'Add new feature X', 48);
      });

      expect(mockWebSocketServer.receivedMessages[0]).toMatchObject({
        type: 'send_proposal',
      });

      // Optimistic update
      expect(result.current.proposals.length).toBe(1);
      expect(result.current.proposals[0].title).toBe('New Feature');
    });

    it('sends vote and updates proposal', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      // First create a proposal
      act(() => {
        mockWebSocketServer.sendProposal({
          id: 'prop-123',
          proposer: '12D3KooWAlice',
          title: 'Test Proposal',
          description: 'Test',
        });
      });

      expect(result.current.proposals[0].votesFor).toBe(0);

      // Then vote on it
      act(() => {
        result.current.sendVote('prop-123', 'for', 1);
      });

      expect(mockWebSocketServer.receivedMessages[0]).toMatchObject({
        type: 'send_vote',
      });

      // Optimistic update should increment votes
      expect(result.current.proposals[0].votesFor).toBe(1);
    });
  });

  describe('Economics: Resources', () => {
    it('sends resource contribution', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        result.current.sendResourceContribution('bandwidth', 100, 'mbps');
      });

      expect(mockWebSocketServer.receivedMessages[0]).toMatchObject({
        type: 'send_resource_contribution',
      });

      // Optimistic update
      expect(result.current.resourceContributions.length).toBe(1);
      expect(result.current.resourceContributions[0].resourceType).toBe('bandwidth');
    });

    it('handles resource_contribution message', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      act(() => {
        mockWebSocketServer.broadcast({
          type: 'resource_contribution',
          peerId: '12D3KooWAlice',
          resourceType: 'storage',
          amount: 500,
          unit: 'GB',
          timestamp: Date.now(),
        });
      });

      expect(result.current.resourceContributions.length).toBe(1);
      expect(result.current.resourceContributions[0].resourceType).toBe('storage');
    });
  });

  describe('Error Handling', () => {
    it('handles WebSocket close gracefully', async () => {
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      expect(result.current.connected).toBe(true);

      act(() => {
        mockWebSocketServer.lastInstance?.simulateClose(1000);
      });

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });

      expect(result.current.connected).toBe(false);
    });

    it('does not send messages when disconnected', async () => {
      mockWebSocketServer.autoConnect = false;
      const { result } = renderHook(() => useP2P({ url: 'ws://test/ws' }));

      act(() => {
        result.current.sendChat('This should not be sent');
      });

      expect(mockWebSocketServer.receivedMessages.length).toBe(0);
    });
  });
});
