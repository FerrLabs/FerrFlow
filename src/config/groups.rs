use std::collections::HashSet;

use super::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    Linked,
    Fixed,
}

impl GroupKind {
    pub fn label(self) -> &'static str {
        match self {
            GroupKind::Linked => "linked",
            GroupKind::Fixed => "fixed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PackageGroup {
    pub kind: GroupKind,
    pub members: Vec<String>,
}

impl Config {
    pub fn package_groups(&self) -> Vec<PackageGroup> {
        let linked = self.workspace.linked.iter().map(|m| PackageGroup {
            kind: GroupKind::Linked,
            members: m.clone(),
        });
        let fixed = self.workspace.fixed.iter().map(|m| PackageGroup {
            kind: GroupKind::Fixed,
            members: m.clone(),
        });
        linked.chain(fixed).collect()
    }

    pub fn validate_groups(&self) -> Result<(), Vec<String>> {
        let groups = self.package_groups();
        if groups.is_empty() {
            return Ok(());
        }

        let known: HashSet<&str> = self.packages.iter().map(|p| p.name.as_str()).collect();
        let mut errors = Vec::new();
        let mut assigned: HashSet<&str> = HashSet::new();

        for group in &groups {
            let kind = group.kind.label();
            if group.members.len() < 2 {
                errors.push(format!(
                    "{kind} group {:?} must list at least two packages",
                    group.members
                ));
            }
            let mut seen_in_group: HashSet<&str> = HashSet::new();
            for member in &group.members {
                if !known.contains(member.as_str()) {
                    errors.push(format!(
                        "package '{member}' in a {kind} group is not defined in package[]"
                    ));
                }
                if !seen_in_group.insert(member.as_str()) {
                    errors.push(format!(
                        "package '{member}' is listed twice in the same {kind} group"
                    ));
                    continue;
                }
                if !assigned.insert(member.as_str()) {
                    errors.push(format!(
                        "package '{member}' appears in more than one linked/fixed group"
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Config;

    fn config(packages: &[&str], linked: &str, fixed: &str) -> Config {
        let pkgs: Vec<String> = packages
            .iter()
            .map(|n| format!(r#"{{"name":"{n}","path":"{n}"}}"#))
            .collect();
        serde_json::from_str(&format!(
            r#"{{"workspace":{{"linked":{linked},"fixed":{fixed}}},"package":[{}]}}"#,
            pkgs.join(",")
        ))
        .unwrap()
    }

    #[test]
    fn valid_groups_pass() {
        let c = config(&["a", "b", "c"], r#"[["a","b"]]"#, r#"[["c","a"]]"#);
        assert!(c.validate_groups().is_err());

        let c = config(&["a", "b", "c"], r#"[["a","b"]]"#, "[]");
        assert!(c.validate_groups().is_ok());
    }

    #[test]
    fn member_missing_from_packages_is_an_error() {
        let c = config(&["a"], r#"[["a","ghost"]]"#, "[]");
        let err = c.validate_groups().unwrap_err();
        assert!(
            err.iter()
                .any(|e| e.contains("ghost") && e.contains("not defined"))
        );
    }

    #[test]
    fn member_in_two_groups_is_an_error() {
        let c = config(&["a", "b", "c"], r#"[["a","b"]]"#, r#"[["a","c"]]"#);
        let err = c.validate_groups().unwrap_err();
        assert!(err.iter().any(|e| e.contains("more than one")));
    }

    #[test]
    fn singleton_group_is_an_error() {
        let c = config(&["a", "b"], r#"[["a"]]"#, "[]");
        let err = c.validate_groups().unwrap_err();
        assert!(err.iter().any(|e| e.contains("at least two")));
    }
}
