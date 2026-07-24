//! Pure classification and derivation helpers for the generator.

/// Recognized source-file extensions, in priority order.
pub const SOURCE_FILE_EXTENSIONS: [&str; 7] =
    [".ts", ".tsx", ".js", ".jsx", ".css", ".html", ".rs"];

const CONFIG_SUFFIXES: [&str; 4] = [".config.ts", ".config.tsx", ".config.js", ".config.jsx"];

/// Decide whether a path (relative to the generation target directory) names
/// a source file that should be numbered for attribution.
pub fn is_source_file(rel_to_target: &str) -> bool {
    let base = match rel_to_target.rsplit_once('/') {
        Some((_, tail)) => tail,
        None => rel_to_target,
    };

    if CONFIG_SUFFIXES.iter().any(|suffix| base.ends_with(suffix)) {
        return false;
    }

    let Some(dot_idx) = base.rfind('.') else {
        return false;
    };

    let ext = &base[dot_idx..];
    SOURCE_FILE_EXTENSIONS.contains(&ext)
}

/// Derive a one-line description from a prompt body.
pub fn first_meaningful_line(body: &str) -> String {
    for line in body.split('\n') {
        let stripped = line.strip_suffix('\r').unwrap_or(line);
        let stripped = stripped.trim_start_matches('#');
        let trimmed = stripped.trim();
        if !trimmed.is_empty() {
            let capped: String = trimmed.chars().take(120).collect();
            return capped;
        }
    }
    "(no description)".to_string()
}

/// Split content into CRLF-aware lines for numbering.
pub fn split_body_lines(content: &str) -> Vec<String> {
    content
        .split('\n')
        .map(|piece| piece.strip_suffix('\r').unwrap_or(piece).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_file_extensions_order() {
        assert_eq!(
            SOURCE_FILE_EXTENSIONS,
            [".ts", ".tsx", ".js", ".jsx", ".css", ".html", ".rs"]
        );
    }

    #[test]
    fn is_source_file_recognizes_extensions() {
        assert!(is_source_file("src/lib.rs"));
        assert!(is_source_file("app.tsx"));
        assert!(is_source_file("page.html"));
        assert!(is_source_file("style.css"));
        assert!(is_source_file("index.js"));
        assert!(is_source_file("index.jsx"));
        assert!(is_source_file("index.ts"));
    }

    #[test]
    fn is_source_file_rejects_config_suffixes() {
        assert!(!is_source_file("vite.config.ts"));
        assert!(!is_source_file("vite.config.tsx"));
        assert!(!is_source_file("vite.config.js"));
        assert!(!is_source_file("vite.config.jsx"));
        assert!(!is_source_file("path/to/vite.config.ts"));
    }

    #[test]
    fn is_source_file_rejects_unrecognized_extension() {
        assert!(!is_source_file("README.md"));
    }

    #[test]
    fn is_source_file_rejects_no_extension() {
        assert!(!is_source_file("noext"));
    }

    #[test]
    fn is_source_file_uses_base_name_after_last_slash() {
        assert!(is_source_file("a/b/c/main.rs"));
        assert!(!is_source_file("a/b/c/noext"));
    }

    #[test]
    fn first_meaningful_line_strips_heading_markers() {
        assert_eq!(first_meaningful_line("# Title\n\nBody line"), "Title");
        assert_eq!(first_meaningful_line("### Deep\nmore"), "Deep");
    }

    #[test]
    fn first_meaningful_line_whitespace_only_body() {
        assert_eq!(first_meaningful_line("   \n\t\n  "), "(no description)");
    }

    #[test]
    fn first_meaningful_line_caps_at_120_chars() {
        let body = "x".repeat(200);
        let result = first_meaningful_line(&body);
        assert_eq!(result.chars().count(), 120);
        assert!(result.chars().all(|c| c == 'x'));
    }

    #[test]
    fn first_meaningful_line_empty_body() {
        assert_eq!(first_meaningful_line(""), "(no description)");
    }

    #[test]
    fn first_meaningful_line_strips_trailing_cr() {
        assert_eq!(first_meaningful_line("# Title\r\nmore"), "Title");
    }

    #[test]
    fn split_body_lines_handles_crlf_and_lf() {
        assert_eq!(
            split_body_lines("a\r\nb\nc"),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn split_body_lines_empty_string() {
        assert_eq!(split_body_lines(""), vec!["".to_string()]);
    }

    #[test]
    fn split_body_lines_trailing_newline_yields_final_empty_line() {
        assert_eq!(
            split_body_lines("x\r\n"),
            vec!["x".to_string(), "".to_string()]
        );
    }
}
