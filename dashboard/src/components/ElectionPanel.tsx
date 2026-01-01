// src/components/ElectionPanel.tsx
// Displays nexus elections for region coordinator selection

import { useMemo } from 'react';
import type { Election, ElectionCandidacy } from '@/types';

interface ElectionPanelProps {
  elections: Map<number, Election>;
  localPeerId?: string | null;
  onClose?: () => void;
  // Action props for interactive controls
  onStartElection?: (regionId: string) => void;
  onRegisterCandidacy?: (electionId: number, uptime: number, cpuAvailable: number, memoryAvailable: number, reputation: number) => void;
  onVoteElection?: (electionId: number, candidate: string) => void;
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

function getStatusColor(status: Election['status']): string {
  switch (status) {
    case 'announced':
      return 'bg-glow-gold/20 text-glow-gold';
    case 'voting':
      return 'bg-spore-purple/20 text-spore-purple';
    case 'completed':
      return 'bg-glow-cyan/20 text-glow-cyan';
    default:
      return 'bg-soft-gray/20 text-soft-gray';
  }
}

function CandidateCard({ candidate, isWinner, voteCount }: {
  candidate: ElectionCandidacy;
  isWinner: boolean;
  voteCount: number;
}) {
  return (
    <div className={`p-3 rounded-lg ${isWinner ? 'bg-glow-cyan/10 border border-glow-cyan/30' : 'bg-bark'}`}>
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <div className={`w-8 h-8 rounded-full flex items-center justify-center ${
            isWinner ? 'bg-glow-cyan/30' : 'bg-spore-purple/20'
          }`}>
            <span className={`text-xs font-display font-bold ${isWinner ? 'text-glow-cyan' : 'text-spore-purple'}`}>
              {candidate.candidate.slice(0, 2).toUpperCase()}
            </span>
          </div>
          <div>
            <div className="font-display text-mycelium-white text-sm">
              {shortenNodeId(candidate.candidate)}
            </div>
            <div className="text-xs text-soft-gray">
              {voteCount} vote{voteCount !== 1 ? 's' : ''}
            </div>
          </div>
        </div>
        {isWinner && (
          <div className="px-2 py-1 rounded bg-glow-cyan/20 text-glow-cyan text-xs font-display">
            Winner
          </div>
        )}
      </div>

      {/* Candidate qualifications */}
      <div className="grid grid-cols-2 gap-2 text-xs">
        <div className="flex justify-between">
          <span className="text-soft-gray">Uptime:</span>
          <span className="text-mycelium-white">{Math.round(candidate.uptime / 3600)}h</span>
        </div>
        <div className="flex justify-between">
          <span className="text-soft-gray">Reputation:</span>
          <span className="text-mycelium-white">{candidate.reputation}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-soft-gray">CPU:</span>
          <span className="text-mycelium-white">{Math.round(candidate.cpuAvailable * 100)}%</span>
        </div>
        <div className="flex justify-between">
          <span className="text-soft-gray">Memory:</span>
          <span className="text-mycelium-white">{Math.round(candidate.memoryAvailable * 100)}%</span>
        </div>
      </div>
    </div>
  );
}

function ElectionCard({ election, isExpanded, onToggle, onVoteElection, onRegisterCandidacy }: {
  election: Election;
  isExpanded: boolean;
  onToggle: () => void;
  onVoteElection?: (electionId: number, candidate: string) => void;
  onRegisterCandidacy?: (electionId: number, uptime: number, cpuAvailable: number, memoryAvailable: number, reputation: number) => void;
}) {
  const votesByCandidate = useMemo(() => {
    const counts = new Map<string, number>();
    election.votes.forEach(vote => {
      counts.set(vote.candidate, (counts.get(vote.candidate) || 0) + 1);
    });
    return counts;
  }, [election.votes]);

  const totalVotes = election.votes.length;
  const candidateCount = election.candidates.length;

  return (
    <div className="bg-moss rounded-lg overflow-hidden">
      {/* Header - always visible */}
      <button
        onClick={onToggle}
        className="w-full p-4 flex items-center justify-between hover:bg-bark/30 transition-colors"
      >
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-full bg-spore-purple/20 flex items-center justify-center">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-spore-purple">
              <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <div className="text-left">
            <div className="font-display text-mycelium-white">
              Region {election.regionId}
            </div>
            <div className="text-xs text-soft-gray">
              Election #{election.id} • {formatTimestamp(election.startedAt)}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <div className={`px-2 py-1 rounded text-xs ${getStatusColor(election.status)}`}>
            {election.status}
          </div>
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            className={`text-soft-gray transition-transform ${isExpanded ? 'rotate-180' : ''}`}
          >
            <path d="M19 9l-7 7-7-7" />
          </svg>
        </div>
      </button>

      {/* Expanded content */}
      {isExpanded && (
        <div className="px-4 pb-4 space-y-4">
          {/* Stats bar */}
          <div className="flex gap-4 text-xs">
            <div className="flex items-center gap-1">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-soft-gray">
                <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
                <circle cx="9" cy="7" r="4" />
                <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
                <path d="M16 3.13a4 4 0 0 1 0 7.75" />
              </svg>
              <span className="text-soft-gray">{candidateCount} candidate{candidateCount !== 1 ? 's' : ''}</span>
            </div>
            <div className="flex items-center gap-1">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-soft-gray">
                <path d="M9 12l2 2 4-4" />
                <rect x="3" y="3" width="18" height="18" rx="2" />
              </svg>
              <span className="text-soft-gray">{totalVotes} vote{totalVotes !== 1 ? 's' : ''}</span>
            </div>
            <div className="flex items-center gap-1">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-soft-gray">
                <circle cx="12" cy="12" r="10" />
                <polyline points="12 6 12 12 16 14" />
              </svg>
              <span className="text-soft-gray">
                {election.completedAt
                  ? `Completed ${formatTimestamp(election.completedAt)}`
                  : 'In progress'
                }
              </span>
            </div>
          </div>

          {/* Winner display for completed elections */}
          {election.status === 'completed' && election.winner && (
            <div className="p-3 bg-glow-cyan/10 border border-glow-cyan/20 rounded-lg">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-glow-cyan/30 flex items-center justify-center">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-glow-cyan">
                    <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
                  </svg>
                </div>
                <div>
                  <div className="text-xs text-soft-gray uppercase tracking-wider">Elected Coordinator</div>
                  <div className="font-display text-glow-cyan">
                    {shortenNodeId(election.winner)}
                  </div>
                  <div className="text-xs text-soft-gray">
                    {election.voteCount} vote{election.voteCount !== 1 ? 's' : ''}
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Candidates list */}
          {election.candidates.length > 0 && (
            <div>
              <h4 className="text-xs font-display uppercase tracking-wider text-soft-gray mb-2">
                Candidates
              </h4>
              <div className="space-y-2">
                {election.candidates.map(candidate => (
                  <CandidateCard
                    key={candidate.candidate}
                    candidate={candidate}
                    isWinner={election.winner === candidate.candidate}
                    voteCount={votesByCandidate.get(candidate.candidate) || 0}
                  />
                ))}
              </div>
            </div>
          )}

          {/* Vote buttons for active voting elections */}
          {election.status === 'voting' && onVoteElection && election.candidates.length > 0 && (
            <div className="mt-3 flex gap-2">
              {election.candidates.map(candidate => (
                <button
                  key={candidate.candidate}
                  onClick={() => onVoteElection(election.id, candidate.candidate)}
                  className="flex-1 px-3 py-2 rounded bg-glow-cyan/20 text-glow-cyan hover:bg-glow-cyan/30 transition-colors text-xs font-display"
                >
                  Vote for {shortenNodeId(candidate.candidate)}
                </button>
              ))}
            </div>
          )}

          {/* Register as candidate for announced elections */}
          {election.status === 'announced' && onRegisterCandidacy && (
            <button
              onClick={() => onRegisterCandidacy(
                election.id,
                3600 * 24, // 24h uptime placeholder
                0.8,       // 80% CPU available
                0.7,       // 70% memory available
                0.5        // 0.5 reputation
              )}
              className="mt-3 w-full px-3 py-2 rounded bg-glow-gold/20 text-glow-gold hover:bg-glow-gold/30 transition-colors text-sm font-display"
            >
              Register as Candidate
            </button>
          )}

          {/* Initiator info */}
          <div className="text-xs text-soft-gray">
            Initiated by {shortenNodeId(election.initiator)}
          </div>
        </div>
      )}
    </div>
  );
}

export function ElectionPanel({
  elections,
  localPeerId: _localPeerId,
  onClose,
  onStartElection,
  onRegisterCandidacy,
  onVoteElection,
}: ElectionPanelProps) {
  // Separate active and completed elections
  const { activeElections, completedElections } = useMemo(() => {
    const active: Election[] = [];
    const completed: Election[] = [];

    Array.from(elections.values()).forEach(election => {
      if (election.status === 'completed') {
        completed.push(election);
      } else {
        active.push(election);
      }
    });

    // Sort by start time (newest first)
    active.sort((a, b) => b.startedAt - a.startedAt);
    completed.sort((a, b) => (b.completedAt || 0) - (a.completedAt || 0));

    return { activeElections: active, completedElections: completed };
  }, [elections]);

  // Track expanded elections
  const expandedIds = useMemo(() => {
    // Auto-expand active elections
    return new Set(activeElections.map(e => e.id));
  }, [activeElections]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-void/80 backdrop-blur-sm">
      <div className="w-full max-w-3xl max-h-[90vh] bg-forest-floor border border-border-subtle rounded-xl shadow-card overflow-hidden">
        {/* Header */}
        <div className="relative px-6 py-4 bg-deep-earth border-b border-border-subtle">
          <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-spore-purple via-glow-cyan to-glow-gold" />
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div>
                <h2 className="text-xl font-display font-bold text-mycelium-white">
                  Nexus Elections
                </h2>
                <p className="text-sm text-soft-gray font-body">
                  Region coordinator selection and voting
                </p>
              </div>
              {onStartElection && (
                <button
                  onClick={() => {
                    const regionId = prompt('Enter region ID for election:');
                    if (regionId) onStartElection(regionId);
                  }}
                  className="px-3 py-1.5 rounded-lg bg-spore-purple/20 text-spore-purple hover:bg-spore-purple/30 transition-colors text-sm font-display flex items-center gap-1.5"
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M12 5v14M5 12h14" />
                  </svg>
                  Start Election
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
          {elections.size === 0 ? (
            <div className="text-center py-12">
              <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-moss flex items-center justify-center">
                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-soft-gray">
                  <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              </div>
              <p className="text-soft-gray">
                No elections in progress
              </p>
              <p className="text-xs text-soft-gray/60 mt-2">
                Elections are initiated when a region needs a new coordinator
              </p>
            </div>
          ) : (
            <div className="space-y-6">
              {/* Active Elections */}
              {activeElections.length > 0 && (
                <div>
                  <div className="flex items-center gap-2 mb-3">
                    <div className="w-2 h-2 rounded-full bg-glow-gold animate-pulse" />
                    <h3 className="text-sm font-display uppercase tracking-wider text-soft-gray">
                      Active Elections ({activeElections.length})
                    </h3>
                  </div>
                  <div className="space-y-3">
                    {activeElections.map(election => (
                      <ElectionCard
                        key={election.id}
                        election={election}
                        isExpanded={expandedIds.has(election.id)}
                        onToggle={() => {}} // Active elections stay expanded
                        onVoteElection={onVoteElection}
                        onRegisterCandidacy={onRegisterCandidacy}
                      />
                    ))}
                  </div>
                </div>
              )}

              {/* Completed Elections */}
              {completedElections.length > 0 && (
                <div>
                  <h3 className="text-sm font-display uppercase tracking-wider text-soft-gray mb-3">
                    Recent Elections ({completedElections.length})
                  </h3>
                  <div className="space-y-3">
                    {completedElections.slice(0, 10).map(election => (
                      <ElectionCard
                        key={election.id}
                        election={election}
                        isExpanded={false}
                        onToggle={() => {}} // Can add state for manual expand
                        onVoteElection={onVoteElection}
                        onRegisterCandidacy={onRegisterCandidacy}
                      />
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
