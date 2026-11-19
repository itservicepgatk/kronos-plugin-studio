import { ChatMessage, GeminiModel, PluginProject } from '../types';

export interface GenerateOptions {
  model: GeminiModel;
  userPrompt: string;
  project: PluginProject;
  maxMode?: boolean;
  history?: Pick<ChatMessage, 'role' | 'content'>[];
}

export interface GenerateResult {
  text: string;
  files: Record<string, string>;
  model: GeminiModel;
}

/**
 * Calls the server-side /api/chat route powered by @google/genai.
 */
export async function sendPromptToGemini(options: GenerateOptions): Promise<GenerateResult> {
  const { model, userPrompt, project, maxMode, history } = options;

  const response = await fetch('/api/chat', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      model,
      userPrompt,
      maxMode: Boolean(maxMode),
      project: {
        name: project.name,
        files: project.files,
      },
      history: history ?? [],
    }),
  });

  const data = await response.json().catch(() => ({}));

  if (!response.ok) {
    const message =
      typeof data?.error === 'string' && data.error.trim().length > 0
        ? data.error
        : `Server error (${response.status}): ${response.statusText}`;
    throw new Error(message);
  }

  return {
    text: typeof data?.text === 'string' ? data.text : 'No response generated.',
    files: data?.files && typeof data.files === 'object' ? data.files : {},
    model: (data?.model as GeminiModel) || model,
  };
}
