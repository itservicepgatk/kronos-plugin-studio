import React, { useState } from 'react';
import { 
  FileCode, 
  FileText, 
  FileJson, 
  Folder, 
  Plus, 
  Trash2, 
  Settings2
} from 'lucide-react';

interface FileTreeProps {
  files: Record<string, string>;
  activeFile: string;
  onSelectFile: (path: string) => void;
  onAddFile: (path: string) => void;
  onDeleteFile: (path: string) => void;
}

export const FileTree: React.FC<FileTreeProps> = ({
  files,
  activeFile,
  onSelectFile,
  onAddFile,
  onDeleteFile,
}) => {
  const [isAdding, setIsAdding] = useState(false);
  const [newFilePath, setNewFilePath] = useState('');

  const getFileIcon = (path: string) => {
    if (path.endsWith('.rs')) return <FileCode className="w-4 h-4 text-orange-400" />;
    if (path.endsWith('.toml')) return <Settings2 className="w-4 h-4 text-yellow-400" />;
    if (path.endsWith('.json')) return <FileJson className="w-4 h-4 text-cyan-400" />;
    if (path.endsWith('.md')) return <FileText className="w-4 h-4 text-blue-400" />;
    return <FileText className="w-4 h-4 text-gray-400" />;
  };

  const handleCreateSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (newFilePath.trim()) {
      onAddFile(newFilePath.trim());
      setNewFilePath('');
      setIsAdding(false);
    }
  };

  const fileList = Object.keys(files).sort();

  return (
    <div className="w-64 border-r border-[#272730] bg-[#121216] flex flex-col h-full select-none text-xs">
      {/* Sidebar Header */}
      <div className="px-3 py-2.5 border-b border-[#272730] flex items-center justify-between">
        <span className="font-semibold text-gray-300 uppercase tracking-wider text-[11px] flex items-center gap-1.5">
          <Folder className="w-3.5 h-3.5 text-amber-500" />
          Project Files
        </span>
        <button
          onClick={() => setIsAdding(!isAdding)}
          className="p-1 hover:bg-[#202028] text-gray-400 hover:text-gray-200 rounded transition-colors"
          title="Create New File"
        >
          <Plus className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* New File Inline Form */}
      {isAdding && (
        <form onSubmit={handleCreateSubmit} className="p-2 border-b border-[#272730] bg-[#181820]">
          <input
            type="text"
            placeholder="e.g. src/commands.rs"
            value={newFilePath}
            onChange={(e) => setNewFilePath(e.target.value)}
            className="w-full bg-[#101014] border border-[#333342] rounded px-2 py-1 text-xs text-gray-200 focus:outline-none focus:border-amber-500"
            autoFocus
          />
          <div className="flex justify-end gap-1 mt-1.5">
            <button
              type="button"
              onClick={() => setIsAdding(false)}
              className="px-2 py-0.5 text-[11px] text-gray-400 hover:text-gray-200"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="px-2 py-0.5 text-[11px] bg-amber-500 text-black font-semibold rounded"
            >
              Add
            </button>
          </div>
        </form>
      )}

      {/* File List */}
      <div className="flex-1 overflow-y-auto py-1">
        {fileList.length === 0 ? (
          <div className="p-4 text-center text-gray-500">
            No files in project. Use Templates or ask Gemini to generate one!
          </div>
        ) : (
          fileList.map((path) => {
            const isActive = path === activeFile;
            return (
              <div
                key={path}
                onClick={() => onSelectFile(path)}
                className={`group px-3 py-1.5 flex items-center justify-between cursor-pointer border-l-2 transition-colors ${
                  isActive
                    ? 'bg-[#1e1e26] text-amber-300 border-amber-500 font-medium'
                    : 'text-gray-400 hover:bg-[#16161c] hover:text-gray-200 border-transparent'
                }`}
              >
                <div className="flex items-center gap-2 truncate">
                  {getFileIcon(path)}
                  <span className="truncate font-mono">{path}</span>
                </div>

                {fileList.length > 1 && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      if (confirm(`Delete ${path}?`)) onDeleteFile(path);
                    }}
                    className="opacity-0 group-hover:opacity-100 p-1 hover:text-red-400 text-gray-500 rounded transition-opacity"
                    title="Delete File"
                  >
                    <Trash2 className="w-3 h-3" />
                  </button>
                )}
              </div>
            );
          })
        )}
      </div>

      {/* Build Target Info Footer */}
      <div className="p-3 border-t border-[#272730] bg-[#141418] text-[11px] text-gray-400 flex flex-col gap-1">
        <div className="flex items-center justify-between">
          <span>Target Architecture:</span>
          <span className="font-mono text-emerald-400 bg-emerald-500/10 px-1 rounded">wasm32-wasip2</span>
        </div>
        <div className="flex items-center justify-between">
          <span>Host Runtime:</span>
          <span className="font-mono text-amber-400">Pumpkin 26.3</span>
        </div>
      </div>
    </div>
  );
};
