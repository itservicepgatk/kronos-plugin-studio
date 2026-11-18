import React, { useState } from 'react';
import { Header } from './components/Header';
import { FileTree } from './components/FileTree';
import { CodeEditor } from './components/CodeEditor';
import { ChatAssistant } from './components/ChatAssistant';
import { ApiKeyModal } from './components/ApiKeyModal';
import { TemplateModal } from './components/TemplateModal';
import { STARTER_TEMPLATES } from './constants/templates';
import { downloadProjectAsZip, loadProjectFromZip } from './services/zip';
import { sendPromptToGemini } from './services/gemini';
import { PluginProject, GeminiModel, ChatMessage, PluginTemplate } from './types';

export const App: React.FC = () => {
  // Load initial template
  const defaultTemplate = STARTER_TEMPLATES[0];

  const [project, setProject] = useState<PluginProject>({
    name: 'custom-plugin',
    version: '0.1.0',
    description: 'KRONOS Rust WASM Plugin',
    files: { ...defaultTemplate.files },
    activeFilePath: 'src/lib.rs',
  });

  const [model, setModel] = useState<GeminiModel>('gemini-4-argon');
  const [maxMode, setMaxMode] = useState<boolean>(true);
  const [apiKey, setApiKey] = useState<string>(() => localStorage.getItem('kronos_gemini_api_key') || '');
  const [isApiKeyModalOpen, setIsApiKeyModalOpen] = useState(false);
  const [isTemplateModalOpen, setIsTemplateModalOpen] = useState(false);

  // Chat State
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: 'welcome',
      role: 'assistant',
      content: `👋 Welcome to **KRONOS Plugin Studio**!\n\nI am configured to architect WebAssembly plugins for our Rust-powered Minecraft server (Pumpkin 26.3). I compile to \`wasm32-wasip1\` and generate pure, efficient Rust code.\n\nAsk me to add commands, custom event handlers, player data storage, tab ranks, or menus. You can download the project as a **.ZIP** at any time or drop an existing ZIP into the window to resume working!`,
      timestamp: Date.now(),
    },
  ]);
  const [isLoading, setIsLoading] = useState(false);

  // Save API key to localStorage
  const handleSaveApiKey = (newKey: string) => {
    setApiKey(newKey);
    localStorage.setItem('kronos_gemini_api_key', newKey);
  };

  // Switch active file
  const handleSelectFile = (path: string) => {
    setProject((prev) => ({ ...prev, activeFilePath: path }));
  };

  // Edit current file content
  const handleChangeContent = (content: string) => {
    setProject((prev) => ({
      ...prev,
      files: {
        ...prev.files,
        [prev.activeFilePath]: content,
      },
    }));
  };

  // Add new file
  const handleAddFile = (path: string) => {
    setProject((prev) => ({
      ...prev,
      files: {
        ...prev.files,
        [path]: '',
      },
      activeFilePath: path,
    }));
  };

  // Delete file
  const handleDeleteFile = (path: string) => {
    setProject((prev) => {
      const nextFiles = { ...prev.files };
      delete nextFiles[path];
      const remainingPaths = Object.keys(nextFiles);
      return {
        ...prev,
        files: nextFiles,
        activeFilePath: remainingPaths.includes(prev.activeFilePath)
          ? prev.activeFilePath
          : remainingPaths[0] || '',
      };
    });
  };

  // Select template
  const handleSelectTemplate = (template: PluginTemplate) => {
    setProject({
      name: template.id,
      version: '0.1.0',
      description: template.description,
      files: { ...template.files },
      activeFilePath: Object.keys(template.files)[0] || 'src/lib.rs',
    });

    setMessages((prev) => [
      ...prev,
      {
        id: Date.now().toString(),
        role: 'system',
        content: `Loaded template: **${template.title}** (${template.description})`,
        timestamp: Date.now(),
      },
    ]);
  };

  // Download project as ZIP
  const handleDownloadZip = async () => {
    await downloadProjectAsZip(project);
  };

  // Upload and restore project from ZIP
  const handleUploadZip = async (file: File) => {
    try {
      const loaded = await loadProjectFromZip(file);
      setProject(loaded);

      setMessages((prev) => [
        ...prev,
        {
          id: Date.now().toString(),
          role: 'system',
          content: `📂 Loaded plugin project from **${file.name}** (${Object.keys(loaded.files).length} files). You can continue developing it right now!`,
          timestamp: Date.now(),
        },
      ]);
    } catch (err: any) {
      alert(`Failed to load ZIP archive: ${err.message}`);
    }
  };

  // Handle Drag and Drop for ZIP files
  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0];
      if (file.name.endsWith('.zip')) {
        await handleUploadZip(file);
      }
    }
  };

  // Send message to Gemini
  const handleSendMessage = async (text: string) => {
    if (!apiKey) {
      setIsApiKeyModalOpen(true);
      return;
    }

    const userMsg: ChatMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: text,
      timestamp: Date.now(),
    };

    setMessages((prev) => [...prev, userMsg]);
    setIsLoading(true);

    try {
      const { text: replyText, files } = await sendPromptToGemini({
        model,
        apiKey,
        userPrompt: text,
        project,
        maxMode,
      });

      const assistantMsg: ChatMessage = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        content: replyText,
        extractedFiles: files,
        timestamp: Date.now(),
      };

      setMessages((prev) => [...prev, assistantMsg]);
    } catch (err: any) {
      setMessages((prev) => [
        ...prev,
        {
          id: (Date.now() + 1).toString(),
          role: 'assistant',
          content: `⚠️ Error communicating with Gemini: ${err.message}`,
          timestamp: Date.now(),
        },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  // Apply proposed files from AI message
  const handleApplyFiles = (proposedFiles: Record<string, string>) => {
    setProject((prev) => {
      const updatedFiles = { ...prev.files, ...proposedFiles };
      return {
        ...prev,
        files: updatedFiles,
        // If current active file was updated, keep it, otherwise set to first updated file
        activeFilePath: proposedFiles[prev.activeFilePath]
          ? prev.activeFilePath
          : Object.keys(proposedFiles)[0] || prev.activeFilePath,
      };
    });

    setMessages((prev) => [
      ...prev,
      {
        id: Date.now().toString(),
        role: 'system',
        content: `✅ Successfully applied ${Object.keys(proposedFiles).length} file(s) to the workspace.`,
        timestamp: Date.now(),
      },
    ]);
  };

  return (
    <div 
      className="flex flex-col h-screen w-screen bg-[#0e0e11] overflow-hidden"
      onDragOver={(e) => e.preventDefault()}
      onDrop={handleDrop}
    >
      {/* Top Header */}
      <Header
        model={model}
        setModel={setModel}
        maxMode={maxMode}
        setMaxMode={setMaxMode}
        hasApiKey={!!apiKey}
        onOpenApiKeyModal={() => setIsApiKeyModalOpen(true)}
        onOpenTemplateModal={() => setIsTemplateModalOpen(true)}
        onDownloadZip={handleDownloadZip}
        onUploadZip={handleUploadZip}
        pluginName={project.name}
      />

      {/* Main 3-Column Layout */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Column: File Tree */}
        <FileTree
          files={project.files}
          activeFile={project.activeFilePath}
          onSelectFile={handleSelectFile}
          onAddFile={handleAddFile}
          onDeleteFile={handleDeleteFile}
        />

        {/* Center Column: Code Editor */}
        <CodeEditor
          activeFile={project.activeFilePath}
          content={project.files[project.activeFilePath] || ''}
          onChangeContent={handleChangeContent}
        />

        {/* Right Column: AI Studio Chat Assistant */}
        <ChatAssistant
          messages={messages}
          isLoading={isLoading}
          onSendMessage={handleSendMessage}
          onApplyFiles={handleApplyFiles}
          onClearChat={() => setMessages([])}
          model={model}
        />
      </div>

      {/* Modals */}
      <ApiKeyModal
        isOpen={isApiKeyModalOpen}
        onClose={() => setIsApiKeyModalOpen(false)}
        apiKey={apiKey}
        onSaveApiKey={handleSaveApiKey}
      />

      <TemplateModal
        isOpen={isTemplateModalOpen}
        onClose={() => setIsTemplateModalOpen(false)}
        onSelectTemplate={handleSelectTemplate}
      />
    </div>
  );
};
