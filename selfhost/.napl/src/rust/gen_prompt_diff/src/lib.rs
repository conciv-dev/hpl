use incremental::diff_body_lines;

pub fn compute_prompt_diff(prior_body: Option<&str>, body: &str) -> String {
    match prior_body {
        Some(prior) if prior != body => diff_body_lines(prior, body).unified,
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_prior_body_returns_empty() {
        assert_eq!(compute_prompt_diff(None, "body"), "");
    }

    #[test]
    fn unchanged_body_returns_empty() {
        assert_eq!(compute_prompt_diff(Some("body"), "body"), "");
    }

    #[test]
    fn changed_body_returns_unified_diff() {
        let result = compute_prompt_diff(Some("old line"), "new line");
        let expected = diff_body_lines("old line", "new line").unified;
        assert_eq!(result, expected);
        assert!(!result.is_empty());
    }
}
