import React, { useState } from 'react';
import { Key, ExternalLink, X, Check } from 'lucide-react';

interface ApiKeyModalProps {
  isOpen: boolean;
  onClose: () => void;
  apiKey: string;
  onSaveApiKey: (key: string) => void;
}

export const ApiKeyModal: React.FC<ApiKeyModalProps> = ({
  isOpen,
  onClose,
  apiKey,
  onSaveApiKey,
}) => {
  const [val, setVal] = useState(apiKey);

  if (!isOpen) return null;

  const handleSave = (e: React.FormEvent) => {
    e.preventDefault();
    onSaveApiKey(val.trim());
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-[#181820] border border-[#2d2d38] w-full max-w-md rounded-2xl shadow-2xl p-5 text-gray-200">
        <div className="flex items-center justify-between pb-3 border-b border-[#282836]">
          <div className="flex items-center gap-2">
            <Key className="w-5 h-5 text-amber-400" />
            <h2 className="font-semibold text-sm">Google AI Studio API Key</h2>
          </div>
          <button onClick={onClose} className="p-1 text-gray-400 hover:text-gray-200 rounded">
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSave} className="mt-4 space-y-4">
          <p className="text-xs text-gray-400 leading-relaxed">
            Your key powers Gemini 4 Argon and Hydrogen models. It is stored exclusively in your local browser storage (<code className="text-amber-400">localStorage</code>) and never transmitted to third parties.
          </p>

          <div>
            <label className="block text-xs font-medium text-gray-300 mb-1.5">
              Gemini API Key
            </label>
            <input
              type="password"
              placeholder="AIzaSy..."
              value={val}
              onChange={(e) => setVal(e.target.value)}
              className="w-full bg-[#121216] border border-[#333342] rounded-lg px-3 py-2 text-xs text-gray-200 focus:outline-none focus:border-amber-500 font-mono"
            />
          </div>

          <div className="flex items-center justify-between text-xs text-gray-400 pt-1">
            <a
              href="https://aistudio.google.com/app/apikey"
              target="_blank"
              rel="noreferrer"
              className="text-amber-400 hover:underline flex items-center gap-1"
            >
              Get free key in Google AI Studio
              <ExternalLink className="w-3 h-3" />
            </a>
          </div>

          <div className="flex justify-end gap-2 pt-2 border-t border-[#282836]">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 text-xs text-gray-400 hover:text-gray-200"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="px-4 py-1.5 bg-amber-500 hover:bg-amber-400 text-black font-semibold text-xs rounded-lg transition-colors"
            >
              Save Key
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
