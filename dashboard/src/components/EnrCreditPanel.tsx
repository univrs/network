// src/components/EnrCreditPanel.tsx
// Displays ENR credit balances and transfers across the network

import { useMemo, useState } from 'react';
import type { NodeEnrState, EnrCreditTransfer } from '@/types';

interface EnrCreditPanelProps {
  nodeEnrStates: Map<string, NodeEnrState>;
  enrTransfers: EnrCreditTransfer[];
  localPeerId?: string | null;
  onClose?: () => void;
  // New action prop
  onSendEnrCredit?: (to: string, amount: number) => void;
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

function formatBalance(amount: number): string {
  if (amount >= 1_000_000) {
    return `${(amount / 1_000_000).toFixed(2)}M`;
  }
  if (amount >= 1_000) {
    return `${(amount / 1_000).toFixed(1)}K`;
  }
  return amount.toLocaleString();
}

function TransferCard({ transfer, localPeerId }: {
  transfer: EnrCreditTransfer;
  localPeerId?: string | null;
}) {
  const isOutgoing = transfer.from === localPeerId;
  const isIncoming = transfer.to === localPeerId;

  return (
    <div className={`p-3 rounded-lg ${
      isIncoming ? 'bg-glow-cyan/10 border border-glow-cyan/20' :
      isOutgoing ? 'bg-spore-purple/10 border border-spore-purple/20' :
      'bg-moss'
    }`}>
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <div className={`w-8 h-8 rounded-full flex items-center justify-center ${
            isIncoming ? 'bg-glow-cyan/20' :
            isOutgoing ? 'bg-spore-purple/20' :
            'bg-bark'
          }`}>
            {isIncoming ? (
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-glow-cyan">
                <path d="M12 19V5M5 12l7 7 7-7" />
              </svg>
            ) : isOutgoing ? (
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-spore-purple">
                <path d="M12 5v14M5 12l7-7 7 7" />
              </svg>
            ) : (
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-soft-gray">
                <path d="M7 17L17 7M7 7h10v10" />
              </svg>
            )}
          </div>
          <div>
            <div className="text-xs text-soft-gray">
              {shortenNodeId(transfer.from)} → {shortenNodeId(transfer.to)}
            </div>
            <div className="text-xs text-soft-gray/60">
              {formatTimestamp(transfer.timestamp)}
            </div>
          </div>
        </div>
        <div className="text-right">
          <div className={`font-display font-bold ${
            isIncoming ? 'text-glow-cyan' :
            isOutgoing ? 'text-spore-purple' :
            'text-mycelium-white'
          }`}>
            {isIncoming ? '+' : isOutgoing ? '-' : ''}{formatBalance(transfer.amount)}
          </div>
          {transfer.tax > 0 && (
            <div className="text-xs text-soft-gray">
              Tax: {formatBalance(transfer.tax)}
            </div>
          )}
        </div>
      </div>
      <div className="text-xs text-soft-gray/60">
        Nonce: {transfer.nonce}
      </div>
    </div>
  );
}

function NodeBalanceCard({ state, rank }: {
  state: NodeEnrState;
  rank: number;
}) {
  const isTopNode = rank <= 3;

  return (
    <div className={`p-3 rounded-lg flex items-center justify-between ${
      isTopNode ? 'bg-glow-gold/10 border border-glow-gold/20' : 'bg-moss'
    }`}>
      <div className="flex items-center gap-3">
        <div className={`w-8 h-8 rounded-full flex items-center justify-center font-display font-bold text-sm ${
          rank === 1 ? 'bg-glow-gold/30 text-glow-gold' :
          rank === 2 ? 'bg-soft-gray/30 text-soft-gray' :
          rank === 3 ? 'bg-spore-purple/30 text-spore-purple' :
          'bg-bark text-soft-gray'
        }`}>
          {rank}
        </div>
        <div>
          <div className="font-display text-mycelium-white text-sm">
            {shortenNodeId(state.nodeId)}
          </div>
          <div className="text-xs text-soft-gray">
            Updated {formatTimestamp(state.lastUpdated)}
          </div>
        </div>
      </div>
      <div className="text-right">
        <div className={`font-display font-bold ${isTopNode ? 'text-glow-gold' : 'text-mycelium-white'}`}>
          {formatBalance(state.balance)}
        </div>
        <div className={`text-xs ${
          state.septalState === 'closed' ? 'text-glow-cyan' :
          state.septalState === 'half_open' ? 'text-glow-gold' :
          'text-red-400'
        }`}>
          {state.septalState}
        </div>
      </div>
    </div>
  );
}

export function EnrCreditPanel({
  nodeEnrStates,
  enrTransfers,
  localPeerId,
  onClose,
  onSendEnrCredit,
}: EnrCreditPanelProps) {
  const [showTransferForm, setShowTransferForm] = useState(false);
  const [transferTo, setTransferTo] = useState('');
  const [transferAmount, setTransferAmount] = useState('');

  // Calculate network statistics
  const stats = useMemo(() => {
    const nodes = Array.from(nodeEnrStates.values());
    const totalBalance = nodes.reduce((acc, n) => acc + n.balance, 0);
    const avgBalance = nodes.length > 0 ? totalBalance / nodes.length : 0;
    const maxBalance = nodes.length > 0 ? Math.max(...nodes.map(n => n.balance)) : 0;
    const minBalance = nodes.length > 0 ? Math.min(...nodes.map(n => n.balance)) : 0;

    const totalTransferred = enrTransfers.reduce((acc, t) => acc + t.amount, 0);
    const totalTax = enrTransfers.reduce((acc, t) => acc + t.tax, 0);

    return {
      nodeCount: nodes.length,
      totalBalance,
      avgBalance,
      maxBalance,
      minBalance,
      transferCount: enrTransfers.length,
      totalTransferred,
      totalTax,
    };
  }, [nodeEnrStates, enrTransfers]);

  // Sort nodes by balance for leaderboard
  const sortedNodes = useMemo(() => {
    return Array.from(nodeEnrStates.values())
      .sort((a, b) => b.balance - a.balance);
  }, [nodeEnrStates]);

  // Recent transfers (most recent first)
  const recentTransfers = useMemo(() => {
    return [...enrTransfers]
      .sort((a, b) => b.timestamp - a.timestamp)
      .slice(0, 20);
  }, [enrTransfers]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-void/80 backdrop-blur-sm">
      <div className="w-full max-w-4xl max-h-[90vh] bg-forest-floor border border-border-subtle rounded-xl shadow-card overflow-hidden">
        {/* Header */}
        <div className="relative px-6 py-4 bg-deep-earth border-b border-border-subtle">
          <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-glow-gold via-spore-purple to-glow-cyan" />
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div>
                <h2 className="text-xl font-display font-bold text-mycelium-white">
                  ENR Credits
                </h2>
                <p className="text-sm text-soft-gray font-body">
                  Network resource credits and transfers
                </p>
              </div>
              {onSendEnrCredit && (
                <button
                  onClick={() => setShowTransferForm(true)}
                  className="px-3 py-1.5 rounded-lg bg-spore-purple/20 text-spore-purple hover:bg-spore-purple/30 transition-colors text-sm font-display flex items-center gap-1.5"
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M12 5v14M5 12h14" />
                  </svg>
                  Send Credits
                </button>
              )}
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
          {nodeEnrStates.size === 0 && enrTransfers.length === 0 ? (
            <div className="text-center py-12">
              <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-moss flex items-center justify-center">
                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-soft-gray">
                  <circle cx="12" cy="12" r="10" />
                  <path d="M16 8h-6a2 2 0 00-2 2v4a2 2 0 002 2h6" />
                  <path d="M12 12h4" />
                </svg>
              </div>
              <p className="text-soft-gray">
                No ENR credit data available
              </p>
              <p className="text-xs text-soft-gray/60 mt-2">
                Credit balances and transfers will appear as nodes participate in the network
              </p>
            </div>
          ) : (
            <div className="space-y-6">
              {/* Network Overview */}
              <div className="p-4 bg-moss rounded-lg">
                <h3 className="text-sm font-display uppercase tracking-wider text-soft-gray mb-4">
                  Network Credit Summary
                </h3>

                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  {/* Total Balance */}
                  <div className="text-center p-3 bg-bark rounded-lg">
                    <div className="text-2xl font-display text-glow-gold">
                      {formatBalance(stats.totalBalance)}
                    </div>
                    <div className="text-xs text-soft-gray mt-1">Total Balance</div>
                  </div>

                  {/* Average Balance */}
                  <div className="text-center p-3 bg-bark rounded-lg">
                    <div className="text-2xl font-display text-mycelium-white">
                      {formatBalance(Math.round(stats.avgBalance))}
                    </div>
                    <div className="text-xs text-soft-gray mt-1">Avg Balance</div>
                  </div>

                  {/* Total Transferred */}
                  <div className="text-center p-3 bg-bark rounded-lg">
                    <div className="text-2xl font-display text-spore-purple">
                      {formatBalance(stats.totalTransferred)}
                    </div>
                    <div className="text-xs text-soft-gray mt-1">Transferred</div>
                  </div>

                  {/* Tax Collected */}
                  <div className="text-center p-3 bg-bark rounded-lg">
                    <div className="text-2xl font-display text-glow-cyan">
                      {formatBalance(stats.totalTax)}
                    </div>
                    <div className="text-xs text-soft-gray mt-1">Tax Collected</div>
                  </div>
                </div>

                {/* Balance range indicator */}
                {stats.nodeCount > 0 && (
                  <div className="mt-4">
                    <div className="flex justify-between text-xs text-soft-gray mb-1">
                      <span>Min: {formatBalance(stats.minBalance)}</span>
                      <span>Max: {formatBalance(stats.maxBalance)}</span>
                    </div>
                    <div className="h-2 bg-bark rounded-full overflow-hidden">
                      <div
                        className="h-full bg-gradient-to-r from-spore-purple via-glow-gold to-glow-cyan"
                        style={{ width: '100%' }}
                      />
                    </div>
                  </div>
                )}
              </div>

              {/* Two-column layout */}
              <div className="grid md:grid-cols-2 gap-6">
                {/* Node Balances Leaderboard */}
                <div>
                  <h3 className="text-sm font-display uppercase tracking-wider text-soft-gray mb-3">
                    Node Balances ({stats.nodeCount})
                  </h3>
                  <div className="space-y-2 max-h-80 overflow-y-auto">
                    {sortedNodes.map((state, index) => (
                      <NodeBalanceCard
                        key={state.nodeId}
                        state={state}
                        rank={index + 1}
                      />
                    ))}
                  </div>
                </div>

                {/* Recent Transfers */}
                <div>
                  <h3 className="text-sm font-display uppercase tracking-wider text-soft-gray mb-3">
                    Recent Transfers ({stats.transferCount})
                  </h3>
                  {recentTransfers.length === 0 ? (
                    <div className="text-center py-8 bg-moss rounded-lg">
                      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-soft-gray mx-auto mb-2">
                        <path d="M7 17L17 7M7 7h10v10" />
                      </svg>
                      <p className="text-sm text-soft-gray">No transfers yet</p>
                    </div>
                  ) : (
                    <div className="space-y-2 max-h-80 overflow-y-auto">
                      {recentTransfers.map((transfer, index) => (
                        <TransferCard
                          key={`${transfer.from}-${transfer.to}-${transfer.nonce}-${index}`}
                          transfer={transfer}
                          localPeerId={localPeerId}
                        />
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Transfer Form Modal */}
      {showTransferForm && (
        <div className="fixed inset-0 z-60 flex items-center justify-center bg-void/80 backdrop-blur-sm">
          <div className="w-full max-w-md bg-forest-floor border border-border-subtle rounded-xl p-6">
            <h3 className="text-lg font-display text-mycelium-white mb-4">Send ENR Credits</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-sm text-soft-gray mb-1">Recipient Node ID</label>
                <input
                  type="text"
                  value={transferTo}
                  onChange={(e) => setTransferTo(e.target.value)}
                  placeholder="Enter node ID..."
                  className="w-full px-3 py-2 bg-bark border border-border-subtle rounded-lg text-mycelium-white placeholder:text-soft-gray/50"
                />
              </div>
              <div>
                <label className="block text-sm text-soft-gray mb-1">Amount</label>
                <input
                  type="number"
                  value={transferAmount}
                  onChange={(e) => setTransferAmount(e.target.value)}
                  placeholder="Enter amount..."
                  className="w-full px-3 py-2 bg-bark border border-border-subtle rounded-lg text-mycelium-white placeholder:text-soft-gray/50"
                />
              </div>
              <div className="flex gap-3">
                <button
                  onClick={() => setShowTransferForm(false)}
                  className="flex-1 px-4 py-2 rounded-lg bg-soft-gray/20 text-soft-gray hover:bg-soft-gray/30 transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={() => {
                    if (transferTo && transferAmount && onSendEnrCredit) {
                      onSendEnrCredit(transferTo, parseInt(transferAmount, 10));
                      setShowTransferForm(false);
                      setTransferTo('');
                      setTransferAmount('');
                    }
                  }}
                  className="flex-1 px-4 py-2 rounded-lg bg-glow-cyan/20 text-glow-cyan hover:bg-glow-cyan/30 transition-colors"
                >
                  Send
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
