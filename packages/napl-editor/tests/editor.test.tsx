import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { StringStream } from '@codemirror/language';
import { forEachDiagnostic, forceLinting } from '@codemirror/lint';
import { EditorView } from '@codemirror/view';
import { cleanup, fireEvent, render } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { NaplEditor } from '../src/NaplEditor.tsx';
import { PlaygroundShell } from '../src/PlaygroundShell.tsx';
import { naplLinter } from '../src/editor-extensions.ts';
import { naplStreamParser, type NaplStreamState } from '../src/napl-language.ts';

const sample = (): string =>
  readFileSync(resolve(process.cwd(), '../../selfhost/body_lines.napl'), 'utf8');

interface Token {
  text: string;
  tag: string | null;
}

const tokenize = (source: string): Token[] => {
  const state: NaplStreamState = naplStreamParser.startState!(2);
  const tokens: Token[] = [];
  for (const line of source.split('\n')) {
    if (line.length === 0) {
      naplStreamParser.blankLine?.(state, 2);
      continue;
    }
    const stream = new StringStream(line, 4, 2);
    while (!stream.eol()) {
      stream.start = stream.pos;
      const tag = naplStreamParser.token(stream, state);
      if (stream.pos === stream.start) {
        stream.pos += 1;
      }
      tokens.push({ text: stream.current(), tag });
    }
  }
  return tokens;
};

afterEach(() => {
  cleanup();
});

describe('NaplEditor', () => {
  it('renders prompt content in a CodeMirror editor', () => {
    const { container } = render(<NaplEditor value={sample()} readOnly />);
    const content = container.querySelector('.cm-content');
    expect(content).not.toBeNull();
    expect(content?.textContent).toContain('module');
    expect(content?.textContent).toContain('body_lines');
  });

  it('surfaces diagnostics from the callback as lint marks', async () => {
    const view = new EditorView({
      doc: '---\ndeps: []\n---\nbody\n',
      extensions: [
        naplLinter(() => [
          { line: 0, severity: 'error', message: 'frontmatter is missing module' },
        ]),
      ],
      parent: document.body,
    });
    forceLinting(view);
    await new Promise((resolve) => setTimeout(resolve, 80));
    const messages: string[] = [];
    forEachDiagnostic(view.state, (diagnostic) => messages.push(diagnostic.message));
    expect(messages).toContain('frontmatter is missing module');
    view.destroy();
  });
});

describe('napl StreamLanguage', () => {
  it('tokenizes frontmatter keys and body distinctly on a real prompt', () => {
    const tokens = tokenize(sample());
    const moduleKey = tokens.find((token) => token.text === 'module');
    const testKey = tokens.find((token) => token.text === 'given' || token.text === 'expect');
    const heading = tokens.find((token) => token.tag === 'heading');

    expect(moduleKey?.tag).toBe('frontmatterKey');
    expect(testKey?.tag).toBe('testKey');
    expect(heading).toBeDefined();
    expect(moduleKey?.tag).not.toBe(heading?.tag);
  });
});

describe('PlaygroundShell', () => {
  const files = [
    { name: 'greeting.napl', language: 'napl' as const, content: '---\nmodule: greeting\n---\nSay hi.\n' },
    { name: 'greeting.ts', language: 'source' as const, content: 'export const greet = () => "hi";\n' },
    { name: 'greeting.mapl', language: 'mapl' as const, content: 'module: greeting\ntarget: typescript\n' },
  ];

  it('switches the editor pane when a tab is clicked', () => {
    const { container, getByRole } = render(<PlaygroundShell files={files} />);
    expect(container.querySelector('.cm-content')?.textContent).toContain('module: greeting');

    fireEvent.click(getByRole('tab', { name: /greeting\.ts/ }));
    expect(container.querySelector('.cm-content')?.textContent).toContain('export const greet');
  });

  it('renders a tab per file with an output pane', () => {
    const { getAllByRole, getByTestId } = render(<PlaygroundShell files={files} />);
    expect(getAllByRole('tab')).toHaveLength(3);
    expect(getByTestId('napl-playground-output')).not.toBeNull();
  });
});
