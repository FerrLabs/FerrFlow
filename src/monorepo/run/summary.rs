/// Tag tuple used by the release loop: (tag_name, message, body,
/// pkg_name, version, commit_count, is_prerelease).
pub(super) type TagToCreate = (String, String, String, String, String, i32, bool);

/// Print the per-package output lines collected during the run,
/// followed by the shared cross-package output lines, in the exact
/// formatting the orchestrator used to do inline.
///
/// `pkg_outputs` entries are `(pkg_name, lines)`. The name is unused at
/// print time today but kept for future grouping needs.
pub(super) fn print_outputs(pkg_outputs: &[(String, Vec<String>)], shared_outputs: &[String]) {
    for (i, (_, lines)) in pkg_outputs.iter().enumerate() {
        if i > 0 {
            println!();
        }
        for line in lines {
            println!("{line}");
        }
    }
    if !shared_outputs.is_empty() {
        println!();
        for line in shared_outputs {
            println!("{line}");
        }
    }
}

/// Write the per-tag release blurb to `$GITHUB_STEP_SUMMARY` when the
/// environment variable is set (GitHub Actions). Silent no-op otherwise.
///
/// Kept out of the main orchestrator so the entry function reads as a
/// pipeline of stages instead of stage + side-effect appendix.
pub(super) fn write_github_step_summary(tags: &[TagToCreate]) {
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
    for (tag_name, _, body, _, _, _, _) in tags {
        let _ = writeln!(file, "### {tag_name}\n");
        let _ = writeln!(file, "{body}");
    }
}
