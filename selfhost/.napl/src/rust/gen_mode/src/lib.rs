//! Full-vs-incremental mode selection: the pure decision core the
//! code-generation command uses to choose between a from-scratch (full)
//! regeneration and an incremental update, and to render the one-line
//! `mode:` status it prints for each module.

/// Reasons a full (non-incremental) generation was chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullModeReason {
    /// There was no prior prompt body or attribution on disk to diff against.
    NoPriorOnDisk,
    /// The caller forced a full generation with `--full`.
    ForcedFull,
    /// There was no prior successful gen for this target.
    NoPriorGen,
}

/// Decide whether an incremental update is possible.
pub fn can_incremental(
    full: bool,
    has_target_record: bool,
    unattributed_is_true: bool,
    has_prompt_hash_at_gen: bool,
) -> bool {
    !full && has_target_record && !unattributed_is_true && has_prompt_hash_at_gen
}

/// Render the status line for a full-mode generation.
pub fn full_mode_message(reason: FullModeReason) -> String {
    match reason {
        FullModeReason::NoPriorOnDisk => {
            "  mode: full (no prior prompt body or attribution on disk to diff against)"
                .to_string()
        }
        FullModeReason::ForcedFull => "  mode: full (forced --full)".to_string(),
        FullModeReason::NoPriorGen => {
            "  mode: full (no prior successful gen for this target)".to_string()
        }
    }
}

/// Render the status line for an incremental generation.
pub fn incremental_mode_message(changed_lines: usize, affected_regions: usize) -> String {
    format!(
        "  mode: INCREMENTAL — {changed_lines} changed prompt line(s), {affected_regions} owned region(s) affected"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_incremental_true_when_all_conditions_hold() {
        assert!(can_incremental(false, true, false, true));
    }

    #[test]
    fn can_incremental_false_when_forced_full() {
        assert!(!can_incremental(true, true, false, true));
    }

    #[test]
    fn can_incremental_false_when_no_target_record() {
        assert!(!can_incremental(false, false, false, true));
    }

    #[test]
    fn can_incremental_false_when_unattributed() {
        assert!(!can_incremental(false, true, true, true));
    }

    #[test]
    fn can_incremental_false_when_no_prompt_hash_at_gen() {
        assert!(!can_incremental(false, true, false, false));
    }

    #[test]
    fn can_incremental_false_when_all_conditions_fail() {
        assert!(!can_incremental(true, false, true, false));
    }

    #[test]
    fn full_mode_message_no_prior_on_disk() {
        assert_eq!(
            full_mode_message(FullModeReason::NoPriorOnDisk),
            "  mode: full (no prior prompt body or attribution on disk to diff against)"
        );
    }

    #[test]
    fn full_mode_message_forced_full() {
        assert_eq!(
            full_mode_message(FullModeReason::ForcedFull),
            "  mode: full (forced --full)"
        );
    }

    #[test]
    fn full_mode_message_no_prior_gen() {
        assert_eq!(
            full_mode_message(FullModeReason::NoPriorGen),
            "  mode: full (no prior successful gen for this target)"
        );
    }

    #[test]
    fn incremental_mode_message_renders_counts() {
        assert_eq!(
            incremental_mode_message(3, 2),
            "  mode: INCREMENTAL — 3 changed prompt line(s), 2 owned region(s) affected"
        );
    }

    #[test]
    fn incremental_mode_message_zero_counts() {
        assert_eq!(
            incremental_mode_message(0, 0),
            "  mode: INCREMENTAL — 0 changed prompt line(s), 0 owned region(s) affected"
        );
    }
}
