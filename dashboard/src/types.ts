// Peer types - flexible to handle both backend formats
export interface Location {
  type?: 'geographic' | 'logical' | 'approximate';
  latitude?: number;
  longitude?: number;
  region?: string;
  country_code?: string;
  city?: string;
}

export interface Reputation {
  score: number;
  contributions?: number;
  interactions?: number;
  tier?: 'excellent' | 'good' | 'neutral' | 'poor' | 'untrusted';
}

// Flexible PeerInfo that handles both REST and WebSocket formats
export interface PeerInfo {
  // REST API format
  id?: string;
  name?: string;
  // WebSocket format
  peer_id?: string;
  display_name?: string;
  // Common
  location?: Location;
  reputation: number | Reputation;
  addresses?: string[];
  created_at?: number;
  last_seen?: number;
}

export interface ChatMessage {
  id: string;
  from: string;
  from_name?: string;
  to?: string;
  content: string;
  timestamp: number;
}

// WebSocket message types
export interface WsMessage {
  type: string;
  [key: string]: any;
}

// Graph data types
export interface GraphNode {
  id: string;
  name: string;
  reputation: number;
  location?: Location;
  isLocal: boolean;
}

export interface GraphLink {
  source: string;
  target: string;
}

// Normalized peer for internal use
export interface NormalizedPeer {
  id: string;
  name: string;
  reputation: number;
  location?: Location;
  addresses: string[];
}