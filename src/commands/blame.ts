import { existsSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { isAbsolute, join, relative, resolve, sep } from 'node:path';
import { blameFile, firstPromptDiffLine } from '../core/blame.js';
import type { BlameLine } from '../core/blame.js';
import { fileHistory, readJournal } from '../core/journal.js';
import type { JournalEntry } from '../core/journal.js';
import { resolvePaths } from '../core/paths.js';

export interface BlameOptions {
  root: string;
  file?: string;
  line?: number;
  gen?: number;
  verbose?: boolean;
  log?: (message: string) => void;
}

export interface BlameResult {
  exitCode: number;
}

function toPosix(path: string): string {
  return path.split(sep).join('/');
}

function normalizeFileArg(root: string, file: string): string {
  const abs = isAbsolute(file) ? file : resolve(root, file);
  return toPosix(relative(root, abs));
}

function whyLine(promptDiff: string): string {
  const first = firstPromptDiffLine(promptDiff);
  return first === '' ? 'initial generation' : first;
}

function formatBlameRow(entry: BlameLine): string {
  return `gen #${entry.gen}  ${entry.timestamp}  ${entry.module}  ${entry.text}`;
}

function blameGen(entries: JournalEntry[], gen: number, log: (message: string) => void): BlameResult {
  const entry = entries.find((candidate) => candidate.gen === gen);
  if (entry === undefined) {
    log(`hl blame: no journal entry for gen #${gen}`);
    return { exitCode: 1 };
  }
  log(`gen #${entry.gen}  ${entry.timestamp}  ${entry.module} (${entry.target})  mode: ${entry.mode}`);
  log('');
  log('prompt edit:');
  log(entry.promptDiff.trim() === '' ? '  initial generation' : entry.promptDiff.replace(/^/gm, '  '));
  log('');
  log('files touched:');
  if (entry.files.length === 0) {
    log('  (none)');
  } else {
    for (const file of entry.files) log(`  ${file.path}`);
  }
  return { exitCode: 0 };
}

export async function runBlame(options: BlameOptions): Promise<BlameResult> {
  const log = options.log ?? ((): void => undefined);
  const paths = resolvePaths(options.root);
  const entries = await readJournal(paths.journalPath, log);

  if (entries.length === 0) {
    log('hl blame: no gen journal found — run `hl gen` to start recording line history.');
    return { exitCode: 1 };
  }

  if (options.gen !== undefined) {
    return blameGen(entries, options.gen, log);
  }

  if (options.file === undefined) {
    log('hl blame: provide a generated file path, or --gen <n>.');
    return { exitCode: 1 };
  }

  const relPath = normalizeFileArg(options.root, options.file);
  const history = fileHistory(entries, relPath);
  if (history.length === 0) {
    log(`hl blame: no journal history for ${relPath} — is it a generated file under .hl/src/?`);
    return { exitCode: 1 };
  }

  const abs = join(options.root, relPath);
  if (!existsSync(abs)) {
    log(`hl blame: file not found on disk: ${relPath}`);
    return { exitCode: 1 };
  }
  const content = await readFile(abs, 'utf8');
  const blamed = blameFile(history, content);

  const rows =
    options.line !== undefined ? blamed.filter((entry) => entry.line === options.line) : blamed;
  if (options.line !== undefined && rows.length === 0) {
    log(`hl blame: line ${options.line} is out of range for ${relPath} (${blamed.length} line(s)).`);
    return { exitCode: 1 };
  }

  const promptDiffByGen = new Map(history.map((entry) => [entry.gen, entry.promptDiff]));
  const shownWhy = new Set<number>();
  for (const entry of rows) {
    log(formatBlameRow(entry));
    if (options.verbose === true && !shownWhy.has(entry.gen)) {
      shownWhy.add(entry.gen);
      log(`    why (gen #${entry.gen}): ${whyLine(promptDiffByGen.get(entry.gen) ?? '')}`);
    }
  }
  return { exitCode: 0 };
}
