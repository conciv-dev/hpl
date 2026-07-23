import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { runBlame } from '../src/commands/blame.js';
import { appendJournalEntry, filePatch } from '../src/core/journal.js';
import type { JournalEntry } from '../src/core/journal.js';

let root: string;
const FILE = '.hl/src/react/src/App.tsx';
const V1 = 'const a = 1;\nconst b = 2;\nconst c = 3;\n';
const V2 = 'const a = 1;\nconst b = 20;\nconst c = 3;\n';

function entry(gen: number, before: string | null, after: string, promptDiff: string): JournalEntry {
  return {
    gen,
    timestamp: `2026-07-2${gen}T09:00:00.000Z`,
    module: 'todo-app',
    target: 'react',
    promptHash: `h${gen}`,
    promptDiff,
    mode: gen === 1 ? 'full' : 'incremental',
    files: [{ path: FILE, patch: filePatch(before, after), hashBefore: before === null ? null : `b${gen}`, hashAfter: `a${gen}` }],
  };
}

async function seed(): Promise<void> {
  const journalPath = join(root, '.hl', 'journal.jsonl');
  await appendJournalEntry(journalPath, entry(1, null, V1, ''));
  await appendJournalEntry(journalPath, entry(2, V1, V2, '@@ -5,1 +5,1 @@\n-old counter\n+new counter'));
  await mkdir(join(root, '.hl', 'src', 'react', 'src'), { recursive: true });
  await writeFile(join(root, FILE), V2, 'utf8');
}

beforeEach(async () => {
  root = await mkdtemp(join(tmpdir(), 'hl-blame-'));
  await seed();
});

afterEach(async () => {
  await rm(root, { recursive: true, force: true });
});

describe('runBlame file mode', () => {
  it('shows the editing gen for a changed line and the original gen for untouched lines', async () => {
    const lines: string[] = [];
    const { exitCode } = await runBlame({ root, file: FILE, log: (m) => lines.push(m) });
    expect(exitCode).toBe(0);
    expect(lines).toHaveLength(3);
    expect(lines[0]).toContain('gen #1');
    expect(lines[1]).toContain('gen #2');
    expect(lines[1]).toContain('const b = 20;');
    expect(lines[2]).toContain('gen #1');
  });

  it('scopes output to a single line with --line', async () => {
    const lines: string[] = [];
    const { exitCode } = await runBlame({ root, file: FILE, line: 2, log: (m) => lines.push(m) });
    expect(exitCode).toBe(0);
    expect(lines).toHaveLength(1);
    expect(lines[0]).toContain('gen #2');
  });

  it('adds the prompt-edit reason in verbose mode', async () => {
    const lines: string[] = [];
    await runBlame({ root, file: FILE, line: 2, verbose: true, log: (m) => lines.push(m) });
    expect(lines.some((l) => l.includes('why (gen #2)') && l.includes('new counter'))).toBe(true);
  });

  it('errors on an out-of-range line', async () => {
    const { exitCode } = await runBlame({ root, file: FILE, line: 99, log: () => undefined });
    expect(exitCode).toBe(1);
  });

  it('errors for a file with no journal history', async () => {
    const { exitCode } = await runBlame({ root, file: '.hl/src/react/src/Nope.tsx', log: () => undefined });
    expect(exitCode).toBe(1);
  });
});

describe('runBlame gen mode', () => {
  it('prints the summary of a single gen entry', async () => {
    const lines: string[] = [];
    const { exitCode } = await runBlame({ root, gen: 2, log: (m) => lines.push(m) });
    expect(exitCode).toBe(0);
    const text = lines.join('\n');
    expect(text).toContain('gen #2');
    expect(text).toContain('todo-app (react)');
    expect(text).toContain('new counter');
    expect(text).toContain(FILE);
  });

  it('marks the initial generation when there is no prompt diff', async () => {
    const lines: string[] = [];
    await runBlame({ root, gen: 1, log: (m) => lines.push(m) });
    expect(lines.join('\n')).toContain('initial generation');
  });

  it('errors for an unknown gen', async () => {
    const { exitCode } = await runBlame({ root, gen: 99, log: () => undefined });
    expect(exitCode).toBe(1);
  });
});

describe('runBlame with no journal', () => {
  it('reports the missing journal and exits 1', async () => {
    const empty = await mkdtemp(join(tmpdir(), 'hl-blame-empty-'));
    try {
      const { exitCode } = await runBlame({ root: empty, file: FILE, log: () => undefined });
      expect(exitCode).toBe(1);
    } finally {
      await rm(empty, { recursive: true, force: true });
    }
  });
});
