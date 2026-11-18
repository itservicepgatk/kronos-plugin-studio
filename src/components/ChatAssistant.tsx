import React, { useState, useRef, useEffect } from 'react';
import { 
  Send, 
  Bot, 
  User, 
  Sparkles, 
  CheckCheck, 
  FileCode, 
  Trash2
} from 'lucide-react';
import { ChatMessage, GeminiModel } from '../types';

interface ChatAssistantProps {
  messages: ChatMessage[];
  isLoading: boolean;
  onSendMessage: (text: string) => void;
  onApplyFiles: (files: Record<string, string>) => void;
  onClearChat: () => void;
  model: GeminiModel;
}

export const ChatAssistant: React.FC<ChatAssistantProps> = ({
  messages,
  isLoading,
  onSendMessage,
  onApplyFiles,
  onClearChat,
  model,
}) => {
  const [input, setInput] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages, isLoading]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (input.trim() && !isLoading) {
      onSendMessage(input.trim());
      setInput('');
    }
  };

  const quickPrompts = [
    "Add /spawn and /setspawn commands",
    "Add VIP and Owner ranks with tab prefixes",
    "Add inventory GUI menu for /shop",
    "Add auto-save player data to JSON file",
  ];

  return (
    <div className="w-96 border-l border-[#272730] bg-[#141418] flex flex-col h-full select-none text-xs">
      {/* Header */}
      <div className="px-3 py-2.5 border-b border-[#272730] flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Sparkles className="w-4 h-4 text-amber-400" />
          <span className="font-semibold text-gray-200">AI Architect ({model})</span>
        </div>
        <button
          onClick={onClearChat}
          className="p-1 text-gray-500 hover:text-gray-300 rounded"
          title="Clear Chat History"
        >
          <Trash2 className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* Messages Scroll Area */}
      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        {messages.map((msg) => {
          const isUser = msg.role === 'user';
          const hasFiles = msg.extractedFiles && Object.keys(msg.extractedFiles).length > 0;

          return (
            <div
              key={msg.id}
              className={`flex flex-col gap-1.5 ${isUser ? 'items-end' : 'items-start'}`}
            >
              <div className="flex items-center gap-1.5 text-[10px] text-gray-500 font-mono">
                {isUser ? <User className="w-3 h-3" /> : <Bot className="w-3 h-3 text-amber-400" />}
                <span>{isUser ? 'You' : 'Gemini Architect'}</span>
              </div>

              <div
                className={`max-w-[90%] p-3 rounded-xl leading-relaxed whitespace-pre-wrap ${
                  isUser
                    ? 'bg-amber-500/10 border border-amber-500/20 text-gray-200'
                    : 'bg-[#1b1b22] border border-[#2d2d38] text-gray-300'
                }`}
              >
                {msg.content}

                {/* If AI proposed file updates, display one-click Apply button */}
                {hasFiles && (
                  <div className="mt-3 pt-2.5 border-t border-[#313140] flex flex-col gap-2">
                    <div className="text-[11px] font-semibold text-amber-300 flex items-center gap-1">
                      <FileCode className="w-3.5 h-3.5" />
                      Files updated ({Object.keys(msg.extractedFiles!).length}):
                    </div>
                    <ul className="text-[11px] font-mono text-gray-400 list-disc list-inside">
                      {Object.keys(msg.extractedFiles!).map((path) => (
                        <li key={path}>{path}</li>
                      ))}
                    </ul>
                    <button
                      onClick={() => onApplyFiles(msg.extractedFiles!)}
                      className="mt-1 flex items-center justify-center gap-1.5 w-full py-1.5 bg-amber-500 hover:bg-amber-400 text-black font-semibold text-xs rounded-lg transition-colors shadow-sm"
                    >
                      <CheckCheck className="w-4 h-4" />
                      Apply All to Workspace
                    </button>
                  </div>
                )}
              </div>
            </div>
          );
        })}

        {isLoading && (
          <div className="flex items-center gap-2 text-gray-400 text-xs py-2">
            <Sparkles className="w-4 h-4 text-amber-400 animate-spin" />
            <span>Reasoning with Gemini...</span>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Quick Prompts */}
      <div className="px-3 py-1.5 border-t border-[#272730] bg-[#121216] flex flex-wrap gap-1">
        {quickPrompts.map((q) => (
          <button
            key={q}
            onClick={() => onSendMessage(q)}
            disabled={isLoading}
            className="text-[10px] bg-[#1c1c24] hover:bg-[#252532] text-gray-400 hover:text-amber-300 px-2 py-0.5 rounded border border-[#2d2d38] transition-colors truncate max-w-full"
          >
            + {q}
          </button>
        ))}
      </div>

      {/* Input Bar */}
      <form onSubmit={handleSubmit} className="p-3 border-t border-[#272730] bg-[#141418] flex gap-2">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Describe feature, command, or bug to fix..."
          disabled={isLoading}
          className="flex-1 bg-[#1b1b22] border border-[#2d2d38] rounded-lg px-3 py-2 text-xs text-gray-200 placeholder-gray-500 focus:outline-none focus:border-amber-500"
        />
        <button
          type="submit"
          disabled={isLoading || !input.trim()}
          className="p-2 bg-amber-500 hover:bg-amber-400 disabled:opacity-40 disabled:hover:bg-amber-500 text-black rounded-lg transition-colors"
        >
          <Send className="w-4 h-4" />
        </button>
      </form>
    </div>
  );
};
