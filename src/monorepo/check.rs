use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::Config;
use crate::git::{get_repo_root, open_repo};
use crate::telemetry;

use super::preview::post_preview_comment;
use super::run::run_release_logic;

pub fn check(
    config_path: Option<&Path>,
    verbose: bool,
    json: bool,
    channel: Option<&str>,
    comment: bool,
) -> Result<()> {
    crate::bot_token::ensure_bot_token()?;
    let repo = open_repo(&std::env::current_dir()?)?;
    let root = get_repo_root(&repo)?;
    let config = Config::load(&root, config_path)?;

    if !json {
        println!("{}", "FerrFlow — Check (dry run)".bold().blue());
        println!();
    }

    let result = run_release_logic(
        &root, &config, true, verbose, json, false, None, channel, false, false,
    );

    if comment {
        post_preview_comment(&repo, &config, &root);
    }

    if config.workspace.anonymous_telemetry {
        telemetry::send_event(telemetry::EventType::Check, None, None, None, None);
    }

    result
}
