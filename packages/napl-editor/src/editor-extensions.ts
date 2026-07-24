import type { Extension } from '@codemirror/state';
import { linter, type Diagnostic } from '@codemirror/lint';
import { EditorView, hoverTooltip } from '@codemirror/view';

export type EditorSeverity = 'error' | 'warning' | 'info';

export interface EditorDiagnostic {
  line: number;
  endLine?: number;
  column?: number;
  endColumn?: number;
  severity: EditorSeverity;
  message: string;
}

export type DiagnosticsSource = (
  content: string,
) => EditorDiagnostic[] | Promise<EditorDiagnostic[]>;

export interface HoverContext {
  line: number;
  column: number;
  content: string;
}

export type HoverSource = (
  context: HoverContext,
) => string | null | Promise<string | null>;

const toCmDiagnostic = (view: EditorView, diagnostic: EditorDiagnostic): Diagnostic => {
  const { doc } = view.state;
  const startLineNo = Math.min(Math.max(diagnostic.line + 1, 1), doc.lines);
  const endLineNo = Math.min(
    Math.max((diagnostic.endLine ?? diagnostic.line) + 1, 1),
    doc.lines,
  );
  const startLine = doc.line(startLineNo);
  const endLine = doc.line(endLineNo);
  const from =
    diagnostic.column != null
      ? Math.min(startLine.from + diagnostic.column, startLine.to)
      : startLine.from;
  const to =
    diagnostic.endColumn != null
      ? Math.min(endLine.from + diagnostic.endColumn, endLine.to)
      : endLine.to;
  return {
    from,
    to: Math.max(to, from),
    severity: diagnostic.severity,
    message: diagnostic.message,
  };
};

export const naplLinter = (source: DiagnosticsSource): Extension =>
  linter(async (view) => {
    const items = await source(view.state.doc.toString());
    return items.map((item) => toCmDiagnostic(view, item));
  });

export const naplHover = (source: HoverSource): Extension =>
  hoverTooltip(async (view, pos) => {
    const line = view.state.doc.lineAt(pos);
    const text = await source({
      line: line.number - 1,
      column: pos - line.from,
      content: view.state.doc.toString(),
    });
    if (!text) {
      return null;
    }
    return {
      pos,
      above: true,
      create: () => {
        const dom = document.createElement('div');
        dom.className = 'cm-napl-tooltip';
        dom.textContent = text;
        return { dom };
      },
    };
  });
