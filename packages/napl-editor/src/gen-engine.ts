export type GenLineRange = [number, number];

export interface AttributionEntryEvent {
  promptLines: GenLineRange;
  file: string;
  lines: GenLineRange;
  note: string;
}

export type MaplKind = 'ambiguity' | 'assumption' | 'note' | 'no-op';

export interface MaplEntryEvent {
  promptLines: GenLineRange;
  kind: MaplKind;
  severity: 'error' | 'warning' | 'info';
  message: string;
  reasoning?: string;
  suggestion?: string | null;
}

export interface LockedFile {
  path: string;
  hash: string;
}

export type GenEvent =
  | { type: 'task'; task: string }
  | { type: 'file-edit'; path: string; content: string }
  | { type: 'diff'; path: string; patch: string }
  | {
      type: 'attribution';
      module: string;
      target: string;
      entries: AttributionEntryEvent[];
    }
  | { type: 'mapl-entry'; path: string; entry: MaplEntryEvent }
  | { type: 'lock'; module: string; target: string; files: LockedFile[] }
  | { type: 'error'; message: string };

export interface GenEngine {
  run(task: string, files: Record<string, string>): AsyncIterable<GenEvent>;
}

export interface RecordedSession {
  task: string;
  files: Record<string, string>;
  events: GenEvent[];
}
