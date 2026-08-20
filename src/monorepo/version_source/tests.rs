use super::VersionSource;
use crate::config::VersionSourcePolicy;

const ALL: [VersionSourcePolicy; 3] = [
    VersionSourcePolicy::Highest,
    VersionSourcePolicy::Tag,
    VersionSourcePolicy::File,
];

fn tag() -> Option<(String, String)> {
    Some(("discord-v2026.8.1".to_string(), "2026.8.1".to_string()))
}

fn file() -> Option<(String, String)> {
    Some(("discord/Cargo.toml".to_string(), "2026.8.18".to_string()))
}

#[test]
fn highest_names_the_file_when_the_file_won_the_comparison() {
    let (version, source) = VersionSource::resolve(tag(), file(), VersionSourcePolicy::Highest);
    assert_eq!(version.as_deref(), Some("2026.8.18"));
    assert_eq!(
        source,
        VersionSource::FileOverTag {
            file: "discord/Cargo.toml".to_string(),
            tag: "discord-v2026.8.1".to_string(),
        }
    );
}

#[test]
fn highest_names_the_tag_when_the_tag_won_the_comparison() {
    let higher_tag = Some(("discord-v2027.1.0".to_string(), "2027.1.0".to_string()));
    let (version, source) =
        VersionSource::resolve(higher_tag, file(), VersionSourcePolicy::Highest);
    assert_eq!(version.as_deref(), Some("2027.1.0"));
    assert_eq!(
        source,
        VersionSource::TagOverFile {
            tag: "discord-v2027.1.0".to_string(),
            file: "discord/Cargo.toml".to_string(),
        }
    );
}

#[test]
fn highest_credits_the_tag_when_both_carry_the_same_version() {
    let same = Some(("discord/Cargo.toml".to_string(), "2026.8.1".to_string()));
    let (version, source) = VersionSource::resolve(tag(), same, VersionSourcePolicy::Highest);
    assert_eq!(version.as_deref(), Some("2026.8.1"));
    assert!(matches!(source, VersionSource::TagOverFile { .. }));
}

#[test]
fn tag_policy_ignores_a_file_that_ratcheted_above_the_tags() {
    let (version, source) = VersionSource::resolve(tag(), file(), VersionSourcePolicy::Tag);
    assert_eq!(
        version.as_deref(),
        Some("2026.8.1"),
        "the inflated file version must not win"
    );
    assert_eq!(
        source,
        VersionSource::TagByPolicy {
            tag: "discord-v2026.8.1".to_string(),
            file: "discord/Cargo.toml".to_string(),
        }
    );
}

#[test]
fn file_policy_ignores_a_tag_that_sits_above_the_file() {
    let higher_tag = Some(("discord-v2027.1.0".to_string(), "2027.1.0".to_string()));
    let (version, source) = VersionSource::resolve(higher_tag, file(), VersionSourcePolicy::File);
    assert_eq!(version.as_deref(), Some("2026.8.18"));
    assert_eq!(
        source,
        VersionSource::FileByPolicy {
            file: "discord/Cargo.toml".to_string(),
            tag: "discord-v2027.1.0".to_string(),
        }
    );
}

#[test]
fn a_policy_choice_is_distinguishable_from_a_comparison() {
    let (_, by_policy) = VersionSource::resolve(tag(), file(), VersionSourcePolicy::Tag);
    let higher_tag = Some(("discord-v2027.1.0".to_string(), "2027.1.0".to_string()));
    let (_, by_height) = VersionSource::resolve(higher_tag, file(), VersionSourcePolicy::Highest);
    assert_ne!(
        by_policy, by_height,
        "chosen because configured must not read as chosen because higher"
    );
    assert!(by_policy.to_string().contains("ignored by versionSource"));
    assert!(by_height.to_string().contains("over"));
}

#[test]
fn a_single_source_is_unaffected_by_the_policy() {
    for policy in ALL {
        let (version, source) = VersionSource::resolve(tag(), None, policy);
        assert_eq!(version.as_deref(), Some("2026.8.1"), "{policy:?}");
        assert_eq!(
            source,
            VersionSource::Tag {
                tag: "discord-v2026.8.1".to_string()
            },
            "{policy:?}"
        );

        let (version, source) = VersionSource::resolve(None, file(), policy);
        assert_eq!(version.as_deref(), Some("2026.8.18"), "{policy:?}");
        assert_eq!(
            source,
            VersionSource::File {
                file: "discord/Cargo.toml".to_string()
            },
            "{policy:?}"
        );
    }
}

#[test]
fn bootstrap_is_unaffected_by_the_policy() {
    for policy in ALL {
        let (version, source) = VersionSource::resolve(None, None, policy);
        assert_eq!(version, None, "{policy:?}");
        assert_eq!(source, VersionSource::Bootstrap, "{policy:?}");
    }
}

#[test]
fn a_missed_tag_is_distinguishable_from_a_real_first_release() {
    let (_, missed_tag) = VersionSource::resolve(None, file(), VersionSourcePolicy::Highest);
    let (_, first_release) = VersionSource::resolve(None, None, VersionSourcePolicy::Highest);
    assert_ne!(missed_tag, first_release);
    assert_eq!(missed_tag.to_string(), "from discord/Cargo.toml");
    assert_eq!(first_release.to_string(), "bootstrapped");
}

#[test]
fn serializes_with_a_kind_discriminator_for_ci() {
    let kind = |policy, tag, file| {
        let (_, source) = VersionSource::resolve(tag, file, policy);
        serde_json::to_value(source).unwrap()
    };

    let json = kind(VersionSourcePolicy::Highest, None, file());
    assert_eq!(json["kind"], "file");
    assert_eq!(json["file"], "discord/Cargo.toml");

    let json = kind(VersionSourcePolicy::Highest, None, None);
    assert_eq!(json["kind"], "bootstrap");

    let json = kind(VersionSourcePolicy::Highest, tag(), file());
    assert_eq!(json["kind"], "file_over_tag");

    let json = kind(VersionSourcePolicy::Tag, tag(), file());
    assert_eq!(json["kind"], "tag_by_policy");
    assert_eq!(json["tag"], "discord-v2026.8.1");
    assert_eq!(json["file"], "discord/Cargo.toml");

    let json = kind(VersionSourcePolicy::File, tag(), file());
    assert_eq!(json["kind"], "file_by_policy");
}
