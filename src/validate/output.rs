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
    tracing::info!("");
    tracing::info!("{}", "ferrflow validate".bold());
    tracing::info!("");

    if let Some(ref cf) = result.config_file {
        tracing::info!("  {} config parsed ({})", "✓".green(), cf);
    }
    if result.package_count > 0 {
        tracing::info!(
            "  {} {} package{} found",
            "✓".green(),
            result.package_count,
            if result.package_count == 1 { "" } else { "s" }
        );
    }

    for e in &result.errors {
        tracing::info!("  {} {}: {}", "✗".red(), e.path, e.message);
    }
    for w in &result.warnings {
        tracing::info!("  {} {}: {}", "⚠".yellow(), w.path, w.message);
    }
    for s in &result.suggestions {
        tracing::info!("  {} {}: {}", "◆".cyan(), s.path, s.message);
    }

    if result.errors.is_empty() && result.warnings.is_empty() && result.suggestions.is_empty() {
        tracing::info!("  {} no issues found", "✓".green());
    }

    tracing::info!("");
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
        tracing::info!("  {}", "all checks passed".green().bold());
    } else {
        tracing::info!("  {}", parts.join(", "));
    }
    tracing::info!("");
}
