import { Compartment, EditorState, type Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { basicSetup } from 'codemirror';
import { useEffect, useRef, type ReactElement } from 'react';
import {
  naplHover,
  naplLinter,
  type DiagnosticsSource,
  type HoverSource,
} from './editor-extensions.ts';
import { naplLanguage } from './napl-language.ts';

export interface NaplEditorProps {
  value: string;
  onChange?: (value: string) => void;
  readOnly?: boolean;
  diagnostics?: DiagnosticsSource;
  hover?: HoverSource;
  className?: string;
}

const readOnlyExtension = (readOnly: boolean): Extension => [
  EditorState.readOnly.of(readOnly),
  EditorView.editable.of(!readOnly),
];

export const NaplEditor = ({
  value,
  onChange,
  readOnly = false,
  diagnostics,
  hover,
  className,
}: NaplEditorProps): ReactElement => {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const viewRef = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  const diagnosticsRef = useRef(diagnostics);
  const hoverRef = useRef(hover);
  const readOnlyCompartment = useRef(new Compartment());

  onChangeRef.current = onChange;
  diagnosticsRef.current = diagnostics;
  hoverRef.current = hover;

  useEffect(() => {
    const host = hostRef.current;
    if (!host) {
      return undefined;
    }
    const view = new EditorView({
      state: EditorState.create({
        doc: value,
        extensions: [
          basicSetup,
          naplLanguage(),
          readOnlyCompartment.current.of(readOnlyExtension(readOnly)),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              onChangeRef.current?.(update.state.doc.toString());
            }
          }),
          naplLinter((content) =>
            diagnosticsRef.current ? diagnosticsRef.current(content) : [],
          ),
          naplHover((context) =>
            hoverRef.current ? hoverRef.current(context) : null,
          ),
        ],
      }),
      parent: host,
    });
    viewRef.current = view;
    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, []);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) {
      return;
    }
    const current = view.state.doc.toString();
    if (current !== value) {
      view.dispatch({ changes: { from: 0, to: current.length, insert: value } });
    }
  }, [value]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) {
      return;
    }
    view.dispatch({
      effects: readOnlyCompartment.current.reconfigure(readOnlyExtension(readOnly)),
    });
  }, [readOnly]);

  return (
    <div
      ref={hostRef}
      data-testid="napl-editor"
      className={className ? `napl-editor ${className}` : 'napl-editor'}
    />
  );
};
