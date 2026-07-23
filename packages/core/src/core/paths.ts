import { readdir } from 'node:fs/promises';
import { join } from 'node:path';

export const PROMPT_EXTENSION = '.napl';
export const MACHINE_EXTENSION = '.mapl';
export const MACHINE_ALIAS = '.🤖';

export const DEFAULT_PROMPT_ALIASES: readonly string[] = ['.🧑', '.🧓', '.👤', '.👨', '.👩', '.🧒'];

export function promptExtensions(aliases?: readonly string[]): string[] {
  return [PROMPT_EXTENSION, ...(aliases ?? DEFAULT_PROMPT_ALIASES)];
}

export function machineExtensions(): string[] {
  return [MACHINE_EXTENSION, MACHINE_ALIAS];
}

export function isPromptFile(path: string, aliases?: readonly string[]): boolean {
  return promptExtensions(aliases).some((ext) => path.endsWith(ext));
}

export function isMachineFile(path: string): boolean {
  return machineExtensions().some((ext) => path.endsWith(ext));
}

export function machineExtensionForPrompt(promptPath: string): string {
  return promptPath.endsWith(PROMPT_EXTENSION) ? MACHINE_EXTENSION : MACHINE_ALIAS;
}

export interface NaplPaths {
  root: string;
  naplDir: string;
  irDir: string;
  srcDir: string;
  mapPath: string;
  lockPath: string;
  genLockPath: string;
  journalPath: string;
  promptsAtGenDir: string;
  examplesDir: string;
}

export function resolvePaths(root: string): NaplPaths {
  const naplDir = join(root, '.napl');
  return {
    root,
    naplDir,
    irDir: join(naplDir, 'ir'),
    srcDir: join(naplDir, 'src'),
    mapPath: join(naplDir, 'map.json'),
    lockPath: join(naplDir, 'lock.json'),
    genLockPath: join(naplDir, 'gen.lock'),
    journalPath: join(naplDir, 'journal.jsonl'),
    promptsAtGenDir: join(naplDir, 'prompts-at-gen'),
    examplesDir: join(root, 'examples'),
  };
}

const IGNORED_DIRS = new Set(['node_modules', '.napl', '.git']);

export async function findPromptFiles(root: string, aliases?: readonly string[]): Promise<string[]> {
  const results: string[] = [];
  async function walk(dir: string): Promise<void> {
    const entries = await readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        if (IGNORED_DIRS.has(entry.name)) continue;
        await walk(full);
      } else if (entry.isFile() && isPromptFile(entry.name, aliases)) {
        results.push(full);
      }
    }
  }
  await walk(root);
  results.sort();
  return results;
}
