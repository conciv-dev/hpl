use napl_core::blame::{blame_file, blame_line_at, BlameLine};
use napl_core::body_lines::{body_line_for_doc_line, prompt_body_lines, PromptBody};
use napl_core::hash::content_hash;
use napl_core::scanner::{
    scan_document, DepSource, DepToken, FrontmatterKeyToken, ModuleValueToken, Position, RefToken,
    RegionSpan, ScanResult, Span,
};
use napl_core::schemas::{
    entries_at_body_line, file_history, ml_entries_at_body_line, parse_frontmatter, parse_map,
    read_journal_str, validate_attribution, validate_ml, AttributionEntry, LineRange, MlEntry,
    MlKind,
};
use napl_core::text_diff::{apply_hunks, parse_hunks};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct Diagnostic {
    message: String,
    line: u32,
    severity: &'static str,
}

#[derive(Serialize)]
struct PositionDto {
    line: u32,
    character: u32,
}

#[derive(Serialize)]
struct SpanDto {
    start: PositionDto,
    end: PositionDto,
}

#[derive(Serialize)]
struct RegionDto {
    present: bool,
    span: Option<SpanDto>,
}

#[derive(Serialize)]
struct KeyDto {
    key: String,
    key_span: SpanDto,
}

#[derive(Serialize)]
struct ModuleValueDto {
    value: String,
    span: SpanDto,
}

#[derive(Serialize)]
struct DepDto {
    value: String,
    span: SpanDto,
    source: &'static str,
}

#[derive(Serialize)]
struct RefDto {
    module: String,
    span: SpanDto,
}

#[derive(Serialize)]
struct ScanDto {
    frontmatter: RegionDto,
    body: RegionDto,
    keys: Vec<KeyDto>,
    module_value: Option<ModuleValueDto>,
    deps: Vec<DepDto>,
    refs: Vec<RefDto>,
}

#[derive(Serialize)]
struct BodyLinesDto {
    body_start_line: u32,
    lines: Vec<String>,
}

#[derive(Serialize)]
struct RangeDto {
    start: u32,
    end: u32,
}

#[derive(Serialize)]
struct AttributionForwardDto {
    prompt_lines: RangeDto,
    file: String,
    lines: RangeDto,
    note: String,
}

#[derive(Serialize)]
struct AttributionReverseDto {
    prompt_lines: RangeDto,
    file: String,
    lines: RangeDto,
    note: String,
}

#[derive(Serialize)]
struct MaplEntryDto {
    prompt_lines: RangeDto,
    kind: &'static str,
    severity: &'static str,
    message: String,
    reasoning: String,
    suggestion: Option<String>,
}

#[derive(Serialize)]
struct BlameLineDto {
    line: u32,
    gen: i64,
    timestamp: String,
    module: String,
    text: String,
}

#[derive(Serialize)]
struct DriftFileDto {
    file: String,
    target: String,
    status: &'static str,
    expected_hash: String,
    actual_hash: Option<String>,
    prompts: Vec<String>,
}

fn position_dto(position: Position) -> PositionDto {
    PositionDto {
        line: position.line as u32,
        character: position.character as u32,
    }
}

fn span_dto(span: Span) -> SpanDto {
    SpanDto {
        start: position_dto(span.start),
        end: position_dto(span.end),
    }
}

fn region_dto(region: &RegionSpan) -> RegionDto {
    RegionDto {
        present: region.present,
        span: region.span.map(span_dto),
    }
}

fn range_dto(range: LineRange) -> RangeDto {
    RangeDto {
        start: range.start,
        end: range.end,
    }
}

fn dep_source_str(source: DepSource) -> &'static str {
    match source {
        DepSource::Deps => "deps",
        DepSource::Extends => "extends",
    }
}

fn kind_str(kind: MlKind) -> &'static str {
    match kind {
        MlKind::Ambiguity => "ambiguity",
        MlKind::Assumption => "assumption",
        MlKind::Note => "note",
        MlKind::NoOp => "no-op",
    }
}

fn kind_severity(kind: MlKind) -> &'static str {
    match kind {
        MlKind::Ambiguity => "error",
        MlKind::Assumption | MlKind::NoOp => "warning",
        MlKind::Note => "info",
    }
}

fn emit<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

fn parse_document(raw: &str) -> Result<serde_json::Value, JsError> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        return Ok(value);
    }
    serde_yaml::from_str::<serde_json::Value>(raw).map_err(|error| JsError::new(&error.to_string()))
}

#[wasm_bindgen]
pub fn parse_frontmatter_diagnostics(content: &str) -> String {
    let diagnostics: Vec<Diagnostic> = match parse_frontmatter(content) {
        Ok(_) => Vec::new(),
        Err(error) => vec![Diagnostic {
            message: error.to_string(),
            line: 0,
            severity: "error",
        }],
    };
    emit(&diagnostics)
}

#[wasm_bindgen]
pub fn scan_document_json(content: &str) -> String {
    let scan: ScanResult = scan_document(content);
    let dto = ScanDto {
        frontmatter: region_dto(&scan.frontmatter),
        body: region_dto(&scan.body),
        keys: scan
            .keys
            .iter()
            .map(|key: &FrontmatterKeyToken| KeyDto {
                key: key.key.clone(),
                key_span: span_dto(key.key_span),
            })
            .collect(),
        module_value: scan
            .module_value
            .as_ref()
            .map(|value: &ModuleValueToken| ModuleValueDto {
                value: value.value.clone(),
                span: span_dto(value.span),
            }),
        deps: scan
            .deps
            .iter()
            .map(|dep: &DepToken| DepDto {
                value: dep.value.clone(),
                span: span_dto(dep.span),
                source: dep_source_str(dep.source),
            })
            .collect(),
        refs: scan
            .refs
            .iter()
            .map(|reference: &RefToken| RefDto {
                module: reference.module.clone(),
                span: span_dto(reference.span),
            })
            .collect(),
    };
    emit(&dto)
}

#[wasm_bindgen]
pub fn body_line_map(content: &str) -> String {
    let body: PromptBody = prompt_body_lines(content);
    let dto = BodyLinesDto {
        body_start_line: body.body_start_line as u32,
        lines: body.lines.clone(),
    };
    emit(&dto)
}

#[wasm_bindgen]
pub fn doc_line_to_body_line(content: &str, doc_line: u32) -> String {
    let body = prompt_body_lines(content);
    let mapped = body_line_for_doc_line(&body, i64::from(doc_line)).map(|line| line as u32);
    emit(&mapped)
}

#[wasm_bindgen]
pub fn body_line_to_doc_line(content: &str, body_line: u32) -> String {
    let body = prompt_body_lines(content);
    let mapped = if body_line >= 1 && (body_line as usize) <= body.lines.len() {
        Some(body.body_start_line as u32 + body_line - 1)
    } else {
        None
    };
    emit(&mapped)
}

#[wasm_bindgen]
pub fn attribution_at_prompt_line(
    attribution: &str,
    prompt_line: u32,
) -> Result<String, JsError> {
    let value = parse_document(attribution)?;
    let document =
        validate_attribution(value).map_err(|error| JsError::new(&error.to_string()))?;
    let entries: Vec<AttributionForwardDto> = entries_at_body_line(&document, prompt_line)
        .into_iter()
        .map(|entry: &AttributionEntry| AttributionForwardDto {
            prompt_lines: range_dto(entry.prompt_lines),
            file: entry.file.clone(),
            lines: range_dto(entry.lines),
            note: entry.note.clone(),
        })
        .collect();
    Ok(emit(&entries))
}

#[wasm_bindgen]
pub fn attribution_at_file_line(
    attribution: &str,
    file: &str,
    line: u32,
) -> Result<String, JsError> {
    let value = parse_document(attribution)?;
    let document =
        validate_attribution(value).map_err(|error| JsError::new(&error.to_string()))?;
    let entries: Vec<AttributionReverseDto> = document
        .entries
        .iter()
        .filter(|entry| entry.file == file && entry.lines.start <= line && line <= entry.lines.end)
        .map(|entry| AttributionReverseDto {
            prompt_lines: range_dto(entry.prompt_lines),
            file: entry.file.clone(),
            lines: range_dto(entry.lines),
            note: entry.note.clone(),
        })
        .collect();
    Ok(emit(&entries))
}

#[wasm_bindgen]
pub fn mapl_parse(content: &str) -> Result<String, JsError> {
    let value = parse_document(content)?;
    let document = validate_ml(value).map_err(|error| JsError::new(&error.to_string()))?;
    let entries: Vec<MaplEntryDto> = document
        .entries
        .iter()
        .map(|entry: &MlEntry| MaplEntryDto {
            prompt_lines: range_dto(entry.prompt_lines),
            kind: kind_str(entry.kind),
            severity: kind_severity(entry.kind),
            message: entry.message.clone(),
            reasoning: entry.reasoning.clone(),
            suggestion: entry.suggestion.clone(),
        })
        .collect();
    Ok(emit(&entries))
}

#[wasm_bindgen]
pub fn mapl_entries_at_prompt_line(content: &str, prompt_line: u32) -> Result<String, JsError> {
    let value = parse_document(content)?;
    let document = validate_ml(value).map_err(|error| JsError::new(&error.to_string()))?;
    let entries: Vec<MaplEntryDto> = ml_entries_at_body_line(&document, prompt_line)
        .into_iter()
        .map(|entry: &MlEntry| MaplEntryDto {
            prompt_lines: range_dto(entry.prompt_lines),
            kind: kind_str(entry.kind),
            severity: kind_severity(entry.kind),
            message: entry.message.clone(),
            reasoning: entry.reasoning.clone(),
            suggestion: entry.suggestion.clone(),
        })
        .collect();
    Ok(emit(&entries))
}

fn reconstruct_file_content(history: &[napl_core::blame::BlameSourceEntry]) -> String {
    let mut content = String::new();
    for entry in history {
        let hunks = parse_hunks(&entry.patch);
        content = apply_hunks(&content, &hunks);
    }
    content
}

fn blame_line_dto(line: &BlameLine) -> BlameLineDto {
    BlameLineDto {
        line: line.line as u32,
        gen: line.gen,
        timestamp: line.timestamp.clone(),
        module: line.module.clone(),
        text: line.text.clone(),
    }
}

#[wasm_bindgen]
pub fn blame_replay(journal_jsonl: &str, file_path: &str, line: Option<u32>) -> String {
    let (entries, _warnings) = read_journal_str(journal_jsonl);
    let history = file_history(&entries, file_path);
    let content = reconstruct_file_content(&history);
    match line {
        Some(target) => {
            let blamed = blame_line_at(&history, &content, target as usize)
                .map(|blame| blame_line_dto(&blame));
            emit(&blamed)
        }
        None => {
            let blamed: Vec<BlameLineDto> = blame_file(&history, &content)
                .iter()
                .map(blame_line_dto)
                .collect();
            emit(&blamed)
        }
    }
}

#[wasm_bindgen]
pub fn drift_detect(map_json: &str, file_contents_json: &str) -> Result<String, JsError> {
    let map = parse_map(map_json).map_err(|error| JsError::new(&error.to_string()))?;
    let contents: std::collections::HashMap<String, String> =
        serde_json::from_str(file_contents_json)
            .map_err(|error| JsError::new(&error.to_string()))?;
    let files: Vec<DriftFileDto> = map
        .files
        .iter()
        .map(|(path, record)| {
            let actual = contents.get(path).map(|content| content_hash(content));
            let status = match &actual {
                None => "missing",
                Some(hash) if *hash == record.hash => "clean",
                Some(_) => "edited",
            };
            DriftFileDto {
                file: path.clone(),
                target: record.target.clone(),
                status,
                expected_hash: record.hash.clone(),
                actual_hash: actual,
                prompts: record.prompts.clone(),
            }
        })
        .collect();
    Ok(emit(&files))
}
