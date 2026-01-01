// src/components/SeptalPanel.tsx
// Displays septal gate (circuit breaker) states across the network

import { useMemo, useState, useEffect } from 'react';
import type { NodeEnrState, SeptalState } from '@/types';

// Tooltip descriptions for each circuit breaker state
const SEPTAL_STATE_TOOLTIPS: Record<SeptalState, string> = {
  closed: 'Circuit is closed and operating normally. All requests are allowed through.',
  half_open: 'Circuit is testing recovery. Limited requests allowed to check if the service has recovered.',
  open: 'Circuit is tripped due to failures. Requests are blocked to prevent cascade failures.',
};

interface SeptalPanelProps {
  nodeEnrStates: Map<string, NodeEnrState>;
  onClose?: () => void;
}

function shortenNodeId(id: string): string {
  if (id.length <= 12) return id;
  return `${id.slice(0, 6)}...${id.slice(-4)}`;
}

function formatTimestamp(ts: number): string {
  const seconds = Math.floor((Date.now() - ts) / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  return `${Math.floor(seconds / 3600)}h ago`;
}

function getSeptalStateConfig(state: SeptalState): {
  color: string;
  bgColor: string;
  icon: JSX.Element;
  label: string;
  description: string;
} {
  switch (state) {
    case 'closed':
      return {
        color: 'text-glow-cyan',
        bgColor: 'bg-glow-cyan/20',
        icon: (
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        ),
        label: 'Closed',
        description: 'Operating normally',
      };
    case 'half_open':
      return {
        color: 'text-glow-gold',
        bgColor: 'bg-glow-gold/20',
        icon: (
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        ),
        label: 'Half Open',
        description: 'Testing recovery',
      };
    case 'open':
      return {
        color: 'text-red-400',
        bgColor: 'bg-red-500/20',
        icon: (
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="10" />
            <line x1="15" y1="9" x2="9" y2="15" />
            <line x1="9" y1="9" x2="15" y2="15" />
          </svg>
        ),
        label: 'Open',
        description: 'Circuit tripped',
      };
  }
}

// Tooltip component for hover information
function Tooltip({ children, content }: { children: React.ReactNode; content: string }) {
  const [show, setShow] = useState(false);

  return (
    <div
      className="relative inline-block"
      onMouseEnter={() => setShow(true)}
      onMouseLeave={() => setShow(false)}
    >
      {children}
      {show && (
        <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 z-50 pointer-events-none">
          <div className="bg-deep-earth border border-border-subtle rounded-lg px-3 py-2 text-xs text-mycelium-white shadow-lg whitespace-nowrap max-w-xs">
            {content}
          </div>
          <div className="absolute top-full left-1/2 -translate-x-1/2 -mt-1">
            <div className="border-4 border-transparent border-t-deep-earth" />
          </div>
        </div>
      )}
    </div>
  );
}

function SeptalGateIndicator({ state }: { state: SeptalState }) {
  const config = getSeptalStateConfig(state);

  return (
    <Tooltip content={SEPTAL_STATE_TOOLTIPS[state]}>
      <div className={`flex items-center gap-2 px-3 py-1.5 rounded-full ${config.bgColor} cursor-help`}>
        <span className={config.color}>{config.icon}</span>
        <span className={`text-sm font-display ${config.color}`}>{config.label}</span>
      </div>
    </Tooltip>
  );
}

function NodeSeptalCard({ state }: { state: NodeEnrState }) {
  const config = getSeptalStateConfig(state.septalState);
  const isOpen = state.septalState === 'open';

  return (
    <div className={`p-4 bg-moss rounded-lg relative overflow-hidden ${
      isOpen ? 'ring-1 ring-red-400/50 animate-[pulse-glow_2s_ease-in-out_infinite]' : ''
    }`} style={isOpen ? {
      animation: 'pulse-glow 2s ease-in-out infinite',
    } : undefined}>
      {/* Attention overlay for open circuits */}
      {isOpen && (
        <div className="absolute inset-0 bg-gradient-to-r from-red-500/5 to-red-500/10 pointer-events-none animate-pulse" />
      )}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <div className={`w-10 h-10 rounded-full ${config.bgColor} flex items-center justify-center`}>
            <span className={config.color}>{config.icon}</span>
          </div>
          <div>
            <div className="font-display text-mycelium-white">
              {shortenNodeId(state.nodeId)}
            </div>
            <div className="text-xs text-soft-gray">
              Updated {formatTimestamp(state.lastUpdated)}
            </div>
          </div>
        </div>
        <SeptalGateIndicator state={state.septalState} />
      </div>

      {/* Status details */}
      <div className="grid grid-cols-3 gap-4 text-sm">
        <div className="text-center p-2 bg-bark rounded">
          <div className={state.septalHealthy ? 'text-glow-cyan' : 'text-red-400'}>
            {state.septalHealthy ? 'Healthy' : 'Unhealthy'}
          </div>
          <div className="text-xs text-soft-gray mt-1">Health Status</div>
        </div>
        <div className="text-center p-2 bg-bark rounded">
          <div className={state.failureCount > 0 ? 'text-glow-gold' : 'text-mycelium-white'}>
            {state.failureCount}
          </div>
          <div className="text-xs text-soft-gray mt-1">Failures</div>
        </div>
        <div className="text-center p-2 bg-bark rounded">
          <div className="text-mycelium-white">
            {state.balance.toLocaleString()}
          </div>
          <div className="text-xs text-soft-gray mt-1">Balance</div>
        </div>
      </div>

      {/* Failure warning */}
      {state.failureCount > 0 && (
        <div className="mt-3 p-2 bg-glow-gold/10 border border-glow-gold/20 rounded text-xs text-glow-gold">
          {state.failureCount} recent failure{state.failureCount !== 1 ? 's' : ''} detected
        </div>
      )}
    </div>
  );
}

export function SeptalPanel({
  nodeEnrStates,
  onClose,
}: SeptalPanelProps) {
  // Track last refresh time for auto-refresh indicator
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());
  const [refreshAgo, setRefreshAgo] = useState<string>('just now');

  // Update refresh time when nodeEnrStates changes
  useEffect(() => {
    setLastRefresh(new Date());
  }, [nodeEnrStates]);

  // Update "time ago" display every second
  useEffect(() => {
    const interval = setInterval(() => {
      const seconds = Math.floor((Date.now() - lastRefresh.getTime()) / 1000);
      if (seconds < 5) {
        setRefreshAgo('just now');
      } else if (seconds < 60) {
        setRefreshAgo(`${seconds}s ago`);
      } else if (seconds < 3600) {
        setRefreshAgo(`${Math.floor(seconds / 60)}m ago`);
      } else {
        setRefreshAgo(`${Math.floor(seconds / 3600)}h ago`);
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [lastRefresh]);

  // Calculate network-wide septal statistics
  const stats = useMemo(() => {
    const nodes = Array.from(nodeEnrStates.values());
    const closed = nodes.filter(n => n.septalState === 'closed').length;
    const halfOpen = nodes.filter(n => n.septalState === 'half_open').length;
    const open = nodes.filter(n => n.septalState === 'open').length;
    const healthy = nodes.filter(n => n.septalHealthy).length;
    const totalFailures = nodes.reduce((acc, n) => acc + n.failureCount, 0);

    return {
      total: nodes.length,
      closed,
      halfOpen,
      open,
      healthy,
      unhealthy: nodes.length - healthy,
      totalFailures,
      healthPercent: nodes.length > 0 ? (healthy / nodes.length) * 100 : 100,
    };
  }, [nodeEnrStates]);

  // Group nodes by septal state
  const groupedNodes = useMemo(() => {
    const nodes = Array.from(nodeEnrStates.values());

    return {
      open: nodes.filter(n => n.septalState === 'open'),
      halfOpen: nodes.filter(n => n.septalState === 'half_open'),
      closed: nodes.filter(n => n.septalState === 'closed'),
    };
  }, [nodeEnrStates]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-void/80 backdrop-blur-sm">
      <div className="w-full max-w-3xl max-h-[90vh] bg-forest-floor border border-border-subtle rounded-xl shadow-card overflow-hidden">
        {/* Header */}
        <div className="relative px-6 py-4 bg-deep-earth border-b border-border-subtle">
          <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-glow-cyan via-glow-gold to-red-400" />
          <div className="flex items-center justify-between">
            <div>
              <div className="flex items-center gap-3">
                <h2 className="text-xl font-display font-bold text-mycelium-white">
                  Septal Gates
                </h2>
                {/* Auto-refresh indicator */}
                <div className="flex items-center gap-1.5 px-2 py-0.5 bg-moss rounded-full">
                  <div className="w-1.5 h-1.5 rounded-full bg-glow-cyan animate-pulse" />
                  <span className="text-xs text-soft-gray">
                    Updated {refreshAgo}
                  </span>
                </div>
              </div>
              <p className="text-sm text-soft-gray font-body mt-0.5">
                Circuit breaker status across network nodes
              </p>
            </div>
            {onClose && (
              <button
                onClick={onClose}
                className="text-soft-gray hover:text-mycelium-white transition-colors"
              >
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M18 6L6 18M6 6l12 12" />
                </svg>
              </button>
            )}
          </div>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto max-h-[calc(90vh-100px)]">
          {nodeEnrStates.size === 0 ? (
            <div className="text-center py-12">
              <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-moss flex items-center justify-center">
                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-soft-gray">
                  <circle cx="12" cy="12" r="10" />
                  <path d="M8 12h8" />
                </svg>
              </div>
              <p className="text-soft-gray">
                No septal gate data available
              </p>
              <p className="text-xs text-soft-gray/60 mt-2">
                Circuit breaker states will appear as nodes report health status
              </p>
            </div>
          ) : (
            <div className="space-y-6">
              {/* Network Overview */}
              <div className="p-4 bg-moss rounded-lg">
                <h3 className="text-sm font-display uppercase tracking-wider text-soft-gray mb-4">
                  Network Health Overview
                </h3>

                <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
                  {/* Health Gauge */}
                  <div className="text-center">
                    <div className="relative w-20 h-20 mx-auto mb-2">
                      <svg className="w-full h-full transform -rotate-90">
                        <circle
                          cx="40" cy="40" r="36"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth="8"
                          className="text-bark"
                        />
                        <circle
                          cx="40" cy="40" r="36"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth="8"
                          strokeDasharray={`${stats.healthPercent * 2.26} 226`}
                          className={stats.healthPercent >= 80 ? 'text-glow-cyan' : stats.healthPercent >= 50 ? 'text-glow-gold' : 'text-red-400'}
                        />
                      </svg>
                      <div className="absolute inset-0 flex items-center justify-center">
                        <span className={`text-lg font-display font-bold ${
                          stats.healthPercent >= 80 ? 'text-glow-cyan' :
                          stats.healthPercent >= 50 ? 'text-glow-gold' : 'text-red-400'
                        }`}>
                          {Math.round(stats.healthPercent)}%
                        </span>
                      </div>
                    </div>
                    <div className="text-xs text-soft-gray uppercase tracking-wider">Network Health</div>
                  </div>

                  {/* State Counts with Tooltips */}
                  <div className="space-y-2">
                    <Tooltip content={SEPTAL_STATE_TOOLTIPS.closed}>
                      <div className="flex items-center justify-between text-sm cursor-help">
                        <div className="flex items-center gap-2">
                          <div className="w-3 h-3 rounded-full bg-glow-cyan" />
                          <span className="text-soft-gray">Closed</span>
                        </div>
                        <span className="text-glow-cyan font-display">{stats.closed}</span>
                      </div>
                    </Tooltip>
                    <Tooltip content={SEPTAL_STATE_TOOLTIPS.half_open}>
                      <div className="flex items-center justify-between text-sm cursor-help">
                        <div className="flex items-center gap-2">
                          <div className="w-3 h-3 rounded-full bg-glow-gold" />
                          <span className="text-soft-gray">Half Open</span>
                        </div>
                        <span className="text-glow-gold font-display">{stats.halfOpen}</span>
                      </div>
                    </Tooltip>
                    <Tooltip content={SEPTAL_STATE_TOOLTIPS.open}>
                      <div className="flex items-center justify-between text-sm cursor-help">
                        <div className="flex items-center gap-2">
                          <div className="w-3 h-3 rounded-full bg-red-400" />
                          <span className="text-soft-gray">Open</span>
                        </div>
                        <span className="text-red-400 font-display">{stats.open}</span>
                      </div>
                    </Tooltip>
                  </div>

                  {/* Total Nodes */}
                  <div className="text-center p-3 bg-bark rounded-lg">
                    <div className="text-2xl font-display text-mycelium-white">{stats.total}</div>
                    <div className="text-xs text-soft-gray">Total Nodes</div>
                  </div>

                  {/* Total Failures */}
                  <div className="text-center p-3 bg-bark rounded-lg">
                    <div className={`text-2xl font-display ${stats.totalFailures > 0 ? 'text-glow-gold' : 'text-mycelium-white'}`}>
                      {stats.totalFailures}
                    </div>
                    <div className="text-xs text-soft-gray">Total Failures</div>
                  </div>
                </div>

                {/* Health bar */}
                <div className="h-3 bg-bark rounded-full overflow-hidden flex">
                  {stats.closed > 0 && (
                    <div
                      className="h-full bg-glow-cyan transition-all duration-500"
                      style={{ width: `${(stats.closed / stats.total) * 100}%` }}
                    />
                  )}
                  {stats.halfOpen > 0 && (
                    <div
                      className="h-full bg-glow-gold transition-all duration-500"
                      style={{ width: `${(stats.halfOpen / stats.total) * 100}%` }}
                    />
                  )}
                  {stats.open > 0 && (
                    <div
                      className="h-full bg-red-400 transition-all duration-500"
                      style={{ width: `${(stats.open / stats.total) * 100}%` }}
                    />
                  )}
                </div>
              </div>

              {/* Tripped circuits (open) - shown first as they need attention */}
              {groupedNodes.open.length > 0 && (
                <div>
                  <div className="flex items-center gap-2 mb-3">
                    <div className="w-2 h-2 rounded-full bg-red-400 animate-pulse" />
                    <h3 className="text-sm font-display uppercase tracking-wider text-red-400">
                      Tripped Circuits ({groupedNodes.open.length})
                    </h3>
                  </div>
                  <div className="space-y-3">
                    {groupedNodes.open.map(state => (
                      <NodeSeptalCard key={state.nodeId} state={state} />
                    ))}
                  </div>
                </div>
              )}

              {/* Half-open (recovering) */}
              {groupedNodes.halfOpen.length > 0 && (
                <div>
                  <div className="flex items-center gap-2 mb-3">
                    <div className="w-2 h-2 rounded-full bg-glow-gold animate-pulse" />
                    <h3 className="text-sm font-display uppercase tracking-wider text-glow-gold">
                      Testing Recovery ({groupedNodes.halfOpen.length})
                    </h3>
                  </div>
                  <div className="space-y-3">
                    {groupedNodes.halfOpen.map(state => (
                      <NodeSeptalCard key={state.nodeId} state={state} />
                    ))}
                  </div>
                </div>
              )}

              {/* Closed (healthy) */}
              {groupedNodes.closed.length > 0 && (
                <div>
                  <h3 className="text-sm font-display uppercase tracking-wider text-soft-gray mb-3">
                    Healthy Nodes ({groupedNodes.closed.length})
                  </h3>
                  <div className="grid gap-3 md:grid-cols-2">
                    {groupedNodes.closed.map(state => (
                      <NodeSeptalCard key={state.nodeId} state={state} />
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
