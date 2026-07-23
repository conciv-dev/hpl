import { describe, expect, it } from 'vitest';
import { applyPatchToBlame, blameFile, blameLineAt, firstPromptDiffLine } from '../src/core/blame.js';
import type { BlameSourceEntry } from '../src/core/blame.js';
import { filePatch } from '../src/core/journal.js';

interface Version {
  gen: number;
  content: string;
  module?: string;
  timestamp?: string;
}

function history(versions: Version[]): BlameSourceEntry[] {
  const entries: BlameSourceEntry[] = [];
  let prev: string | null = null;
  for (const version of versions) {
    entries.push({
      gen: version.gen,
      timestamp: version.timestamp ?? `2026-07-2${version.gen}T00:00:00.000Z`,
      module: version.module ?? 'demo',
      patch: filePatch(prev, version.content),
      promptDiff: '',
    });
    prev = version.content;
  }
  return entries;
}

function gens(entries: BlameSourceEntry[], content: string): number[] {
  return blameFile(entries, content).map((line) => line.gen);
}

describe('blameFile', () => {
  it('attributes every line of a file created in a single gen to that gen', () => {
    const content = 'A\nB\nC\n';
    const entries = history([{ gen: 1, content }]);
    expect(gens(entries, content)).toEqual([1, 1, 1]);
  });

  it('keeps untouched lines on the oldest gen and moves a modified line to the editing gen', () => {
    const v1 = 'A\nB\nC\n';
    const v3 = 'A\nB2\nC\n';
    const entries = history([
      { gen: 1, content: v1 },
      { gen: 3, content: v3 },
    ]);
    expect(gens(entries, v3)).toEqual([1, 3, 1]);
  });

  it('attributes an appended line to the gen that added it, untouched lines stay old', () => {
    const v1 = 'A\n';
    const v2 = 'A\nB\n';
    const entries = history([
      { gen: 1, content: v1 },
      { gen: 2, content: v2 },
    ]);
    expect(gens(entries, v2)).toEqual([1, 2]);
  });

  it('handles a line moved down by an insertion above (git-blame class behavior)', () => {
    const v1 = 'A\nB\n';
    const v2 = 'X\nA\nB\n';
    const entries = history([
      { gen: 1, content: v1 },
      { gen: 2, content: v2 },
    ]);
    expect(gens(entries, v2)).toEqual([2, 1, 1]);
  });

  it('tracks a file created in gen 1 then edited in gen 3 across multiple hunks', () => {
    const v1 = 'a\nb\nc\nd\ne\n';
    const v3 = 'a\nB\nc\nd\nE\nf\n';
    const entries = history([
      { gen: 1, content: v1 },
      { gen: 3, content: v3 },
    ]);
    expect(gens(entries, v3)).toEqual([1, 3, 1, 1, 3, 3]);
  });

  it('carries the timestamp and module from the attributing gen', () => {
    const v1 = 'A\nB\n';
    const v2 = 'A\nB2\n';
    const entries = history([
      { gen: 1, content: v1, module: 'first', timestamp: 'ts1' },
      { gen: 4, content: v2, module: 'second', timestamp: 'ts4' },
    ]);
    const blamed = blameFile(entries, v2);
    expect(blamed[0]).toMatchObject({ gen: 1, module: 'first', timestamp: 'ts1', text: 'A' });
    expect(blamed[1]).toMatchObject({ gen: 4, module: 'second', timestamp: 'ts4', text: 'B2' });
  });
});

describe('blameLineAt', () => {
  it('returns the single blame line for a 1-based line number', () => {
    const v1 = 'A\nB\nC\n';
    const v2 = 'A\nB2\nC\n';
    const entries = history([
      { gen: 1, content: v1 },
      { gen: 2, content: v2 },
    ]);
    expect(blameLineAt(entries, v2, 2)?.gen).toBe(2);
    expect(blameLineAt(entries, v2, 1)?.gen).toBe(1);
  });

  it('returns null for an out-of-range line', () => {
    const entries = history([{ gen: 1, content: 'A\n' }]);
    expect(blameLineAt(entries, 'A\n', 9)).toBeNull();
  });
});

describe('applyPatchToBlame', () => {
  it('applies a creation patch by tagging every inserted line with the gen', () => {
    const patch = filePatch(null, 'x\ny\n');
    expect(applyPatchToBlame([], patch, 7)).toEqual([7, 7]);
  });

  it('is a no-op on an empty patch', () => {
    expect(applyPatchToBlame([1, 2, 3], '', 9)).toEqual([1, 2, 3]);
  });
});

describe('firstPromptDiffLine', () => {
  it('prefers the first added line (the new wording) of a prompt diff', () => {
    const diff = '@@ -1,2 +1,2 @@\n old\n-was this\n+now that\n';
    expect(firstPromptDiffLine(diff)).toBe('now that');
  });

  it('falls back to the removed line when there is no addition', () => {
    expect(firstPromptDiffLine('@@ -1,1 +0,0 @@\n-gone now\n')).toBe('gone now');
  });

  it('returns empty string for an empty prompt diff', () => {
    expect(firstPromptDiffLine('')).toBe('');
  });
});
