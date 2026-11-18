import { SYSTEM_PROMPT_GEMINI } from '../constants/pumpkinRules';
import { GeminiModel, PluginProject } from '../types';

export interface GenerateOptions {
  model: GeminiModel;
  apiKey: string;
  userPrompt: string;
  project: PluginProject;
  maxMode?: boolean;
}

/**
 * Extracts proposed file updates from Gemini's markdown response
 */
export function extractFilesFromResponse(markdown: string): Record<string, string> {
  const files: Record<string, string> = {};
  // Regex to match ```lang:filepath or ```filepath\ncontent```
  const regex = /```(?:[a-zA-Z0-9_-]+:)?([a-zA-Z0-9_.\-\/\\]+)\n([\s\S]*?)```/g;
  let match;

  while ((match = regex.exec(markdown)) !== null) {
    const rawPath = match[1].trim();
    const content = match[2];

    // Only treat as a file if it looks like a valid path (contains extension or known name)
    if (rawPath.includes('.') || rawPath.toLowerCase() === 'dockerfile') {
      // Normalize slashes
      const normalizedPath = rawPath.replace(/\\/g, '/').replace(/^\.\//, '');
      files[normalizedPath] = content;
    }
  }

  return files;
}

/**
 * Calls Gemini API to generate or modify plugin files
 */
export async function sendPromptToGemini(options: GenerateOptions): Promise<{ text: string; files: Record<string, string> }> {
  const { model, apiKey, userPrompt, project, maxMode } = options;

  if (!apiKey) {
    throw new Error('Google AI Studio API Key is missing. Click the Key icon in the header to set it.');
  }

  // Map internal model name to Google API model ID
  let apiModel = model;
  if (model === 'gemini-4-argon') {
    apiModel = 'gemini-2.5-pro' as any; // Map argon to latest reasoning model endpoint
  } else if (model === 'gemini-4-hydrogen') {
    apiModel = 'gemini-2.5-flash' as any; // Map hydrogen to ultra-fast model endpoint
  }

  // Construct context of existing files
  const existingFilesContext = Object.entries(project.files)
    .map(([path, content]) => `File: \`${path}\`:\n\`\`\`\n${content}\n\`\`\``)
    .join('\n\n');

  const fullUserMessage = `
Current Plugin Name: "${project.name}"

CURRENT PROJECT FILES:
${existingFilesContext || '(No files yet)'}

USER REQUEST:
${userPrompt}
`;

  const requestBody: any = {
    contents: [
      {
        role: 'user',
        parts: [{ text: fullUserMessage }]
      }
    ],
    systemInstruction: {
      parts: [{ text: SYSTEM_PROMPT_GEMINI }]
    },
    generationConfig: {
      temperature: maxMode ? 0.4 : 0.7,
      maxOutputTokens: 8192,
    }
  };

  if (maxMode) {
    // Enable reasoning budget if supported
    requestBody.generationConfig.thinkingConfig = {
      thinkingBudget: 2048
    };
  }

  const endpoint = `https://generativelanguage.googleapis.com/v1beta/models/${apiModel}:generateContent?key=${apiKey}`;

  const response = await fetch(endpoint, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(requestBody)
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({}));
    const message = errorData?.error?.message || `API error (${response.status}): ${response.statusText}`;
    throw new Error(message);
  }

  const data = await response.json();
  const text = data?.candidates?.[0]?.content?.parts?.[0]?.text || 'No response generated.';
  const files = extractFilesFromResponse(text);

  return { text, files };
}
