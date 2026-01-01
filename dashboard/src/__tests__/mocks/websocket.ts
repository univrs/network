/**
 * Mock WebSocket for testing
 *
 * Provides a controllable WebSocket mock for testing the useP2P hook
 * and components that depend on WebSocket connections.
 */

import { vi } from 'vitest';

type MessageHandler = (event: MessageEvent) => void;
type EventHandler = (event: Event) => void;
type CloseHandler = (event: CloseEvent) => void;

interface MockWebSocketInstance {
  url: string;
  readyState: number;
  onopen: EventHandler | null;
  onmessage: MessageHandler | null;
  onclose: CloseHandler | null;
  onerror: EventHandler | null;
  send: ReturnType<typeof vi.fn>;
  close: ReturnType<typeof vi.fn>;
  // Test helpers
  simulateOpen: () => void;
  simulateMessage: (data: unknown) => void;
  simulateClose: (code?: number, reason?: string) => void;
  simulateError: () => void;
}

// Store all created WebSocket instances for test access
const instances: MockWebSocketInstance[] = [];

/**
 * Mock WebSocket class that simulates WebSocket behavior
 */
export class MockWebSocket implements MockWebSocketInstance {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  url: string;
  readyState: number = MockWebSocket.CONNECTING;
  onopen: EventHandler | null = null;
  onmessage: MessageHandler | null = null;
  onclose: CloseHandler | null = null;
  onerror: EventHandler | null = null;

  send = vi.fn((data: string) => {
    if (this.readyState !== MockWebSocket.OPEN) {
      throw new Error('WebSocket is not open');
    }
    // Store sent messages for assertions
    mockWebSocketServer.receivedMessages.push(JSON.parse(data));
  });

  close = vi.fn((code?: number, reason?: string) => {
    this.readyState = MockWebSocket.CLOSING;
    setTimeout(() => {
      this.readyState = MockWebSocket.CLOSED;
      if (this.onclose) {
        this.onclose(new CloseEvent('close', { code: code || 1000, reason: reason || '' }));
      }
    }, 0);
  });

  constructor(url: string) {
    this.url = url;
    instances.push(this);

    // Auto-open after a tick (simulating connection)
    if (mockWebSocketServer.autoConnect) {
      setTimeout(() => this.simulateOpen(), 0);
    }
  }

  // Test helper methods
  simulateOpen() {
    this.readyState = MockWebSocket.OPEN;
    if (this.onopen) {
      this.onopen(new Event('open'));
    }
  }

  simulateMessage(data: unknown) {
    if (this.onmessage) {
      const messageData = typeof data === 'string' ? data : JSON.stringify(data);
      this.onmessage(new MessageEvent('message', { data: messageData }));
    }
  }

  simulateClose(code = 1000, reason = '') {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose(new CloseEvent('close', { code, reason }));
    }
  }

  simulateError() {
    if (this.onerror) {
      this.onerror(new Event('error'));
    }
  }
}

/**
 * Mock WebSocket server for controlling test scenarios
 */
export const mockWebSocketServer = {
  /** All WebSocket instances created during tests */
  get instances() {
    return instances;
  },

  /** Get the most recent WebSocket instance */
  get lastInstance(): MockWebSocketInstance | undefined {
    return instances[instances.length - 1];
  },

  /** Messages received from clients */
  receivedMessages: [] as unknown[],

  /** Whether to auto-connect new WebSockets */
  autoConnect: true,

  /** Reset all mock state */
  reset() {
    instances.length = 0;
    this.receivedMessages.length = 0;
    this.autoConnect = true;
  },

  /** Send a message to all connected clients */
  broadcast(data: unknown) {
    for (const instance of instances) {
      if (instance.readyState === MockWebSocket.OPEN) {
        instance.simulateMessage(data);
      }
    }
  },

  /** Simulate server sending initial peers list */
  sendPeersList(peers: Array<{ id: string; name?: string; reputation?: number }>) {
    this.broadcast({
      type: 'peers_list',
      peers: peers.map((p) => ({
        id: p.id,
        name: p.name || `Peer-${p.id.slice(0, 8)}`,
        reputation: p.reputation || 0.5,
        addresses: [],
      })),
    });
  },

  /** Simulate server sending a chat message */
  sendChatMessage(message: {
    id?: string;
    from: string;
    from_name?: string;
    content: string;
    to?: string;
    room_id?: string;
  }) {
    this.broadcast({
      type: 'chat_message',
      id: message.id || `msg-${Date.now()}`,
      from: message.from,
      from_name: message.from_name || `Peer-${message.from.slice(0, 8)}`,
      content: message.content,
      to: message.to || null,
      room_id: message.room_id || null,
      timestamp: Date.now(),
    });
  },

  /** Simulate server sending stats */
  sendStats(stats: { peer_count: number; message_count: number; uptime_seconds: number }) {
    this.broadcast({
      type: 'stats',
      ...stats,
    });
  },

  /** Simulate peer joined */
  sendPeerJoined(peer: { peer_id: string; name?: string }) {
    this.broadcast({
      type: 'peer_joined',
      peer_id: peer.peer_id,
      name: peer.name,
    });
  },

  /** Simulate peer left */
  sendPeerLeft(peerId: string) {
    this.broadcast({
      type: 'peer_left',
      peer_id: peerId,
    });
  },

  /** Simulate room joined */
  sendRoomJoined(room: {
    id: string;
    name: string;
    description?: string;
    members?: string[];
    created_by: string;
  }) {
    this.broadcast({
      type: 'room_joined',
      id: room.id,
      name: room.name,
      description: room.description || null,
      topic: `/mycelial/1.0.0/room/${room.id}`,
      members: room.members || [room.created_by],
      created_by: room.created_by,
      created_at: Date.now(),
      is_public: true,
    });
  },

  /** Simulate vouch request */
  sendVouchRequest(vouch: { id: string; fromPeerId: string; toPeerId: string; stake: number; timestamp?: number; message?: string }) {
    this.broadcast({
      type: 'vouch_request',
      id: vouch.id,
      voucher: vouch.fromPeerId,
      vouchee: vouch.toPeerId,
      weight: vouch.stake,
      message: vouch.message,
      timestamp: vouch.timestamp || Date.now(),
    });
  },

  /** Simulate credit line */
  sendCreditLine(credit: {
    id: string;
    creditor: string;
    debtor: string;
    limit: number;
    balance?: number;
  }) {
    this.broadcast({
      type: 'credit_line',
      id: credit.id,
      creditor: credit.creditor,
      debtor: credit.debtor,
      limit: credit.limit,
      balance: credit.balance || 0,
      timestamp: Date.now(),
    });
  },

  /** Simulate proposal */
  sendProposal(proposal: {
    id: string;
    proposer: string;
    title: string;
    description: string;
    proposal_type?: string;
  }) {
    this.broadcast({
      type: 'proposal',
      id: proposal.id,
      proposer: proposal.proposer,
      title: proposal.title,
      description: proposal.description,
      proposal_type: proposal.proposal_type || 'text',
      status: 'active',
      votesFor: 0,
      votesAgainst: 0,
      quorum: 0.5,
      expiresAt: Date.now() + 86400000,
      createdAt: Date.now(),
    });
  },
};
