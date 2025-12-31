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
  room_id?: string;
  content: string;
  timestamp: number;
}

// Conversation types for enhanced chat
export type ConversationType = 'community' | 'dm' | 'room';

export interface Conversation {
  id: string;
  type: ConversationType;
  name: string;
  peerId?: string; // For DMs - the other peer's ID
  roomId?: string; // For rooms - the room ID
  lastMessage?: ChatMessage;
  unreadCount: number;
  createdAt: number;
}

export interface Room {
  id: string;
  name: string;
  description?: string;
  topic: string; // Gossipsub topic
  members: string[]; // Peer IDs
  createdBy: string;
  createdAt: number;
  isPublic: boolean;
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

// Phase 6: Onboarding types
export interface GeneratedIdentity {
  peerId: string;
  publicKey: string;
  privateKey: string;
  createdAt: number;
}

export interface InviteLink {
  bootstrapAddress: string;
  inviteCode: string;
  expiresAt?: number;
  createdBy?: string;
}

export interface OnboardingStep {
  id: string;
  title: string;
  description: string;
  completed: boolean;
}

// Phase 6: Reputation types
export interface ReputationScore {
  score: number;
  tier: 'excellent' | 'good' | 'neutral' | 'poor' | 'untrusted';
  contributions: number;
  interactions: number;
  vouches: number;
  lastUpdated: number;
}

export interface VouchRequest {
  fromPeerId: string;
  toPeerId: string;
  message?: string;
  stake: number;
  timestamp: number;
}

// Phase 6: Mutual Credit types
export interface CreditLine {
  id: string;
  peerId1: string;
  peerId2: string;
  limit: number;
  balance: number;
  createdAt: number;
  lastTransaction?: number;
}

export interface CreditTransfer {
  id: string;
  from: string;
  to: string;
  amount: number;
  memo?: string;
  timestamp: number;
}

// Phase 6: Governance types
export interface Proposal {
  id: string;
  title: string;
  description: string;
  proposer: string;
  createdAt: number;
  expiresAt: number;
  status: 'active' | 'passed' | 'rejected' | 'expired';
  votesFor: number;
  votesAgainst: number;
  quorum: number;
}

export interface Vote {
  proposalId: string;
  voterId: string;
  vote: 'for' | 'against' | 'abstain';
  weight: number;
  timestamp: number;
}

// Phase 6.4: Resource Sharing types
export interface ResourceContribution {
  peerId: string;
  resourceType: 'bandwidth' | 'storage' | 'compute';
  amount: number;
  unit: string;
  timestamp: number;
}

export interface ResourceMetrics {
  peerId: string;
  bandwidth: {
    uploaded: number;  // bytes
    downloaded: number;
    uploadRate: number;  // bytes/sec
    downloadRate: number;
  };
  storage: {
    provided: number;  // bytes
    used: number;
    available: number;
  };
  compute: {
    tasksCompleted: number;
    averageLatency: number;  // ms
    cpuContributed: number;  // core-hours
  };
  uptime: number;  // seconds
  lastUpdated: number;
}

export interface ResourcePool {
  totalBandwidth: number;
  totalStorage: number;
  totalCompute: number;
  activeContributors: number;
  topContributors: {
    peerId: string;
    peerName: string;
    contribution: number;
    resourceType: string;
  }[];
}

// Orchestrator Types for WorkloadList, NodeStatus, ClusterOverview

export type WorkloadStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
export type WorkloadPriority = 'low' | 'medium' | 'high' | 'critical';

// Container resource requests from API
export interface ContainerResourceRequests {
  cpu_cores?: number;
  memory_mb?: number;
  disk_mb?: number;
}

export interface WorkloadContainer {
  name: string;
  image: string;
  resource_requests?: ContainerResourceRequests;
}

export interface Workload {
  id: string;
  name: string;
  description?: string;
  status: WorkloadStatus;
  priority: WorkloadPriority;
  assignedNode?: string;
  createdAt: number;
  startedAt?: number;
  completedAt?: number;
  progress: number; // 0-100
  // Legacy mock format
  resourceRequirements?: {
    cpu: number;
    memory: number; // bytes
    storage: number; // bytes
  };
  // Real API format
  replicas?: number;
  containers?: WorkloadContainer[];
  metadata?: Record<string, unknown>;
}

export type NodeHealthStatus = 'healthy' | 'degraded' | 'unhealthy' | 'offline' | 'Ready' | 'NotReady';

// Resource capacity/allocatable from real API
export interface ResourceCapacity {
  cpu_cores: number;
  memory_mb: number;
  disk_mb: number;
}

// Real API node format
export interface ApiNode {
  id: string;
  address: string;
  status: 'Ready' | 'NotReady';
  resources_capacity: ResourceCapacity;
  resources_allocatable: ResourceCapacity;
}

// Normalized NodeHealth that works with both mock and real API
export interface NodeHealth {
  // Core fields (required)
  nodeId: string;
  nodeName: string;
  status: NodeHealthStatus;

  // Real API fields (from orchestrator)
  address?: string;
  resources_capacity?: ResourceCapacity;
  resources_allocatable?: ResourceCapacity;

  // Legacy mock fields (optional for backwards compatibility)
  lastHeartbeat?: number;
  uptime?: number; // seconds
  cpu?: {
    usage: number; // percentage 0-100
    cores: number;
    temperature?: number;
  };
  memory?: {
    used: number; // bytes
    total: number;
    available: number;
  };
  disk?: {
    used: number;
    total: number;
    readRate: number; // bytes/sec
    writeRate: number;
  };
  network?: {
    bytesIn: number;
    bytesOut: number;
    connections: number;
    latency: number; // ms
  };
  workloads?: {
    running: number;
    queued: number;
    completed: number;
    failed: number;
  };
  version?: string;
  region?: string;
}

// Helper to convert API node to NodeHealth
export function apiNodeToNodeHealth(node: ApiNode): NodeHealth {
  return {
    nodeId: node.id,
    nodeName: node.id.slice(0, 8), // Use first 8 chars of ID as name
    status: node.status,
    address: node.address,
    resources_capacity: node.resources_capacity,
    resources_allocatable: node.resources_allocatable,
  };
}

export interface ClusterMetrics {
  clusterId: string;
  clusterName: string;
  totalNodes: number;
  healthyNodes: number;
  degradedNodes: number;
  offlineNodes: number;
  totalWorkloads: number;
  runningWorkloads: number;
  pendingWorkloads: number;
  completedWorkloads: number;
  failedWorkloads: number;
  resources: {
    totalCpu: number;
    usedCpu: number;
    totalMemory: number;
    usedMemory: number;
    totalStorage: number;
    usedStorage: number;
  };
  throughput: {
    workloadsPerHour: number;
    avgCompletionTime: number; // ms
    successRate: number; // percentage
  };
  lastUpdated: number;
}

// WebSocket event types for orchestrator
export interface OrchestratorEvent {
  type:
    | 'workload_created'
    | 'workload_updated'
    | 'workload_completed'
    | 'workload_failed'
    | 'node_status'
    | 'node_joined'
    | 'node_left'
    | 'cluster_metrics'
    | 'workload_list'
    | 'node_list';
  data: unknown;
  timestamp: number;
}

// ============ ENR Bridge Types (Phase 3) ============

// Resource gradient update from a node
export interface GradientUpdate {
  source: string;
  cpuAvailable: number;
  memoryAvailable: number;
  bandwidthAvailable: number;
  storageAvailable: number;
  timestamp: number;
}

// ENR credit transfer (different from mutual credit)
export interface EnrCreditTransfer {
  from: string;
  to: string;
  amount: number;
  tax: number;
  nonce: number;
  timestamp: number;
}

// ENR balance update
export interface EnrBalanceUpdate {
  nodeId: string;
  balance: number;
  timestamp: number;
}

// Nexus election announcement
export interface ElectionAnnouncement {
  electionId: number;
  initiator: string;
  regionId: string;
  timestamp: number;
}

// Nexus election candidacy
export interface ElectionCandidacy {
  electionId: number;
  candidate: string;
  uptime: number;
  cpuAvailable: number;
  memoryAvailable: number;
  reputation: number;
  timestamp: number;
}

// Nexus election vote
export interface ElectionVote {
  electionId: number;
  voter: string;
  candidate: string;
  timestamp: number;
}

// Nexus election result
export interface ElectionResult {
  electionId: number;
  winner: string;
  regionId: string;
  voteCount: number;
  timestamp: number;
}

// Septal gate state (circuit breaker)
export type SeptalState = 'closed' | 'open' | 'half_open';

// Septal gate state change
export interface SeptalStateChange {
  nodeId: string;
  fromState: SeptalState;
  toState: SeptalState;
  reason: string;
  timestamp: number;
}

// Septal health probe response
export interface SeptalHealthStatus {
  nodeId: string;
  isHealthy: boolean;
  failureCount: number;
  timestamp: number;
}

// Combined election state for tracking active elections
export interface Election {
  id: number;
  regionId: string;
  initiator: string;
  status: 'announced' | 'voting' | 'completed';
  candidates: ElectionCandidacy[];
  votes: ElectionVote[];
  winner?: string;
  voteCount?: number;
  startedAt: number;
  completedAt?: number;
}

// Per-node ENR state
export interface NodeEnrState {
  nodeId: string;
  balance: number;
  gradient?: GradientUpdate;
  septalState: SeptalState;
  septalHealthy: boolean;
  failureCount: number;
  lastUpdated: number;
}

// Economics summary for the whole network
export interface EconomicsSummary {
  totalCreditLines: number;
  totalCreditLimit: number;
  totalCreditBalance: number;
  activeProposals: number;
  totalVouches: number;
  totalResourceContributions: number;
  enrTotalBalance: number;
  activeElections: number;
}