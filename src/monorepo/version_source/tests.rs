use super::VersionSource;

fn tag() -> Option<(String, String)> {
    Some(("discord-v2026.8.1".to_string(), "2026.8.1".to_string()))
}

fn file() -> Option<(String, String)> {
    Some(("discord/Cargo.toml".to_string(), "2026.8.18".to_string()))
}

#[test]
fn resolve_names_the_file_when_the_file_won_the_comparison() {
    assert_eq!(
        VersionSource::resolve(tag(), file(), "2026.8.18"),
        VersionSource::FileOverTag {
            file: "discord/Cargo.toml".to_string(),
            tag: "discord-v2026.8.1".to_string(),
        }
    );
}

#[test]
fn resolve_names_the_tag_when_the_tag_won_the_comparison() {
    assert_eq!(
        VersionSource::resolve(tag(), file(), "2026.8.1"),
        VersionSource::TagOverFile {
            tag: "discord-v2026.8.1".to_string(),
            file: "discord/Cargo.toml".to_string(),
        }
    );
}

#[test]
fn resolve_credits_the_tag_when_both_carry_the_same_version() {
    let same = Some(("discord/Cargo.toml".to_string(), "2026.8.1".to_string()));
    assert_eq!(
        VersionSource::resolve(tag(), same, "2026.8.1"),
        VersionSource::TagOverFile {
            tag: "discord-v2026.8.1".to_string(),
            file: "discord/Cargo.toml".to_string(),
        }
    );
}

#[test]
fn resolve_reports_the_single_available_source() {
    assert_eq!(
        VersionSource::resolve(tag(), None, "2026.8.1"),
        VersionSource::Tag {
            tag: "discord-v2026.8.1".to_string()
        }
    );
    assert_eq!(
        VersionSource::resolve(None, file(), "2026.8.18"),
        VersionSource::File {
            file: "discord/Cargo.toml".to_string()
        }
    );
}

#[test]
fn resolve_reports_bootstrap_when_neither_source_exists() {
    assert_eq!(
        VersionSource::resolve(None, None, "0.0.0"),
        VersionSource::Bootstrap
    );
}

#[test]
fn a_missed_tag_is_distinguishable_from_a_real_first_release() {
    let missed_tag = VersionSource::resolve(None, file(), "2026.8.18");
    let first_release = VersionSource::resolve(None, None, "0.0.0");
    assert_ne!(missed_tag, first_release);
    assert_eq!(missed_tag.to_string(), "from discord/Cargo.toml");
    assert_eq!(first_release.to_string(), "bootstrapped");
}

#[test]
fn serializes_with_a_kind_discriminator_for_ci() {
    let json = serde_json::to_value(VersionSource::resolve(None, file(), "2026.8.18")).unwrap();
    assert_eq!(json["kind"], "file");
    assert_eq!(json["file"], "discord/Cargo.toml");

    let json = serde_json::to_value(VersionSource::Bootstrap).unwrap();
    assert_eq!(json["kind"], "bootstrap");

    let json = serde_json::to_value(VersionSource::resolve(tag(), file(), "2026.8.18")).unwrap();
    assert_eq!(json["kind"], "file_over_tag");
    assert_eq!(json["file"], "discord/Cargo.toml");
    assert_eq!(json["tag"], "discord-v2026.8.1");
}
