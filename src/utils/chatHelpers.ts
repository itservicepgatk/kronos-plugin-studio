import type { GeminiModel, PluginProject } from '../types';

export type ResolvedThinkingLevel = 'MINIMAL' | 'LOW' | 'MEDIUM' | 'HIGH';

export interface ResolvedModelConfig {
  model: GeminiModel;
  thinkingLevel: ResolvedThinkingLevel;
}

export const SUPPORTED_MODELS: readonly GeminiModel[] = [
  'gemini-3.1-flash-lite',
  'gemini-4-hydrogen',
] as const;

export const DEFAULT_GEMINI_MODEL: GeminiModel = 'gemini-3.1-flash-lite';

/**
 * Normalizes the requested model ID to a supported model and selects
 * a compatible thinking level based on Max Mode.
 */
export function resolveModelConfig(
  requestedModel: string | undefined,
  maxMode: boolean
): ResolvedModelConfig {
  const model: GeminiModel = SUPPORTED_MODELS.includes(requestedModel as GeminiModel)
    ? (requestedModel as GeminiModel)
    : DEFAULT_GEMINI_MODEL;

  if (model === 'gemini-3.1-flash-lite') {
    return {
      model,
      thinkingLevel: maxMode ? 'HIGH' : 'LOW',
    };
  }

  return {
    model: 'gemini-4-hydrogen',
    thinkingLevel: maxMode ? 'HIGH' : 'LOW',
  };
}

/**
 * Extracts proposed file updates from Gemini's markdown response.
 * Matches fences formatted as ```lang:path/to/file or ```path/to/file
 */
export function extractFilesFromResponse(markdown: string): Record<string, string> {
  const files: Record<string, string> = {};
  const regex = /```(?:[a-zA-Z0-9_-]+:)?([^\n`]+)\n([\s\S]*?)```/g;
  let match: RegExpExecArray | null;

  while ((match = regex.exec(markdown)) !== null) {
    const rawHeader = match[1].trim();
    const content = match[2];

    // Ignore plain language tags like "rust", "toml", "json", "bash" that have no file extension or path
    if (
      !rawHeader ||
      /\s/.test(rawHeader) ||
      (!rawHeader.includes('.') && rawHeader.toLowerCase() !== 'dockerfile')
    ) {
      continue;
    }

    const normalizedPath = rawHeader
      .replace(/\\/g, '/')
      .replace(/^\.\//, '')
      .replace(/^\/+/, '');

    if (normalizedPath && !normalizedPath.includes('..')) {
      files[normalizedPath] = content;
    }
  }

  return files;
}

/**
 * Builds the user prompt enriched with the current plugin workspace files.
 */
export function buildProjectPrompt(
  project: Pick<PluginProject, 'name' | 'files'>,
  userPrompt: string
): string {
  const existingFilesContext = Object.entries(project.files)
    .map(([path, content]) => `File: \`${path}\`:\n\`\`\`\n${content}\n\`\`\``)
    .join('\n\n');

  return [
    `Current Plugin Name: "${project.name}"`,
    '',
    'CURRENT PROJECT FILES:',
    existingFilesContext || '(No files yet)',
    '',
    'USER REQUEST:',
    userPrompt.trim(),
  ].join('\n');
}
