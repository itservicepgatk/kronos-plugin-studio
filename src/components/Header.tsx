import React from 'react';
import { 
  Download, 
  Upload, 
  Sparkles, 
  Layers, 
  Cpu
} from 'lucide-react';
import { GeminiModel } from '../types';

interface HeaderProps {
  model: GeminiModel;
  setModel: (model: GeminiModel) => void;
  maxMode: boolean;
  setMaxMode: (val: boolean) => void;
  onOpenTemplateModal: () => void;
  onDownloadZip: () => void;
  onUploadZip: (file: File) => void;
  pluginName: string;
}

export const Header: React.FC<HeaderProps> = ({
  model,
  setModel,
  maxMode,
  setMaxMode,
  onOpenTemplateModal,
  onDownloadZip,
  onUploadZip,
  pluginName,
}) => {
  const fileInputRef = React.useRef<HTMLInputElement>(null);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      onUploadZip(e.target.files[0]);
      e.target.value = '';
    }
  };

  return (
    <header className="h-14 border-b border-[#272730] bg-[#141418] px-4 flex items-center justify-between select-none">
      {/* Brand & Project Info */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-amber-500 to-orange-600 flex items-center justify-center font-bold text-black text-lg shadow-md shadow-orange-500/20">
            🦀
          </div>
          <div>
            <h1 className="font-bold text-sm tracking-wide text-gray-100 flex items-center gap-2">
              KRONOS Plugin Studio
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-amber-500/10 border border-amber-500/30 text-amber-400 font-mono">
                WASM 26.3
              </span>
            </h1>
            <p className="text-[11px] text-gray-400 font-mono">
              Editing: <span className="text-gray-200">{pluginName || 'untitled-plugin'}</span>
            </p>
          </div>
        </div>
      </div>

      {/* Model & AI Settings */}
      <div className="flex items-center gap-2">
        {/* Model Switcher */}
        <div className="flex items-center gap-1.5 bg-[#1b1b22] border border-[#2d2d38] rounded-lg px-2.5 py-1 text-xs">
          <Cpu className="w-3.5 h-3.5 text-amber-400" />
          <select 
            aria-label="Select Gemini model"
            value={model} 
            onChange={(e) => setModel(e.target.value as GeminiModel)}
            className="bg-transparent text-gray-200 font-medium focus:outline-none cursor-pointer"
          >
            <option value="gemini-3.1-flash-lite" className="bg-[#1b1b22] text-gray-200">
              Gemini 3.1 Flash Lite (Fast)
            </option>
            <option value="gemini-4-hydrogen" className="bg-[#1b1b22] text-gray-200">
              Gemini 4 Hydrogen (Extended)
            </option>
          </select>
        </div>

        {/* Max Mode Toggle */}
        <button
          type="button"
          aria-pressed={maxMode}
          onClick={() => setMaxMode(!maxMode)}
          className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium transition-all ${
            maxMode 
              ? 'bg-gradient-to-r from-amber-500 to-orange-500 text-black font-semibold shadow-lg shadow-amber-500/25 ring-1 ring-amber-400' 
              : 'bg-[#1b1b22] text-gray-400 hover:text-gray-200 border border-[#2d2d38]'
          }`}
          title="Enable deep reasoning mode for complex plugin architecture"
        >
          <Sparkles className="w-3.5 h-3.5" />
          Max Mode
        </button>
      </div>

      {/* Project Actions (Zip, Templates) */}
      <div className="flex items-center gap-2">
        <button
          onClick={onOpenTemplateModal}
          className="flex items-center gap-1.5 px-2.5 py-1 bg-[#1b1b22] hover:bg-[#252530] text-gray-300 text-xs font-medium rounded-lg border border-[#2d2d38] transition-colors"
        >
          <Layers className="w-3.5 h-3.5 text-blue-400" />
          Templates
        </button>

        <input 
          ref={fileInputRef} 
          type="file" 
          accept=".zip" 
          className="hidden" 
          onChange={handleFileChange} 
        />
        
        <button
          onClick={() => fileInputRef.current?.click()}
          className="flex items-center gap-1.5 px-2.5 py-1 bg-[#1b1b22] hover:bg-[#252530] text-gray-300 text-xs font-medium rounded-lg border border-[#2d2d38] transition-colors"
          title="Upload an existing plugin .ZIP archive to continue editing"
        >
          <Upload className="w-3.5 h-3.5 text-purple-400" />
          Open .ZIP
        </button>

        <button
          onClick={onDownloadZip}
          className="flex items-center gap-1.5 px-3 py-1 bg-amber-500 hover:bg-amber-400 text-black text-xs font-semibold rounded-lg shadow-md shadow-amber-500/20 transition-colors"
          title="Download the complete plugin project as a .ZIP archive"
        >
          <Download className="w-3.5 h-3.5" />
          Download .ZIP
        </button>
      </div>
    </header>
  );
};
