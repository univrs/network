// src/hooks/useEconomicsAPI.ts
// Hook for fetching economics data from REST API endpoints

import { useState, useEffect, useCallback, useRef } from 'react';
import type {
  CreditLine,
  Proposal,
  VouchRequest,
  ResourceContribution,
  EconomicsSummary,
  NodeEnrState,
  GradientUpdate,
  Election,
} from '@/types';

interface UseEconomicsAPIOptions {
  apiUrl?: string;
  autoFetch?: boolean;
  pollInterval?: number;
}

// Environment configuration - uses same port as P2P node
const ENV_API_URL = import.meta.env.VITE_P2P_API_URL || 'http://localhost:8080';
const USE_MOCK_DATA = import.meta.env.VITE_USE_MOCK_DATA === 'true' || import.meta.env.VITE_USE_MOCK_DATA === '1';

export interface EconomicsState {
  creditLines: CreditLine[];
  proposals: Proposal[];
  vouches: VouchRequest[];
  resourceContributions: ResourceContribution[];
  nodeEnrStates: Map<string, NodeEnrState>;
  gradients: Map<string, GradientUpdate>;
  elections: Map<number, Election>;
  summary: EconomicsSummary | null;
}

export function useEconomicsAPI(options: UseEconomicsAPIOptions = {}) {
  const apiUrl = options.apiUrl ?? ENV_API_URL;
  const autoFetch = options.autoFetch ?? true;
  const pollInterval = options.pollInterval ?? 10000;

  // Refs to avoid dependency cycles
  const apiUrlRef = useRef(apiUrl);
  const pollIntervalRef = useRef(pollInterval);
  const isMountedRef = useRef(true);

  // Update refs when options change
  apiUrlRef.current = apiUrl;
  pollIntervalRef.current = pollInterval;

  // State
  const [creditLines, setCreditLines] = useState<CreditLine[]>([]);
  const [proposals, setProposals] = useState<Proposal[]>([]);
  const [vouches, setVouches] = useState<VouchRequest[]>([]);
  const [resourceContributions, setResourceContributions] = useState<ResourceContribution[]>([]);
  const [nodeEnrStates, setNodeEnrStates] = useState<Map<string, NodeEnrState>>(new Map());
  const [gradients, setGradients] = useState<Map<string, GradientUpdate>>(new Map());
  const [elections, setElections] = useState<Map<number, Election>>(new Map());
  const [summary, setSummary] = useState<EconomicsSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Refs for polling
  const pollTimerRef = useRef<number | undefined>(undefined);

  // Generate mock data for development
  const generateMockData = useCallback((): EconomicsState => {
    const mockCreditLines: CreditLine[] = [
      {
        id: 'cl-1',
        peerId1: 'peer-alice',
        peerId2: 'peer-bob',
        limit: 1000,
        balance: 250,
        createdAt: Date.now() - 86400000 * 7,
        lastTransaction: Date.now() - 3600000,
      },
      {
        id: 'cl-2',
        peerId1: 'peer-alice',
        peerId2: 'peer-charlie',
        limit: 500,
        balance: -100,
        createdAt: Date.now() - 86400000 * 3,
      },
    ];

    const mockProposals: Proposal[] = [
      {
        id: 'prop-1',
        title: 'Increase base credit limit',
        description: 'Proposal to increase the default credit limit from 500 to 1000 units',
        proposer: 'peer-alice',
        createdAt: Date.now() - 86400000 * 2,
        expiresAt: Date.now() + 86400000 * 5,
        status: 'active',
        votesFor: 12,
        votesAgainst: 3,
        quorum: 20,
      },
      {
        id: 'prop-2',
        title: 'Add new resource type: GPU',
        description: 'Allow tracking and sharing of GPU compute resources',
        proposer: 'peer-bob',
        createdAt: Date.now() - 86400000 * 5,
        expiresAt: Date.now() - 86400000,
        status: 'passed',
        votesFor: 25,
        votesAgainst: 5,
        quorum: 20,
      },
    ];

    const mockVouches: VouchRequest[] = [
      {
        fromPeerId: 'peer-alice',
        toPeerId: 'peer-dave',
        stake: 0.5,
        message: 'Dave is a reliable node operator',
        timestamp: Date.now() - 3600000 * 2,
      },
    ];

    const mockContributions: ResourceContribution[] = [
      {
        peerId: 'peer-alice',
        resourceType: 'bandwidth',
        amount: 1024 * 1024 * 100,
        unit: 'bytes',
        timestamp: Date.now() - 3600000,
      },
      {
        peerId: 'peer-bob',
        resourceType: 'storage',
        amount: 1024 * 1024 * 1024 * 10,
        unit: 'bytes',
        timestamp: Date.now() - 7200000,
      },
    ];

    const mockSummary: EconomicsSummary = {
      totalCreditLines: mockCreditLines.length,
      totalCreditLimit: mockCreditLines.reduce((acc, cl) => acc + cl.limit, 0),
      totalCreditBalance: mockCreditLines.reduce((acc, cl) => acc + Math.abs(cl.balance), 0),
      activeProposals: mockProposals.filter(p => p.status === 'active').length,
      totalVouches: mockVouches.length,
      totalResourceContributions: mockContributions.length,
      enrTotalBalance: 10000,
      activeElections: 0,
    };

    return {
      creditLines: mockCreditLines,
      proposals: mockProposals,
      vouches: mockVouches,
      resourceContributions: mockContributions,
      nodeEnrStates: new Map(),
      gradients: new Map(),
      elections: new Map(),
      summary: mockSummary,
    };
  }, []);

  // Fetch all economics data from API
  const fetchData = useCallback(async () => {
    if (!isMountedRef.current) return;

    // If mock mode is enabled, use mock data
    if (USE_MOCK_DATA) {
      const mock = generateMockData();
      setCreditLines(mock.creditLines);
      setProposals(mock.proposals);
      setVouches(mock.vouches);
      setResourceContributions(mock.resourceContributions);
      setNodeEnrStates(mock.nodeEnrStates);
      setGradients(mock.gradients);
      setElections(mock.elections);
      setSummary(mock.summary);
      setError(null);
      setLoading(false);
      return;
    }

    try {
      setLoading(true);

      // Try to fetch from real API endpoints
      const [
        creditLinesRes,
        proposalsRes,
        vouchesRes,
        resourcesRes,
        summaryRes,
      ] = await Promise.allSettled([
        fetch(`${apiUrlRef.current}/api/economics/credit-lines`),
        fetch(`${apiUrlRef.current}/api/economics/proposals`),
        fetch(`${apiUrlRef.current}/api/economics/vouches`),
        fetch(`${apiUrlRef.current}/api/economics/resources`),
        fetch(`${apiUrlRef.current}/api/economics`),
      ]);

      if (!isMountedRef.current) return;

      let hasRealData = false;

      // Process credit lines
      if (creditLinesRes.status === 'fulfilled' && creditLinesRes.value.ok) {
        const data = await creditLinesRes.value.json();
        const lines = data.credit_lines || data.creditLines || data || [];
        setCreditLines(lines.map((line: Record<string, unknown>) => ({
          id: line.id as string,
          peerId1: (line.creditor || line.peerId1) as string,
          peerId2: (line.debtor || line.peerId2) as string,
          limit: line.limit as number,
          balance: line.balance as number,
          createdAt: line.created_at || line.createdAt || line.timestamp,
          lastTransaction: line.last_transaction || line.lastTransaction,
        })));
        hasRealData = true;
      }

      // Process proposals
      if (proposalsRes.status === 'fulfilled' && proposalsRes.value.ok) {
        const data = await proposalsRes.value.json();
        const props = data.proposals || data || [];
        setProposals(props.map((p: Record<string, unknown>) => ({
          id: p.id as string,
          title: p.title as string,
          description: p.description as string,
          proposer: p.proposer as string,
          createdAt: p.created_at || p.createdAt || p.timestamp,
          expiresAt: p.deadline || p.expires_at || p.expiresAt,
          status: p.status as Proposal['status'],
          votesFor: p.yes_votes || p.votesFor || 0,
          votesAgainst: p.no_votes || p.votesAgainst || 0,
          quorum: p.quorum || 0,
        })));
        hasRealData = true;
      }

      // Process vouches
      if (vouchesRes.status === 'fulfilled' && vouchesRes.value.ok) {
        const data = await vouchesRes.value.json();
        const v = data.vouches || data || [];
        setVouches(v.map((vouch: Record<string, unknown>) => ({
          fromPeerId: (vouch.voucher || vouch.fromPeerId) as string,
          toPeerId: (vouch.vouchee || vouch.toPeerId) as string,
          stake: (vouch.weight || vouch.stake) as number,
          message: vouch.message as string | undefined,
          timestamp: vouch.timestamp as number,
        })));
        hasRealData = true;
      }

      // Process resource contributions
      if (resourcesRes.status === 'fulfilled' && resourcesRes.value.ok) {
        const data = await resourcesRes.value.json();
        const resources = data.contributions || data.resources || data || [];
        setResourceContributions(resources.map((r: Record<string, unknown>) => ({
          peerId: (r.peer_id || r.peerId) as string,
          resourceType: (r.resource_type || r.resourceType) as ResourceContribution['resourceType'],
          amount: r.amount as number,
          unit: r.unit as string,
          timestamp: r.timestamp as number,
        })));
        hasRealData = true;
      }

      // Process summary
      if (summaryRes.status === 'fulfilled' && summaryRes.value.ok) {
        const data = await summaryRes.value.json();
        setSummary({
          totalCreditLines: data.total_credit_lines || data.totalCreditLines || 0,
          totalCreditLimit: data.total_credit_limit || data.totalCreditLimit || 0,
          totalCreditBalance: data.total_credit_balance || data.totalCreditBalance || 0,
          activeProposals: data.active_proposals || data.activeProposals || 0,
          totalVouches: data.total_vouches || data.totalVouches || 0,
          totalResourceContributions: data.total_resource_contributions || data.totalResourceContributions || 0,
          enrTotalBalance: data.enr_total_balance || data.enrTotalBalance || 0,
          activeElections: data.active_elections || data.activeElections || 0,
        });
        hasRealData = true;
      }

      if (hasRealData) {
        setError(null);
      } else {
        // Fall back to mock data if API not available
        const mock = generateMockData();
        setCreditLines(mock.creditLines);
        setProposals(mock.proposals);
        setVouches(mock.vouches);
        setResourceContributions(mock.resourceContributions);
        setSummary(mock.summary);
        setError('Economics API not available - using mock data');
      }
    } catch (err) {
      if (!isMountedRef.current) return;

      console.error('Failed to fetch economics data:', err);

      // Fall back to mock data
      const mock = generateMockData();
      setCreditLines(mock.creditLines);
      setProposals(mock.proposals);
      setVouches(mock.vouches);
      setResourceContributions(mock.resourceContributions);
      setSummary(mock.summary);
      setError(err instanceof Error ? err.message : 'Connection failed');
    } finally {
      if (isMountedRef.current) {
        setLoading(false);
      }
    }
  }, [generateMockData]);

  // Fetch economics data for a specific peer
  const fetchPeerEconomics = useCallback(async (peerId: string) => {
    try {
      const response = await fetch(`${apiUrlRef.current}/api/economics/peer/${peerId}`);
      if (response.ok) {
        return await response.json();
      }
      return null;
    } catch (err) {
      console.error('Failed to fetch peer economics:', err);
      return null;
    }
  }, []);

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

  // Refresh data manually
  const refreshData = useCallback(() => {
    return fetchData();
  }, [fetchData]);

  // Clear error
  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // Update ENR state for a node (called from WebSocket updates)
  const updateNodeEnrState = useCallback((nodeId: string, update: Partial<NodeEnrState>) => {
    setNodeEnrStates(prev => {
      const newMap = new Map(prev);
      const existing = newMap.get(nodeId) || {
        nodeId,
        balance: 0,
        septalState: 'closed' as const,
        septalHealthy: true,
        failureCount: 0,
        lastUpdated: Date.now(),
      };
      newMap.set(nodeId, { ...existing, ...update, lastUpdated: Date.now() });
      return newMap;
    });
  }, []);

  // Update gradient for a node
  const updateGradient = useCallback((update: GradientUpdate) => {
    setGradients(prev => {
      const newMap = new Map(prev);
      newMap.set(update.source, update);
      return newMap;
    });
  }, []);

  // Update election state
  const updateElection = useCallback((electionId: number, update: Partial<Election>) => {
    setElections(prev => {
      const newMap = new Map(prev);
      const existing = newMap.get(electionId);
      if (existing) {
        newMap.set(electionId, { ...existing, ...update });
      } else if (update.regionId && update.initiator) {
        newMap.set(electionId, {
          id: electionId,
          regionId: update.regionId,
          initiator: update.initiator,
          status: 'announced',
          candidates: [],
          votes: [],
          startedAt: Date.now(),
          ...update,
        });
      }
      return newMap;
    });
  }, []);

  // Initialize on mount
  useEffect(() => {
    isMountedRef.current = true;

    if (autoFetch) {
      fetchData();
      startPolling();
    }

    return () => {
      isMountedRef.current = false;
      stopPolling();
    };
  }, []); // Empty deps - only run on mount/unmount

  return {
    // State
    creditLines,
    proposals,
    vouches,
    resourceContributions,
    nodeEnrStates,
    gradients,
    elections,
    summary,
    loading,
    error,

    // Actions
    fetchPeerEconomics,
    refreshData,
    clearError,

    // ENR state update methods (for WebSocket integration)
    updateNodeEnrState,
    updateGradient,
    updateElection,
  };
}

export default useEconomicsAPI;
