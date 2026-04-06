use crate::config::{HooksConfig, OnFailure};

use super::HookPoint;

pub fn resolve_hook(
    pkg_hooks: Option<&HooksConfig>,
    ws_hooks: Option<&HooksConfig>,
    point: HookPoint,
) -> Option<String> {
    fn get(h: &HooksConfig, point: HookPoint) -> Option<&String> {
        match point {
            HookPoint::PreBump => h.pre_bump.as_ref(),
            HookPoint::PostBump => h.post_bump.as_ref(),
            HookPoint::PreCommit => h.pre_commit.as_ref(),
            HookPoint::PrePublish => h.pre_publish.as_ref(),
            HookPoint::PostPublish => h.post_publish.as_ref(),
        }
    }

    if let Some(pkg) = pkg_hooks
        && let Some(cmd) = get(pkg, point)
    {
        return Some(cmd.clone());
    }

    if let Some(ws) = ws_hooks
        && let Some(cmd) = get(ws, point)
    {
        return Some(cmd.clone());
    }

    None
}

pub fn resolve_on_failure(
    pkg_hooks: Option<&HooksConfig>,
    ws_hooks: Option<&HooksConfig>,
) -> OnFailure {
    if let Some(pkg) = pkg_hooks
        && let Some(v) = pkg.on_failure
    {
        return v;
    }
    if let Some(ws) = ws_hooks
        && let Some(v) = ws.on_failure
    {
        return v;
    }
    OnFailure::Abort
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws_hooks(pre_bump: Option<&str>, post_publish: Option<&str>) -> HooksConfig {
        HooksConfig {
            pre_bump: pre_bump.map(String::from),
            post_publish: post_publish.map(String::from),
            ..Default::default()
        }
    }

    #[test]
    fn resolve_falls_back_to_workspace() {
        let ws = ws_hooks(Some("echo ws"), None);
        let result = resolve_hook(None, Some(&ws), HookPoint::PreBump);
        assert_eq!(result.as_deref(), Some("echo ws"));
    }

    #[test]
    fn resolve_package_overrides_workspace() {
        let ws = ws_hooks(Some("echo ws"), None);
        let pkg = HooksConfig {
            pre_bump: Some("echo pkg".into()),
            ..Default::default()
        };
        let result = resolve_hook(Some(&pkg), Some(&ws), HookPoint::PreBump);
        assert_eq!(result.as_deref(), Some("echo pkg"));
    }

    #[test]
    fn resolve_returns_none_when_unset() {
        let ws = ws_hooks(Some("echo ws"), None);
        let result = resolve_hook(None, Some(&ws), HookPoint::PostBump);
        assert!(result.is_none());
    }

    #[test]
    fn resolve_no_hooks_at_all() {
        let result = resolve_hook(None, None, HookPoint::PreBump);
        assert!(result.is_none());
    }

    #[test]
    fn on_failure_defaults_to_abort() {
        assert_eq!(resolve_on_failure(None, None), OnFailure::Abort);
    }

    #[test]
    fn on_failure_inherits_workspace() {
        let ws = HooksConfig {
            on_failure: Some(OnFailure::Continue),
            ..Default::default()
        };
        assert_eq!(resolve_on_failure(None, Some(&ws)), OnFailure::Continue);
    }

    #[test]
    fn on_failure_package_overrides_workspace() {
        let ws = HooksConfig {
            on_failure: Some(OnFailure::Continue),
            ..Default::default()
        };
        let pkg = HooksConfig {
            on_failure: Some(OnFailure::Abort),
            ..Default::default()
        };
        assert_eq!(resolve_on_failure(Some(&pkg), Some(&ws)), OnFailure::Abort);
    }

    #[test]
    fn resolve_all_hook_points() {
        let hooks = HooksConfig {
            pre_bump: Some("a".into()),
            post_bump: Some("b".into()),
            pre_commit: Some("c".into()),
            pre_publish: Some("d".into()),
            post_publish: Some("e".into()),
            on_failure: None,
        };
        assert_eq!(
            resolve_hook(Some(&hooks), None, HookPoint::PreBump).as_deref(),
            Some("a")
        );
        assert_eq!(
            resolve_hook(Some(&hooks), None, HookPoint::PostBump).as_deref(),
            Some("b")
        );
        assert_eq!(
            resolve_hook(Some(&hooks), None, HookPoint::PreCommit).as_deref(),
            Some("c")
        );
        assert_eq!(
            resolve_hook(Some(&hooks), None, HookPoint::PrePublish).as_deref(),
            Some("d")
        );
        assert_eq!(
            resolve_hook(Some(&hooks), None, HookPoint::PostPublish).as_deref(),
            Some("e")
        );
    }
}
