// src/hooks/useOrchestrator.ts
import { useState, useEffect, useCallback, useRef } from 'react';
import type { Workload, NodeHealth, ClusterMetrics } from '@/types';

interface UseOrchestratorOptions {
  apiUrl?: string;
  wsUrl?: string;
  autoConnect?: boolean;
  pollInterval?: number;
}

// Environment configuration
const ENV_WS_URL = import.meta.env.VITE_ORCHESTRATOR_WS_URL || import.meta.env.VITE_WS_URL || 'ws://localhost:9090/api/v1/events';
const ENV_API_URL = import.meta.env.VITE_ORCHESTRATOR_API_URL || import.meta.env.VITE_API_URL || 'http://localhost:9090/api';
const USE_MOCK_DATA = import.meta.env.VITE_USE_MOCK_DATA === 'true' || import.meta.env.VITE_USE_MOCK_DATA === '1';

export function useOrchestrator(options: UseOrchestratorOptions = {}) {
  const apiUrl = options.apiUrl ?? ENV_API_URL;
  const wsUrl = options.wsUrl ?? ENV_WS_URL;
  const autoConnect = options.autoConnect ?? true;
  const pollInterval = options.pollInterval ?? 5000;

  // Store options in refs to avoid dependency cycles
  const apiUrlRef = useRef(apiUrl);
  const wsUrlRef = useRef(wsUrl);
  const pollIntervalRef = useRef(pollInterval);
  const isMountedRef = useRef(true);

  // Update refs when options change
  apiUrlRef.current = apiUrl;
  wsUrlRef.current = wsUrl;
  pollIntervalRef.current = pollInterval;

  // State
  const [workloads, setWorkloads] = useState<Map<string, Workload>>(new Map());
  const [nodes, setNodes] = useState<Map<string, NodeHealth>>(new Map());
  const [clusterMetrics, setClusterMetrics] = useState<ClusterMetrics | null>(null);
  const [connected, setConnected] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Refs for polling
  const pollTimerRef = useRef<number | undefined>(undefined);

  // Generate mock data for development/fallback
  const generateMockData = useCallback(() => {
    const mockNodes = new Map<string, NodeHealth>();
    const mockWorkloads = new Map<string, Workload>();

    // Create mock nodes
    const nodeStatuses: NodeHealth['status'][] = ['healthy', 'healthy', 'healthy', 'degraded', 'healthy'];
    const regions = ['us-east-1', 'us-west-2', 'eu-west-1', 'ap-south-1', 'us-east-2'];

    for (let i = 0; i < 5; i++) {
      const nodeId = `node-${i + 1}-${Math.random().toString(36).slice(2, 10)}`;
      mockNodes.set(nodeId, {
        nodeId,
        nodeName: `worker-node-${i + 1}`,
        status: nodeStatuses[i],
        lastHeartbeat: Date.now() - Math.random() * 60000,
        uptime: Math.floor(Math.random() * 86400 * 7),
        cpu: {
          usage: 20 + Math.random() * 60,
          cores: 8,
          temperature: 45 + Math.random() * 20,
        },
        memory: {
          used: Math.floor(8 * 1024 * 1024 * 1024 * (0.3 + Math.random() * 0.5)),
          total: 16 * 1024 * 1024 * 1024,
          available: Math.floor(16 * 1024 * 1024 * 1024 * (0.2 + Math.random() * 0.3)),
        },
        disk: {
          used: Math.floor(200 * 1024 * 1024 * 1024 * (0.4 + Math.random() * 0.4)),
          total: 500 * 1024 * 1024 * 1024,
          readRate: Math.floor(Math.random() * 100 * 1024 * 1024),
          writeRate: Math.floor(Math.random() * 50 * 1024 * 1024),
        },
        network: {
          bytesIn: Math.floor(Math.random() * 10 * 1024 * 1024 * 1024),
          bytesOut: Math.floor(Math.random() * 5 * 1024 * 1024 * 1024),
          connections: Math.floor(Math.random() * 100),
          latency: Math.floor(5 + Math.random() * 50),
        },
        workloads: {
          running: Math.floor(Math.random() * 5),
          queued: Math.floor(Math.random() * 3),
          completed: Math.floor(Math.random() * 100),
          failed: Math.floor(Math.random() * 5),
        },
        version: '1.2.3',
        region: regions[i],
      });
    }

    // Create mock workloads
    const workloadNames = ['data-processing', 'model-training', 'batch-inference', 'etl-pipeline', 'backup-sync'];
    const statuses: Workload['status'][] = ['running', 'running', 'pending', 'completed', 'failed'];
    const priorities: Workload['priority'][] = ['high', 'critical', 'medium', 'low', 'medium'];

    for (let i = 0; i < 5; i++) {
      const workloadId = `wl-${i + 1}-${Math.random().toString(36).slice(2, 10)}`;
      mockWorkloads.set(workloadId, {
        id: workloadId,
        name: `${workloadNames[i]}-${Math.random().toString(36).slice(2, 6)}`,
        description: `${workloadNames[i].replace('-', ' ')} task`,
        status: statuses[i],
        priority: priorities[i],
        assignedNode: Array.from(mockNodes.keys())[i % mockNodes.size],
        createdAt: Date.now() - Math.random() * 3600000,
        startedAt: statuses[i] !== 'pending' ? Date.now() - Math.random() * 1800000 : undefined,
        completedAt: statuses[i] === 'completed' || statuses[i] === 'failed' ? Date.now() - Math.random() * 600000 : undefined,
        progress: statuses[i] === 'completed' ? 100 : statuses[i] === 'failed' ? Math.floor(Math.random() * 50) : Math.floor(Math.random() * 80),
        resourceRequirements: {
          cpu: 1 + Math.floor(Math.random() * 4),
          memory: (1 + Math.floor(Math.random() * 8)) * 1024 * 1024 * 1024,
          storage: (10 + Math.floor(Math.random() * 100)) * 1024 * 1024 * 1024,
        },
      });
    }

    // Create mock cluster metrics
    const nodesList = Array.from(mockNodes.values());
    const workloadsList = Array.from(mockWorkloads.values());

    const mockMetrics: ClusterMetrics = {
      clusterId: 'cluster-primary',
      clusterName: 'Mycelial Primary Cluster',
      totalNodes: nodesList.length,
      healthyNodes: nodesList.filter(n => n.status === 'healthy').length,
      degradedNodes: nodesList.filter(n => n.status === 'degraded').length,
      offlineNodes: nodesList.filter(n => n.status === 'offline').length,
      totalWorkloads: workloadsList.length,
      runningWorkloads: workloadsList.filter(w => w.status === 'running').length,
      pendingWorkloads: workloadsList.filter(w => w.status === 'pending').length,
      completedWorkloads: workloadsList.filter(w => w.status === 'completed').length,
      failedWorkloads: workloadsList.filter(w => w.status === 'failed').length,
      resources: {
        totalCpu: nodesList.reduce((acc, n) => acc + n.cpu.cores, 0),
        usedCpu: nodesList.reduce((acc, n) => acc + (n.cpu.cores * n.cpu.usage / 100), 0),
        totalMemory: nodesList.reduce((acc, n) => acc + n.memory.total, 0),
        usedMemory: nodesList.reduce((acc, n) => acc + n.memory.used, 0),
        totalStorage: nodesList.reduce((acc, n) => acc + n.disk.total, 0),
        usedStorage: nodesList.reduce((acc, n) => acc + n.disk.used, 0),
      },
      throughput: {
        workloadsPerHour: 45,
        avgCompletionTime: 127000,
        successRate: 94.5,
      },
      lastUpdated: Date.now(),
    };

    return { mockNodes, mockWorkloads, mockMetrics };
  }, []);

  // Fetch data from API
  const fetchData = useCallback(async () => {
    if (!isMountedRef.current) return;

    // If mock mode is explicitly enabled, use mock data without error
    if (USE_MOCK_DATA) {
      const { mockNodes, mockWorkloads, mockMetrics } = generateMockData();
      setNodes(mockNodes);
      setWorkloads(mockWorkloads);
      setClusterMetrics(mockMetrics);
      setConnected(true); // Show as "connected" in mock mode
      setError(null);
      setLoading(false);
      return;
    }

    try {
      setLoading(true);

      // Try to fetch from real API
      const [nodesRes, workloadsRes, statusRes] = await Promise.allSettled([
        fetch(`${apiUrlRef.current}/v1/nodes`),
        fetch(`${apiUrlRef.current}/v1/workloads`),
        fetch(`${apiUrlRef.current}/v1/cluster/status`),
      ]);

      if (!isMountedRef.current) return;

      let hasRealData = false;

      // Process nodes
      if (nodesRes.status === 'fulfilled' && nodesRes.value.ok) {
        const data = await nodesRes.value.json();
        const nodesArray = data.nodes || data || [];
        const nodesMap = new Map<string, NodeHealth>();
        for (const node of nodesArray) {
          nodesMap.set(node.nodeId || node.id, node);
        }
        setNodes(nodesMap);
        hasRealData = true;
      }

      // Process workloads
      if (workloadsRes.status === 'fulfilled' && workloadsRes.value.ok) {
        const data = await workloadsRes.value.json();
        const workloadsArray = data.workloads || data || [];
        const workloadsMap = new Map<string, Workload>();
        for (const workload of workloadsArray) {
          workloadsMap.set(workload.id, workload);
        }
        setWorkloads(workloadsMap);
        hasRealData = true;
      }

      // Process cluster status
      if (statusRes.status === 'fulfilled' && statusRes.value.ok) {
        const data = await statusRes.value.json();
        setClusterMetrics(data);
        hasRealData = true;
      }

      if (hasRealData) {
        setConnected(true);
        setError(null);
      } else {
        // Fall back to mock data
        const { mockNodes, mockWorkloads, mockMetrics } = generateMockData();
        setNodes(mockNodes);
        setWorkloads(mockWorkloads);
        setClusterMetrics(mockMetrics);
        setConnected(false);
        setError('Using mock data - API unavailable');
      }
    } catch (err) {
      if (!isMountedRef.current) return;

      console.error('Failed to fetch orchestrator data:', err);

      // Fall back to mock data
      const { mockNodes, mockWorkloads, mockMetrics } = generateMockData();
      setNodes(mockNodes);
      setWorkloads(mockWorkloads);
      setClusterMetrics(mockMetrics);
      setConnected(false);
      setError(err instanceof Error ? err.message : 'Connection failed');
    } finally {
      if (isMountedRef.current) {
        setLoading(false);
      }
    }
  }, [generateMockData]);

  // Start polling
  const startPolling = useCallback(() => {
    if (pollTimerRef.current) {
      clearInterval(pollTimerRef.current);
    }
    pollTimerRef.current = window.setInterval(() => {
      if (isMountedRef.current) {
        fetchData();
      }
    }, pollIntervalRef.current);
  }, [fetchData]);

  // Stop polling
  const stopPolling = useCallback(() => {
    if (pollTimerRef.current) {
      clearInterval(pollTimerRef.current);
      pollTimerRef.current = undefined;
    }
  }, []);

  // Cancel a workload
  const cancelWorkload = useCallback(async (workloadId: string) => {
    try {
      const response = await fetch(`${apiUrlRef.current}/v1/workloads/${workloadId}/cancel`, {
        method: 'POST',
      });
      if (response.ok) {
        setWorkloads(prev => {
          const newMap = new Map(prev);
          const workload = newMap.get(workloadId);
          if (workload) {
            newMap.set(workloadId, { ...workload, status: 'cancelled' });
          }
          return newMap;
        });
      }
    } catch (err) {
      // Optimistic update for mock mode
      setWorkloads(prev => {
        const newMap = new Map(prev);
        const workload = newMap.get(workloadId);
        if (workload) {
          newMap.set(workloadId, { ...workload, status: 'cancelled' });
        }
        return newMap;
      });
    }
  }, []);

  // Retry a failed workload
  const retryWorkload = useCallback(async (workloadId: string) => {
    try {
      const response = await fetch(`${apiUrlRef.current}/v1/workloads/${workloadId}/retry`, {
        method: 'POST',
      });
      if (response.ok) {
        setWorkloads(prev => {
          const newMap = new Map(prev);
          const workload = newMap.get(workloadId);
          if (workload) {
            newMap.set(workloadId, { ...workload, status: 'pending', progress: 0 });
          }
          return newMap;
        });
      }
    } catch (err) {
      // Optimistic update for mock mode
      setWorkloads(prev => {
        const newMap = new Map(prev);
        const workload = newMap.get(workloadId);
        if (workload) {
          newMap.set(workloadId, { ...workload, status: 'pending', progress: 0 });
        }
        return newMap;
      });
    }
  }, []);

  // Refresh data manually
  const refreshData = useCallback(() => {
    return fetchData();
  }, [fetchData]);

  // Clear error
  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // Initialize on mount
  useEffect(() => {
    isMountedRef.current = true;

    if (autoConnect) {
      fetchData();
      startPolling();
    }

    return () => {
      isMountedRef.current = false;
      stopPolling();
    };
  }, []); // Empty deps - only run on mount/unmount

  return {
    workloads,
    nodes,
    clusterMetrics,
    connected,
    loading,
    error,
    cancelWorkload,
    retryWorkload,
    refreshData,
    clearError,
  };
}

export default useOrchestrator;
