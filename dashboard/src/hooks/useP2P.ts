import { useState, useEffect, useCallback, useRef } from 'react';
import type { ChatMessage, GraphNode, GraphLink, NormalizedPeer, Location } from '@/types';

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

  const [state, setState] = useState<P2PState>({
    connected: false,
    localPeerId: null,
    peers: new Map(),
    messages: [],
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

      case 'chat_message':
        setState(s => ({
          ...s,
          messages: [...s.messages.slice(-99), (message.data || message) as ChatMessage],
        }));
        break;

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

  // Send chat message
  const sendChat = useCallback((content: string, to?: string) => {
    console.log('sendChat called:', { content, to, readyState: wsRef.current?.readyState });

    if (wsRef.current?.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not open! State:', wsRef.current?.readyState);
      return;
    }

    const message = JSON.stringify({ type: 'send_chat', content, to });
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
        content,
        timestamp: Date.now(),
      };
      return {
        ...s,
        messages: [...s.messages.slice(-99), chatMessage],
      };
    });
  }, []);

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
  };
}

export default useP2P;
