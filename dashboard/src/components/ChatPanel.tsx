import { useState, useRef, useEffect } from 'react';
import type { ChatMessage } from '@/types';

interface ChatPanelProps {
  messages: ChatMessage[];
  onSendMessage: (content: string, to?: string) => void;
  selectedPeer?: string | null;
}

export function ChatPanel({ messages, onSendMessage, selectedPeer }: ChatPanelProps) {
  const [input, setInput] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;
    console.log('ChatPanel handleSubmit:', { input: input.trim(), selectedPeer });
    onSendMessage(input.trim(), selectedPeer || undefined);
    setInput('');
  };

  const formatTime = (timestamp: number) => {
    return new Date(timestamp).toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return (
    <div className="flex flex-col h-full bg-forest-floor border border-border-subtle rounded-lg card-hover">
      {/* Header */}
      <div className="px-4 py-3 border-b border-border-subtle bg-deep-earth rounded-t-lg">
        <h3 className="text-lg font-display font-semibold text-mycelium-white">
          {selectedPeer ? `Chat with ${selectedPeer.slice(0, 8)}...` : 'Network Chat'}
        </h3>
        <p className="text-sm font-body text-soft-gray">
          {selectedPeer ? 'Direct message' : 'Broadcasting to all peers'}
        </p>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {messages.length === 0 ? (
          <div className="text-center text-soft-gray py-8 font-body italic">
            No messages yet. Start the conversation!
          </div>
        ) : (
          messages.map((msg) => (
            <div
              key={msg.id}
              className="group flex flex-col"
            >
              <div className="flex items-baseline gap-2">
                <span className="font-display font-medium text-glow-cyan">
                  {msg.from_name || msg.from.slice(0, 8)}
                </span>
                <span className="text-xs text-soft-gray font-mono">
                  {formatTime(msg.timestamp)}
                </span>
                {msg.to && (
                  <span className="text-xs text-spore-purple font-mono">
                    → {msg.to.slice(0, 8)}
                  </span>
                )}
              </div>
              <p className="text-mycelium-white font-body break-words">{msg.content}</p>
            </div>
          ))
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <form onSubmit={handleSubmit} className="p-4 border-t border-border-subtle">
        <div className="flex gap-2">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={selectedPeer ? 'Send direct message...' : 'Broadcast message...'}
            className="flex-1 bg-moss border border-border-subtle rounded px-4 py-2 text-mycelium-white placeholder-soft-gray font-body focus:outline-none focus:border-glow-cyan focus:shadow-glow-sm transition-all"
          />
          <button
            type="submit"
            disabled={!input.trim()}
            className="btn-primary px-4 py-2 rounded disabled:opacity-50 disabled:cursor-not-allowed disabled:shadow-none"
          >
            Send
          </button>
        </div>
      </form>
    </div>
  );
}
