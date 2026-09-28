use super::text_lines;
use crate::validate::{ValidationEntry, ValidationLevel, ValidationResult};

fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_escape = false;
    for ch in text.chars() {
        match (in_escape, ch) {
            (false, '\u{1b}') => in_escape = true,
            (true, 'm') => in_escape = false,
            (true, _) => {}
            (false, c) => out.push(c),
        }
    }
    out
}

fn entry(level: ValidationLevel, path: &str, message: &str) -> ValidationEntry {
    ValidationEntry {
        level,
        path: path.to_string(),
        message: message.to_string(),
    }
}

fn rendered(result: &ValidationResult) -> Vec<String> {
    text_lines(result).iter().map(|l| strip_ansi(l)).collect()
}

fn summary(out: &[String]) -> &str {
    out.iter()
        .rev()
        .find(|l| !l.is_empty())
        .map(String::as_str)
        .unwrap()
}

#[test]
fn a_clean_config_reports_its_file_packages_and_no_issues() {
    let mut result = ValidationResult::from_entries(Vec::new());
    result.config_file = Some("ferrflow.json".to_string());
    result.package_count = 3;
    let out = rendered(&result);

    assert!(
        out.contains(&"  ✓ config parsed (ferrflow.json)".to_string()),
        "{out:#?}"
    );
    assert!(
        out.contains(&"  ✓ 3 packages found".to_string()),
        "{out:#?}"
    );
    assert!(out.contains(&"  ✓ no issues found".to_string()), "{out:#?}");
    assert_eq!(summary(&out), "  all checks passed");
}

#[test]
fn a_single_package_is_not_pluralised_and_zero_packages_say_nothing() {
    let mut one = ValidationResult::from_entries(Vec::new());
    one.package_count = 1;
    assert!(rendered(&one).contains(&"  ✓ 1 package found".to_string()));

    let none = rendered(&ValidationResult::from_entries(Vec::new()));
    assert!(!none.iter().any(|l| l.contains("package")), "{none:#?}");
    assert!(
        !none.iter().any(|l| l.contains("config parsed")),
        "{none:#?}"
    );
}

#[test]
fn entries_are_grouped_errors_then_warnings_then_suggestions() {
    let result = ValidationResult::from_entries(vec![
        entry(
            ValidationLevel::Suggestion,
            "packages[0]",
            "add a changelog",
        ),
        entry(
            ValidationLevel::Warning,
            "packages[1].path",
            "empty directory",
        ),
        entry(ValidationLevel::Error, "packages[0].name", "duplicate name"),
    ]);
    let out = rendered(&result);
    let at = |needle: &str| {
        out.iter()
            .position(|l| l == needle)
            .unwrap_or_else(|| panic!("missing {needle:?} in {out:#?}"))
    };
    let error = at("  ✗ packages[0].name: duplicate name");
    let warning = at("  ⚠ packages[1].path: empty directory");
    let suggestion = at("  ◆ packages[0]: add a changelog");
    assert!(error < warning && warning < suggestion);
    assert!(!out.iter().any(|l| l.contains("no issues found")));
}

#[test]
fn the_summary_counts_each_level_and_pluralises_independently() {
    let result = ValidationResult::from_entries(vec![
        entry(ValidationLevel::Error, "a", "x"),
        entry(ValidationLevel::Error, "b", "y"),
        entry(ValidationLevel::Suggestion, "c", "z"),
    ]);
    assert_eq!(summary(&rendered(&result)), "  2 errors, 1 suggestion");

    let warned = ValidationResult::from_entries(vec![entry(ValidationLevel::Warning, "a", "x")]);
    assert_eq!(summary(&rendered(&warned)), "  1 warning");
}
