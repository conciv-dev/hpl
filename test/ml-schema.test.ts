import { describe, expect, it } from 'vitest';
import {
  mlEntriesAtBodyLine,
  parseMlEntries,
  validateMl,
} from '../src/core/ml-schema.js';

describe('validateMl', () => {
  it('accepts a valid machine-layer document with a normalized single-number range', () => {
    const ml = validateMl({
      module: 'todo-app',
      target: 'react',
      entries: [
        { promptLines: 18, kind: 'ambiguity', message: 'odd phrase', reasoning: 'unclear' },
      ],
    });
    expect(ml.entries[0].promptLines).toEqual([18, 18]);
    expect(ml.entries[0].kind).toBe('ambiguity');
    expect(ml.entries[0].suggestion).toBeUndefined();
  });

  it('defaults an empty entries list and an empty reasoning', () => {
    const ml = validateMl({ module: 'm', target: 'react' });
    expect(ml.entries).toEqual([]);
  });

  it('rejects an unknown kind', () => {
    expect(() => validateMl({ module: 'm', target: 'react', entries: [{ promptLines: [1, 1], kind: 'bogus', message: 'x' }] })).toThrow(
      /machine-layer validation failed/,
    );
  });

  it('rejects an entry with an empty message', () => {
    expect(() =>
      validateMl({ module: 'm', target: 'react', entries: [{ promptLines: [1, 1], kind: 'note', message: '' }] }),
    ).toThrow(/machine-layer validation failed/);
  });
});

describe('parseMlEntries', () => {
  it('parses a list and treats a non-list as empty', () => {
    expect(parseMlEntries([{ promptLines: [2, 3], kind: 'assumption', message: 'a' }])).toHaveLength(1);
    expect(parseMlEntries({})).toEqual([]);
  });

  it('throws on a malformed entry', () => {
    expect(() => parseMlEntries([{ promptLines: 'nope', kind: 'note', message: 'x' }])).toThrow(
      /machine-layer entries invalid/,
    );
  });
});

describe('mlEntriesAtBodyLine', () => {
  const ml = validateMl({
    module: 'm',
    target: 'react',
    entries: [
      { promptLines: [1, 2], kind: 'note', message: 'a' },
      { promptLines: [5, 7], kind: 'ambiguity', message: 'b' },
    ],
  });

  it('returns entries whose range covers the body line', () => {
    expect(mlEntriesAtBodyLine(ml, 6).map((e) => e.message)).toEqual(['b']);
    expect(mlEntriesAtBodyLine(ml, 1).map((e) => e.message)).toEqual(['a']);
  });

  it('returns nothing outside every range', () => {
    expect(mlEntriesAtBodyLine(ml, 4)).toEqual([]);
  });
});
