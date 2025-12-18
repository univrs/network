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

export type NodeHealthStatus = 'healthy' | 'degraded' | 'unhealthy' | 'offline';

export interface NodeHealth {
  nodeId: string;
  nodeName: string;
  status: NodeHealthStatus;
  lastHeartbeat: number;
  uptime: number; // seconds
  cpu: {
    usage: number; // percentage 0-100
    cores: number;
    temperature?: number;
  };
  memory: {
    used: number; // bytes
    total: number;
    available: number;
  };
  disk: {
    used: number;
    total: number;
    readRate: number; // bytes/sec
    writeRate: number;
  };
  network: {
    bytesIn: number;
    bytesOut: number;
    connections: number;
    latency: number; // ms
  };
  workloads: {
    running: number;
    queued: number;
    completed: number;
    failed: number;
  };
  version: string;
  region?: string;
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