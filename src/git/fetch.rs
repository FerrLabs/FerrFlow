use anyhow::Result;
use git2::{RemoteCallbacks, Repository};

use super::auth::{credentials_callback, get_authenticated_remote};

pub(super) fn make_fetch_options() -> git2::FetchOptions<'static> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(credentials_callback);
    let mut opts = git2::FetchOptions::new();
    opts.remote_callbacks(callbacks);
    opts
}

pub fn fetch_tags(repo: &Repository, remote_name: &str) -> Result<()> {
    let mut remote = get_authenticated_remote(repo, remote_name)?;
    let mut opts = make_fetch_options();
    remote.fetch(&["refs/tags/*:refs/tags/*"], Some(&mut opts), None)?;
    Ok(())
}
