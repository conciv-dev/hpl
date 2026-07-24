/* tslint:disable */
/* eslint-disable */

export function attribution_at_file_line(attribution: string, file: string, line: number): string;

export function attribution_at_prompt_line(attribution: string, prompt_line: number): string;

export function blame_replay(journal_jsonl: string, file_path: string, line?: number | null): string;

export function body_line_map(content: string): string;

export function body_line_to_doc_line(content: string, body_line: number): string;

export function doc_line_to_body_line(content: string, doc_line: number): string;

export function drift_detect(map_json: string, file_contents_json: string): string;

export function mapl_entries_at_prompt_line(content: string, prompt_line: number): string;

export function mapl_parse(content: string): string;

export function parse_frontmatter_diagnostics(content: string): string;

export function scan_document_json(content: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly attribution_at_file_line: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly attribution_at_prompt_line: (a: number, b: number, c: number) => [number, number, number, number];
    readonly blame_replay: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly body_line_map: (a: number, b: number) => [number, number];
    readonly body_line_to_doc_line: (a: number, b: number, c: number) => [number, number];
    readonly doc_line_to_body_line: (a: number, b: number, c: number) => [number, number];
    readonly drift_detect: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly mapl_entries_at_prompt_line: (a: number, b: number, c: number) => [number, number, number, number];
    readonly mapl_parse: (a: number, b: number) => [number, number, number, number];
    readonly parse_frontmatter_diagnostics: (a: number, b: number) => [number, number];
    readonly scan_document_json: (a: number, b: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
