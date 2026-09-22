use crate::config::{ForgeKind, WorkspaceConfig};

#[derive(Debug, Clone, Copy)]
pub struct Remote<'a> {
    pub name: &'a str,
    pub forge: ForgeKind,
}

impl<'a> Remote<'a> {
    pub fn of(workspace: &'a WorkspaceConfig) -> Self {
        Self {
            name: &workspace.remote,
            forge: workspace.forge,
        }
    }
}
