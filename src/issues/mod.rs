use anyhow::{Result, bail};

use crate::cli::IssuesAction;

pub fn run(action: IssuesAction) -> Result<()> {
    match action {
        IssuesAction::List => bail!("ferrflow issues list is not yet implemented"),
        IssuesAction::Create => bail!("ferrflow issues create is not yet implemented"),
        IssuesAction::Show { .. } => bail!("ferrflow issues show is not yet implemented"),
        IssuesAction::Update { .. } => bail!("ferrflow issues update is not yet implemented"),
        IssuesAction::Comment { .. } => bail!("ferrflow issues comment is not yet implemented"),
    }
}
