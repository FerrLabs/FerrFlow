use anyhow::Result;
use colored::Colorize;

use super::result::ValidationResult;

pub(super) fn output_result(result: &ValidationResult, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
    } else {
        print_text_result(result);
    }
    if !result.valid {
        std::process::exit(1);
    }
    Ok(())
}

fn print_text_result(result: &ValidationResult) {
    println!();
    println!("{}", "ferrflow validate".bold());
    println!();

    if let Some(ref cf) = result.config_file {
        println!("  {} config parsed ({})", "✓".green(), cf);
    }
    if result.package_count > 0 {
        println!(
            "  {} {} package{} found",
            "✓".green(),
            result.package_count,
            if result.package_count == 1 { "" } else { "s" }
        );
    }

    for e in &result.errors {
        println!("  {} {}: {}", "✗".red(), e.path, e.message);
    }
    for w in &result.warnings {
        println!("  {} {}: {}", "⚠".yellow(), w.path, w.message);
    }
    for s in &result.suggestions {
        println!("  {} {}: {}", "◆".cyan(), s.path, s.message);
    }

    if result.errors.is_empty() && result.warnings.is_empty() && result.suggestions.is_empty() {
        println!("  {} no issues found", "✓".green());
    }

    println!();
    let parts: Vec<String> = [
        (result.errors.len(), "error"),
        (result.warnings.len(), "warning"),
        (result.suggestions.len(), "suggestion"),
    ]
    .iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, label)| format!("{n} {label}{}", if *n > 1 { "s" } else { "" }))
    .collect();

    if parts.is_empty() {
        println!("  {}", "all checks passed".green().bold());
    } else {
        println!("  {}", parts.join(", "));
    }
    println!();
}
