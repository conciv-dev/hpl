# Prompt-file discovery: the filesystem walk

This module is the CLI's prompt-discovery seam: it walks a project directory tree
and collects every prompt file, skipping the state and dependency directories that
must never be treated as source. It is an **I/O shell**, not a pure module — it
reads the real filesystem — so it declares no given/expect corpus; its behavior is
pinned by the conformance suite (prompt discovery underlies nearly every scenario)
and by its own filesystem walk test. The pure path algebra (`resolve_paths`,
`NaplPaths`, `rel_to`) lives in a separate crate and is not part of this module.

## Where this code lives

The working directory is a Cargo workspace whose root manifest is written and
owned by the toolchain — leave it alone. Create this module as its own member
crate in a subdirectory named `paths_walk/`: `paths_walk/Cargo.toml` (package name
`paths_walk`) and `paths_walk/src/lib.rs`. Touch nothing outside `paths_walk/`.
Ensure `cargo test` passes from the workspace root before finishing.

## Builds on one module of this workspace

- **`extensions`** (`../extensions`): `is_prompt_file(path: &str, aliases:
  Option<&[&str]>) -> bool` decides whether a file name is a prompt file, given the
  configured prompt-file aliases. Use its public API — do not reimplement its types
  or logic, and do not depend on any hand-written crate. Note the alias parameter
  is a slice of string slices (`&[&str]`); this module receives aliases as
  `&[String]` and must borrow them into that shape before calling.

## The ignored directories

Three directory names are never descended into during discovery: `node_modules`,
`.napl`, and `.git`. These are dependency and toolchain-state trees, never source.

## `find_prompt_files(root, aliases)`

`find_prompt_files(root: &std::path::Path, aliases: &[String]) ->
std::io::Result<Vec<std::path::PathBuf>>`: discover every prompt file under `root`.
Walk the directory tree rooted at `root` and collect the full path of each entry
that is a file and whose name is a prompt file (per `is_prompt_file`, passing the
`aliases`). Descend into subdirectories, **except** any directory whose name is one
of the three ignored names above (skip those without descending). Return the
collected paths **sorted** (the default path ordering).

The walk is tolerant of a missing directory: if a directory to be read does not
exist (the not-found I/O error kind), treat it as empty rather than an error — so
discovering under a `root` that does not exist yields an empty list, not an `Err`.
Any other I/O error (reading a directory, reading an entry, or querying an entry's
file type) is propagated as `Err`.

## Filesystem walk test to include

Write the crate's own `#[cfg(test)]` test against a unique temporary directory.
Create `examples/` with `a.napl` and `b.napl` and a `notprompt.txt`; create a
`.napl/src/` with an `ignored.napl`; create `node_modules/` with a `dep.napl`.
Discovering under the temp directory with `extensions::default_prompt_aliases()` as
the aliases returns exactly the two example prompts (`examples/a.napl` and
`examples/b.napl`) in sorted order — the `.napl` and `node_modules` trees are
skipped and the non-prompt file is excluded.
