import type { NormalizedPeer } from '@/types';

interface ReputationCardProps {
  peer: NormalizedPeer | null;
  onClose?: () => void;
}

const tierConfig = {
  excellent: { label: 'Excellent', color: 'text-glow-cyan', bgColor: 'bg-glow-cyan' },
  good: { label: 'Good', color: 'text-glow-gold', bgColor: 'bg-glow-gold' },
  neutral: { label: 'Neutral', color: 'text-soft-gray', bgColor: 'bg-soft-gray' },
  poor: { label: 'Poor', color: 'text-amber-400', bgColor: 'bg-amber-400' },
  untrusted: { label: 'Untrusted', color: 'text-red-400', bgColor: 'bg-red-400' },
};

// Derive tier from score
function getTierFromScore(score: number): keyof typeof tierConfig {
  if (score >= 0.9) return 'excellent';
  if (score >= 0.7) return 'good';
  if (score >= 0.5) return 'neutral';
  if (score >= 0.3) return 'poor';
  return 'untrusted';
}

export function ReputationCard({ peer, onClose }: ReputationCardProps) {
  if (!peer) {
    return (
      <div className="bg-forest-floor border border-border-subtle rounded-lg p-6 text-center text-soft-gray font-body italic">
        Select a peer to view details
      </div>
    );
  }

  const tierKey = getTierFromScore(peer.reputation);
  const tier = tierConfig[tierKey];
  const scorePercent = Math.round(peer.reputation * 100);

  const formatLocation = () => {
    if (!peer.location) return 'Location not shared';

    switch (peer.location.type) {
      case 'geographic':
        return `${peer.location.latitude?.toFixed(2)}, ${peer.location.longitude?.toFixed(2)}`;
      case 'logical':
        return `${peer.location.region}`;
      case 'approximate':
        return `${peer.location.city || ''} ${peer.location.country_code}`;
      default:
        return 'Unknown';
    }
  };

  return (
    <div className="bg-forest-floor border border-border-subtle rounded-lg overflow-hidden card-hover">
      {/* Header with gradient accent */}
      <div className="relative px-6 py-4 bg-deep-earth border-b border-border-subtle">
        <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-glow-cyan via-glow-gold to-spore-purple" />
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-lg font-display font-semibold text-mycelium-white">{peer.name}</h3>
            <p className="text-sm text-soft-gray font-mono">{peer.id.slice(0, 16)}...</p>
          </div>
          {onClose && (
            <button
              onClick={onClose}
              className="text-soft-gray hover:text-glow-cyan transition-colors"
            >
              ✕
            </button>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="p-6 space-y-4">
        {/* Reputation Score */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-display uppercase tracking-wider text-soft-gray">Reputation</span>
            <span className={`font-display font-semibold ${tier.color}`}>
              {tier.label}
            </span>
          </div>
          <div className="h-3 bg-bark rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-glow-cyan to-glow-gold transition-all duration-500 shadow-glow-sm"
              style={{ width: `${scorePercent}%` }}
            />
          </div>
          <div className="mt-1 text-right text-sm font-display text-glow-cyan">
            {scorePercent}%
          </div>
        </div>

        {/* Location */}
        <div className="flex items-center gap-2 text-soft-gray font-body">
          <span className="text-spore-purple">Location:</span>
          <span>{formatLocation()}</span>
        </div>

        {/* Addresses */}
        {peer.addresses.length > 0 && (
          <div className="text-sm">
            <div className="text-spore-purple font-display text-xs uppercase tracking-wider mb-1">Addresses</div>
            <div className="font-mono text-xs text-soft-gray break-all">
              {peer.addresses[0]}
              {peer.addresses.length > 1 && (
                <span className="text-bark"> +{peer.addresses.length - 1} more</span>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
