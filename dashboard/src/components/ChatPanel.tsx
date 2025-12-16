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
    <div className="flex flex-col h-full bg-surface rounded-lg">
      {/* Header */}
      <div className="px-4 py-3 border-b border-gray-800">
        <h3 className="text-lg font-semibold text-white">
          {selectedPeer ? `Chat with ${selectedPeer.slice(0, 8)}...` : 'Network Chat'}
        </h3>
        <p className="text-sm text-gray-400">
          {selectedPeer ? 'Direct message' : 'Broadcasting to all peers'}
        </p>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {messages.length === 0 ? (
          <div className="text-center text-gray-500 py-8">
            No messages yet. Start the conversation!
          </div>
        ) : (
          messages.map((msg) => (
            <div
              key={msg.id}
              className="group flex flex-col"
            >
              <div className="flex items-baseline gap-2">
                <span className="font-medium text-mycelial-400">
                  {msg.from_name || msg.from.slice(0, 8)}
                </span>
                <span className="text-xs text-gray-500">
                  {formatTime(msg.timestamp)}
                </span>
                {msg.to && (
                  <span className="text-xs text-gray-600">
                    → {msg.to.slice(0, 8)}
                  </span>
                )}
              </div>
              <p className="text-gray-200 break-words">{msg.content}</p>
            </div>
          ))
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <form onSubmit={handleSubmit} className="p-4 border-t border-gray-800">
        <div className="flex gap-2">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={selectedPeer ? 'Send direct message...' : 'Broadcast message...'}
            className="flex-1 bg-surface-light border border-gray-700 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-mycelial-500 focus:ring-1 focus:ring-mycelial-500"
          />
          <button
            type="submit"
            disabled={!input.trim()}
            className="px-4 py-2 bg-mycelial-600 hover:bg-mycelial-500 disabled:bg-gray-700 disabled:cursor-not-allowed text-white rounded-lg transition-colors"
          >
            Send
          </button>
        </div>
      </form>
    </div>
  );
}
