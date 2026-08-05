use colored::Colorize;

use super::{Decision, Explanation, Trigger};

pub(super) fn lines(x: &Explanation) -> Vec<String> {
    let mut out = vec![
        format!("{} {}", "Package:".dimmed(), x.package.bold()),
        format!("  {:<14} {}", "Path:", x.path),
    ];
    if !x.shared_paths.is_empty() {
        out.push(format!(
            "  {:<14} {}",
            "Shared paths:",
            x.shared_paths.join(", ")
        ));
    }
    out.push(format!("  {:<14} {}", "Strategy:", x.strategy));
    out.push(format!("  {:<14} {}", "Version:", x.current_version));
    if let Some(channel) = &x.channel {
        out.push(format!("  {:<14} {}", "Channel:", channel));
    }
    out.push(String::new());

    push_tag(&mut out, x);
    push_touch(&mut out, x);
    push_commits(&mut out, x);
    push_dependencies(&mut out, x);
    push_decision(&mut out, x);
    out
}

fn push_tag(out: &mut Vec<String>, x: &Explanation) {
    match &x.last_tag {
        Some(tag) => {
            let mut detail = Vec::new();
            if let Some(commit) = &tag.commit {
                detail.push(commit.clone());
            }
            if let Some(age) = &tag.age {
                detail.push(age.clone());
            }
            detail.push(if tag.reachable_from_head {
                "reachable from HEAD".to_string()
            } else {
                "NOT reachable from HEAD".to_string()
            });
            let line = format!("Last tag: {} ({})", tag.name.cyan(), detail.join(", "));
            out.push(if tag.reachable_from_head {
                line
            } else {
                line.yellow().to_string()
            });
        }
        None => out.push("Last tag: none — this package has never been released".to_string()),
    }
    out.push(String::new());
}

fn push_touch(out: &mut Vec<String>, x: &Explanation) {
    let scope = if x.touch.recovered {
        "changed files since the last tag"
    } else {
        "changed files at HEAD"
    };
    out.push(format!("Touch check ({scope}):"));
    if x.touch.files.is_empty() {
        out.push("  (none)".dimmed().to_string());
    }
    for file in &x.touch.files {
        match &file.matched {
            Some(rule) => out.push(format!(
                "  {} {:<46} {}",
                "✓".green(),
                file.path,
                format!("matches {rule}").dimmed()
            )),
            None => out.push(format!(
                "  {} {:<46} {}",
                "✗".dimmed(),
                file.path.dimmed(),
                "no match".dimmed()
            )),
        }
    }
    out.push(if x.touch.touched {
        let verdict = "→ touched".green().to_string();
        if x.touch.recovered {
            format!("{verdict} (recovered by recoverMissedReleases)")
        } else {
            verdict
        }
    } else {
        "→ not touched".yellow().to_string()
    });
    out.push(String::new());
}

fn push_commits(out: &mut Vec<String>, x: &Explanation) {
    if !x.touch.touched {
        return;
    }
    out.push(format!("Commits considered ({}):", x.commits.len()));
    if x.commits.is_empty() {
        out.push("  (none)".dimmed().to_string());
    }
    for commit in &x.commits {
        let label = if commit.bump == "none" {
            "—".dimmed().to_string()
        } else {
            commit.bump.cyan().to_string()
        };
        out.push(format!(
            "  {}  {:<52} {}",
            commit.hash.dimmed(),
            truncate(&commit.subject, 52),
            label
        ));
    }
    out.push(String::new());
}

fn push_dependencies(out: &mut Vec<String>, x: &Explanation) {
    if x.dependencies.is_empty() {
        return;
    }
    out.push("Dependencies:".to_string());
    for dep in &x.dependencies {
        let upstream = match &dep.upstream_bump {
            Some(bump) => format!("bumping ({bump})"),
            None => "not bumping".to_string(),
        };
        out.push(format!(
            "  {:<24} {:<20} {}",
            dep.name,
            upstream,
            format!("propagate: {} → {}", dep.propagate, dep.resulting_bump).dimmed()
        ));
    }
    out.push(String::new());
}

fn push_decision(out: &mut Vec<String>, x: &Explanation) {
    match &x.decision {
        Decision::Bump {
            bump,
            from,
            to,
            tag,
            prerelease,
            triggered_by,
        } => {
            let cause = match triggered_by {
                Trigger::Commits => "from its own commits",
                Trigger::Dependency => "from the dependency cascade",
                Trigger::Forced => "forced",
            };
            let pre = if *prerelease { " (prerelease)" } else { "" };
            out.push(format!(
                "{} {} bump {cause}{pre} — {} → {}, tag {}",
                "Decision:".bold(),
                bump.green().bold(),
                from.dimmed(),
                to.green().bold(),
                tag.cyan()
            ));
        }
        Decision::Skipped { reason } => out.push(format!(
            "{} no release — {}",
            "Decision:".bold(),
            reason.yellow()
        )),
    }
}

fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let kept: String = text.chars().take(width.saturating_sub(1)).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn truncate_keeps_short_subjects_intact() {
        assert_eq!(truncate("feat: short", 52), "feat: short");
    }

    // Commit subjects are arbitrary user text; slicing on bytes would panic on
    // a multi-byte character straddling the cut.
    #[test]
    fn truncate_cuts_on_characters_not_bytes() {
        let subject = "féat: ".repeat(20);
        let cut = truncate(&subject, 10);
        assert_eq!(cut.chars().count(), 10);
        assert!(cut.ends_with('…'));
    }
}
