import { describe, expect, it } from 'vitest';
import { DEFAULT_BACKEND, parseLock, resolvePromptAliases } from '../src/core/lock.js';
import { DEFAULT_PROMPT_ALIASES } from '../src/core/paths.js';

describe('parseLock', () => {
  it('defaults backend to claude-cli when the field is missing', () => {
    const lock = parseLock(JSON.stringify({ model: 'claude-sonnet-5' }));
    expect(lock.backend).toBe('claude-cli');
    expect(DEFAULT_BACKEND).toBe('claude-cli');
  });

  it('keeps an explicit anthropic-api backend', () => {
    const lock = parseLock(JSON.stringify({ model: 'claude-sonnet-5', backend: 'anthropic-api' }));
    expect(lock.backend).toBe('anthropic-api');
  });

  it('keeps an explicit claude-cli backend', () => {
    const lock = parseLock(JSON.stringify({ model: 'claude-opus-5', backend: 'claude-cli' }));
    expect(lock.backend).toBe('claude-cli');
    expect(lock.model).toBe('claude-opus-5');
  });

  it('rejects an unknown backend value', () => {
    expect(() => parseLock(JSON.stringify({ model: 'x', backend: 'openai' }))).toThrow(/invalid lock\.json/);
  });

  it('rejects corrupt json', () => {
    expect(() => parseLock('{not json')).toThrow(/corrupt lock\.json/);
  });
});

describe('lock promptAliases', () => {
  it('defaults to the curated list when the field is absent', () => {
    const lock = parseLock(JSON.stringify({ model: 'm' }));
    expect(lock.promptAliases).toBeUndefined();
    expect(resolvePromptAliases(lock)).toEqual([...DEFAULT_PROMPT_ALIASES]);
  });

  it('accepts a valid override list and returns it verbatim', () => {
    const lock = parseLock(JSON.stringify({ model: 'm', promptAliases: ['.🧑', '.🤠'] }));
    expect(lock.promptAliases).toEqual(['.🧑', '.🤠']);
    expect(resolvePromptAliases(lock)).toEqual(['.🧑', '.🤠']);
  });

  it('rejects an alias that does not start with a dot', () => {
    expect(() => parseLock(JSON.stringify({ model: 'm', promptAliases: ['🧑'] }))).toThrow(
      /invalid lock\.json/,
    );
  });

  it('rejects an alias with more than two code points after the dot', () => {
    expect(() => parseLock(JSON.stringify({ model: 'm', promptAliases: ['.abc'] }))).toThrow(
      /invalid lock\.json/,
    );
  });

  it('rejects a ZWJ (zero-width joiner) sequence', () => {
    expect(() => parseLock(JSON.stringify({ model: 'm', promptAliases: ['.👨‍💻'] }))).toThrow(
      /invalid lock\.json/,
    );
  });
});
