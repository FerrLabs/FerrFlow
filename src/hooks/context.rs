pub struct HookContext {
    pub package: String,
    pub old_version: String,
    pub new_version: String,
    pub bump_type: String,
    pub tag: String,
    pub dry_run: bool,
    pub package_path: String,
    pub channel: Option<String>,
}
