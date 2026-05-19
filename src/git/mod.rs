mod auth;
mod commits;
mod diff;
mod fetch;
mod push;
mod repo;
mod retry;
mod tags;

pub use auth::get_remote_url;
#[allow(unused_imports)]
pub use commits::{
    GitLog, create_branch_and_commit, create_branch_and_commits, create_commit,
    get_commits_since_last_stable_tag, get_commits_since_last_tag,
};
pub use diff::{get_changed_files, get_changed_files_since_tag};
pub use fetch::fetch_tags;
#[allow(unused_imports)]
pub use push::{
    force_push_tags, push, push_branch, push_tags, reset_branch_to_remote, verify_remote_branch,
};
pub use repo::{get_repo_root, open_repo, resolve_current_branch};
pub use retry::is_push_rejected_error;
pub use tags::{
    build_head_ancestors, collect_all_tags, create_or_move_tag, create_tag,
    find_highest_semver_tag_with_cache, find_last_tag_name, find_last_tag_name_with_cache,
    get_tag_message, tag_exists,
};

#[cfg(test)]
mod tests;
