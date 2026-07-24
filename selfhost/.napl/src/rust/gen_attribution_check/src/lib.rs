use schemas_attribution::Attribution;

pub fn assert_attribution_sane(attribution: &Attribution, allowed: &[String]) -> Result<(), String> {
    if !allowed.is_empty() && attribution.entries.is_empty() {
        return Err("attribution has no entries but the module has attributed source files".to_string());
    }

    for entry in &attribution.entries {
        if !allowed.iter().any(|file| file == &entry.file) {
            return Err(format!(
                "attribution entry references file \"{}\" which is outside the attributed file set",
                entry.file
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemas_attribution::AttributionEntry;
    use schemas_line_range::LineRange;

    fn entry(file: &str) -> AttributionEntry {
        AttributionEntry {
            prompt_lines: LineRange::new(1, 1),
            file: file.to_string(),
            lines: LineRange::new(1, 1),
            note: String::new(),
        }
    }

    fn attribution(entries: Vec<AttributionEntry>) -> Attribution {
        Attribution {
            module: "m".to_string(),
            target: "t".to_string(),
            entries,
        }
    }

    #[test]
    fn empty_allowed_and_empty_entries_is_ok() {
        let attr = attribution(vec![]);
        assert_eq!(assert_attribution_sane(&attr, &[]), Ok(()));
    }

    #[test]
    fn nonempty_allowed_and_empty_entries_errors() {
        let attr = attribution(vec![]);
        let allowed = vec!["a.ts".to_string()];
        assert_eq!(
            assert_attribution_sane(&attr, &allowed),
            Err("attribution has no entries but the module has attributed source files".to_string())
        );
    }

    #[test]
    fn entry_within_allowed_is_ok() {
        let attr = attribution(vec![entry("a.ts")]);
        let allowed = vec!["a.ts".to_string()];
        assert_eq!(assert_attribution_sane(&attr, &allowed), Ok(()));
    }

    #[test]
    fn entry_outside_allowed_errors_with_exact_message() {
        let attr = attribution(vec![entry("b.ts")]);
        let allowed = vec!["a.ts".to_string()];
        assert_eq!(
            assert_attribution_sane(&attr, &allowed),
            Err("attribution entry references file \"b.ts\" which is outside the attributed file set".to_string())
        );
    }

    #[test]
    fn returns_first_violation_in_order() {
        let attr = attribution(vec![entry("a.ts"), entry("b.ts"), entry("c.ts")]);
        let allowed = vec!["a.ts".to_string(), "c.ts".to_string()];
        assert_eq!(
            assert_attribution_sane(&attr, &allowed),
            Err("attribution entry references file \"b.ts\" which is outside the attributed file set".to_string())
        );
    }

    #[test]
    fn all_entries_within_allowed_multiple_is_ok() {
        let attr = attribution(vec![entry("a.ts"), entry("c.ts")]);
        let allowed = vec!["a.ts".to_string(), "b.ts".to_string(), "c.ts".to_string()];
        assert_eq!(assert_attribution_sane(&attr, &allowed), Ok(()));
    }
}
