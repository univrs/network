import { useEffect, useRef, useState, useCallback } from 'react';
import type { ChatMessage, GraphNode, GraphLink, NormalizedPeer } from '../types';

interface UseP2POptions {
  url?: string;
  reconnectInterval?: number;
  apiUrl?: string;
}

interface P2PState {
  connected: boolean;
  localPeerId: string | null;
  peers: Map<string, NormalizedPeer>;
  messages: ChatMessage[];
}

// Normalize peer data from different backend formats
function normalizePeer(peer: any): NormalizedPeer {
  const id = peer.id || peer.peer_id || '';
  const name = peer.name || peer.display_name || `Peer-${id.slice(0, 12)}`;
  const reputation = typeof peer.reputation === 'number' 
    ? peer.reputation 
    : (peer.reputation?.score ?? 0.5);
  
  return {
    id,
    name,
    reputation,
    location: peer.location,
    addresses: peer.addresses || [],
  };
}

export function useP2P(options: UseP2POptions = {}) {
  const {
    url = import.meta.env.VITE_WS_URL || 'ws://localhost:8080/ws',
    apiUrl = import.meta.env.VITE_API_URL || 'http://localhost:8080',
    reconnectInterval = 3000,
  } = options;

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<number>();

  const [state, setState] = useState<P2PState>({
    connected: false,
    localPeerId: null,
    peers: new Map(),
    messages: [],
  });

  // Fetch peers via REST API (more reliable than WebSocket for initial load)
  const fetchPeers = useCallback(async () => {
    try {
      console.log('Fetching peers from:', `${apiUrl}/api/peers`);
      const response = await fetch(`${apiUrl}/api/peers`);
      const peers = await response.json();
      console.log('REST API returned', peers.length, 'peers');
      
      setState(s => {
        const newPeers = new Map<string, NormalizedPeer>();
        for (const peer of peers) {
          const normalized = normalizePeer(peer);
          if (normalized.id) {
            newPeers.set(normalized.id, normalized);
          }
        }
        console.log('Normalized peers:', newPeers.size);
        return { ...s, peers: newPeers };
      });
    } catch (e) {
      console.error('Failed to fetch peers:', e);
    }
  }, [apiUrl]);

  // Fetch local peer info
  const fetchInfo = useCallback(async () => {
    try {
      const response = await fetch(`${apiUrl}/api/info`);
      const info = await response.json();
      console.log('Local node info:', info);
      setState(s => ({ ...s, localPeerId: info.peer_id }));
    } catch (e) {
      console.error('Failed to fetch info:', e);
    }
  }, [apiUrl]);

  const handleMessage = useCallback((message: any) => {
    console.log('WS Message:', message);
    
    switch (message.type) {
      case 'peers_list': {
        const peers = message.peers || message.data?.peers || [];
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
        const peerId = message.peer_id || message.data?.peer_id;
        const peerInfo = message.peer_info || message.data?.peer_info || message;
        if (peerId) {
          setState(s => {
            const newPeers = new Map(s.peers);
            const normalized = normalizePeer({ ...peerInfo, id: peerId });
            newPeers.set(peerId, normalized);
            return { ...s, peers: newPeers };
          });
        }
        break;
      }

      case 'peer_left': {
        const peerId = message.peer_id || message.data?.peer_id;
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
          messages: [...s.messages.slice(-99), message.data || message],
        }));
        break;

      default:
        console.log('Unhandled message type:', message.type);
    }
  }, []);

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    console.log('Connecting to WebSocket:', url);
    const ws = new WebSocket(url);

    ws.onopen = () => {
      console.log('WebSocket connected!');
      setState(s => ({ ...s, connected: true }));
      // Fetch data via REST (more reliable)
      fetchInfo();
      fetchPeers();
    };

    ws.onclose = () => {
      console.log('WebSocket disconnected');
      setState(s => ({ ...s, connected: false }));
      reconnectTimerRef.current = window.setTimeout(connect, reconnectInterval);
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
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
  }, [url, reconnectInterval, handleMessage, fetchPeers, fetchInfo]);

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

  const disconnect = useCallback(() => {
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
    }
    wsRef.current?.close();
  }, []);

  useEffect(() => {
    connect();
    return () => disconnect();
  }, [connect, disconnect]);

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

  return {
    ...state,
    sendChat,
    disconnect,
    graphData,
    refreshPeers: fetchPeers,
  };
}