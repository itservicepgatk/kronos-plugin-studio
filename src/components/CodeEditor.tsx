import React from 'react';
import { Copy, Check, FileCode, Save } from 'lucide-react';

interface CodeEditorProps {
  activeFile: string;
  content: string;
  onChangeContent: (content: string) => void;
}

export const CodeEditor: React.FC<CodeEditorProps> = ({
  activeFile,
  content,
  onChangeContent,
}) => {
  const [copied, setCopied] = React.useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const lines = (content || '').split('\n');

  return (
    <div className="flex-1 flex flex-col h-full bg-[#16161a] overflow-hidden">
      {/* Editor Tab Bar */}
      <div className="h-9 border-b border-[#272730] bg-[#121216] px-3 flex items-center justify-between select-none">
        <div className="flex items-center gap-2">
          <div className="px-3 py-1 bg-[#1e1e26] border-t-2 border-amber-500 rounded-t text-xs font-mono text-gray-200 flex items-center gap-1.5 shadow-sm">
            <FileCode className="w-3.5 h-3.5 text-amber-400" />
            <span>{activeFile || 'No file selected'}</span>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleCopy}
            className="flex items-center gap-1 px-2 py-1 text-xs text-gray-400 hover:text-gray-200 hover:bg-[#202028] rounded transition-colors"
            title="Copy content"
          >
            {copied ? (
              <>
                <Check className="w-3 h-3 text-emerald-400" />
                <span className="text-emerald-400">Copied</span>
              </>
            ) : (
              <>
                <Copy className="w-3 h-3" />
                <span>Copy</span>
              </>
            )}
          </button>
        </div>
      </div>

      {/* Editor Body with Line Numbers */}
      <div className="flex-1 flex overflow-hidden font-mono text-xs">
        {/* Line Numbers */}
        <div className="w-12 bg-[#121216] border-r border-[#272730] py-3 text-right pr-3 select-none text-gray-600 font-mono">
          {lines.map((_, i) => (
            <div key={i} className="leading-5 h-5 text-[11px]">
              {i + 1}
            </div>
          ))}
        </div>

        {/* Text Area */}
        <textarea
          value={content || ''}
          onChange={(e) => onChangeContent(e.target.value)}
          spellCheck={false}
          className="flex-1 bg-transparent text-gray-200 py-3 px-3 focus:outline-none resize-none leading-5 font-mono text-[12px] whitespace-pre overflow-auto selection:bg-amber-500/30"
          placeholder="// File is empty. Start typing or use AI to generate logic..."
        />
      </div>

      {/* Status Bar */}
      <div className="h-6 border-t border-[#272730] bg-[#121216] px-3 flex items-center justify-between text-[11px] text-gray-500 font-mono select-none">
        <div>
          <span>{lines.length} lines</span>
          <span className="mx-2">•</span>
          <span>{content?.length || 0} characters</span>
        </div>
        <div className="flex items-center gap-3">
          <span>UTF-8</span>
          <span>Rust WASM Edition 2021</span>
        </div>
      </div>
    </div>
  );
};
