import { describe, expect, it } from 'vitest';
import { parseHunks, toLines, unifiedDiff } from '../src/core/text-diff.js';

describe('toLines', () => {
  it('drops a single trailing newline and splits', () => {
    expect(toLines('a\nb\n')).toEqual(['a', 'b']);
    expect(toLines('a\nb')).toEqual(['a', 'b']);
  });

  it('returns an empty array for empty content', () => {
    expect(toLines('')).toEqual([]);
  });
});

describe('unifiedDiff', () => {
  it('emits an all-insert hunk from empty to content', () => {
    expect(unifiedDiff('', 'a\nb\n')).toBe('@@ -0,0 +1,2 @@\n+a\n+b');
  });

  it('emits a scoped hunk header for a modification', () => {
    const diff = unifiedDiff('a\nb\nc\n', 'a\nB\nc\n');
    expect(diff.split('\n')[0]).toMatch(/^@@ -\d+,\d+ \+\d+,\d+ @@$/);
    expect(diff).toContain('-b');
    expect(diff).toContain('+B');
  });
});

describe('parseHunks', () => {
  it('parses hunk headers and line kinds', () => {
    const diff = '@@ -1,3 +1,3 @@\n a\n-b\n+B\n c\n';
    const hunks = parseHunks(diff);
    expect(hunks).toHaveLength(1);
    expect(hunks[0]).toMatchObject({ oldStart: 1, oldCount: 3, newStart: 1, newCount: 3 });
    expect(hunks[0].lines.map((l) => l.kind)).toEqual([' ', '-', '+', ' ']);
  });

  it('returns no hunks for an empty diff', () => {
    expect(parseHunks('')).toEqual([]);
  });
});
