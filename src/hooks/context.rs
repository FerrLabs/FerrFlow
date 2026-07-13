use std::path::Path;

#[derive(Clone)]
pub struct HookContext {
    pub package: String,
    pub old_version: String,
    pub new_version: String,
    pub bump_type: String,
    pub tag: String,
    pub dry_run: bool,
    pub package_path: String,
    pub channel: Option<String>,
    pub error_code: Option<String>,
}

impl HookContext {
    pub fn release_summary(root: &Path, tags: &[String], dry_run: bool) -> Self {
        HookContext {
            package: String::new(),
            old_version: String::new(),
            new_version: String::new(),
            bump_type: String::new(),
            tag: tags.join(","),
            dry_run,
            package_path: root.to_string_lossy().into_owned(),
            channel: None,
            error_code: None,
        }
    }
}
