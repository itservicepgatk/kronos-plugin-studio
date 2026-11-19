import test from 'node:test';
import assert from 'node:assert/strict';
import {
  buildProjectPrompt,
  extractFilesFromResponse,
  resolveModelConfig,
} from './chatHelpers';

test('resolveModelConfig maps supported models and thinking levels correctly', () => {
  assert.deepEqual(resolveModelConfig('gemini-4-hydrogen', true), {
    model: 'gemini-4-hydrogen',
    thinkingLevel: 'HIGH',
  });
  assert.deepEqual(resolveModelConfig('gemini-4-hydrogen', false), {
    model: 'gemini-4-hydrogen',
    thinkingLevel: 'LOW',
  });
  assert.deepEqual(resolveModelConfig('gemini-3.1-flash-lite', true), {
    model: 'gemini-3.1-flash-lite',
    thinkingLevel: 'HIGH',
  });
  assert.deepEqual(resolveModelConfig('gemini-3.1-flash-lite', false), {
    model: 'gemini-3.1-flash-lite',
    thinkingLevel: 'LOW',
  });
});

test('resolveModelConfig falls back safely to gemini-3.1-flash-lite on unknown models', () => {
  assert.deepEqual(resolveModelConfig('unknown-model', true), {
    model: 'gemini-3.1-flash-lite',
    thinkingLevel: 'HIGH',
  });
  assert.deepEqual(resolveModelConfig(undefined, false), {
    model: 'gemini-3.1-flash-lite',
    thinkingLevel: 'LOW',
  });
});

test('extractFilesFromResponse parses tagged code blocks and ignores plain snippets', () => {
  const markdown = [
    'Here is your updated plugin:',
    '```rust:src/lib.rs',
    'pub fn init() -> i32 { 0 }',
    '```',
    '',
    '```toml:./Cargo.toml',
    '[package]',
    'name = "demo"',
    '```',
    '',
    'Example command:',
    '```bash',
    'cargo build --target wasm32-wasip1',
    '```',
    '',
    '```rust:src\\commands\\ping.rs',
    'pub fn ping() -> &\'static str { "Pong!" }',
    '```',
  ].join('\n');

  const files = extractFilesFromResponse(markdown);
  assert.deepEqual(Object.keys(files).sort(), [
    'Cargo.toml',
    'src/commands/ping.rs',
    'src/lib.rs',
  ]);
  assert.equal(files['src/lib.rs'], 'pub fn init() -> i32 { 0 }\n');
  assert.equal(files['Cargo.toml'], '[package]\nname = "demo"\n');
  assert.equal(files['src/commands/ping.rs'], 'pub fn ping() -> &\'static str { "Pong!" }\n');
});

test('extractFilesFromResponse ignores path traversal attempts', () => {
  const markdown = '```rust:../secret.rs\nfn bad() {}\n```';
  assert.deepEqual(extractFilesFromResponse(markdown), {});
});

test('buildProjectPrompt includes plugin name, file contents, and user request', () => {
  const prompt = buildProjectPrompt(
    { name: 'spawn-plugin', files: { 'src/lib.rs': 'fn init() {}' } },
    'Add /spawn command'
  );
  assert.match(prompt, /Current Plugin Name: "spawn-plugin"/);
  assert.match(prompt, /File: `src\/lib\.rs`:/);
  assert.match(prompt, /fn init\(\) \{\}/);
  assert.match(prompt, /USER REQUEST:\nAdd \/spawn command/);
});
