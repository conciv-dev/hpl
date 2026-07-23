import { DiagnosticSeverity } from 'vscode-languageserver/node.js';
import type { Diagnostic } from 'vscode-languageserver/node.js';
import type { Ml, MlEntry, MlKind } from '@napl/core';

export function mlSeverity(kind: MlKind): DiagnosticSeverity {
  switch (kind) {
    case 'ambiguity':
      return DiagnosticSeverity.Error;
    case 'assumption':
      return DiagnosticSeverity.Warning;
    case 'no-op':
      return DiagnosticSeverity.Warning;
    case 'note':
      return DiagnosticSeverity.Information;
  }
}

export function mlDiagnostics(ml: Ml, bodyStartLine: number, docLines: string[]): Diagnostic[] {
  return ml.entries.map((entry) => {
    const startLine = bodyStartLine + entry.promptLines[0] - 1;
    const endLine = bodyStartLine + entry.promptLines[1] - 1;
    const endChar = docLines[endLine]?.length ?? 200;
    return {
      severity: mlSeverity(entry.kind),
      range: { start: { line: startLine, character: 0 }, end: { line: endLine, character: endChar } },
      message: entry.message,
      source: 'napl-mapl',
    };
  });
}

export function mlHoverMarkdown(entries: readonly MlEntry[]): string {
  const lines: string[] = ['**machine says**', ''];
  for (const entry of entries) {
    lines.push(`- _${entry.kind}_ — ${entry.message}`);
    if (entry.reasoning.trim() !== '') lines.push(`  ${entry.reasoning}`);
    if (entry.suggestion !== undefined && entry.suggestion.trim() !== '') {
      lines.push('```');
      lines.push(entry.suggestion);
      lines.push('```');
    }
  }
  return lines.join('\n');
}
