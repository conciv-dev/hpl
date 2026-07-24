export type Severity = 'error' | 'warning' | 'info';

export interface Diagnostic {
  message: string;
  line: number;
  severity: Severity;
}

export interface DocPosition {
  line: number;
  character: number;
}

export interface DocSpan {
  start: DocPosition;
  end: DocPosition;
}

export interface Region {
  present: boolean;
  span: DocSpan | null;
}

export interface FrontmatterKey {
  key: string;
  key_span: DocSpan;
}

export interface ModuleValue {
  value: string;
  span: DocSpan;
}

export type DepSource = 'deps' | 'extends';

export interface Dep {
  value: string;
  span: DocSpan;
  source: DepSource;
}

export interface Ref {
  module: string;
  span: DocSpan;
}

export interface ScanResult {
  frontmatter: Region;
  body: Region;
  keys: FrontmatterKey[];
  module_value: ModuleValue | null;
  deps: Dep[];
  refs: Ref[];
}

export interface BodyLines {
  body_start_line: number;
  lines: string[];
}

export interface LineRange {
  start: number;
  end: number;
}

export interface AttributionSpan {
  prompt_lines: LineRange;
  file: string;
  lines: LineRange;
  note: string;
}

export type MaplKind = 'ambiguity' | 'assumption' | 'note' | 'no-op';

export interface MaplEntry {
  prompt_lines: LineRange;
  kind: MaplKind;
  severity: Severity;
  message: string;
  reasoning: string;
  suggestion: string | null;
}

export interface BlameLine {
  line: number;
  gen: number;
  timestamp: string;
  module: string;
  text: string;
}

export type DriftStatus = 'clean' | 'edited' | 'missing';

export interface DriftFile {
  file: string;
  target: string;
  status: DriftStatus;
  expected_hash: string;
  actual_hash: string | null;
  prompts: string[];
}

export type WasmInitInput =
  | Response
  | BufferSource
  | WebAssembly.Module
  | URL
  | Promise<Response | BufferSource | WebAssembly.Module>;

export function initNaplWasm(input?: WasmInitInput): Promise<void>;

export function parseFrontmatterDiagnostics(content: string): Diagnostic[];
export function scanDocument(content: string): ScanResult;
export function bodyLineMap(content: string): BodyLines;
export function docLineToBodyLine(content: string, docLine: number): number | null;
export function bodyLineToDocLine(content: string, bodyLine: number): number | null;
export function attributionAtPromptLine(
  attribution: string,
  promptLine: number,
): AttributionSpan[];
export function attributionAtFileLine(
  attribution: string,
  file: string,
  line: number,
): AttributionSpan[];
export function maplParse(content: string): MaplEntry[];
export function maplEntriesAtPromptLine(content: string, promptLine: number): MaplEntry[];
export function blameReplay(
  journalJsonl: string,
  filePath: string,
  line?: number,
): BlameLine | null | BlameLine[];
export function driftDetect(mapJson: string, fileContentsJson: string): DriftFile[];
