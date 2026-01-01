// src/components/GradientPanel.tsx
// Displays network-wide resource gradients from ENR bridge

import { useMemo, useState } from 'react';
import type { GradientUpdate, NodeEnrState } from '@/types';

interface GradientPanelProps {
  gradients: Map<string, GradientUpdate>;
  nodeEnrStates: Map<string, NodeEnrState>;
  onClose?: () => void;
  onReportGradient?: (cpu: number, mem: number, bw: number, storage: number) => void;
}

function formatPercent(value: number): string {
  return `${Math.round(value * 100)}%`;
}

function getGradientColor(value: number): string {
  if (value >= 0.7) return 'text-glow-cyan';
  if (value >= 0.4) return 'text-glow-gold';
  return 'text-red-400';
}

function getBarColor(value: number): string {
  if (value >= 0.7) return 'bg-glow-cyan';
  if (value >= 0.4) return 'bg-glow-gold';
  return 'bg-red-400';
}

function formatTimestamp(ts: number): string {
  const seconds = Math.floor((Date.now() - ts) / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  return `${Math.floor(seconds / 3600)}h ago`;
}

function shortenNodeId(id: string): string {
  if (id.length <= 12) return id;
  return `${id.slice(0, 6)}...${id.slice(-4)}`;
}

export function GradientPanel({
  gradients,
  nodeEnrStates,
  onClose,
  onReportGradient,
}: GradientPanelProps) {
  // State for report gradient modal
  const [showReportModal, setShowReportModal] = useState(false);
  const [cpuValue, setCpuValue] = useState(50);
  const [memoryValue, setMemoryValue] = useState(50);
  const [bandwidthValue, setBandwidthValue] = useState(50);
  const [storageValue, setStorageValue] = useState(50);

  const handleReportSubmit = () => {
    if (onReportGradient) {
      onReportGradient(
        cpuValue / 100,
        memoryValue / 100,
        bandwidthValue / 100,
        storageValue / 100
      );
    }
    setShowReportModal(false);
  };

  // Calculate network-wide aggregate gradient
  const networkGradient = useMemo(() => {
    const values = Array.from(gradients.values());
    if (values.length === 0) {
      return {
        cpu: 0,
        memory: 0,
        bandwidth: 0,
        storage: 0,
        nodeCount: 0,
      };
    }

    return {
      cpu: values.reduce((acc, g) => acc + g.cpuAvailable, 0) / values.length,
      memory: values.reduce((acc, g) => acc + g.memoryAvailable, 0) / values.length,
      bandwidth: values.reduce((acc, g) => acc + g.bandwidthAvailable, 0) / values.length,
      storage: values.reduce((acc, g) => acc + g.storageAvailable, 0) / values.length,
      nodeCount: values.length,
    };
  }, [gradients]);

  // Get list of nodes with their gradients
  const nodeList = useMemo(() => {
    return Array.from(gradients.entries())
      .map(([nodeId, gradient]) => ({
        nodeId,
        gradient,
        enrState: nodeEnrStates.get(nodeId),
      }))
      .sort((a, b) => {
        // Sort by average availability
        const avgA = (a.gradient.cpuAvailable + a.gradient.memoryAvailable) / 2;
        const avgB = (b.gradient.cpuAvailable + b.gradient.memoryAvailable) / 2;
        return avgB - avgA;
      });
  }, [gradients, nodeEnrStates]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-void/80 backdrop-blur-sm">
      <div className="w-full max-w-4xl max-h-[90vh] bg-forest-floor border border-border-subtle rounded-xl shadow-card overflow-hidden">
        {/* Header */}
        <div className="relative px-6 py-4 bg-deep-earth border-b border-border-subtle">
          <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-glow-cyan via-spore-purple to-glow-gold" />
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-xl font-display font-bold text-mycelium-white">
                Network Gradients
              </h2>
              <p className="text-sm text-soft-gray font-body">
                Real-time resource availability across nodes
              </p>
            </div>
            <div className="flex items-center gap-3">
              {onReportGradient && (
                <button
                  onClick={() => setShowReportModal(true)}
                  className="px-4 py-2 bg-glow-cyan/20 text-glow-cyan text-sm font-display rounded-lg hover:bg-glow-cyan/30 transition-colors"
                >
                  Report Gradient
                </button>
              )}
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
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto max-h-[calc(90vh-100px)]">
          {gradients.size === 0 ? (
            <div className="text-center py-12">
              <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-moss flex items-center justify-center">
                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-soft-gray">
                  <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
                </svg>
              </div>
              <p className="text-soft-gray">
                No gradient updates received yet
              </p>
              <p className="text-xs text-soft-gray/60 mt-2">
                Gradients will appear as nodes broadcast their resource availability
              </p>
            </div>
          ) : (
            <div className="space-y-6">
              {/* Network Overview */}
              <div className="p-4 bg-moss rounded-lg">
                <div className="flex items-center justify-between mb-4">
                  <h3 className="text-sm font-display uppercase tracking-wider text-soft-gray">
                    Network Aggregate
                  </h3>
                  <span className="text-xs text-soft-gray">
                    {networkGradient.nodeCount} node{networkGradient.nodeCount !== 1 ? 's' : ''} reporting
                  </span>
                </div>

                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  {/* CPU */}
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
                          strokeDasharray={`${networkGradient.cpu * 226} 226`}
                          className={getGradientColor(networkGradient.cpu)}
                        />
                      </svg>
                      <div className="absolute inset-0 flex items-center justify-center">
                        <span className={`text-lg font-display font-bold ${getGradientColor(networkGradient.cpu)}`}>
                          {formatPercent(networkGradient.cpu)}
                        </span>
                      </div>
                    </div>
                    <div className="text-xs text-soft-gray uppercase tracking-wider">CPU</div>
                  </div>

                  {/* Memory */}
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
                          strokeDasharray={`${networkGradient.memory * 226} 226`}
                          className={getGradientColor(networkGradient.memory)}
                        />
                      </svg>
                      <div className="absolute inset-0 flex items-center justify-center">
                        <span className={`text-lg font-display font-bold ${getGradientColor(networkGradient.memory)}`}>
                          {formatPercent(networkGradient.memory)}
                        </span>
                      </div>
                    </div>
                    <div className="text-xs text-soft-gray uppercase tracking-wider">Memory</div>
                  </div>

                  {/* Bandwidth */}
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
                          strokeDasharray={`${networkGradient.bandwidth * 226} 226`}
                          className={getGradientColor(networkGradient.bandwidth)}
                        />
                      </svg>
                      <div className="absolute inset-0 flex items-center justify-center">
                        <span className={`text-lg font-display font-bold ${getGradientColor(networkGradient.bandwidth)}`}>
                          {formatPercent(networkGradient.bandwidth)}
                        </span>
                      </div>
                    </div>
                    <div className="text-xs text-soft-gray uppercase tracking-wider">Bandwidth</div>
                  </div>

                  {/* Storage */}
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
                          strokeDasharray={`${networkGradient.storage * 226} 226`}
                          className={getGradientColor(networkGradient.storage)}
                        />
                      </svg>
                      <div className="absolute inset-0 flex items-center justify-center">
                        <span className={`text-lg font-display font-bold ${getGradientColor(networkGradient.storage)}`}>
                          {formatPercent(networkGradient.storage)}
                        </span>
                      </div>
                    </div>
                    <div className="text-xs text-soft-gray uppercase tracking-wider">Storage</div>
                  </div>
                </div>
              </div>

              {/* Per-Node Gradients */}
              <div>
                <h3 className="text-sm font-display uppercase tracking-wider text-soft-gray mb-3">
                  Per-Node Resources
                </h3>
                <div className="space-y-2">
                  {nodeList.map(({ nodeId, gradient, enrState }) => (
                    <div
                      key={nodeId}
                      className="p-4 bg-moss rounded-lg"
                    >
                      <div className="flex items-center justify-between mb-3">
                        <div className="flex items-center gap-3">
                          <div className="w-8 h-8 rounded-full bg-glow-cyan/20 flex items-center justify-center">
                            <span className="text-xs font-display font-bold text-glow-cyan">
                              {nodeId.slice(0, 2).toUpperCase()}
                            </span>
                          </div>
                          <div>
                            <div className="font-display text-mycelium-white text-sm">
                              {shortenNodeId(nodeId)}
                            </div>
                            <div className="text-xs text-soft-gray">
                              {formatTimestamp(gradient.timestamp)}
                              {enrState && (
                                <span className="ml-2">
                                  • Balance: {enrState.balance}
                                </span>
                              )}
                            </div>
                          </div>
                        </div>
                        {enrState && (
                          <div className={`px-2 py-1 rounded text-xs ${
                            enrState.septalState === 'closed'
                              ? 'bg-glow-cyan/20 text-glow-cyan'
                              : enrState.septalState === 'half_open'
                              ? 'bg-glow-gold/20 text-glow-gold'
                              : 'bg-red-500/20 text-red-400'
                          }`}>
                            {enrState.septalState}
                          </div>
                        )}
                      </div>

                      {/* Resource Bars */}
                      <div className="grid grid-cols-4 gap-3">
                        <div>
                          <div className="flex justify-between text-xs text-soft-gray mb-1">
                            <span>CPU</span>
                            <span className={getGradientColor(gradient.cpuAvailable)}>
                              {formatPercent(gradient.cpuAvailable)}
                            </span>
                          </div>
                          <div className="h-2 bg-bark rounded-full overflow-hidden">
                            <div
                              className={`h-full ${getBarColor(gradient.cpuAvailable)} transition-all duration-500`}
                              style={{ width: formatPercent(gradient.cpuAvailable) }}
                            />
                          </div>
                        </div>
                        <div>
                          <div className="flex justify-between text-xs text-soft-gray mb-1">
                            <span>Mem</span>
                            <span className={getGradientColor(gradient.memoryAvailable)}>
                              {formatPercent(gradient.memoryAvailable)}
                            </span>
                          </div>
                          <div className="h-2 bg-bark rounded-full overflow-hidden">
                            <div
                              className={`h-full ${getBarColor(gradient.memoryAvailable)} transition-all duration-500`}
                              style={{ width: formatPercent(gradient.memoryAvailable) }}
                            />
                          </div>
                        </div>
                        <div>
                          <div className="flex justify-between text-xs text-soft-gray mb-1">
                            <span>BW</span>
                            <span className={getGradientColor(gradient.bandwidthAvailable)}>
                              {formatPercent(gradient.bandwidthAvailable)}
                            </span>
                          </div>
                          <div className="h-2 bg-bark rounded-full overflow-hidden">
                            <div
                              className={`h-full ${getBarColor(gradient.bandwidthAvailable)} transition-all duration-500`}
                              style={{ width: formatPercent(gradient.bandwidthAvailable) }}
                            />
                          </div>
                        </div>
                        <div>
                          <div className="flex justify-between text-xs text-soft-gray mb-1">
                            <span>Disk</span>
                            <span className={getGradientColor(gradient.storageAvailable)}>
                              {formatPercent(gradient.storageAvailable)}
                            </span>
                          </div>
                          <div className="h-2 bg-bark rounded-full overflow-hidden">
                            <div
                              className={`h-full ${getBarColor(gradient.storageAvailable)} transition-all duration-500`}
                              style={{ width: formatPercent(gradient.storageAvailable) }}
                            />
                          </div>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Report Gradient Modal */}
      {showReportModal && (
        <div className="fixed inset-0 z-60 flex items-center justify-center bg-void/90 backdrop-blur-sm">
          <div className="w-full max-w-md bg-forest-floor border border-border-subtle rounded-xl shadow-card overflow-hidden">
            {/* Modal Header */}
            <div className="relative px-6 py-4 bg-deep-earth border-b border-border-subtle">
              <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-glow-cyan via-spore-purple to-glow-gold" />
              <div className="flex items-center justify-between">
                <h3 className="text-lg font-display font-bold text-mycelium-white">
                  Report Resource Gradient
                </h3>
                <button
                  onClick={() => setShowReportModal(false)}
                  className="text-soft-gray hover:text-mycelium-white transition-colors"
                >
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M18 6L6 18M6 6l12 12" />
                  </svg>
                </button>
              </div>
            </div>

            {/* Modal Content */}
            <div className="p-6 space-y-6">
              {/* CPU Slider */}
              <div>
                <div className="flex justify-between items-center mb-2">
                  <label className="text-sm font-display text-soft-gray uppercase tracking-wider">
                    CPU Available
                  </label>
                  <span className={`text-sm font-display font-bold ${getGradientColor(cpuValue / 100)}`}>
                    {cpuValue}%
                  </span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={cpuValue}
                  onChange={(e) => setCpuValue(Number(e.target.value))}
                  className="w-full h-2 bg-bark rounded-full appearance-none cursor-pointer accent-glow-cyan"
                />
              </div>

              {/* Memory Slider */}
              <div>
                <div className="flex justify-between items-center mb-2">
                  <label className="text-sm font-display text-soft-gray uppercase tracking-wider">
                    Memory Available
                  </label>
                  <span className={`text-sm font-display font-bold ${getGradientColor(memoryValue / 100)}`}>
                    {memoryValue}%
                  </span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={memoryValue}
                  onChange={(e) => setMemoryValue(Number(e.target.value))}
                  className="w-full h-2 bg-bark rounded-full appearance-none cursor-pointer accent-glow-cyan"
                />
              </div>

              {/* Bandwidth Slider */}
              <div>
                <div className="flex justify-between items-center mb-2">
                  <label className="text-sm font-display text-soft-gray uppercase tracking-wider">
                    Bandwidth Available
                  </label>
                  <span className={`text-sm font-display font-bold ${getGradientColor(bandwidthValue / 100)}`}>
                    {bandwidthValue}%
                  </span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={bandwidthValue}
                  onChange={(e) => setBandwidthValue(Number(e.target.value))}
                  className="w-full h-2 bg-bark rounded-full appearance-none cursor-pointer accent-glow-cyan"
                />
              </div>

              {/* Storage Slider */}
              <div>
                <div className="flex justify-between items-center mb-2">
                  <label className="text-sm font-display text-soft-gray uppercase tracking-wider">
                    Storage Available
                  </label>
                  <span className={`text-sm font-display font-bold ${getGradientColor(storageValue / 100)}`}>
                    {storageValue}%
                  </span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={storageValue}
                  onChange={(e) => setStorageValue(Number(e.target.value))}
                  className="w-full h-2 bg-bark rounded-full appearance-none cursor-pointer accent-glow-cyan"
                />
              </div>

              {/* Action Buttons */}
              <div className="flex gap-3 pt-4">
                <button
                  onClick={() => setShowReportModal(false)}
                  className="flex-1 px-4 py-2 bg-moss text-soft-gray font-display rounded-lg hover:bg-bark transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={handleReportSubmit}
                  className="flex-1 px-4 py-2 bg-glow-cyan text-void font-display font-bold rounded-lg hover:bg-glow-cyan/80 transition-colors"
                >
                  Submit
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
