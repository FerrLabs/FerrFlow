#![no_main]

use std::collections::BTreeMap;

use arbitrary::Arbitrary;
use ferrflow_api::engine::{CommitInput, compute_preview, parse_config};
use libfuzzer_sys::fuzz_target;

const COMMIT_TYPES: [&str; 8] = [
    "feat", "fix", "perf", "refactor", "docs", "chore", "build", "ci",
];

#[derive(Debug, Arbitrary)]
struct Commit {
    kind: u8,
    scope: Option<String>,
    breaking: bool,
    description: String,
    raw_message: Option<String>,
    hash: Option<String>,
    files: Vec<String>,
}

impl Commit {
    fn into_input(self) -> CommitInput {
        let message = self.raw_message.unwrap_or_else(|| {
            let kind = COMMIT_TYPES[usize::from(self.kind) % COMMIT_TYPES.len()];
            let scope = self.scope.map_or_else(String::new, |s| format!("({s})"));
            let bang = if self.breaking { "!" } else { "" };
            format!("{kind}{scope}{bang}: {}", self.description)
        });
        CommitInput {
            message,
            hash: self.hash,
            files: self.files,
        }
    }
}

#[derive(Debug, Arbitrary)]
struct Input {
    raw_config: Option<(String, Option<String>)>,
    packages: Vec<(String, String)>,
    commits: Vec<Commit>,
    current_versions: Vec<(String, String)>,
}

fuzz_target!(|input: Input| {
    let parsed = match &input.raw_config {
        Some((text, format)) => parse_config(text, format.as_deref()),
        None => {
            let packages: Vec<_> = input
                .packages
                .iter()
                .map(|(name, path)| serde_json::json!({ "name": name, "path": path }))
                .collect();
            let text = serde_json::json!({ "package": packages }).to_string();
            parse_config(&text, Some("json"))
        }
    };
    let Ok(config) = parsed else {
        return;
    };

    let commits: Vec<CommitInput> = input.commits.into_iter().map(Commit::into_input).collect();
    let versions: BTreeMap<String, String> = input.current_versions.into_iter().collect();

    let previews = compute_preview(&config, &commits, &versions);

    assert!(previews.len() <= config.packages.len());
    for preview in &previews {
        assert_ne!(preview.bump, "none");
        assert!(!preview.commits.is_empty());
        assert!(preview.commits.len() <= commits.len());
    }
});
