use anyhow::{Result, bail};

use crate::cli::AuthAction;

pub fn run(action: AuthAction) -> Result<()> {
    match action {
        AuthAction::Login => bail!("ferrflow auth login is not yet implemented"),
        AuthAction::Logout => bail!("ferrflow auth logout is not yet implemented"),
        AuthAction::Status => bail!("ferrflow auth status is not yet implemented"),
    }
}
