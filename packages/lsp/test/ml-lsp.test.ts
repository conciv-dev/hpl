import { describe, expect, it } from 'vitest';
import { DiagnosticSeverity } from 'vscode-languageserver/node.js';
import { validateMl } from '@napl/core';
import { mlDiagnostics, mlHoverMarkdown, mlSeverity } from '../src/ml.js';

describe('mlSeverity', () => {
  it('maps each kind to the intended severity', () => {
    expect(mlSeverity('ambiguity')).toBe(DiagnosticSeverity.Error);
    expect(mlSeverity('assumption')).toBe(DiagnosticSeverity.Warning);
    expect(mlSeverity('no-op')).toBe(DiagnosticSeverity.Warning);
    expect(mlSeverity('note')).toBe(DiagnosticSeverity.Information);
  });
});

describe('mlDiagnostics', () => {
  it('places diagnostics at document positions, converting body-relative prompt lines', () => {
    const ml = validateMl({
      module: 'todo-app',
      target: 'react',
      entries: [
        { promptLines: [3, 3], kind: 'ambiguity', message: 'vague phrase', reasoning: 'r' },
        { promptLines: [1, 2], kind: 'assumption', message: 'assumed default' },
      ],
    });
    const docLines = ['---', 'module: x', '---', 'body line 1', 'body line 2', 'body line 3'];
    const diagnostics = mlDiagnostics(ml, 3, docLines);

    expect(diagnostics[0].severity).toBe(DiagnosticSeverity.Error);
    expect(diagnostics[0].source).toBe('napl-mapl');
    expect(diagnostics[0].message).toBe('vague phrase');
    expect(diagnostics[0].range.start.line).toBe(5);
    expect(diagnostics[0].range.end.line).toBe(5);
    expect(diagnostics[0].range.end.character).toBe('body line 3'.length);

    expect(diagnostics[1].severity).toBe(DiagnosticSeverity.Warning);
    expect(diagnostics[1].range.start.line).toBe(3);
    expect(diagnostics[1].range.end.line).toBe(4);
  });
});

describe('mlHoverMarkdown', () => {
  it('renders a machine-says section with kind, message, reasoning and suggestion fence', () => {
    const md = mlHoverMarkdown([
      { promptLines: [1, 1], kind: 'ambiguity', message: 'odd literal', reasoning: 'why it is odd', suggestion: 'reword to X' },
    ]);
    expect(md).toContain('**machine says**');
    expect(md).toContain('_ambiguity_ — odd literal');
    expect(md).toContain('why it is odd');
    expect(md).toContain('```\nreword to X\n```');
  });

  it('omits the suggestion fence when there is no suggestion', () => {
    const md = mlHoverMarkdown([{ promptLines: [1, 1], kind: 'note', message: 'm', reasoning: '' }]);
    expect(md).toContain('_note_ — m');
    expect(md).not.toContain('```');
  });
});
