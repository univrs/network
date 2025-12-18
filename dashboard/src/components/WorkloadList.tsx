import { useState, useMemo } from 'react';
import type { Workload, WorkloadStatus, WorkloadPriority } from '@/types';

interface WorkloadListProps {
  workloads: Map<string, Workload>;
  onCancelWorkload?: (workloadId: string) => void;
  onRetryWorkload?: (workloadId: string) => void;
  onClose?: () => void;
}

const statusColors: Record<WorkloadStatus, string> = {
  pending: 'text-glow-gold bg-glow-gold/20',
  running: 'text-glow-cyan bg-glow-cyan/20',
  completed: 'text-green-400 bg-green-400/20',
  failed: 'text-red-400 bg-red-400/20',
  cancelled: 'text-soft-gray bg-soft-gray/20',
};

const priorityColors: Record<WorkloadPriority, string> = {
  low: 'text-soft-gray',
  medium: 'text-glow-gold',
  high: 'text-orange-400',
  critical: 'text-red-400',
};

function formatDuration(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
  return `${seconds}s`;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

function formatTimeAgo(timestamp: number): string {
  const diff = Date.now() - timestamp;
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) return `${hours}h ago`;
  if (minutes > 0) return `${minutes}m ago`;
  return 'Just now';
}

// Helper to extract resources from both API and mock formats
function getWorkloadResources(workload: Workload) {
  // Try legacy resourceRequirements format first (mock data)
  if (workload.resourceRequirements) {
    return {
      cpu: workload.resourceRequirements.cpu,
      memory: workload.resourceRequirements.memory,
      storage: workload.resourceRequirements.storage,
    };
  }

  // Try containers[].resource_requests format (real API)
  if (workload.containers?.length) {
    const totals = workload.containers.reduce(
      (acc, container) => {
        const req = container.resource_requests;
        return {
          cpu: acc.cpu + (req?.cpu_cores ?? 0),
          memory: acc.memory + ((req?.memory_mb ?? 0) * 1024 * 1024), // MB to bytes
          storage: acc.storage + ((req?.disk_mb ?? 0) * 1024 * 1024), // MB to bytes
        };
      },
      { cpu: 0, memory: 0, storage: 0 }
    );
    return totals;
  }

  // Default fallback
  return { cpu: 0, memory: 0, storage: 0 };
}

export function WorkloadList({
  workloads,
  onCancelWorkload,
  onRetryWorkload,
  onClose,
}: WorkloadListProps) {
  const [activeTab, setActiveTab] = useState<'all' | 'running' | 'pending' | 'completed' | 'failed'>('all');
  const [sortBy, setSortBy] = useState<'created' | 'priority' | 'progress'>('created');

  const filteredWorkloads = useMemo(() => {
    let filtered = Array.from(workloads.values());

    // Filter by tab
    if (activeTab !== 'all') {
      filtered = filtered.filter(w => w.status === activeTab);
    }

    // Sort
    filtered.sort((a, b) => {
      switch (sortBy) {
        case 'priority': {
          const priorityOrder = { critical: 0, high: 1, medium: 2, low: 3 };
          return priorityOrder[a.priority] - priorityOrder[b.priority];
        }
        case 'progress':
          return b.progress - a.progress;
        case 'created':
        default:
          return b.createdAt - a.createdAt;
      }
    });

    return filtered;
  }, [workloads, activeTab, sortBy]);

  const stats = useMemo(() => {
    const all = Array.from(workloads.values());
    return {
      total: all.length,
      running: all.filter(w => w.status === 'running').length,
      pending: all.filter(w => w.status === 'pending').length,
      completed: all.filter(w => w.status === 'completed').length,
      failed: all.filter(w => w.status === 'failed').length,
    };
  }, [workloads]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-void/80 backdrop-blur-sm">
      <div className="w-full max-w-5xl max-h-[90vh] bg-forest-floor border border-border-subtle rounded-xl shadow-card overflow-hidden">
        {/* Header */}
        <div className="relative px-6 py-4 bg-deep-earth border-b border-border-subtle">
          <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-glow-cyan via-spore-purple to-glow-gold" />
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-xl font-display font-bold text-mycelium-white flex items-center gap-3">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-glow-cyan">
                  <rect x="3" y="3" width="18" height="18" rx="2" />
                  <path d="M9 3v18M15 3v18M3 9h18M3 15h18" />
                </svg>
                Workload Queue
              </h2>
              <p className="text-sm text-soft-gray font-body">
                Distributed task orchestration and monitoring
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

          {/* Stats */}
          <div className="grid grid-cols-5 gap-2 mt-4">
            {[
              { label: 'Total', value: stats.total, color: 'text-mycelium-white' },
              { label: 'Running', value: stats.running, color: 'text-glow-cyan' },
              { label: 'Pending', value: stats.pending, color: 'text-glow-gold' },
              { label: 'Completed', value: stats.completed, color: 'text-green-400' },
              { label: 'Failed', value: stats.failed, color: 'text-red-400' },
            ].map(stat => (
              <div key={stat.label} className="text-center p-2 bg-moss rounded-lg">
                <div className={`text-lg font-display font-bold ${stat.color}`}>{stat.value}</div>
                <div className="text-xs text-soft-gray uppercase tracking-wider">{stat.label}</div>
              </div>
            ))}
          </div>

          {/* Tabs */}
          <div className="flex gap-2 mt-4">
            {(['all', 'running', 'pending', 'completed', 'failed'] as const).map((tab) => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className={`px-3 py-1.5 rounded-lg text-sm font-display capitalize transition-colors ${
                  activeTab === tab
                    ? 'bg-glow-cyan/20 text-glow-cyan'
                    : 'text-soft-gray hover:text-mycelium-white hover:bg-moss'
                }`}
              >
                {tab}
              </button>
            ))}
            <div className="flex-1" />
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as typeof sortBy)}
              className="px-3 py-1.5 rounded-lg text-sm font-display bg-moss text-soft-gray border border-border-subtle focus:outline-none focus:border-glow-cyan"
            >
              <option value="created">Sort: Recent</option>
              <option value="priority">Sort: Priority</option>
              <option value="progress">Sort: Progress</option>
            </select>
          </div>
        </div>

        {/* Workload List */}
        <div className="p-4 overflow-y-auto max-h-[calc(90vh-250px)]">
          <div className="space-y-3">
            {filteredWorkloads.map((workload) => (
              <div
                key={workload.id}
                className="p-4 bg-moss rounded-lg border border-border-subtle hover:border-glow-cyan/30 transition-colors"
              >
                <div className="flex items-start justify-between gap-4">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <h3 className="font-display font-semibold text-mycelium-white truncate">
                        {workload.name}
                      </h3>
                      <span className={`px-2 py-0.5 rounded text-xs font-display uppercase ${statusColors[workload.status]}`}>
                        {workload.status}
                      </span>
                      <span className={`text-xs font-display uppercase ${priorityColors[workload.priority]}`}>
                        {workload.priority}
                      </span>
                    </div>
                    {workload.description && (
                      <p className="text-sm text-soft-gray mb-2 truncate">{workload.description}</p>
                    )}
                    <div className="flex items-center gap-4 text-xs text-soft-gray">
                      <span>ID: {workload.id.slice(0, 12)}...</span>
                      {workload.assignedNode && (
                        <span className="flex items-center gap-1">
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <rect x="2" y="2" width="20" height="8" rx="2" />
                            <rect x="2" y="14" width="20" height="8" rx="2" />
                            <circle cx="6" cy="6" r="1" />
                            <circle cx="6" cy="18" r="1" />
                          </svg>
                          {workload.assignedNode}
                        </span>
                      )}
                      <span>{formatTimeAgo(workload.createdAt)}</span>
                      {workload.startedAt && workload.status === 'running' && (
                        <span>Running: {formatDuration(Date.now() - workload.startedAt)}</span>
                      )}
                    </div>
                  </div>

                  {/* Resource Requirements */}
                  {(() => {
                    const resources = getWorkloadResources(workload);
                    return (
                      <div className="flex items-center gap-3 text-xs">
                        <div className="text-center">
                          <div className="text-glow-cyan font-display">{resources.cpu || '-'}</div>
                          <div className="text-soft-gray/60">CPU</div>
                        </div>
                        <div className="text-center">
                          <div className="text-spore-purple font-display">{resources.memory ? formatBytes(resources.memory) : '-'}</div>
                          <div className="text-soft-gray/60">RAM</div>
                        </div>
                        <div className="text-center">
                          <div className="text-glow-gold font-display">{resources.storage ? formatBytes(resources.storage) : '-'}</div>
                          <div className="text-soft-gray/60">Disk</div>
                        </div>
                      </div>
                    );
                  })()}

                  {/* Actions */}
                  <div className="flex items-center gap-2">
                    {workload.status === 'running' && onCancelWorkload && (
                      <button
                        onClick={() => onCancelWorkload(workload.id)}
                        className="p-2 rounded-lg bg-red-400/10 text-red-400 hover:bg-red-400/20 transition-colors"
                        title="Cancel workload"
                      >
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <rect x="3" y="3" width="18" height="18" rx="2" />
                        </svg>
                      </button>
                    )}
                    {workload.status === 'failed' && onRetryWorkload && (
                      <button
                        onClick={() => onRetryWorkload(workload.id)}
                        className="p-2 rounded-lg bg-glow-cyan/10 text-glow-cyan hover:bg-glow-cyan/20 transition-colors"
                        title="Retry workload"
                      >
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                          <path d="M1 4v6h6M23 20v-6h-6" />
                          <path d="M20.49 9A9 9 0 005.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 013.51 15" />
                        </svg>
                      </button>
                    )}
                  </div>
                </div>

                {/* Progress Bar */}
                {(workload.status === 'running' || workload.status === 'pending') && (
                  <div className="mt-3">
                    <div className="flex justify-between text-xs text-soft-gray mb-1">
                      <span>Progress</span>
                      <span>{workload.progress}%</span>
                    </div>
                    <div className="h-2 bg-bark rounded-full overflow-hidden">
                      <div
                        className={`h-full transition-all duration-500 ${
                          workload.status === 'running'
                            ? 'bg-gradient-to-r from-glow-cyan to-spore-purple animate-pulse'
                            : 'bg-glow-gold'
                        }`}
                        style={{ width: `${workload.progress}%` }}
                      />
                    </div>
                  </div>
                )}
              </div>
            ))}

            {filteredWorkloads.length === 0 && (
              <div className="text-center py-12">
                <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-moss flex items-center justify-center">
                  <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-soft-gray">
                    <rect x="3" y="3" width="18" height="18" rx="2" />
                    <path d="M9 3v18M15 3v18M3 9h18M3 15h18" />
                  </svg>
                </div>
                <p className="text-soft-gray">
                  No {activeTab === 'all' ? '' : activeTab} workloads found
                </p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
