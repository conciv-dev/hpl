# Pure classification and derivation helpers for the generator

This module is a small set of **pure** helpers the code-generation command uses to
classify files and derive one-line descriptions from prompt bodies. Every function
is pure — no filesystem access, no process spawning, no dependencies on other
project modules. The I/O shell reads files and prompt text off disk, then calls
these helpers over the already-read strings.

## Where this code lives

The working directory is a Cargo workspace whose root manifest is written and
owned by the toolchain — leave it alone. Create this module as its own member
crate in a subdirectory named `gen_classify/`: `gen_classify/Cargo.toml` (package
name `gen_classify`) and `gen_classify/src/lib.rs`. Touch nothing outside
`gen_classify/`. Ensure `cargo test` passes from the workspace root before
finishing.

## The source-file extensions

Expose a public constant array of the recognized source-file extensions, in
exactly this order, each including its leading dot: `.ts`, `.tsx`, `.js`, `.jsx`,
`.css`, `.html`, `.rs`.

## `is_source_file(rel_to_target: &str) -> bool`

Decide whether a path (relative to the generation target directory) names a source
file that should be numbered for attribution. Steps:

1. Take the **base name** — the substring after the last `/`, or the whole string
   when there is no `/`.
2. If the base name ends with any of these four config suffixes, it is **not** a
   source file (return `false`): `.config.ts`, `.config.tsx`, `.config.js`,
   `.config.jsx`.
3. Find the last `.` in the base name. If there is none, it is not a source file
   (return `false`).
4. Otherwise the extension is the base name from that last `.` to the end
   (including the dot). Return `true` when that extension is one of the recognized
   source-file extensions above, and `false` otherwise.

Worked decisions: `src/lib.rs` → `true` (`.rs`); `app.tsx` → `true`; `page.html`
→ `true`; `vite.config.ts` → `false` (config suffix); `README.md` → `false`
(`.md` is not recognized); `noext` → `false` (no extension).

## `first_meaningful_line(body: &str) -> String`

Derive a one-line description from a prompt body. Split the body on `\n`. For each
line, in order: strip a single trailing `\r` if present, then strip **all** leading
`#` characters, then trim leading and trailing whitespace. Return the first such
result that is non-empty, capped to at most the first 120 Unicode characters. If no
line produces a non-empty result, return the literal string `(no description)`.

Worked decisions: `# Title\n\nBody line` → `Title` (the heading marker and blanks
are skipped/stripped); `### Deep\nmore` → `Deep`; a body of only whitespace lines
→ `(no description)`; a single line of 200 `x` characters → exactly 120 `x`
characters.

## `split_body_lines(content: &str) -> Vec<String>`

Split content into lines for numbering. Split on `\n`; for each piece, strip a
single trailing `\r` if present, and collect the results as owned strings. This is
the CRLF-aware line split: `a\r\nb\nc` → `["a", "b", "c"]`; the empty string →
`[""]` (one empty line); `x\r\n` → `["x", ""]` (a trailing newline yields a final
empty line).
