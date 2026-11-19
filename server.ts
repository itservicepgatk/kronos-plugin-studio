import express from 'express';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { GoogleGenAI, ThinkingLevel } from '@google/genai';
import { SYSTEM_PROMPT_GEMINI } from './src/constants/pumpkinRules.ts';
import {
  buildProjectPrompt,
  extractFilesFromResponse,
  resolveModelConfig,
} from './src/utils/chatHelpers.ts';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const PORT = Number(process.env.PORT) || 3000;

interface IncomingHistoryItem {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

interface ChatRequestBody {
  model?: string;
  userPrompt?: string;
  maxMode?: boolean;
  project?: {
    name: string;
    files: Record<string, string>;
  };
  history?: IncomingHistoryItem[];
}

function getGenAIClient(): GoogleGenAI {
  const apiKey = process.env.GEMINI_API_KEY;
  if (!apiKey) {
    const err = new Error(
      'GEMINI_API_KEY is not configured in the server environment. Please check Settings > Secrets.'
    ) as Error & { status?: number };
    err.status = 500;
    throw err;
  }

  return new GoogleGenAI({
    apiKey,
    httpOptions: {
      headers: {
        'User-Agent': 'aistudio-build',
      },
    },
  });
}

function mapThinkingLevel(level: 'MINIMAL' | 'LOW' | 'MEDIUM' | 'HIGH'): ThinkingLevel {
  switch (level) {
    case 'MINIMAL':
      return ThinkingLevel.MINIMAL;
    case 'LOW':
      return ThinkingLevel.LOW;
    case 'MEDIUM':
      return ThinkingLevel.MEDIUM;
    case 'HIGH':
      return ThinkingLevel.HIGH;
  }
}

async function startServer() {
  const app = express();

  app.use(express.json({ limit: '5mb' }));

  app.get('/api/health', (_req, res) => {
    res.json({ ok: true });
  });

  app.post('/api/chat', async (req, res) => {
    try {
      const body = (req.body || {}) as ChatRequestBody;
      const userPrompt = typeof body.userPrompt === 'string' ? body.userPrompt.trim() : '';

      if (!userPrompt) {
        res.status(400).json({ error: 'Prompt cannot be empty.' });
        return;
      }

      const project = body.project && typeof body.project === 'object'
        ? {
            name: typeof body.project.name === 'string' ? body.project.name : 'custom-plugin',
            files:
              body.project.files && typeof body.project.files === 'object'
                ? body.project.files
                : {},
          }
        : { name: 'custom-plugin', files: {} };

      const { model, thinkingLevel } = resolveModelConfig(body.model, Boolean(body.maxMode));
      const ai = getGenAIClient();

      const priorTurns = Array.isArray(body.history)
        ? body.history
            .filter(
              (item): item is IncomingHistoryItem =>
                (item.role === 'user' || item.role === 'assistant') &&
                typeof item.content === 'string' &&
                item.content.trim().length > 0
            )
            .slice(-12)
            .map((item) => ({
              role: item.role === 'assistant' ? 'model' : 'user',
              parts: [{ text: item.content }],
            }))
        : [];

      const currentTurnPrompt = buildProjectPrompt(project, userPrompt);

      const response = await ai.models.generateContent({
        model,
        contents: [
          ...priorTurns,
          {
            role: 'user',
            parts: [{ text: currentTurnPrompt }],
          },
        ],
        config: {
          systemInstruction: SYSTEM_PROMPT_GEMINI,
          abortSignal: AbortSignal.timeout(50_000),
          thinkingConfig: {
            thinkingLevel: mapThinkingLevel(thinkingLevel),
          },
        },
      });

      const text = response.text?.trim() || 'No response generated.';
      const files = extractFilesFromResponse(text);

      res.json({
        text,
        files,
        model,
      });
    } catch (error: unknown) {
      const err = error as { status?: number; name?: string; message?: string };
      if (err?.name === 'TimeoutError' || err?.name === 'AbortError') {
        res.status(504).json({
          error:
            'The model took too long to respond. Try switching to Gemini 3.1 Flash Lite or sending a shorter prompt.',
        });
        return;
      }
      const status = typeof err?.status === 'number' && err.status >= 400 && err.status < 600
        ? err.status
        : 500;
      const rawMessage = err?.message || 'Unexpected error while generating response.';
      console.error('[POST /api/chat] Gemini API error:', rawMessage);
      res.status(status).json({ error: rawMessage });
    }
  });

  if (process.env.NODE_ENV !== 'production') {
    const { createServer: createViteServer } = await import('vite');
    const vite = await createViteServer({
      server: { middlewareMode: true },
      appType: 'spa',
    });
    app.use(vite.middlewares);
  } else {
    const distDir = path.resolve(__dirname, 'dist');
    app.use(express.static(distDir));
    app.use((_req, res) => {
      res.sendFile(path.join(distDir, 'index.html'));
    });
  }

  app.listen(PORT, '0.0.0.0', () => {
    console.log(`KRONOS Plugin Studio running at http://localhost:${PORT}`);
  });
}

startServer().catch((err) => {
  console.error('Failed to start KRONOS Plugin Studio server:', err);
  process.exit(1);
});
