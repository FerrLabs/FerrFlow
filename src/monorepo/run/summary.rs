use colored::Colorize;
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct PlannedTag {
    pub tag: String,
    pub message: String,
    pub body: String,
    pub package: String,
    pub version: String,
    pub commit_count: i32,
    pub is_prerelease: bool,
}

pub(super) fn untouched_hint(skipped: usize, recover_missed_releases: bool) -> Option<String> {
    if skipped == 0 || recover_missed_releases {
        return None;
    }
    let packages = if skipped == 1 { "package" } else { "packages" };
    Some(format!(
        "{} {skipped} {packages} not touched by the last commit. \
         Set recoverMissedReleases to compare against the last tag instead.",
        "○".dimmed()
    ))
}

pub(super) fn collect_outputs(
    pkg_outputs: &[(String, Vec<String>)],
    shared_outputs: &[String],
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (i, (_, lines)) in pkg_outputs.iter().enumerate() {
        if i > 0 {
            out.push(String::new());
        }
        out.extend(lines.iter().cloned());
    }
    if !shared_outputs.is_empty() {
        out.push(String::new());
        out.extend(shared_outputs.iter().cloned());
    }
    out
}

pub(super) fn write_github_step_summary(tags: &[PlannedTag]) {
    let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") else {
        return;
    };
    use std::io::Write;
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&summary_path)
    else {
        return;
    };
    let _ = writeln!(file, "## Released\n");
    for t in tags {
        let _ = writeln!(file, "### {}\n", t.tag);
        let _ = writeln!(file, "{}", t.body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(hint: Option<String>) -> Option<String> {
        hint.map(|h| String::from_utf8(strip_ansi(h.as_bytes())).unwrap())
    }

    fn strip_ansi(bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(bytes.len());
        let mut in_escape = false;
        for &b in bytes {
            match (in_escape, b) {
                (false, 0x1b) => in_escape = true,
                (true, b'm') => in_escape = false,
                (true, _) => {}
                (false, _) => out.push(b),
            }
        }
        out
    }

    #[test]
    fn nothing_is_said_when_every_package_was_considered() {
        assert!(untouched_hint(0, false).is_none());
    }

    #[test]
    fn nothing_is_said_when_the_setting_already_widens_the_comparison() {
        assert!(
            untouched_hint(3, true).is_none(),
            "with recoverMissedReleases on, a skipped package really has nothing to release"
        );
    }

    #[test]
    fn the_hint_names_the_setting_that_explains_the_short_plan() {
        let hint = plain(untouched_hint(3, false)).expect("three skipped packages want a hint");
        assert!(hint.contains("3 packages"), "{hint}");
        assert!(
            hint.contains("recoverMissedReleases"),
            "the point of the line is naming the setting: {hint}"
        );
    }

    #[test]
    fn a_single_skipped_package_is_not_called_packages() {
        let hint = plain(untouched_hint(1, false)).unwrap();
        assert!(hint.contains("1 package not touched"), "{hint}");
    }
}
