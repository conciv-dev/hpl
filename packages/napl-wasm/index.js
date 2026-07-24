import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import init, {
  attribution_at_file_line,
  attribution_at_prompt_line,
  blame_replay,
  body_line_map,
  body_line_to_doc_line,
  doc_line_to_body_line,
  drift_detect,
  mapl_entries_at_prompt_line,
  mapl_parse,
  parse_frontmatter_diagnostics,
  scan_document_json,
} from './pkg/napl_wasm.js';

let ready;

export const initNaplWasm = async (input) => {
  if (input !== undefined) {
    await init({ module_or_path: input });
    return;
  }
  if (!ready) {
    const wasmUrl = new URL('./pkg/napl_wasm_bg.wasm', import.meta.url);
    const bytes = await readFile(fileURLToPath(wasmUrl));
    ready = init({ module_or_path: bytes });
  }
  await ready;
};

export const parseFrontmatterDiagnostics = (content) =>
  JSON.parse(parse_frontmatter_diagnostics(content));

export const scanDocument = (content) => JSON.parse(scan_document_json(content));

export const bodyLineMap = (content) => JSON.parse(body_line_map(content));

export const docLineToBodyLine = (content, docLine) =>
  JSON.parse(doc_line_to_body_line(content, docLine));

export const bodyLineToDocLine = (content, bodyLine) =>
  JSON.parse(body_line_to_doc_line(content, bodyLine));

export const attributionAtPromptLine = (attribution, promptLine) =>
  JSON.parse(attribution_at_prompt_line(attribution, promptLine));

export const attributionAtFileLine = (attribution, file, line) =>
  JSON.parse(attribution_at_file_line(attribution, file, line));

export const maplParse = (content) => JSON.parse(mapl_parse(content));

export const maplEntriesAtPromptLine = (content, promptLine) =>
  JSON.parse(mapl_entries_at_prompt_line(content, promptLine));

export const blameReplay = (journalJsonl, filePath, line) =>
  JSON.parse(blame_replay(journalJsonl, filePath, line ?? undefined));

export const driftDetect = (mapJson, fileContentsJson) =>
  JSON.parse(drift_detect(mapJson, fileContentsJson));
