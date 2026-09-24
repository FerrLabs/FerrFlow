#![no_main]

use libfuzzer_sys::fuzz_target;

use ferrflow::changelog::{self, ChangelogRender, GitLog};
use ferrflow::config::CommitFormats;
use ferrflow::conventional_commits::determine_bump;

const VERSION: &str = "1.2.3";

fuzz_target!(|messages: Vec<&str>| {
    let formats = CommitFormats::default();
    let commits: Vec<GitLog> = messages
        .iter()
        .enumerate()
        .map(|(i, message)| {
            let _ = determine_bump(message, &formats);
            GitLog {
                id: String::new(),
                hash: format!("{i:07x}"),
                message: (*message).to_string(),
            }
        })
        .collect();

    let section = changelog::build_section_with(VERSION, &commits, &ChangelogRender::default());

    let headings = section.matches("\n## ").count() + usize::from(section.starts_with("## "));
    assert!(
        headings <= 1,
        "a commit message forged a second changelog heading, which truncates \
         section_for_version and misplaces splice_section: {section:?}"
    );
});
