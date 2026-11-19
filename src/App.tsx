import React, { useState } from 'react';
import { Header } from './components/Header';
import { FileTree } from './components/FileTree';
import { CodeEditor } from './components/CodeEditor';
import { ChatAssistant } from './components/ChatAssistant';
import { TemplateModal } from './components/TemplateModal';
import { STARTER_TEMPLATES } from './constants/templates';
import { downloadProjectAsZip, loadProjectFromZip } from './services/zip';
import { sendPromptToGemini } from './services/gemini';
import { PluginProject, GeminiModel, ChatMessage, PluginTemplate } from './types';

export const App: React.FC = () => {
  // Load initial template
  const defaultTemplate = STARTER_TEMPLATES[0];

  const [project, setProject] = useState<PluginProject>({
    name: defaultTemplate.id,
    version: '2.0.0',
    description: defaultTemplate.description,
    files: { ...defaultTemplate.files },
    activeFilePath: 'src/lib.rs',
  });

  const [model, setModel] = useState<GeminiModel>('gemini-3.1-flash-lite');
  const [maxMode, setMaxMode] = useState<boolean>(true);
  const [isTemplateModalOpen, setIsTemplateModalOpen] = useState(false);

  // Chat State
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: 'welcome',
      role: 'assistant',
      content: `Welcome to **KRONOS Plugin Studio**!\n\nI am configured to architect WebAssembly plugins for your Rust-powered Minecraft server (Pumpkin 26.3), compiling to \`wasm32-wasip1\` with pure Rust code.\n\nAsk me directly right here to add commands, event handlers, player data storage, tab ranks, or menus. You can download the project as a **.ZIP** at any time or drop an existing ZIP into the window to resume working.`,
      timestamp: Date.now(),
    },
  ]);
  const [isLoading, setIsLoading] = useState(false);

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
      setMessages((prev) => [
        ...prev,
        {
          id: Date.now().toString(),
          role: 'system',
          content: `⚠️ Failed to load ZIP archive: ${err?.message || 'Invalid archive'}`,
          timestamp: Date.now(),
        },
      ]);
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

  // Send message to server-side Gemini endpoint
  const handleSendMessage = async (text: string) => {
    const history = messages
      .filter((m) => m.id !== 'welcome' && (m.role === 'user' || m.role === 'assistant'))
      .map(({ role, content }) => ({ role, content }));

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
        userPrompt: text,
        project,
        maxMode,
        history,
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
      <TemplateModal
        isOpen={isTemplateModalOpen}
        onClose={() => setIsTemplateModalOpen(false)}
        onSelectTemplate={handleSelectTemplate}
      />
    </div>
  );
};
