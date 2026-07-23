import { parseHunks, toLines } from './text-diff.js';

export interface BlameSourceEntry {
  gen: number;
  timestamp: string;
  module: string;
  patch: string;
  promptDiff: string;
}

export interface BlameLine {
  line: number;
  gen: number;
  timestamp: string;
  module: string;
  text: string;
}

export function applyPatchToBlame(blame: readonly number[], patch: string, gen: number): number[] {
  const hunks = parseHunks(patch);
  if (hunks.length === 0) return [...blame];
  const result: number[] = [];
  let oldIdx = 0;
  for (const hunk of hunks) {
    const copyUntil = hunk.oldStart - 1;
    while (oldIdx < copyUntil && oldIdx < blame.length) {
      result.push(blame[oldIdx]);
      oldIdx += 1;
    }
    for (const line of hunk.lines) {
      if (line.kind === ' ') {
        result.push(oldIdx < blame.length ? blame[oldIdx] : gen);
        oldIdx += 1;
      } else if (line.kind === '-') {
        oldIdx += 1;
      } else {
        result.push(gen);
      }
    }
  }
  while (oldIdx < blame.length) {
    result.push(blame[oldIdx]);
    oldIdx += 1;
  }
  return result;
}

export function blameFile(history: readonly BlameSourceEntry[], currentContent: string): BlameLine[] {
  const ordered = [...history].sort((a, b) => a.gen - b.gen);
  let blame: number[] = [];
  for (const entry of ordered) {
    blame = applyPatchToBlame(blame, entry.patch, entry.gen);
  }
  const byGen = new Map(ordered.map((entry) => [entry.gen, entry]));
  const fallbackGen = ordered.length > 0 ? ordered[ordered.length - 1].gen : 0;
  const currentLines = toLines(currentContent);
  return currentLines.map((text, index) => {
    const gen = blame[index] ?? fallbackGen;
    const entry = byGen.get(gen);
    return {
      line: index + 1,
      gen,
      timestamp: entry?.timestamp ?? '',
      module: entry?.module ?? '',
      text,
    };
  });
}

export function blameLineAt(history: readonly BlameSourceEntry[], currentContent: string, line: number): BlameLine | null {
  const blamed = blameFile(history, currentContent);
  return blamed.find((entry) => entry.line === line) ?? null;
}

export function firstPromptDiffLine(promptDiff: string): string {
  const lines = promptDiff.split(/\r?\n/);
  let seenHeader = false;
  let firstRemoval = '';
  for (const line of lines) {
    if (line.startsWith('@@')) {
      seenHeader = true;
      continue;
    }
    if (!seenHeader) continue;
    const text = line.slice(1).trim();
    if (text === '') continue;
    if (line.startsWith('+')) return text;
    if (line.startsWith('-') && firstRemoval === '') firstRemoval = text;
  }
  return firstRemoval;
}
