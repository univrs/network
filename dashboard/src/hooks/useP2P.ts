import { useState, useEffect, useCallback, useRef } from 'react';
import type {
  ChatMessage,
  GraphNode,
  GraphLink,
  NormalizedPeer,
  Location,
  CreditLine,
  CreditTransfer,
  Proposal,
  Vote,
  VouchRequest,
  ResourceContribution,
  ResourcePool,
  Conversation,
  Room,
  // ENR Bridge types
  GradientUpdate,
  EnrCreditTransfer,
  ElectionAnnouncement,
  ElectionCandidacy,
  ElectionVote,
  ElectionResult,
  SeptalStateChange,
  SeptalHealthStatus,
  Election,
  NodeEnrState,
  SeptalState,
} from '@/types';

interface UseP2POptions {
  url?: string;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  apiUrl?: string;
}

interface P2PState {
  connected: boolean;
  localPeerId: string | null;
  peers: Map<string, NormalizedPeer>;
  messages: ChatMessage[];
  // Conversation state
  conversations: Map<string, Conversation>;
  rooms: Map<string, Room>;
  activeConversationId: string;
  // Economics state
  creditLines: CreditLine[];
  creditTransfers: CreditTransfer[];
  proposals: Proposal[];
  vouches: VouchRequest[];
  resourceContributions: ResourceContribution[];
  resourcePool: ResourcePool | null;
  // ENR Bridge state
  gradients: Map<string, GradientUpdate>;
  enrTransfers: EnrCreditTransfer[];
  nodeEnrStates: Map<string, NodeEnrState>;
  elections: Map<number, Election>;
}

// Community conversation ID constant
const COMMUNITY_CONVERSATION_ID = 'community';

// Helper to get conversation ID for a message
function getConversationId(msg: ChatMessage, _localPeerId: string | null): string {
  if (msg.room_id) {
    return `room:${msg.room_id}`;
  }
  if (msg.to) {
    // DM conversation - use sorted peer IDs to ensure consistency
    const peers = [msg.from, msg.to].sort();
    return `dm:${peers[0]}:${peers[1]}`;
  }
  // Broadcast/community message
  return COMMUNITY_CONVERSATION_ID;
}

// Helper to create or update conversation from message
function updateConversation(
  conversations: Map<string, Conversation>,
  msg: ChatMessage,
  localPeerId: string | null,
  peers: Map<string, NormalizedPeer>,
  isActive: boolean
): Map<string, Conversation> {
  const convId = getConversationId(msg, localPeerId);
  const existing = conversations.get(convId);

  let name = 'Community';
  let peerId: string | undefined;
  let roomId: string | undefined;
  let type: 'community' | 'dm' | 'room' = 'community';

  if (msg.room_id) {
    type = 'room';
    roomId = msg.room_id;
    name = `Room: ${msg.room_id}`;
  } else if (msg.to) {
    type = 'dm';
    // Get the other peer's ID (not our own)
    peerId = msg.from === localPeerId ? msg.to : msg.from;
    const peer = peers.get(peerId);
    name = peer?.name || `Peer-${peerId.slice(0, 8)}`;
  }

  const updated = new Map(conversations);
  updated.set(convId, {
    id: convId,
    type,
    name,
    peerId,
    roomId,
    lastMessage: msg,
    unreadCount: existing ? (isActive ? 0 : existing.unreadCount + 1) : (isActive ? 0 : 1),
    createdAt: existing?.createdAt || msg.timestamp,
  });

  return updated;
}

// Environment configuration - P2P node runs on port 8080
// Note: Orchestrator is separate at port 9090, handled by useOrchestrator hook
const ENV_WS_URL = import.meta.env.VITE_P2P_WS_URL || 'ws://localhost:8080/ws';
const ENV_API_URL = import.meta.env.VITE_P2P_API_URL || 'http://localhost:8080';

// Normalize peer data from different backend formats
function normalizePeer(peer: unknown): NormalizedPeer {
  const p = peer as Record<string, unknown>;
  const id = (p.id || p.peer_id || '') as string;
  const name = (p.name || p.display_name || `Peer-${id.slice(0, 12)}`) as string;
  const repValue = p.reputation;
  const reputation = typeof repValue === 'number'
    ? repValue
    : ((repValue as Record<string, unknown>)?.score as number ?? 0.5);

  return {
    id,
    name,
    reputation,
    location: p.location as Location | undefined,
    addresses: (p.addresses || []) as string[],
  };
}

export function useP2P(options: UseP2POptions = {}) {
  // Extract options with defaults
  const wsUrl = options.url ?? ENV_WS_URL;
  const apiUrl = options.apiUrl ?? ENV_API_URL;
  const reconnectInterval = options.reconnectInterval ?? 3000;
  const maxReconnectAttempts = options.maxReconnectAttempts ?? 5;

  // Refs for WebSocket management
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const reconnectAttemptsRef = useRef(0);
  const isConnectingRef = useRef(false);
  const isMountedRef = useRef(true);

  // Store options in refs to avoid dependency cycles
  const wsUrlRef = useRef(wsUrl);
  const apiUrlRef = useRef(apiUrl);
  const reconnectIntervalRef = useRef(reconnectInterval);
  const maxReconnectAttemptsRef = useRef(maxReconnectAttempts);

  // Update refs when options change
  wsUrlRef.current = wsUrl;
  apiUrlRef.current = apiUrl;
  reconnectIntervalRef.current = reconnectInterval;
  maxReconnectAttemptsRef.current = maxReconnectAttempts;

  // Initialize with default community conversation
  const defaultConversations = new Map<string, Conversation>([
    [COMMUNITY_CONVERSATION_ID, {
      id: COMMUNITY_CONVERSATION_ID,
      type: 'community',
      name: 'Community',
      unreadCount: 0,
      createdAt: Date.now(),
    }],
  ]);

  const [state, setState] = useState<P2PState>({
    connected: false,
    localPeerId: null,
    peers: new Map(),
    messages: [],
    // Conversation initial state
    conversations: defaultConversations,
    rooms: new Map(),
    activeConversationId: COMMUNITY_CONVERSATION_ID,
    // Economics initial state
    creditLines: [],
    creditTransfers: [],
    proposals: [],
    vouches: [],
    resourceContributions: [],
    resourcePool: null,
    // ENR Bridge initial state
    gradients: new Map(),
    enrTransfers: [],
    nodeEnrStates: new Map(),
    elections: new Map(),
  });

  // Fetch peers from P2P node REST API
  const fetchPeers = useCallback(async () => {
    if (!isMountedRef.current) return;
    try {
      const response = await fetch(`${apiUrlRef.current}/api/peers`);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json();
      const peers = data.peers || data || [];
      setState(s => {
        const newPeers = new Map(s.peers);
        for (const peer of peers) {
          const normalized = normalizePeer(peer);
          if (normalized.id) {
            newPeers.set(normalized.id, normalized);
          }
        }
        return { ...s, peers: newPeers };
      });
      console.log('P2P: Fetched peers via REST:', peers.length);
    } catch (err) {
      console.warn('P2P: Failed to fetch peers via REST:', err);
      // Peers will be populated via WebSocket events as fallback
    }
  }, []);

  // Fetch local node info from P2P node REST API
  const fetchInfo = useCallback(async () => {
    if (!isMountedRef.current) return;
    try {
      const response = await fetch(`${apiUrlRef.current}/api/info`);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json();
      const peerId = data.peer_id || data.peerId || data.id;
      if (peerId) {
        setState(s => ({ ...s, localPeerId: peerId }));
        console.log('P2P: Got local peer ID:', peerId);
      }
    } catch (err) {
      console.warn('P2P: Failed to fetch node info via REST:', err);
      // Generate fallback local ID
      const localId = `local-${Date.now().toString(36)}`;
      setState(s => ({ ...s, localPeerId: localId }));
      console.log('P2P: Generated fallback peer ID:', localId);
    }
  }, []);

  // Handle incoming WebSocket messages
  const handleMessage = useCallback((message: Record<string, unknown>) => {
    if (!isMountedRef.current) return;
    console.log('WS Message:', message);

    switch (message.type) {
      case 'peers_list': {
        const peers = (message.peers || (message.data as Record<string, unknown>)?.peers || []) as unknown[];
        setState(s => {
          const newPeers = new Map(s.peers);
          for (const peer of peers) {
            const normalized = normalizePeer(peer);
            if (normalized.id) {
              newPeers.set(normalized.id, normalized);
            }
          }
          return { ...s, peers: newPeers };
        });
        break;
      }

      case 'peer_joined': {
        const peerId = (message.peer_id || (message.data as Record<string, unknown>)?.peer_id) as string | undefined;
        const peerInfo = message.peer_info || (message.data as Record<string, unknown>)?.peer_info || message;
        if (peerId) {
          setState(s => {
            const newPeers = new Map(s.peers);
            const normalized = normalizePeer({ ...peerInfo as object, id: peerId });
            newPeers.set(peerId, normalized);
            return { ...s, peers: newPeers };
          });
        }
        break;
      }

      case 'peer_left': {
        const peerId = (message.peer_id || (message.data as Record<string, unknown>)?.peer_id) as string | undefined;
        if (peerId) {
          setState(s => {
            const newPeers = new Map(s.peers);
            newPeers.delete(peerId);
            return { ...s, peers: newPeers };
          });
        }
        break;
      }

      case 'chat_message': {
        const chatMsg = (message.data || message) as ChatMessage;
        setState(s => {
          const convId = getConversationId(chatMsg, s.localPeerId);
          const isActive = convId === s.activeConversationId;
          return {
            ...s,
            messages: [...s.messages.slice(-99), chatMsg],
            conversations: updateConversation(s.conversations, chatMsg, s.localPeerId, s.peers, isActive),
          };
        });
        break;
      }

      case 'room_joined': {
        const room = (message.data || message) as Room;
        setState(s => {
          const newRooms = new Map(s.rooms);
          newRooms.set(room.id, room);
          // Add conversation for the room
          const newConversations = new Map(s.conversations);
          newConversations.set(`room:${room.id}`, {
            id: `room:${room.id}`,
            type: 'room',
            name: room.name,
            roomId: room.id,
            unreadCount: 0,
            createdAt: room.createdAt,
          });
          return { ...s, rooms: newRooms, conversations: newConversations };
        });
        break;
      }

      case 'room_left': {
        const roomId = (message.room_id || (message.data as Record<string, unknown>)?.room_id) as string;
        setState(s => {
          const newRooms = new Map(s.rooms);
          newRooms.delete(roomId);
          const newConversations = new Map(s.conversations);
          newConversations.delete(`room:${roomId}`);
          return { ...s, rooms: newRooms, conversations: newConversations };
        });
        break;
      }

      case 'room_list': {
        const rooms = (message.rooms || (message.data as Record<string, unknown>)?.rooms || []) as Room[];
        setState(s => {
          const newRooms = new Map(s.rooms);
          const newConversations = new Map(s.conversations);
          for (const room of rooms) {
            newRooms.set(room.id, room);
            newConversations.set(`room:${room.id}`, {
              id: `room:${room.id}`,
              type: 'room',
              name: room.name,
              roomId: room.id,
              unreadCount: 0,
              createdAt: room.createdAt,
            });
          }
          return { ...s, rooms: newRooms, conversations: newConversations };
        });
        break;
      }

      // Economics message handlers
      case 'vouch_request': {
        // Server sends: voucher, vouchee, weight; Client type uses: fromPeerId, toPeerId, stake
        type ServerVouch = { voucher?: string; vouchee?: string; weight?: number; fromPeerId?: string; toPeerId?: string; stake?: number; message?: string; timestamp?: number };
        const raw = (message.data || message) as ServerVouch;
        const vouch: VouchRequest = {
          fromPeerId: raw.voucher || raw.fromPeerId || '',
          toPeerId: raw.vouchee || raw.toPeerId || '',
          stake: raw.weight ?? raw.stake ?? 0,
          message: raw.message,
          timestamp: raw.timestamp || Date.now(),
        };
        setState(s => ({
          ...s,
          vouches: [...s.vouches, vouch],
        }));
        console.log('Received vouch request:', vouch);
        break;
      }

      case 'credit_line': {
        const creditLine = (message.data || message) as CreditLine;
        setState(s => {
          // Update existing or add new credit line
          const existingIndex = s.creditLines.findIndex(cl => cl.id === creditLine.id);
          if (existingIndex >= 0) {
            const updated = [...s.creditLines];
            updated[existingIndex] = creditLine;
            return { ...s, creditLines: updated };
          }
          return { ...s, creditLines: [...s.creditLines, creditLine] };
        });
        console.log('Received credit line:', creditLine);
        break;
      }

      case 'credit_transfer': {
        const transfer = (message.data || message) as CreditTransfer;
        setState(s => ({
          ...s,
          creditTransfers: [...s.creditTransfers.slice(-99), transfer],
        }));
        console.log('Received credit transfer:', transfer);
        break;
      }

      case 'proposal': {
        const proposal = (message.data || message) as Proposal;
        setState(s => {
          // Update existing or add new proposal
          const existingIndex = s.proposals.findIndex(p => p.id === proposal.id);
          if (existingIndex >= 0) {
            const updated = [...s.proposals];
            updated[existingIndex] = proposal;
            return { ...s, proposals: updated };
          }
          return { ...s, proposals: [...s.proposals, proposal] };
        });
        console.log('Received proposal:', proposal);
        break;
      }

      case 'vote_cast': {
        const vote = (message.data || message) as Vote;
        // Update the proposal's vote counts
        setState(s => {
          const proposalIndex = s.proposals.findIndex(p => p.id === vote.proposalId);
          if (proposalIndex >= 0) {
            const updated = [...s.proposals];
            const proposal = { ...updated[proposalIndex] };
            if (vote.vote === 'for') {
              proposal.votesFor += vote.weight;
            } else {
              proposal.votesAgainst += vote.weight;
            }
            updated[proposalIndex] = proposal;
            return { ...s, proposals: updated };
          }
          return s;
        });
        console.log('Received vote:', vote);
        break;
      }

      case 'resource_contribution': {
        const contribution = (message.data || message) as ResourceContribution;
        setState(s => ({
          ...s,
          resourceContributions: [...s.resourceContributions.slice(-99), contribution],
        }));
        console.log('Received resource contribution:', contribution);
        break;
      }

      case 'resource_pool': {
        const pool = (message.data || message) as ResourcePool;
        setState(s => ({ ...s, resourcePool: pool }));
        console.log('Received resource pool update:', pool);
        break;
      }

      // ============ ENR Bridge Message Handlers ============

      case 'gradient_update': {
        const data = message.data || message;
        const gradient: GradientUpdate = {
          source: (data as Record<string, unknown>).source as string,
          cpuAvailable: (data as Record<string, unknown>).cpu_available as number,
          memoryAvailable: (data as Record<string, unknown>).memory_available as number,
          bandwidthAvailable: (data as Record<string, unknown>).bandwidth_available as number,
          storageAvailable: (data as Record<string, unknown>).storage_available as number,
          timestamp: (data as Record<string, unknown>).timestamp as number,
        };
        setState(s => {
          const newGradients = new Map(s.gradients);
          newGradients.set(gradient.source, gradient);
          return { ...s, gradients: newGradients };
        });
        console.log('Received gradient update:', gradient);
        break;
      }

      case 'enr_credit_transfer': {
        const data = message.data || message;
        const transfer: EnrCreditTransfer = {
          from: (data as Record<string, unknown>).from as string,
          to: (data as Record<string, unknown>).to as string,
          amount: (data as Record<string, unknown>).amount as number,
          tax: (data as Record<string, unknown>).tax as number,
          nonce: (data as Record<string, unknown>).nonce as number,
          timestamp: (data as Record<string, unknown>).timestamp as number,
        };
        setState(s => ({
          ...s,
          enrTransfers: [...s.enrTransfers.slice(-99), transfer],
        }));
        console.log('Received ENR credit transfer:', transfer);
        break;
      }

      case 'enr_balance_update': {
        const data = (message.data || message) as Record<string, unknown>;
        const nodeId = (data.node_id || data.nodeId) as string;
        const balance = data.balance as number;
        setState(s => {
          const newStates = new Map(s.nodeEnrStates);
          const existing = newStates.get(nodeId) || {
            nodeId,
            balance: 0,
            septalState: 'closed' as SeptalState,
            septalHealthy: true,
            failureCount: 0,
            lastUpdated: Date.now(),
          };
          newStates.set(nodeId, { ...existing, balance, lastUpdated: Date.now() });
          return { ...s, nodeEnrStates: newStates };
        });
        console.log('Received ENR balance update:', { nodeId, balance });
        break;
      }

      case 'election_announcement': {
        const data = (message.data || message) as Record<string, unknown>;
        const announcement: ElectionAnnouncement = {
          electionId: (data.election_id || data.electionId) as number,
          initiator: data.initiator as string,
          regionId: (data.region_id || data.regionId) as string,
          timestamp: data.timestamp as number,
        };
        setState(s => {
          const newElections = new Map(s.elections);
          newElections.set(announcement.electionId, {
            id: announcement.electionId,
            regionId: announcement.regionId,
            initiator: announcement.initiator,
            status: 'announced',
            candidates: [],
            votes: [],
            startedAt: announcement.timestamp,
          });
          return { ...s, elections: newElections };
        });
        console.log('Received election announcement:', announcement);
        break;
      }

      case 'election_candidacy': {
        const data = (message.data || message) as Record<string, unknown>;
        const candidacy: ElectionCandidacy = {
          electionId: (data.election_id || data.electionId) as number,
          candidate: data.candidate as string,
          uptime: data.uptime as number,
          cpuAvailable: (data.cpu_available || data.cpuAvailable) as number,
          memoryAvailable: (data.memory_available || data.memoryAvailable) as number,
          reputation: data.reputation as number,
          timestamp: data.timestamp as number,
        };
        setState(s => {
          const newElections = new Map(s.elections);
          const election = newElections.get(candidacy.electionId);
          if (election) {
            const existingIndex = election.candidates.findIndex(c => c.candidate === candidacy.candidate);
            if (existingIndex >= 0) {
              election.candidates[existingIndex] = candidacy;
            } else {
              election.candidates.push(candidacy);
            }
            election.status = 'voting';
            newElections.set(candidacy.electionId, { ...election });
          }
          return { ...s, elections: newElections };
        });
        console.log('Received election candidacy:', candidacy);
        break;
      }

      case 'election_vote': {
        const data = (message.data || message) as Record<string, unknown>;
        const vote: ElectionVote = {
          electionId: (data.election_id || data.electionId) as number,
          voter: data.voter as string,
          candidate: data.candidate as string,
          timestamp: data.timestamp as number,
        };
        setState(s => {
          const newElections = new Map(s.elections);
          const election = newElections.get(vote.electionId);
          if (election) {
            election.votes.push(vote);
            newElections.set(vote.electionId, { ...election });
          }
          return { ...s, elections: newElections };
        });
        console.log('Received election vote:', vote);
        break;
      }

      case 'election_result': {
        const data = (message.data || message) as Record<string, unknown>;
        const result: ElectionResult = {
          electionId: (data.election_id || data.electionId) as number,
          winner: data.winner as string,
          regionId: (data.region_id || data.regionId) as string,
          voteCount: (data.vote_count || data.voteCount) as number,
          timestamp: data.timestamp as number,
        };
        setState(s => {
          const newElections = new Map(s.elections);
          const election = newElections.get(result.electionId);
          if (election) {
            newElections.set(result.electionId, {
              ...election,
              status: 'completed',
              winner: result.winner,
              voteCount: result.voteCount,
              completedAt: result.timestamp,
            });
          }
          return { ...s, elections: newElections };
        });
        console.log('Received election result:', result);
        break;
      }

      case 'septal_state_change': {
        const data = (message.data || message) as Record<string, unknown>;
        const change: SeptalStateChange = {
          nodeId: (data.node_id || data.nodeId) as string,
          fromState: (data.from_state || data.fromState) as SeptalState,
          toState: (data.to_state || data.toState) as SeptalState,
          reason: data.reason as string,
          timestamp: data.timestamp as number,
        };
        setState(s => {
          const newStates = new Map(s.nodeEnrStates);
          const existing = newStates.get(change.nodeId) || {
            nodeId: change.nodeId,
            balance: 0,
            septalState: 'closed' as SeptalState,
            septalHealthy: true,
            failureCount: 0,
            lastUpdated: Date.now(),
          };
          newStates.set(change.nodeId, {
            ...existing,
            septalState: change.toState,
            lastUpdated: Date.now(),
          });
          return { ...s, nodeEnrStates: newStates };
        });
        console.log('Received septal state change:', change);
        break;
      }

      case 'septal_health_status': {
        const data = (message.data || message) as Record<string, unknown>;
        const status: SeptalHealthStatus = {
          nodeId: (data.node_id || data.nodeId) as string,
          isHealthy: (data.is_healthy || data.isHealthy) as boolean,
          failureCount: (data.failure_count || data.failureCount) as number,
          timestamp: data.timestamp as number,
        };
        setState(s => {
          const newStates = new Map(s.nodeEnrStates);
          const existing = newStates.get(status.nodeId) || {
            nodeId: status.nodeId,
            balance: 0,
            septalState: 'closed' as SeptalState,
            septalHealthy: true,
            failureCount: 0,
            lastUpdated: Date.now(),
          };
          newStates.set(status.nodeId, {
            ...existing,
            septalHealthy: status.isHealthy,
            failureCount: status.failureCount,
            lastUpdated: Date.now(),
          });
          return { ...s, nodeEnrStates: newStates };
        });
        console.log('Received septal health status:', status);
        break;
      }

      default:
        console.log('Unhandled message type:', message.type);
    }
  }, []);

  // Connect to WebSocket
  const connect = useCallback(() => {
    // Prevent multiple simultaneous connection attempts
    if (isConnectingRef.current) return;
    if (!isMountedRef.current) return;
    if (wsRef.current?.readyState === WebSocket.OPEN) return;
    if (wsRef.current?.readyState === WebSocket.CONNECTING) return;

    // Check if we've exceeded max reconnect attempts
    if (reconnectAttemptsRef.current >= maxReconnectAttemptsRef.current) {
      console.warn(`P2P: Max reconnect attempts (${maxReconnectAttemptsRef.current}) reached. Stopping reconnection.`);
      return;
    }

    isConnectingRef.current = true;
    const currentUrl = wsUrlRef.current;
    console.log(`Connecting to P2P WebSocket: ${currentUrl} (attempt ${reconnectAttemptsRef.current + 1}/${maxReconnectAttemptsRef.current})`);

    try {
      const ws = new WebSocket(currentUrl);

      ws.onopen = () => {
        if (!isMountedRef.current) {
          ws.close(1000, 'Component unmounted');
          return;
        }
        console.log('P2P WebSocket connected!');
        isConnectingRef.current = false;
        reconnectAttemptsRef.current = 0; // Reset on successful connection
        setState(s => ({ ...s, connected: true }));
        // Fetch data via REST API (more reliable for initial load)
        fetchInfo();
        fetchPeers();
      };

      ws.onclose = (event) => {
        console.log('P2P WebSocket disconnected:', event.code);
        isConnectingRef.current = false;
        wsRef.current = null;

        if (!isMountedRef.current) return;
        setState(s => ({ ...s, connected: false }));

        // Auto-reconnect with exponential backoff if not a clean close
        if (event.code !== 1000 && reconnectAttemptsRef.current < maxReconnectAttemptsRef.current && isMountedRef.current) {
          reconnectAttemptsRef.current++;
          // Exponential backoff: 3s, 6s, 12s, 24s, 48s...
          const backoffDelay = reconnectIntervalRef.current * Math.pow(2, reconnectAttemptsRef.current - 1);
          console.log(`P2P: Reconnecting in ${backoffDelay / 1000}s...`);
          reconnectTimerRef.current = setTimeout(() => {
            if (isMountedRef.current) {
              connect();
            }
          }, backoffDelay);
        }
      };

      ws.onerror = () => {
        // Don't log the full error object as it's not useful
        console.warn('P2P WebSocket connection error');
        isConnectingRef.current = false;
      };

      ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data);
          handleMessage(message);
        } catch (e) {
          console.error('Failed to parse message:', e, event.data);
        }
      };

      wsRef.current = ws;
    } catch (err) {
      console.error('P2P WebSocket connection error:', err);
      isConnectingRef.current = false;
    }
  }, [fetchInfo, fetchPeers, handleMessage]);

  // Send chat message (to DM, room, or community)
  const sendChat = useCallback((content: string, to?: string, roomId?: string) => {
    console.log('sendChat called:', { content, to, roomId, readyState: wsRef.current?.readyState });

    if (wsRef.current?.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not open! State:', wsRef.current?.readyState);
      return;
    }

    const message = JSON.stringify({ type: 'send_chat', content, to, room_id: roomId });
    console.log('Sending via WebSocket:', message);
    wsRef.current.send(message);

    // Optimistically add our own message to local state
    // (gossipsub doesn't echo messages back to the sender)
    setState(s => {
      const localId = s.localPeerId || 'unknown';
      const shortId = localId.slice(0, 8);
      const chatMessage: ChatMessage = {
        id: `local-${Date.now()}`,
        from: localId,
        from_name: `Peer-${shortId} (you)`,
        to: to || undefined,
        room_id: roomId || undefined,
        content,
        timestamp: Date.now(),
      };
      return {
        ...s,
        messages: [...s.messages.slice(-99), chatMessage],
        conversations: updateConversation(s.conversations, chatMessage, localId, s.peers, true),
      };
    });
  }, []);

  // Switch active conversation
  const setActiveConversation = useCallback((conversationId: string) => {
    setState(s => {
      // Clear unread count for the conversation we're switching to
      const newConversations = new Map(s.conversations);
      const conv = newConversations.get(conversationId);
      if (conv) {
        newConversations.set(conversationId, { ...conv, unreadCount: 0 });
      }
      return {
        ...s,
        activeConversationId: conversationId,
        conversations: newConversations,
      };
    });
  }, []);

  // Start a DM conversation with a peer
  const startDMConversation = useCallback((peerId: string) => {
    setState(s => {
      const localId = s.localPeerId || 'unknown';
      const peers = [localId, peerId].sort();
      const convId = `dm:${peers[0]}:${peers[1]}`;

      // Check if conversation already exists
      if (s.conversations.has(convId)) {
        return { ...s, activeConversationId: convId };
      }

      // Create new DM conversation
      const peer = s.peers.get(peerId);
      const newConversations = new Map(s.conversations);
      newConversations.set(convId, {
        id: convId,
        type: 'dm',
        name: peer?.name || `Peer-${peerId.slice(0, 8)}`,
        peerId,
        unreadCount: 0,
        createdAt: Date.now(),
      });

      return {
        ...s,
        conversations: newConversations,
        activeConversationId: convId,
      };
    });
  }, []);

  // Join a room
  const joinRoom = useCallback((roomId: string, roomName?: string) => {
    if (wsRef.current?.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not open for joining room');
      return;
    }
    const message = JSON.stringify({ type: 'join_room', room_id: roomId, room_name: roomName });
    wsRef.current.send(message);
    console.log('Joining room:', roomId);
  }, []);

  // Leave a room
  const leaveRoom = useCallback((roomId: string) => {
    if (wsRef.current?.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not open for leaving room');
      return;
    }
    const message = JSON.stringify({ type: 'leave_room', room_id: roomId });
    wsRef.current.send(message);
    console.log('Leaving room:', roomId);

    // Optimistically remove from state
    setState(s => {
      const newRooms = new Map(s.rooms);
      newRooms.delete(roomId);
      const newConversations = new Map(s.conversations);
      newConversations.delete(`room:${roomId}`);
      // Switch to community if we were in this room
      const newActiveId = s.activeConversationId === `room:${roomId}` ? COMMUNITY_CONVERSATION_ID : s.activeConversationId;
      return { ...s, rooms: newRooms, conversations: newConversations, activeConversationId: newActiveId };
    });
  }, []);

  // Create a new room
  const createRoom = useCallback((name: string, description?: string, isPublic: boolean = true) => {
    if (wsRef.current?.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not open for creating room');
      return;
    }
    const roomId = `room-${Date.now().toString(36)}`;
    const message = JSON.stringify({
      type: 'create_room',
      room_id: roomId,
      room_name: name,
      description,
      is_public: isPublic,
    });
    wsRef.current.send(message);
    console.log('Creating room:', name);
  }, []);

  // Get messages filtered by active conversation
  const getActiveMessages = useCallback((): ChatMessage[] => {
    const convId = state.activeConversationId;

    if (convId === COMMUNITY_CONVERSATION_ID) {
      // Community: show messages without 'to' and without 'room_id'
      return state.messages.filter(m => !m.to && !m.room_id);
    }

    if (convId.startsWith('dm:')) {
      // DM: show messages between the two peers
      const parts = convId.split(':');
      const peer1 = parts[1];
      const peer2 = parts[2];
      return state.messages.filter(m =>
        m.to && ((m.from === peer1 && m.to === peer2) || (m.from === peer2 && m.to === peer1))
      );
    }

    if (convId.startsWith('room:')) {
      // Room: show messages with matching room_id
      const roomId = convId.replace('room:', '');
      return state.messages.filter(m => m.room_id === roomId);
    }

    return [];
  }, [state.activeConversationId, state.messages]);

  // Send vouch request
  const sendVouch = useCallback((request: VouchRequest) => {
    if (wsRef.current?.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not open for vouch');
      return;
    }
    const message = JSON.stringify({ type: 'send_vouch', data: request });
    wsRef.current.send(message);
    console.log('Sent vouch request:', request);

    // Optimistically add to local state
    setState(s => ({
      ...s,
      vouches: [...s.vouches, request],
    }));
  }, []);

  // Create credit line
  const sendCreditLine = useCallback((peerId: string, limit: number) => {
    if (wsRef.current?.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not open for credit line');
      return;
    }
    const creditLine: CreditLine = {
      id: `cl-${Date.now()}`,
      peerId1: state.localPeerId || 'unknown',
      peerId2: peerId,
      limit,
      balance: 0,
      createdAt: Date.now(),
    };
    const message = JSON.stringify({ type: 'send_credit_line', data: creditLine });
    wsRef.current.send(message);
    console.log('Sent credit line:', creditLine);

    // Optimistically add to local state
    setState(s => ({
      ...s,
      creditLines: [...s.creditLines, creditLine],
    }));
  }, [state.localPeerId]);

  // Send credit transfer
  const sendCreditTransfer = useCallback((to: string, amount: number, memo?: string) => {
    if (wsRef.current?.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not open for credit transfer');
      return;
    }
    const transfer: CreditTransfer = {
      id: `tx-${Date.now()}`,
      from: state.localPeerId || 'unknown',
      to,
      amount,
      memo,
      timestamp: Date.now(),
    };
    const message = JSON.stringify({ type: 'send_credit_transfer', data: transfer });
    wsRef.current.send(message);
    console.log('Sent credit transfer:', transfer);

    // Optimistically add to local state
    setState(s => ({
      ...s,
      creditTransfers: [...s.creditTransfers, transfer],
    }));
  }, [state.localPeerId]);

  // Create governance proposal
  const sendProposal = useCallback((title: string, description: string, expiresInHours: number = 72) => {
    if (wsRef.current?.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not open for proposal');
      return;
    }
    const now = Date.now();
    const proposal: Proposal = {
      id: `prop-${now}`,
      title,
      description,
      proposer: state.localPeerId || 'unknown',
      createdAt: now,
      expiresAt: now + expiresInHours * 60 * 60 * 1000,
      status: 'active',
      votesFor: 0,
      votesAgainst: 0,
      quorum: 0.5,
    };
    const message = JSON.stringify({ type: 'send_proposal', data: proposal });
    wsRef.current.send(message);
    console.log('Sent proposal:', proposal);

    // Optimistically add to local state
    setState(s => ({
      ...s,
      proposals: [...s.proposals, proposal],
    }));
  }, [state.localPeerId]);

  // Cast vote on proposal
  const sendVote = useCallback((proposalId: string, vote: 'for' | 'against' | 'abstain', weight: number = 1) => {
    if (wsRef.current?.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not open for vote');
      return;
    }
    const voteData: Vote = {
      proposalId,
      voterId: state.localPeerId || 'unknown',
      vote,
      weight,
      timestamp: Date.now(),
    };
    const message = JSON.stringify({ type: 'send_vote', data: voteData });
    wsRef.current.send(message);
    console.log('Sent vote:', voteData);

    // Optimistically update proposal in local state
    setState(s => {
      const proposalIndex = s.proposals.findIndex(p => p.id === proposalId);
      if (proposalIndex >= 0) {
        const updated = [...s.proposals];
        const proposal = { ...updated[proposalIndex] };
        if (vote === 'for') {
          proposal.votesFor += weight;
        } else {
          proposal.votesAgainst += weight;
        }
        updated[proposalIndex] = proposal;
        return { ...s, proposals: updated };
      }
      return s;
    });
  }, [state.localPeerId]);

  // Send resource contribution
  const sendResourceContribution = useCallback((resourceType: 'bandwidth' | 'storage' | 'compute', amount: number, unit: string) => {
    if (wsRef.current?.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not open for resource contribution');
      return;
    }
    const contribution: ResourceContribution = {
      peerId: state.localPeerId || 'unknown',
      resourceType,
      amount,
      unit,
      timestamp: Date.now(),
    };
    const message = JSON.stringify({ type: 'send_resource_contribution', data: contribution });
    wsRef.current.send(message);
    console.log('Sent resource contribution:', contribution);

    // Optimistically add to local state
    setState(s => ({
      ...s,
      resourceContributions: [...s.resourceContributions, contribution],
    }));
  }, [state.localPeerId]);

  // Disconnect from WebSocket
  const disconnect = useCallback(() => {
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = undefined;
    }
    reconnectAttemptsRef.current = 0;
    isConnectingRef.current = false;
    if (wsRef.current) {
      wsRef.current.close(1000, 'Client disconnect');
      wsRef.current = null;
    }
  }, []);

  // Reset connection state and retry connecting
  const resetConnection = useCallback(() => {
    reconnectAttemptsRef.current = 0;
    disconnect();
    setTimeout(() => {
      if (isMountedRef.current) {
        connect();
      }
    }, 100);
  }, [disconnect, connect]);

  // Generate graph data from peers
  const graphData = useCallback((): { nodes: GraphNode[]; links: GraphLink[] } => {
    const nodes: GraphNode[] = Array.from(state.peers.values()).map(peer => ({
      id: peer.id,
      name: peer.name,
      reputation: peer.reputation,
      location: peer.location,
      isLocal: peer.id === state.localPeerId,
    }));

    // Create mesh links between peers
    const links: GraphLink[] = [];
    const peerIds = Array.from(state.peers.keys());
    for (let i = 0; i < peerIds.length; i++) {
      for (let j = i + 1; j < peerIds.length; j++) {
        links.push({ source: peerIds[i], target: peerIds[j] });
      }
    }

    return { nodes, links };
  }, [state.peers, state.localPeerId]);

  // Connect on mount, disconnect on unmount
  useEffect(() => {
    isMountedRef.current = true;
    connect();

    return () => {
      isMountedRef.current = false;
      disconnect();
    };
  }, []); // Empty deps - only run on mount/unmount

  return {
    ...state,
    sendChat,
    disconnect,
    resetConnection,
    graphData,
    refreshPeers: fetchPeers,
    // Conversation functions
    setActiveConversation,
    startDMConversation,
    getActiveMessages,
    // Room functions
    joinRoom,
    leaveRoom,
    createRoom,
    // Economics functions
    sendVouch,
    sendCreditLine,
    sendCreditTransfer,
    sendProposal,
    sendVote,
    sendResourceContribution,
  };
}

export default useP2P;
