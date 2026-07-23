import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import { z } from 'zod';
import { DEFAULT_PROMPT_ALIASES } from './paths.js';

export const DEFAULT_MODEL = 'claude-sonnet-5';

export const backendSchema = z.enum(['claude-cli', 'anthropic-api']);

export type Backend = z.infer<typeof backendSchema>;

export const DEFAULT_BACKEND: Backend = 'claude-cli';

const ZWJ = '‍';

export const promptAliasSchema = z
  .string()
  .refine((value) => value.startsWith('.'), { message: 'a prompt alias must start with "."' })
  .refine(
    (value) => {
      const codepoints = [...value.slice(1)];
      return codepoints.length >= 1 && codepoints.length <= 2;
    },
    { message: 'a prompt alias must have 1-2 code points after the "."' },
  )
  .refine((value) => !value.includes(ZWJ), {
    message: 'a prompt alias must not contain a ZWJ (zero-width joiner) sequence',
  });

export const lockSchema = z.object({
  model: z.string().min(1),
  backend: backendSchema.default(DEFAULT_BACKEND),
  promptAliases: z.array(promptAliasSchema).optional(),
});

export type HlLock = z.infer<typeof lockSchema>;

export function resolvePromptAliases(lock: HlLock): string[] {
  return lock.promptAliases ?? [...DEFAULT_PROMPT_ALIASES];
}

export async function loadPromptAliases(lockPath: string): Promise<string[]> {
  if (!existsSync(lockPath)) return [...DEFAULT_PROMPT_ALIASES];
  try {
    return resolvePromptAliases(parseLock(await readFile(lockPath, 'utf8')));
  } catch {
    return [...DEFAULT_PROMPT_ALIASES];
  }
}

export function parseLock(raw: string): HlLock {
  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch (cause) {
    throw new Error('corrupt lock.json', { cause });
  }
  const parsed = lockSchema.safeParse(data);
  if (!parsed.success) {
    throw new Error(`invalid lock.json: ${parsed.error.message}`, { cause: parsed.error });
  }
  return parsed.data;
}

export async function readLock(lockPath: string): Promise<HlLock> {
  if (!existsSync(lockPath)) {
    throw new Error("missing .napl/lock.json — run 'napl init' first");
  }
  return parseLock(await readFile(lockPath, 'utf8'));
}

export async function writeLock(lockPath: string, lock: HlLock): Promise<void> {
  await mkdir(dirname(lockPath), { recursive: true });
  await writeFile(lockPath, `${JSON.stringify(lock, null, 2)}\n`, 'utf8');
}
