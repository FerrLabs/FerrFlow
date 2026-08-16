#[derive(Debug, Clone)]
pub(super) struct PlannedTag {
    pub tag: String,
    pub message: String,
    pub body: String,
    pub package: String,
    pub version: String,
    pub commit_count: i32,
    pub is_prerelease: bool,
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
