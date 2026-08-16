use gix::ObjectId;
use serde::Serialize;
use std::collections::HashSet;

use crate::config::Config;
use crate::git::Repository;

#[derive(Serialize)]
pub(super) struct TagReport {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<String>,
    pub reachable_from_head: bool,
}

pub(super) fn for_package(
    repo: &Repository,
    tag_prefix: &str,
    config: &Config,
    head_ancestors: Option<&HashSet<ObjectId>>,
) -> Option<TagReport> {
    let name = crate::git::find_last_tag_name_with_cache(
        repo,
        tag_prefix,
        config.workspace.orphaned_tag_strategy,
        head_ancestors,
    )
    .ok()
    .flatten()?;

    let oid = crate::git::resolve_tag_name_to_commit(repo, &name);
    let reachable = match (oid, head_ancestors) {
        (Some(oid), Some(set)) => set.contains(&oid),
        (Some(_), None) => true,
        (None, _) => false,
    };

    Some(TagReport {
        commit: oid.map(|o| o.to_string()[..7].to_string()),
        age: oid.and_then(|o| commit_seconds(repo, o)).map(humanize_age),
        reachable_from_head: reachable,
        name,
    })
}

fn commit_seconds(repo: &Repository, oid: ObjectId) -> Option<i64> {
    let commit = repo.find_object(oid).ok()?.try_into_commit().ok()?;
    commit.time().ok().map(|t| t.seconds)
}

fn humanize_age(seconds: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(seconds);
    describe_elapsed(now.saturating_sub(seconds))
}

fn describe_elapsed(elapsed: i64) -> String {
    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;
    const MONTH: i64 = 30 * DAY;
    const YEAR: i64 = 365 * DAY;

    if elapsed < 0 {
        return "in the future".to_string();
    }
    let (count, unit) = match elapsed {
        e if e < MINUTE => return "just now".to_string(),
        e if e < HOUR => (e / MINUTE, "minute"),
        e if e < DAY => (e / HOUR, "hour"),
        e if e < MONTH => (e / DAY, "day"),
        e if e < YEAR => (e / MONTH, "month"),
        e => (e / YEAR, "year"),
    };
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {unit}{plural} ago")
}

#[cfg(test)]
mod tests {
    use super::describe_elapsed;

    #[test]
    fn describes_each_unit_and_its_singular() {
        for (elapsed, expected) in [
            (0, "just now"),
            (59, "just now"),
            (60, "1 minute ago"),
            (3 * 60, "3 minutes ago"),
            (3_600, "1 hour ago"),
            (5 * 3_600, "5 hours ago"),
            (86_400, "1 day ago"),
            (2 * 86_400, "2 days ago"),
            (31 * 86_400, "1 month ago"),
            (400 * 86_400, "1 year ago"),
        ] {
            assert_eq!(describe_elapsed(elapsed), expected, "elapsed={elapsed}");
        }
    }

    #[test]
    fn a_tag_from_the_future_is_labelled_not_negative() {
        assert_eq!(describe_elapsed(-500), "in the future");
    }
}
