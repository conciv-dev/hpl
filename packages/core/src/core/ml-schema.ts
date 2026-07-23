import { z } from 'zod';
import { lineRangeSchema } from './attribution-schema.js';

export const mlKindSchema = z.enum(['ambiguity', 'assumption', 'note', 'no-op']);

export const mlEntrySchema = z.object({
  promptLines: lineRangeSchema,
  kind: mlKindSchema,
  message: z.string().min(1),
  reasoning: z.string().default(''),
  suggestion: z.string().optional(),
});

export const mlSchema = z.object({
  module: z.string().min(1),
  target: z.string().min(1),
  entries: z.array(mlEntrySchema).default([]),
});

export type MlKind = z.infer<typeof mlKindSchema>;
export type MlEntry = z.infer<typeof mlEntrySchema>;
export type Ml = z.infer<typeof mlSchema>;

export function validateMl(data: unknown): Ml {
  const parsed = mlSchema.safeParse(data);
  if (!parsed.success) {
    throw new Error(`machine-layer validation failed: ${parsed.error.message}`, { cause: parsed.error });
  }
  return parsed.data;
}

export function parseMlEntries(data: unknown): MlEntry[] {
  const list = Array.isArray(data) ? data : [];
  const parsed = z.array(mlEntrySchema).safeParse(list);
  if (!parsed.success) {
    throw new Error(`machine-layer entries invalid: ${parsed.error.message}`, { cause: parsed.error });
  }
  return parsed.data;
}

export function mlEntriesAtBodyLine(ml: Ml, bodyLine: number): MlEntry[] {
  return ml.entries.filter(
    (entry) => bodyLine >= entry.promptLines[0] && bodyLine <= entry.promptLines[1],
  );
}
