export interface PluginFile {
  path: string;
  content: string;
}

export interface PluginProject {
  name: string;
  version: string;
  description: string;
  files: Record<string, string>; // path -> content
  activeFilePath: string;
}

export type GeminiModel =
  | 'gemini-4-hydrogen'
  | 'gemini-3.1-flash-lite';

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
  extractedFiles?: Record<string, string>; // files proposed by AI
}

export interface PluginTemplate {
  id: string;
  title: string;
  description: string;
  tags: string[];
  files: Record<string, string>;
}
