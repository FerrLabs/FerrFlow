use crate::config::ReleaseCommitBody;

use super::summary::PlannedTag;

pub(super) fn build_commit_message(
    subject: &str,
    tags: &[PlannedTag],
    mode: ReleaseCommitBody,
) -> String {
    match render_body(tags, mode) {
        Some(body) => format!("{subject}\n\n{body}"),
        None => subject.to_string(),
    }
}

fn render_body(tags: &[PlannedTag], mode: ReleaseCommitBody) -> Option<String> {
    match mode {
        ReleaseCommitBody::None => None,
        ReleaseCommitBody::Summary => render_summary(tags),
        ReleaseCommitBody::Full => render_full(tags),
    }
}

fn render_summary(tags: &[PlannedTag]) -> Option<String> {
    let lines: Vec<String> = tags
        .iter()
        .map(|t| {
            let plural = if t.commit_count == 1 {
                "commit"
            } else {
                "commits"
            };
            format!(
                "- {} {} ({} {plural})",
                t.package, t.version, t.commit_count
            )
        })
        .collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn render_full(tags: &[PlannedTag]) -> Option<String> {
    let multi = tags.len() > 1;
    let sections: Vec<String> = tags
        .iter()
        .filter_map(|t| {
            let trimmed = t.body.trim();
            if trimmed.is_empty() {
                return None;
            }
            Some(if multi {
                format!("## {} {}\n\n{trimmed}", t.package, t.version)
            } else {
                trimmed.to_string()
            })
        })
        .collect();
    (!sections.is_empty()).then(|| sections.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(name: &str, version: &str, body: &str, commits: i32) -> PlannedTag {
        PlannedTag {
            tag: format!("{name}@v{version}"),
            message: format!("Release {name}@v{version}"),
            body: body.to_string(),
            package: name.to_string(),
            version: version.to_string(),
            commit_count: commits,
            is_prerelease: false,
        }
    }

    const SUBJECT: &str = "chore(release): api v1.2.0";

    #[test]
    fn none_is_byte_identical_to_the_bare_subject() {
        let tags = vec![tag("api", "1.2.0", "### Features\n\n- thing", 3)];
        assert_eq!(
            build_commit_message(SUBJECT, &tags, ReleaseCommitBody::None),
            SUBJECT
        );
    }

    #[test]
    fn full_appends_the_changelog_after_a_blank_line() {
        let tags = vec![tag("api", "1.2.0", "### Features\n\n- thing", 3)];
        let msg = build_commit_message(SUBJECT, &tags, ReleaseCommitBody::Full);
        assert_eq!(msg, format!("{SUBJECT}\n\n### Features\n\n- thing"));
    }

    #[test]
    fn full_single_package_omits_the_package_header() {
        let tags = vec![tag("api", "1.2.0", "- one", 1)];
        let msg = build_commit_message(SUBJECT, &tags, ReleaseCommitBody::Full);
        assert!(!msg.contains("## api"), "single package needs no header");
    }

    #[test]
    fn full_multi_package_labels_each_section() {
        let tags = vec![
            tag("api", "1.2.0", "- api change", 1),
            tag("web", "2.0.0", "- web change", 1),
        ];
        let msg = build_commit_message(SUBJECT, &tags, ReleaseCommitBody::Full);
        assert!(msg.contains("## api 1.2.0"));
        assert!(msg.contains("## web 2.0.0"));
        assert!(msg.find("## api").unwrap() < msg.find("## web").unwrap());
    }

    #[test]
    fn full_skips_packages_with_an_empty_changelog() {
        let tags = vec![
            tag("api", "1.2.0", "- real change", 1),
            tag("web", "2.0.0", "   \n  ", 1),
        ];
        let msg = build_commit_message(SUBJECT, &tags, ReleaseCommitBody::Full);
        assert!(msg.contains("api"));
        assert!(!msg.contains("## web"));
    }

    #[test]
    fn full_with_every_body_empty_falls_back_to_the_bare_subject() {
        let tags = vec![tag("api", "1.2.0", "", 1), tag("web", "2.0.0", "  ", 1)];
        assert_eq!(
            build_commit_message(SUBJECT, &tags, ReleaseCommitBody::Full),
            SUBJECT
        );
    }

    #[test]
    fn summary_is_one_bounded_line_per_package() {
        let tags: Vec<PlannedTag> = (0..50)
            .map(|i| {
                tag(
                    &format!("pkg{i}"),
                    "1.0.0",
                    "### Features\n\n- a\n- b\n- c",
                    9,
                )
            })
            .collect();
        let msg = build_commit_message(SUBJECT, &tags, ReleaseCommitBody::Summary);
        let body_lines = msg.lines().skip(2).count();
        assert_eq!(body_lines, 50, "summary must not scale with changelog size");
        assert!(msg.contains("- pkg0 1.0.0 (9 commits)"));
    }

    #[test]
    fn summary_singularises_a_single_commit() {
        let tags = vec![tag("api", "1.2.0", "- x", 1)];
        let msg = build_commit_message(SUBJECT, &tags, ReleaseCommitBody::Summary);
        assert!(msg.ends_with("- api 1.2.0 (1 commit)"), "got: {msg}");
    }

    #[test]
    fn no_tags_yields_the_bare_subject_in_every_mode() {
        for mode in [
            ReleaseCommitBody::None,
            ReleaseCommitBody::Summary,
            ReleaseCommitBody::Full,
        ] {
            assert_eq!(build_commit_message(SUBJECT, &[], mode), SUBJECT);
        }
    }

    #[test]
    fn skip_ci_marker_stays_on_the_subject_line() {
        let subject = "chore(release): api v1.2.0 [skip ci]";
        let tags = vec![tag("api", "1.2.0", "- thing", 1)];
        let msg = build_commit_message(subject, &tags, ReleaseCommitBody::Full);
        assert_eq!(
            msg.lines().next().unwrap(),
            subject,
            "CI providers only scan the subject line"
        );
    }
}
