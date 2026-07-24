use std::collections::BTreeSet;

use healing_core::{lcs_len, move_verdict, MoveCandidate, MoveVerdict};

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

fn candidate(path: &str, hash: &str, content: &str) -> MoveCandidate {
    MoveCandidate {
        path: path.to_string(),
        hash: hash.to_string(),
        content: content.to_string(),
    }
}

fn claimed(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn lcs_len_matches_the_head_implementation() {
    assert_eq!(lcs_len(&lines(&["x", "y", "z"]), &lines(&["x", "y", "z"])), 3);
    assert_eq!(lcs_len(&lines(&["a", "b", "c"]), &lines(&["a", "c"])), 2);
    assert_eq!(lcs_len(&lines(&["a", "b", "c", "d"]), &lines(&["b", "d"])), 2);
    assert_eq!(lcs_len(&lines(&[]), &lines(&["a"])), 0);
    assert_eq!(lcs_len(&lines(&["a"]), &lines(&[])), 0);
    assert_eq!(lcs_len(&lines(&["a", "b"]), &lines(&["c", "d"])), 0);
}

#[test]
fn clean_move_on_a_single_exact_hash_match() {
    let candidates = vec![candidate("src/moved/greet.ts", "H", "irrelevant body")];
    let verdict = move_verdict(
        "src/greet.ts",
        "typescript",
        "H",
        "",
        &candidates,
        &claimed(&[]),
    );
    match verdict {
        MoveVerdict::Clean(new_path) => assert_eq!(new_path, "src/moved/greet.ts"),
        _ => panic!("expected clean"),
    }
}

#[test]
fn ambiguous_when_two_untracked_files_share_the_hash() {
    let candidates = vec![
        candidate("src/one/greet.ts", "H", "a"),
        candidate("src/two/greet.ts", "H", "a"),
    ];
    let verdict = move_verdict(
        "src/greet.ts",
        "typescript",
        "H",
        "",
        &candidates,
        &claimed(&[]),
    );
    match verdict {
        MoveVerdict::Ambiguous(message) => assert_eq!(
            message,
            "cannot heal moved file 'src/greet.ts' (typescript): 2 untracked files have identical content (src/one/greet.ts, src/two/greet.ts). Rename or remove the duplicate so the move is unambiguous."
        ),
        _ => panic!("expected ambiguous"),
    }
}

#[test]
fn drifted_move_on_a_single_line_similar_match() {
    let candidates = vec![candidate(
        "src/moved/greet.ts",
        "DIFFERENT",
        "alpha\nbeta\ngamma\nDELTA-EDITED\n",
    )];
    let verdict = move_verdict(
        "src/greet.ts",
        "typescript",
        "H",
        "alpha\nbeta\ngamma\ndelta\n",
        &candidates,
        &claimed(&[]),
    );
    match verdict {
        MoveVerdict::Drifted(new_path) => assert_eq!(new_path, "src/moved/greet.ts"),
        _ => panic!("expected drifted"),
    }
}

#[test]
fn no_match_when_neither_hash_nor_similarity_holds() {
    let candidates = vec![candidate("src/moved/greet.ts", "DIFFERENT", "zeta\n")];
    let verdict = move_verdict(
        "src/greet.ts",
        "typescript",
        "H",
        "alpha\nbeta\ngamma\ndelta\n",
        &candidates,
        &claimed(&[]),
    );
    assert!(matches!(verdict, MoveVerdict::NoMatch));
}

#[test]
fn ambiguous_when_two_untracked_files_are_similar_candidates() {
    let candidates = vec![
        candidate("src/one/greet.ts", "A", "alpha\nbeta\ngamma\nDELTA-EDITED\n"),
        candidate("src/two/greet.ts", "B", "alpha\nbeta\ngamma\nDELTA-CHANGED\n"),
    ];
    let verdict = move_verdict(
        "src/greet.ts",
        "typescript",
        "H",
        "alpha\nbeta\ngamma\ndelta\n",
        &candidates,
        &claimed(&[]),
    );
    match verdict {
        MoveVerdict::Ambiguous(message) => assert_eq!(
            message,
            "cannot heal moved file 'src/greet.ts' (typescript): 2 untracked files are similar candidates (src/one/greet.ts, src/two/greet.ts). Rename or remove the duplicate so the move is unambiguous."
        ),
        _ => panic!("expected ambiguous"),
    }
}

#[test]
fn a_claimed_exact_candidate_is_skipped() {
    let candidates = vec![candidate("src/moved/greet.ts", "H", "body")];
    let verdict = move_verdict(
        "src/greet.ts",
        "typescript",
        "H",
        "",
        &candidates,
        &claimed(&["src/moved/greet.ts"]),
    );
    assert!(matches!(verdict, MoveVerdict::NoMatch));
}

#[test]
fn an_exact_hash_match_wins_over_similarity() {
    let candidates = vec![
        candidate("src/similar/greet.ts", "OTHER", "alpha\nbeta\ngamma\ndelta\n"),
        candidate("src/exact/greet.ts", "H", "totally different one line\n"),
    ];
    let verdict = move_verdict(
        "src/greet.ts",
        "typescript",
        "H",
        "alpha\nbeta\ngamma\ndelta\n",
        &candidates,
        &claimed(&[]),
    );
    match verdict {
        MoveVerdict::Clean(new_path) => assert_eq!(new_path, "src/exact/greet.ts"),
        _ => panic!("expected clean"),
    }
}
