//! The pure rendering core of the CLI's status command.
//!
//! Holds the status enum and the one-line rendering of a classified prompt.
//! Pure — no filesystem access, no hashing, no dependencies on other modules.

/// The classification of a prompt relative to its generated files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// Locked, attributed, and the prompt matches its last gen.
    Clean,
    /// Not generated, or the prompt changed since gen.
    PromptStale,
    /// A generated file was edited or deleted.
    Drift,
    /// Generated files exist but attribution failed.
    Unattributed,
}

impl FileStatus {
    /// The fixed label used when rendering this status.
    pub fn label(self) -> &'static str {
        match self {
            FileStatus::Clean => "clean",
            FileStatus::PromptStale => "prompt-stale",
            FileStatus::Drift => "DRIFT",
            FileStatus::Unattributed => "unattributed",
        }
    }

    /// Whether this status fails the CI gate.
    pub fn is_error(self) -> bool {
        matches!(self, FileStatus::Drift | FileStatus::Unattributed)
    }
}

/// A classified prompt ready to be printed as one status line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    /// The prompt path relative to the project root.
    pub file: String,
    pub status: FileStatus,
    /// A human-facing detail, possibly empty.
    pub detail: String,
}

impl StatusEntry {
    /// Render the entry exactly as the CLI prints one status line.
    pub fn line(&self) -> String {
        let suffix = if self.detail.is_empty() {
            String::new()
        } else {
            format!(" ({})", self.detail)
        };
        format!("{:<12} {}{}", self.status.label(), self.file, suffix)
    }

    /// Whether this status fails the CI gate.
    pub fn is_error(&self) -> bool {
        self.status.is_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_fixed() {
        assert_eq!(FileStatus::Clean.label(), "clean");
        assert_eq!(FileStatus::PromptStale.label(), "prompt-stale");
        assert_eq!(FileStatus::Drift.label(), "DRIFT");
        assert_eq!(FileStatus::Unattributed.label(), "unattributed");
    }

    fn entry(status: FileStatus, detail: &str) -> StatusEntry {
        StatusEntry {
            file: "examples/greeting.napl".to_string(),
            status,
            detail: detail.to_string(),
        }
    }

    #[test]
    fn line_clean_empty_detail() {
        assert_eq!(
            entry(FileStatus::Clean, "").line(),
            "clean        examples/greeting.napl"
        );
    }

    #[test]
    fn line_prompt_stale_with_detail_has_single_separator() {
        assert_eq!(
            entry(FileStatus::PromptStale, "never generated").line(),
            "prompt-stale examples/greeting.napl (never generated)"
        );
    }

    #[test]
    fn line_drift_with_detail() {
        assert_eq!(
            entry(FileStatus::Drift, "typescript: x was edited").line(),
            "DRIFT        examples/greeting.napl (typescript: x was edited)"
        );
    }

    #[test]
    fn line_unattributed_with_detail() {
        assert_eq!(
            entry(FileStatus::Unattributed, "run napl gen typescript --force").line(),
            "unattributed examples/greeting.napl (run napl gen typescript --force)"
        );
    }

    #[test]
    fn line_no_detail_has_no_trailing_space_or_parens() {
        let rendered = entry(FileStatus::Drift, "").line();
        assert_eq!(rendered, "DRIFT        examples/greeting.napl");
        assert!(!rendered.ends_with(' '));
        assert!(!rendered.contains('('));
    }

    #[test]
    fn line_exact_twelve_char_label_gets_single_separator() {
        assert_eq!(FileStatus::PromptStale.label().len(), 12);
        let rendered = entry(FileStatus::PromptStale, "").line();
        assert_eq!(rendered, "prompt-stale examples/greeting.napl");
    }

    #[test]
    fn line_shorter_labels_pad_to_width_twelve() {
        let prefix = "clean        ";
        assert_eq!(prefix.len(), 13);
        assert!(entry(FileStatus::Clean, "").line().starts_with(prefix));
    }

    #[test]
    fn is_error_for_all_variants() {
        assert!(!entry(FileStatus::Clean, "").is_error());
        assert!(!entry(FileStatus::PromptStale, "").is_error());
        assert!(entry(FileStatus::Drift, "").is_error());
        assert!(entry(FileStatus::Unattributed, "").is_error());

        assert!(!FileStatus::Clean.is_error());
        assert!(!FileStatus::PromptStale.is_error());
        assert!(FileStatus::Drift.is_error());
        assert!(FileStatus::Unattributed.is_error());
    }
}
