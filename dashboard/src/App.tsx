import { useState, useMemo, useCallback } from 'react';
import { useP2P } from '@/hooks/useP2P';
import { PeerGraph } from '@/components/PeerGraph';
import { ChatPanel } from '@/components/ChatPanel';
import { ReputationCard } from '@/components/ReputationCard';
import type { NormalizedPeer } from '@/types';

function App() {
  const { connected, peers, messages, sendChat, graphData } = useP2P();
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null);

  // Memoize graph data to prevent infinite re-renders
  const { nodes, links } = useMemo(() => graphData(), [graphData]);

  const selectedPeer: NormalizedPeer | null = selectedPeerId
    ? peers.get(selectedPeerId) || null
    : null;

  // Stable click handler that only uses the node ID
  const handleNodeClick = useCallback((nodeId: string) => {
    setSelectedPeerId(prev => prev === nodeId ? null : nodeId);
  }, []);

  return (
    <div className="min-h-screen bg-surface-dark text-white">
      {/* Header */}
      <header className="border-b border-gray-800 px-6 py-4">
        <div className="flex items-center justify-between max-w-7xl mx-auto">
          <div className="flex items-center gap-3">
            <div className="text-2xl">🍄</div>
            <div>
              <h1 className="text-xl font-bold">Mycelial Network</h1>
              <p className="text-sm text-gray-400">P2P Agent Dashboard</p>
            </div>
          </div>
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2">
              <div
                className={`w-2 h-2 rounded-full ${
                  connected ? 'bg-green-500 animate-pulse' : 'bg-red-500'
                }`}
              />
              <span className="text-sm text-gray-400">
                {connected ? 'Connected' : 'Disconnected'}
              </span>
            </div>
            <div className="text-sm text-gray-400">
              {peers.size} peer{peers.size !== 1 ? 's' : ''} online
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto p-6">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 h-[calc(100vh-140px)]">
          {/* Peer Graph - takes 2 columns on large screens */}
          <div className="lg:col-span-2 min-h-[400px]">
            <PeerGraph
              nodes={nodes}
              links={links}
              onNodeClick={handleNodeClick}
              selectedNodeId={selectedPeerId}
            />
          </div>

          {/* Sidebar */}
          <div className="flex flex-col gap-6 min-h-0">
            {/* Reputation Card */}
            <div className="flex-shrink-0">
              <ReputationCard
                peer={selectedPeer}
                onClose={() => setSelectedPeerId(null)}
              />
            </div>

            {/* Chat Panel */}
            <div className="flex-1 min-h-[300px]">
              <ChatPanel
                messages={messages}
                onSendMessage={sendChat}
                selectedPeer={selectedPeerId}
              />
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}

export default App;
