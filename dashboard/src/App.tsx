import { useState, useMemo, useCallback } from 'react';
import { useP2P } from '@/hooks/useP2P';
import { useTheme } from '@/hooks/useTheme';
import { useOrchestrator } from '@/hooks/useOrchestrator';
import { PeerGraph } from '@/components/PeerGraph';
import { ChatPanel } from '@/components/ChatPanel';
import { ConversationSidebar } from '@/components/ConversationSidebar';
import { ReputationCard } from '@/components/ReputationCard';
import { ThemeToggle } from '@/components/ThemeToggle';
import { OnboardingPanel } from '@/components/OnboardingPanel';
import { CreditPanel } from '@/components/CreditPanel';
import { GovernancePanel } from '@/components/GovernancePanel';
import { ResourcePanel } from '@/components/ResourcePanel';
import { WorkloadList } from '@/components/WorkloadList';
import { NodeStatus } from '@/components/NodeStatus';
import { ClusterOverview } from '@/components/ClusterOverview';
import type { NormalizedPeer, GeneratedIdentity, VouchRequest, CreditTransfer, Proposal, Vote } from '@/types';

function App() {
  const {
    connected,
    localPeerId,
    peers,
    messages: _messages, // Used via getActiveMessages
    sendChat,
    graphData,
    // Conversation state
    conversations,
    rooms,
    activeConversationId,
    // Conversation functions
    setActiveConversation,
    startDMConversation,
    getActiveMessages,
    // Room functions
    joinRoom,
    leaveRoom,
    createRoom,
    // Economics state
    creditLines,
    creditTransfers,
    proposals,
    vouches: _vouches, // Available for future use
    resourceContributions: _resourceContributions, // Available for future use
    resourcePool,
    // ENR Bridge state
    gradients: _gradients, // Available for ENR UI
    enrTransfers: _enrTransfers, // Available for ENR UI
    nodeEnrStates: _nodeEnrStates, // Available for ENR UI
    elections: _elections, // Available for ENR UI
    // Economics functions
    sendVouch,
    sendCreditLine,
    sendCreditTransfer,
    sendProposal,
    sendVote,
    sendResourceContribution: _sendResourceContribution, // Available for future use
  } = useP2P();
  const { theme, toggleTheme } = useTheme();
  const {
    workloads,
    nodes: orchestratorNodes,
    clusterMetrics,
    connected: orchestratorConnected,
    loading: orchestratorLoading,
    error: orchestratorError,
    cancelWorkload,
    retryWorkload,
    refreshData: refreshOrchestratorData,
    clearError: clearOrchestratorError,
  } = useOrchestrator();
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null);
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [showCredit, setShowCredit] = useState(false);
  const [showGovernance, setShowGovernance] = useState(false);
  const [showResources, setShowResources] = useState(false);
  const [showWorkloads, setShowWorkloads] = useState(false);
  const [showNodes, setShowNodes] = useState(false);
  const [showCluster, setShowCluster] = useState(false);
  const [localIdentity, setLocalIdentity] = useState<GeneratedIdentity | null>(null);

  const handleOnboardingComplete = useCallback((identity: GeneratedIdentity) => {
    setLocalIdentity(identity);
    console.log('Identity created:', identity.peerId);
  }, []);

  // Memoize graph data to prevent infinite re-renders
  const { nodes, links } = useMemo(() => graphData() || { nodes: [], links: [] }, [graphData]);

  const selectedPeer: NormalizedPeer | null = selectedPeerId
    ? peers.get(selectedPeerId) || null
    : null;

  // Stable click handler that only uses the node ID
  const handleNodeClick = useCallback((nodeId: string) => {
    setSelectedPeerId(prev => prev === nodeId ? null : nodeId);
  }, []);

  // Handle vouching for a peer
  const handleVouch = useCallback((request: VouchRequest) => {
    console.log('Vouch request:', request);
    sendVouch(request);
  }, [sendVouch]);

  // Handle direct message to a peer
  const handleDirectMessage = useCallback((peerId: string) => {
    setSelectedPeerId(peerId);
    startDMConversation(peerId);
  }, [startDMConversation]);

  // Get active conversation object
  const activeConversation = conversations.get(activeConversationId);

  // Get filtered messages for active conversation
  const activeMessages = useMemo(() => getActiveMessages(), [getActiveMessages]);

  // Handle credit line creation
  const handleCreateCreditLine = useCallback((peerId: string, limit: number) => {
    console.log('Create credit line:', { peerId, limit });
    sendCreditLine(peerId, limit);
  }, [sendCreditLine]);

  // Handle credit transfer
  const handleCreditTransfer = useCallback((transfer: CreditTransfer) => {
    console.log('Credit transfer:', transfer);
    sendCreditTransfer(transfer.to, transfer.amount, transfer.memo);
  }, [sendCreditTransfer]);

  // Handle proposal creation
  const handleCreateProposal = useCallback((proposal: Omit<Proposal, 'id' | 'status' | 'createdAt' | 'votesFor' | 'votesAgainst'>) => {
    console.log('Create proposal:', proposal);
    // expiresAt is milliseconds from epoch, convert to hours from now
    const expiresInHours = Math.max(1, Math.floor((proposal.expiresAt - Date.now()) / (60 * 60 * 1000)));
    sendProposal(proposal.title, proposal.description, expiresInHours);
  }, [sendProposal]);

  // Handle voting on a proposal
  const handleVote = useCallback((vote: Vote) => {
    console.log('Vote:', vote);
    sendVote(vote.proposalId, vote.vote, vote.weight);
  }, [sendVote]);

  return (
    <div className="min-h-screen bg-void text-mycelium-white">
      {/* Header - univrs.io style nav */}
      <header className="border-b border-border-subtle px-6 py-4 bg-gradient-to-b from-void to-transparent">
        <div className="flex items-center justify-between max-w-7xl mx-auto">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full overflow-hidden flex items-center justify-center">
              <img
                src="/icon.jpg"
                alt="Univrs.io"
                className="w-full h-full object-cover"
              />
            </div>
            <div>
              <h1 className="text-xl font-display font-bold text-mycelium-white">Univrs</h1>
              <p className="text-sm text-soft-gray font-body">Orchestration Platform</p>
            </div>
          </div>
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2">
              <div
                className={`w-2 h-2 rounded-full ${
                  connected ? 'bg-cyan-500/70 animate-pulse' : 'bg-red-500/70'
                }`}
              />
              <span className="text-sm font-display text-soft-gray uppercase tracking-wider">
                {connected ? 'Connected' : 'Disconnected'}
              </span>
            </div>
            <div className="text-sm font-display text-cyan-400/70">
              {peers.size} peer{peers.size !== 1 ? 's' : ''} online
            </div>
            <div className="flex items-center gap-2">
              <div
                className={`w-2 h-2 rounded-full ${
                  orchestratorLoading
                    ? 'bg-amber-500/70 animate-pulse'
                    : orchestratorConnected
                    ? 'bg-purple-500/70'
                    : 'bg-soft-gray'
                }`}
              />
              <span className="text-sm font-display text-soft-gray uppercase tracking-wider">
                {orchestratorLoading ? 'Loading' : orchestratorConnected ? 'Orchestrator' : 'Offline'}
              </span>
            </div>
            <button
              onClick={() => setShowCredit(true)}
              className="btn-outline px-3 py-1.5 rounded-lg text-xs opacity-75 hover:opacity-100 transition-opacity flex items-center gap-1.5"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <line x1="12" y1="1" x2="12" y2="23" />
                <path d="M17 5H9.5a3.5 3.5 0 000 7h5a3.5 3.5 0 010 7H6" />
              </svg>
              Credit
            </button>
            <button
              onClick={() => setShowGovernance(true)}
              className="btn-outline px-3 py-1.5 rounded-lg text-xs opacity-75 hover:opacity-100 transition-opacity flex items-center gap-1.5"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M3 21h18M3 7v1a3 3 0 006 0V7M9 7v1a3 3 0 006 0V7M15 7v1a3 3 0 006 0V7M3 4h18v3H3z" />
                <path d="M5 21V11M12 21V11M19 21V11" />
              </svg>
              Govern
            </button>
            <button
              onClick={() => setShowResources(true)}
              className="btn-outline px-3 py-1.5 rounded-lg text-xs opacity-75 hover:opacity-100 transition-opacity flex items-center gap-1.5"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="3" />
                <path d="M12 1v6M12 17v6M4.22 4.22l4.24 4.24M15.54 15.54l4.24 4.24M1 12h6M17 12h6M4.22 19.78l4.24-4.24M15.54 8.46l4.24-4.24" />
              </svg>
              Resources
            </button>
            <button
              onClick={() => setShowWorkloads(true)}
              className="btn-outline px-3 py-1.5 rounded-lg text-xs opacity-75 hover:opacity-100 transition-opacity flex items-center gap-1.5"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="3" y="3" width="18" height="18" rx="2" />
                <path d="M9 3v18M15 3v18M3 9h18M3 15h18" />
              </svg>
              Workloads
            </button>
            <button
              onClick={() => setShowNodes(true)}
              className="btn-outline px-3 py-1.5 rounded-lg text-xs opacity-75 hover:opacity-100 transition-opacity flex items-center gap-1.5"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="2" y="2" width="20" height="8" rx="2" />
                <rect x="2" y="14" width="20" height="8" rx="2" />
                <circle cx="6" cy="6" r="1" fill="currentColor" />
                <circle cx="6" cy="18" r="1" fill="currentColor" />
              </svg>
              Nodes
            </button>
            <button
              onClick={() => setShowCluster(true)}
              className="btn-outline px-3 py-1.5 rounded-lg text-xs opacity-75 hover:opacity-100 transition-opacity flex items-center gap-1.5"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="10" />
                <circle cx="12" cy="12" r="6" />
                <circle cx="12" cy="12" r="2" />
              </svg>
              Cluster
            </button>
            <button
              onClick={() => setShowOnboarding(true)}
              className="btn-outline px-3 py-1.5 rounded-lg text-xs opacity-75 hover:opacity-100 transition-opacity"
            >
              {localIdentity ? 'New Identity' : 'Join Network'}
            </button>
            <ThemeToggle theme={theme} onToggle={toggleTheme} />
          </div>
        </div>
      </header>

      {/* Orchestrator Error Banner */}
      {orchestratorError && (
        <div className="bg-red-500/10 border-b border-red-500/30 px-6 py-3">
          <div className="max-w-7xl mx-auto flex items-center justify-between">
            <div className="flex items-center gap-3">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-red-400">
                <circle cx="12" cy="12" r="10" />
                <line x1="12" y1="8" x2="12" y2="12" />
                <line x1="12" y1="16" x2="12.01" y2="16" />
              </svg>
              <span className="text-sm text-red-400">{orchestratorError}</span>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={refreshOrchestratorData}
                className="text-xs px-3 py-1 rounded bg-red-500/20 text-red-400 hover:bg-red-500/30 transition-colors"
              >
                Retry
              </button>
              <button
                onClick={clearOrchestratorError}
                className="text-red-400 hover:text-red-300 transition-colors"
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M18 6L6 18M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>
        </div>
      )}

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
                localPeerId={localPeerId}
                onClose={() => setSelectedPeerId(null)}
                onVouch={handleVouch}
                onMessage={handleDirectMessage}
              />
            </div>

            {/* Chat Section with Conversation Sidebar */}
            <div className="flex-1 min-h-[300px] flex gap-4">
              {/* Conversation Sidebar */}
              <div className="w-64 flex-shrink-0">
                <ConversationSidebar
                  conversations={conversations}
                  rooms={rooms}
                  activeConversationId={activeConversationId}
                  onSelectConversation={setActiveConversation}
                  onCreateRoom={createRoom}
                  onJoinRoom={joinRoom}
                  onLeaveRoom={leaveRoom}
                />
              </div>
              {/* Chat Panel */}
              <div className="flex-1">
                <ChatPanel
                  messages={activeMessages}
                  onSendMessage={sendChat}
                  activeConversation={activeConversation}
                  selectedPeer={selectedPeerId}
                />
              </div>
            </div>
          </div>
        </div>
      </main>

      {/* Onboarding Modal */}
      {showOnboarding && (
        <OnboardingPanel
          onComplete={handleOnboardingComplete}
          onClose={() => setShowOnboarding(false)}
        />
      )}

      {/* Credit Panel Modal */}
      {showCredit && (
        <CreditPanel
          localPeerId={localPeerId}
          peers={peers}
          creditLines={creditLines}
          creditTransfers={creditTransfers}
          onCreateCreditLine={handleCreateCreditLine}
          onTransfer={handleCreditTransfer}
          onClose={() => setShowCredit(false)}
        />
      )}

      {/* Governance Panel Modal */}
      {showGovernance && (
        <GovernancePanel
          localPeerId={localPeerId}
          peers={peers}
          proposals={proposals}
          onCreateProposal={handleCreateProposal}
          onVote={handleVote}
          onClose={() => setShowGovernance(false)}
        />
      )}

      {/* Resource Panel Modal */}
      {showResources && (
        <ResourcePanel
          localPeerId={localPeerId}
          peers={peers}
          resourcePool={resourcePool}
          onClose={() => setShowResources(false)}
        />
      )}

      {/* Workload List Modal */}
      {showWorkloads && (
        <WorkloadList
          workloads={workloads}
          onCancelWorkload={cancelWorkload}
          onRetryWorkload={retryWorkload}
          onClose={() => setShowWorkloads(false)}
        />
      )}

      {/* Node Status Modal */}
      {showNodes && (
        <NodeStatus
          nodes={orchestratorNodes}
          onClose={() => setShowNodes(false)}
        />
      )}

      {/* Cluster Overview Modal */}
      {showCluster && (
        <ClusterOverview
          clusterMetrics={clusterMetrics}
          nodes={orchestratorNodes}
          workloads={workloads}
          onClose={() => setShowCluster(false)}
        />
      )}
    </div>
  );
}

export default App;
