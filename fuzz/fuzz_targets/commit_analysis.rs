#![no_main]

use libfuzzer_sys::fuzz_target;

use ferrflow::changelog::{self, ChangelogRender, GitLog};
use ferrflow::config::{ChangelogConfig, CommitFormats};
use ferrflow::conventional_commits::determine_bump;

const VERSION: &str = "1.2.3";

#[derive(arbitrary::Arbitrary, Debug)]
struct Input<'a> {
    rich: bool,
    group_by_scope: bool,
    include_commit_links: bool,
    include_compare_link: bool,
    forge_base: Option<&'a str>,
    last_tag: Option<&'a str>,
    new_tag: Option<&'a str>,
    messages: Vec<&'a str>,
}

fuzz_target!(|input: Input| {
    let formats = CommitFormats::default();
    let commits: Vec<GitLog> = input
        .messages
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

    let config = ChangelogConfig {
        sections: None,
        group_by_scope: input.group_by_scope,
        include_commit_links: input.include_commit_links,
        include_compare_link: input.include_compare_link,
    };

    let render = ChangelogRender {
        config: input.rich.then_some(&config),
        formats: Some(&formats),
        forge_base: input.forge_base.map(str::to_string),
        last_tag: input.last_tag.map(str::to_string),
        new_tag: input.new_tag.map(str::to_string),
    };

    let section = changelog::build_section_with(VERSION, &commits, &render);

    let document = format!("# Changelog\n\n{section}\n\n## [0.9.0] - 2020-01-01\n\n- older\n");
    let read_back = changelog::section_for_version(&document, VERSION)
        .expect("the section that was just written is findable by its own heading");

    assert_eq!(
        read_back,
        section.trim(),
        "a commit message forged a heading, so section_for_version stops early and \
         the next release splices into the wrong place"
    );
});
