import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { runGen } from '../src/commands/gen.js';
import { runBlame } from '../src/commands/blame.js';
import type { AgentRunner } from '@hpl/core';
import type { CommandResult } from '@hpl/core';
import { contentHash } from '@hpl/core';
import { readJournal } from '@hpl/core';
import type { LlmClient } from '@hpl/core';
import { emptyMap, recordAttribution, writeMap } from '@hpl/core';

let root: string;

const PROMPT = `---
module: greeting
deps: []
targets: [typescript]
tests: []
---
Greet a person by name.
`;

const NEW_PROMPT = `---
module: greeting
deps: []
targets: [typescript]
tests: []
---
Greet a person by name loudly.
`;

const V1 = 'export const greet = (n: string) => `Hello, ${n}!`;\n';
const V2 = 'export const greet = (n: string) => `HELLO, ${n}!`;\n';

const IR_YAML = '```yaml\nmodule: greeting\ndeps: []\ntypes: []\nfunctions: []\ntests: []\n```';
const VALID_ATTR =
  '```yaml\n- promptLines: [1, 1]\n  file: greeting.ts\n  lines: [1, 1]\n  note: "builds the greeting"\n```';
const NOTE_ML =
  '```yaml\n- promptLines: [1, 1]\n  kind: note\n  message: "ok"\n  reasoning: "clear"\n```';

function llm(): LlmClient {
  return {
    complete: vi.fn(async ({ system }: { system: string }) => {
      if (system.includes('intermediate representation')) return IR_YAML;
      if (system.includes('MACHINE LAYER')) return NOTE_ML;
      return VALID_ATTR;
    }),
  };
}

function agentWriting(content: string): AgentRunner {
  return {
    run: vi.fn(async () => {
      const targetDir = join(root, '.hl', 'src', 'typescript');
      await mkdir(targetDir, { recursive: true });
      await writeFile(join(targetDir, 'greeting.ts'), content, 'utf8');
      return { output: 'done', code: 0 };
    }),
  };
}

const PASS = vi.fn(async (): Promise<CommandResult> => ({ code: 0, output: 'ok' }));

async function seedPriorGen(): Promise<void> {
  const targetDir = join(root, '.hl', 'src', 'typescript');
  await mkdir(targetDir, { recursive: true });
  await writeFile(join(targetDir, 'greeting.ts'), V1, 'utf8');
  const map = emptyMap();
  recordAttribution(map, {
    rel: 'examples/greeting.hl',
    module: 'greeting',
    promptHash: contentHash(PROMPT),
    target: 'typescript',
    declaredTargets: ['typescript'],
    files: [{ filePath: '.hl/src/typescript/greeting.ts', hash: contentHash(V1) }],
  });
  await writeMap(join(root, '.hl', 'map.json'), map);
  await mkdir(join(root, '.hl', 'prompts-at-gen'), { recursive: true });
  await writeFile(join(root, '.hl', 'prompts-at-gen', 'greeting.md'), 'Greet a person by name.\n', 'utf8');
  await mkdir(join(root, '.hl', 'attribution'), { recursive: true });
  await writeFile(
    join(root, '.hl', 'attribution', 'greeting.yaml'),
    'module: greeting\ntarget: typescript\nentries:\n  - promptLines: [1, 1]\n    file: greeting.ts\n    lines: [1, 1]\n    note: greet\n',
    'utf8',
  );
}

beforeEach(async () => {
  root = await mkdtemp(join(tmpdir(), 'hl-genjournal-'));
  await mkdir(join(root, 'examples'), { recursive: true });
  await writeFile(join(root, 'examples', 'greeting.hl'), PROMPT, 'utf8');
});

afterEach(async () => {
  await rm(root, { recursive: true, force: true });
});

describe('runGen journal', () => {
  it('records a full-mode journal entry with created file patches on first gen', async () => {
    await runGen({
      root,
      target: 'typescript',
      agent: agentWriting(V1),
      llm: llm(),
      model: 'm',
      exec: PASS,
      now: () => '2026-07-23T00:00:00.000Z',
    });
    const entries = await readJournal(join(root, '.hl', 'journal.jsonl'));
    expect(entries).toHaveLength(1);
    expect(entries[0]).toMatchObject({ gen: 1, module: 'greeting', target: 'typescript', mode: 'full' });
    expect(entries[0].promptDiff).toBe('');
    const filePatch = entries[0].files.find((f) => f.path.endsWith('greeting.ts'));
    expect(filePatch?.hashBefore).toBeNull();
    expect(filePatch?.patch).toContain('+export const greet');
  });

  it('records an incremental entry with a prompt diff and a scoped file patch, and blame reflects the new gen', async () => {
    await seedPriorGen();
    await writeFile(join(root, 'examples', 'greeting.hl'), NEW_PROMPT, 'utf8');
    await runGen({
      root,
      target: 'typescript',
      agent: agentWriting(V2),
      llm: llm(),
      model: 'm',
      exec: PASS,
      now: () => '2026-07-23T12:00:00.000Z',
    });

    const entries = await readJournal(join(root, '.hl', 'journal.jsonl'));
    expect(entries).toHaveLength(1);
    expect(entries[0]).toMatchObject({ gen: 1, mode: 'incremental' });
    expect(entries[0].promptDiff).toContain('loudly');
    const patch = entries[0].files.find((f) => f.path.endsWith('greeting.ts'));
    expect(patch?.patch).toContain('HELLO');

    const lines: string[] = [];
    await runBlame({ root, file: '.hl/src/typescript/greeting.ts', log: (m) => lines.push(m) });
    expect(lines.join('\n')).toContain('gen #1');
  });
});
