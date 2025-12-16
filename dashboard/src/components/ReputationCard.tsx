import type { NormalizedPeer } from '@/types';

interface ReputationCardProps {
  peer: NormalizedPeer | null;
  onClose?: () => void;
}

const tierConfig = {
  excellent: { emoji: '⭐', label: 'Excellent', color: 'text-green-400' },
  good: { emoji: '✅', label: 'Good', color: 'text-lime-400' },
  neutral: { emoji: '➖', label: 'Neutral', color: 'text-gray-400' },
  poor: { emoji: '⚠️', label: 'Poor', color: 'text-amber-400' },
  untrusted: { emoji: '🚫', label: 'Untrusted', color: 'text-red-400' },
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
      <div className="bg-surface rounded-lg p-6 text-center text-gray-500">
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
        return `📍 ${peer.location.latitude?.toFixed(2)}, ${peer.location.longitude?.toFixed(2)}`;
      case 'logical':
        return `🌐 ${peer.location.region}`;
      case 'approximate':
        return `🏙️ ${peer.location.city || ''} ${peer.location.country_code}`;
      default:
        return 'Unknown';
    }
  };

  return (
    <div className="bg-surface rounded-lg overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 bg-surface-light border-b border-gray-800 flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold text-white">{peer.name}</h3>
          <p className="text-sm text-gray-400 font-mono">{peer.id.slice(0, 16)}...</p>
        </div>
        {onClose && (
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white"
          >
            ✕
          </button>
        )}
      </div>

      {/* Content */}
      <div className="p-6 space-y-4">
        {/* Reputation Score */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-400">Reputation</span>
            <span className={`font-medium ${tier.color}`}>
              {tier.emoji} {tier.label}
            </span>
          </div>
          <div className="h-3 bg-gray-800 rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-mycelial-600 to-mycelial-400 transition-all duration-500"
              style={{ width: `${scorePercent}%` }}
            />
          </div>
          <div className="mt-1 text-right text-sm text-gray-400">
            {scorePercent}%
          </div>
        </div>

        {/* Location */}
        <div className="flex items-center gap-2 text-gray-400">
          <span>{formatLocation()}</span>
        </div>

        {/* Addresses */}
        {peer.addresses.length > 0 && (
          <div className="text-sm text-gray-500">
            <div className="text-gray-400 mb-1">Addresses:</div>
            <div className="font-mono text-xs break-all">
              {peer.addresses[0]}
              {peer.addresses.length > 1 && (
                <span className="text-gray-600"> +{peer.addresses.length - 1} more</span>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
