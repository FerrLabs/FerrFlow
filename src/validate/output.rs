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
    for line in text_lines(result) {
        tracing::info!("{line}");
    }
}

fn text_lines(result: &ValidationResult) -> Vec<String> {
    let mut out = Vec::new();
    out.push(String::new());
    out.push("ferrflow validate".bold().to_string());
    out.push(String::new());

    if let Some(ref cf) = result.config_file {
        out.push(format!("  {} config parsed ({})", "✓".green(), cf));
    }
    if result.package_count > 0 {
        out.push(format!(
            "  {} {} package{} found",
            "✓".green(),
            result.package_count,
            if result.package_count == 1 { "" } else { "s" }
        ));
    }

    for e in &result.errors {
        out.push(format!("  {} {}: {}", "✗".red(), e.path, e.message));
    }
    for w in &result.warnings {
        out.push(format!("  {} {}: {}", "⚠".yellow(), w.path, w.message));
    }
    for s in &result.suggestions {
        out.push(format!("  {} {}: {}", "◆".cyan(), s.path, s.message));
    }

    if result.errors.is_empty() && result.warnings.is_empty() && result.suggestions.is_empty() {
        out.push(format!("  {} no issues found", "✓".green()));
    }

    out.push(String::new());
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
        out.push(format!("  {}", "all checks passed".green().bold()));
    } else {
        out.push(format!("  {}", parts.join(", ")));
    }
    out.push(String::new());
    out
}

#[cfg(test)]
mod tests;
