import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  DEFAULT_PROMPT_ALIASES,
  MACHINE_ALIAS,
  MACHINE_EXTENSION,
  PROMPT_EXTENSION,
  findPromptFiles,
  isMachineFile,
  isPromptFile,
  machineExtensionForPrompt,
  machineExtensions,
  promptExtensions,
} from '../src/core/paths.js';

describe('prompt extension aliases', () => {
  it('recognises the canonical extension and every curated emoji alias', () => {
    expect(isPromptFile(`greeting${PROMPT_EXTENSION}`)).toBe(true);
    for (const alias of DEFAULT_PROMPT_ALIASES) {
      expect(isPromptFile(`greeting${alias}`)).toBe(true);
    }
  });

  it('matches multi-byte emoji extensions by code point via endsWith', () => {
    expect('greeting.🧑'.endsWith('.🧑')).toBe(true);
    expect(isPromptFile('a/b/c/person.🧓')).toBe(true);
    expect(isPromptFile('robot.🤖')).toBe(false);
  });

  it('rejects a ZWJ multi-person sequence that is not on the curated list', () => {
    expect(isPromptFile('team.👨‍💻')).toBe(false);
    expect(DEFAULT_PROMPT_ALIASES).not.toContain('.👨‍💻');
  });

  it('promptExtensions defaults to the canonical spelling plus the curated list', () => {
    expect(promptExtensions()).toEqual([PROMPT_EXTENSION, ...DEFAULT_PROMPT_ALIASES]);
  });

  it('promptExtensions honours a caller-supplied override list', () => {
    expect(promptExtensions(['.🧑'])).toEqual([PROMPT_EXTENSION, '.🧑']);
    expect(isPromptFile('greeting.🧓', ['.🧑'])).toBe(false);
    expect(isPromptFile('greeting.🧑', ['.🧑'])).toBe(true);
  });
});

describe('machine extension aliases', () => {
  it('recognises both machine spellings', () => {
    expect(machineExtensions()).toEqual([MACHINE_EXTENSION, MACHINE_ALIAS]);
    expect(isMachineFile(`greeting${MACHINE_EXTENSION}`)).toBe(true);
    expect(isMachineFile('greeting.🤖')).toBe(true);
    expect(isMachineFile('greeting.napl')).toBe(false);
  });

  it('mirrors the prompt spelling: canonical prompt keeps .mapl, emoji prompt gets .🤖', () => {
    expect(machineExtensionForPrompt('examples/greeting.napl')).toBe(MACHINE_EXTENSION);
    for (const alias of DEFAULT_PROMPT_ALIASES) {
      expect(machineExtensionForPrompt(`examples/greeting${alias}`)).toBe(MACHINE_ALIAS);
    }
  });

  it('resolves a machine file written in either spelling', () => {
    const module = 'greeting';
    const dir = '/some/.napl/mapl';
    const present = new Set([join(dir, `${module}.🤖`)]);
    const resolved = machineExtensions()
      .map((ext) => join(dir, `${module}${ext}`))
      .find((path) => present.has(path));
    expect(resolved).toBe(join(dir, `${module}.🤖`));
  });
});

describe('findPromptFiles discovery', () => {
  let root: string;

  beforeEach(async () => {
    root = await mkdtemp(join(tmpdir(), 'napl-ext-'));
  });

  afterEach(async () => {
    await rm(root, { recursive: true, force: true });
  });

  it('discovers emoji-aliased and canonical prompt files, ignoring other files and .napl/', async () => {
    await mkdir(join(root, 'examples'), { recursive: true });
    await writeFile(join(root, 'examples', 'greeting.🧑'), 'x', 'utf8');
    await writeFile(join(root, 'examples', 'todo.napl'), 'x', 'utf8');
    await writeFile(join(root, 'notes.txt'), 'x', 'utf8');
    await mkdir(join(root, '.napl', 'mapl'), { recursive: true });
    await writeFile(join(root, '.napl', 'mapl', 'greeting.🤖'), 'x', 'utf8');

    const found = (await findPromptFiles(root)).map((f) => relative(root, f));
    expect(found).toEqual([join('examples', 'greeting.🧑'), join('examples', 'todo.napl')].sort());
    expect(existsSync(join(root, '.napl', 'mapl', 'greeting.🤖'))).toBe(true);
  });

  it('respects a custom alias list', async () => {
    await writeFile(join(root, 'a.🧑'), 'x', 'utf8');
    await writeFile(join(root, 'b.🧓'), 'x', 'utf8');
    const found = (await findPromptFiles(root, ['.🧑'])).map((f) => relative(root, f));
    expect(found).toEqual(['a.🧑']);
  });
});
