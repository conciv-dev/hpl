import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { contentHash } from '@napl/core';
import { emptyMap, recordAttribution, recordUnattributed, writeMap } from '@napl/core';
import type { NaplMap } from '@napl/core';
import { runStatus } from '../src/commands/status.js';

let root: string;

const PROMPT = `---
module: greeting
deps: []
targets: [typescript]
tests: []
---
Greet a person.
`;

beforeEach(async () => {
  root = await mkdtemp(join(tmpdir(), 'napl-status-'));
  await mkdir(join(root, 'examples'), { recursive: true });
  await writeFile(join(root, 'examples', 'greeting.napl'), PROMPT, 'utf8');
});

afterEach(async () => {
  await rm(root, { recursive: true, force: true });
});

async function seedMap(genContent: string, recordedHashOverride?: string): Promise<void> {
  const srcFile = join('.napl', 'src', 'typescript', 'greeting.ts');
  await mkdir(join(root, '.napl', 'src', 'typescript'), { recursive: true });
  await writeFile(join(root, srcFile), genContent, 'utf8');
  const map: NaplMap = emptyMap();
  recordAttribution(map, {
    rel: 'examples/greeting.napl',
    module: 'greeting',
    promptHash: contentHash(PROMPT),
    target: 'typescript',
    declaredTargets: ['typescript'],
    files: [{ filePath: srcFile.split('\\').join('/'), hash: recordedHashOverride ?? contentHash(genContent) }],
  });
  await writeMap(join(root, '.napl', 'map.json'), map);
}

describe('runStatus classification', () => {
  it('reports clean when everything is in sync', async () => {
    await seedMap('export const greet = () => "hi";\n');
    const { entries, exitCode } = await runStatus({ root });
    expect(entries[0].status).toBe('clean');
    expect(exitCode).toBe(0);
  });

  it('reports prompt-stale when never generated', async () => {
    const { entries, exitCode } = await runStatus({ root });
    expect(entries[0].status).toBe('prompt-stale');
    expect(entries[0].detail).toBe('never generated');
    expect(exitCode).toBe(0);
  });

  it('reports prompt-stale when the prompt changed since gen', async () => {
    await seedMap('export const greet = () => "hi";\n');
    await writeFile(join(root, 'examples', 'greeting.napl'), PROMPT + '\nMore prose.\n', 'utf8');
    const { entries } = await runStatus({ root });
    expect(entries[0].status).toBe('prompt-stale');
  });

  it('reports DRIFT and exits 1 when a locked generated file was edited', async () => {
    await seedMap('export const greet = () => "hi";\n', 'a-different-recorded-hash');
    const { entries, exitCode } = await runStatus({ root });
    expect(entries[0].status).toBe('DRIFT');
    expect(exitCode).toBe(1);
  });

  it('reports DRIFT when a locked generated file is missing', async () => {
    await seedMap('export const greet = () => "hi";\n');
    await rm(join(root, '.napl', 'src', 'typescript', 'greeting.ts'));
    const { entries } = await runStatus({ root });
    expect(entries[0].status).toBe('DRIFT');
  });

  it('reports unattributed and exits 1 when a target carries the failure marker', async () => {
    const srcFile = join('.napl', 'src', 'typescript', 'greeting.ts');
    await mkdir(join(root, '.napl', 'src', 'typescript'), { recursive: true });
    await writeFile(join(root, srcFile), 'export const greet = () => "hi";\n', 'utf8');
    const map: NaplMap = emptyMap();
    recordUnattributed(map, {
      rel: 'examples/greeting.napl',
      module: 'greeting',
      promptHash: contentHash(PROMPT),
      target: 'typescript',
      declaredTargets: ['typescript'],
      files: [srcFile.split('\\').join('/')],
    });
    await writeMap(join(root, '.napl', 'map.json'), map);

    const { entries, exitCode } = await runStatus({ root });
    expect(entries[0].status).toBe('unattributed');
    expect(entries[0].detail).toContain('run napl gen typescript --force');
    expect(exitCode).toBe(1);
  });
});
