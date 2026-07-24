//! The move-match verdict: healing decisions without any I/O.
//!
//! When the module map records a generated file at a path that no longer exists
//! on disk, a hand `mv` or an editor rename most likely relocated it. Given one
//! lost file and the pool of untracked candidate files in its target, this
//! module decides whether the move is a clean relocation, a moved-and-drifted
//! file, no move at all, or an ambiguous case that must be refused.

use std::collections::BTreeSet;

/// The length of the longest common subsequence of two slices of lines.
///
/// Computed with a rolling two-row table so memory is proportional to the
/// shorter dimension.
pub fn lcs_len(a: &[String], b: &[String]) -> usize {
    if a.is_empty() || b.is_empty() {
        return 0;
    }

    // Iterate over the longer slice so the rows stay proportional to the
    // shorter dimension. The LCS length is symmetric, so the swap is free.
    let (outer, inner) = if a.len() >= b.len() { (a, b) } else { (b, a) };

    let mut prev = vec![0usize; inner.len() + 1];
    let mut curr = vec![0usize; inner.len() + 1];

    for outer_line in outer {
        for (j, inner_line) in inner.iter().enumerate() {
            curr[j + 1] = if outer_line == inner_line {
                prev[j] + 1
            } else {
                prev[j + 1].max(curr[j])
            };
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[inner.len()]
}

/// Whether two file contents are line-similar: their longest common
/// subsequence covers at least half of the longer file.
fn is_similar(before: &str, current: &str) -> bool {
    let before_lines = text_diff::to_lines(before);
    let current_lines = text_diff::to_lines(current);
    let denom = before_lines.len().max(current_lines.len());
    if denom == 0 {
        return true;
    }
    lcs_len(&before_lines, &current_lines) * 2 >= denom
}

/// One untracked file that might be where the lost file moved to.
pub struct MoveCandidate {
    pub path: String,
    pub hash: String,
    pub content: String,
}

/// The fate of a lost file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveVerdict {
    /// An exact-content relocation: the shell relocks the file at this path.
    Clean(String),
    /// A moved-and-drifted file: the shell heals the path, drift machinery
    /// reports the change.
    Drifted(String),
    /// No candidate is the move.
    NoMatch,
    /// The move cannot be decided without guessing; carries the exact error
    /// text the shell surfaces.
    Ambiguous(String),
}

/// Decide the fate of the lost file `old_path` against the candidate pool,
/// ignoring any candidate already claimed by an earlier heal.
pub fn move_verdict(
    old_path: &str,
    target: &str,
    old_hash: &str,
    baseline: &str,
    candidates: &[MoveCandidate],
    claimed: &BTreeSet<String>,
) -> MoveVerdict {
    let available = || {
        candidates
            .iter()
            .filter(|candidate| !claimed.contains(&candidate.path))
    };

    let exact: Vec<&MoveCandidate> = available()
        .filter(|candidate| candidate.hash == old_hash)
        .collect();

    match exact.len() {
        1 => return MoveVerdict::Clean(exact[0].path.clone()),
        0 => {}
        _ => {
            return MoveVerdict::Ambiguous(ambiguous_message(
                old_path,
                target,
                "have identical content",
                &exact,
            ))
        }
    }

    let similar: Vec<&MoveCandidate> = available()
        .filter(|candidate| is_similar(baseline, &candidate.content))
        .collect();

    match similar.len() {
        1 => MoveVerdict::Drifted(similar[0].path.clone()),
        0 => MoveVerdict::NoMatch,
        _ => MoveVerdict::Ambiguous(ambiguous_message(
            old_path,
            target,
            "are similar candidates",
            &similar,
        )),
    }
}

fn ambiguous_message(
    old_path: &str,
    target: &str,
    phrase: &str,
    matches: &[&MoveCandidate],
) -> String {
    let list = matches
        .iter()
        .map(|candidate| candidate.path.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "cannot heal moved file '{old_path}' ({target}): {count} untracked files {phrase} ({list}). Rename or remove the duplicate so the move is unambiguous.",
        count = matches.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn candidate(path: &str, hash: &str, content: &str) -> MoveCandidate {
        MoveCandidate {
            path: path.to_string(),
            hash: hash.to_string(),
            content: content.to_string(),
        }
    }

    fn claimed(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn lcs_of_identical_slices_is_the_length() {
        let a = lines(&["a", "b", "c"]);
        assert_eq!(lcs_len(&a, &a), 3);
    }

    #[test]
    fn lcs_skips_missing_lines() {
        assert_eq!(lcs_len(&lines(&["a", "b", "c"]), &lines(&["a", "c"])), 2);
        assert_eq!(lcs_len(&lines(&["a", "c"]), &lines(&["a", "b", "c"])), 2);
    }

    #[test]
    fn lcs_with_an_empty_slice_is_zero() {
        assert_eq!(lcs_len(&lines(&["a"]), &[]), 0);
        assert_eq!(lcs_len(&[], &lines(&["a"])), 0);
        assert_eq!(lcs_len(&[], &[]), 0);
    }

    #[test]
    fn lcs_with_nothing_shared_is_zero() {
        assert_eq!(lcs_len(&lines(&["a", "b"]), &lines(&["x", "y", "z"])), 0);
    }

    #[test]
    fn lcs_respects_subsequence_order() {
        assert_eq!(lcs_len(&lines(&["a", "b", "c"]), &lines(&["c", "b", "a"])), 1);
    }

    #[test]
    fn empty_contents_are_similar() {
        assert!(is_similar("", ""));
    }

    #[test]
    fn similarity_is_at_least_half_of_the_longer_file() {
        // 4 lines vs 2 shared out of 4 -> exactly half, similar.
        assert!(is_similar("a\nb\nc\nd\n", "a\nb\n"));
        // 5 lines, 2 shared -> below half.
        assert!(!is_similar("a\nb\nc\nd\ne\n", "a\nb\n"));
    }

    #[test]
    fn similarity_ignores_carriage_returns_like_to_lines() {
        assert!(is_similar("a\r\nb\r\n", "a\nb\n"));
    }

    #[test]
    fn a_single_exact_hash_match_is_clean() {
        let candidates = vec![
            candidate("src/other.rs", "hash-other", "other"),
            candidate("src/moved.rs", "hash-old", "same"),
        ];
        assert_eq!(
            move_verdict(
                "src/old.rs",
                "rust",
                "hash-old",
                "same",
                &candidates,
                &BTreeSet::new()
            ),
            MoveVerdict::Clean("src/moved.rs".to_string())
        );
    }

    #[test]
    fn exact_match_wins_over_similarity() {
        let candidates = vec![
            candidate("src/similar.rs", "hash-similar", "a\nb\nc\n"),
            candidate("src/exact.rs", "hash-old", "a\nb\nc\n"),
        ];
        assert_eq!(
            move_verdict(
                "src/old.rs",
                "rust",
                "hash-old",
                "a\nb\nc\n",
                &candidates,
                &BTreeSet::new()
            ),
            MoveVerdict::Clean("src/exact.rs".to_string())
        );
    }

    #[test]
    fn duplicate_exact_matches_are_ambiguous() {
        let candidates = vec![
            candidate("src/b.rs", "hash-old", "same"),
            candidate("src/a.rs", "hash-old", "same"),
        ];
        assert_eq!(
            move_verdict(
                "src/old.rs",
                "rust",
                "hash-old",
                "same",
                &candidates,
                &BTreeSet::new()
            ),
            MoveVerdict::Ambiguous(
                "cannot heal moved file 'src/old.rs' (rust): 2 untracked files have identical content (src/b.rs, src/a.rs). Rename or remove the duplicate so the move is unambiguous."
                    .to_string()
            )
        );
    }

    #[test]
    fn a_single_similar_candidate_is_drifted() {
        let candidates = vec![
            candidate("src/far.rs", "hash-far", "x\ny\nz\nw\n"),
            candidate("src/near.rs", "hash-near", "a\nb\nc\nd2\n"),
        ];
        assert_eq!(
            move_verdict(
                "src/old.rs",
                "rust",
                "hash-old",
                "a\nb\nc\nd\n",
                &candidates,
                &BTreeSet::new()
            ),
            MoveVerdict::Drifted("src/near.rs".to_string())
        );
    }

    #[test]
    fn duplicate_similar_candidates_are_ambiguous() {
        let candidates = vec![
            candidate("src/one.rs", "h1", "a\nb\nc\nd1\n"),
            candidate("src/two.rs", "h2", "a\nb\nc\nd2\n"),
        ];
        assert_eq!(
            move_verdict(
                "src/old.rs",
                "rust",
                "hash-old",
                "a\nb\nc\nd\n",
                &candidates,
                &BTreeSet::new()
            ),
            MoveVerdict::Ambiguous(
                "cannot heal moved file 'src/old.rs' (rust): 2 untracked files are similar candidates (src/one.rs, src/two.rs). Rename or remove the duplicate so the move is unambiguous."
                    .to_string()
            )
        );
    }

    #[test]
    fn no_candidate_matches() {
        let candidates = vec![candidate("src/far.rs", "hash-far", "x\ny\nz\nw\n")];
        assert_eq!(
            move_verdict(
                "src/old.rs",
                "rust",
                "hash-old",
                "a\nb\nc\nd\n",
                &candidates,
                &BTreeSet::new()
            ),
            MoveVerdict::NoMatch
        );
    }

    #[test]
    fn an_empty_pool_is_no_match() {
        assert_eq!(
            move_verdict("src/old.rs", "rust", "h", "a\n", &[], &BTreeSet::new()),
            MoveVerdict::NoMatch
        );
    }

    #[test]
    fn claimed_candidates_are_skipped_in_the_exact_pass() {
        let candidates = vec![
            candidate("src/taken.rs", "hash-old", "same"),
            candidate("src/free.rs", "hash-old", "same"),
        ];
        assert_eq!(
            move_verdict(
                "src/old.rs",
                "rust",
                "hash-old",
                "same",
                &candidates,
                &claimed(&["src/taken.rs"])
            ),
            MoveVerdict::Clean("src/free.rs".to_string())
        );
    }

    #[test]
    fn claimed_candidates_are_skipped_in_the_similar_pass() {
        let candidates = vec![
            candidate("src/taken.rs", "h1", "a\nb\nc\nd1\n"),
            candidate("src/free.rs", "h2", "a\nb\nc\nd2\n"),
        ];
        assert_eq!(
            move_verdict(
                "src/old.rs",
                "rust",
                "hash-old",
                "a\nb\nc\nd\n",
                &candidates,
                &claimed(&["src/taken.rs"])
            ),
            MoveVerdict::Drifted("src/free.rs".to_string())
        );
    }

    #[test]
    fn the_ambiguous_list_preserves_candidate_order() {
        let candidates = vec![
            candidate("z.rs", "hash-old", "same"),
            candidate("m.rs", "hash-old", "same"),
            candidate("a.rs", "hash-old", "same"),
        ];
        let MoveVerdict::Ambiguous(message) = move_verdict(
            "src/old.rs",
            "rust",
            "hash-old",
            "same",
            &candidates,
            &BTreeSet::new(),
        ) else {
            panic!("expected an ambiguous verdict");
        };
        assert!(message.contains("3 untracked files have identical content (z.rs, m.rs, a.rs)"));
    }

    #[test]
    fn an_empty_baseline_is_similar_only_to_empty_content() {
        let candidates = vec![candidate("src/empty.rs", "h", "")];
        assert_eq!(
            move_verdict(
                "src/old.rs",
                "rust",
                "hash-old",
                "",
                &candidates,
                &BTreeSet::new()
            ),
            MoveVerdict::Drifted("src/empty.rs".to_string())
        );
    }
}
