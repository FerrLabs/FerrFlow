use super::oidc_verify::GhaClaims;

const ALLOWED_EVENTS: [&str; 4] = ["push", "workflow_dispatch", "schedule", "release"];

#[derive(Debug, PartialEq, Eq)]
pub enum Denial {
    Event,
    Ref,
}

pub fn check_event(claims: &GhaClaims) -> Result<(), Denial> {
    if ALLOWED_EVENTS.contains(&claims.event_name.as_str()) {
        return Ok(());
    }
    Err(Denial::Event)
}

pub fn check_ref(claims: &GhaClaims, default_branch: &str) -> Result<(), Denial> {
    if claims.ref_.starts_with("refs/tags/")
        || claims.ref_ == format!("refs/heads/{default_branch}")
    {
        return Ok(());
    }
    Err(Denial::Ref)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::oidc_verify::Audience;

    fn claims(ref_: &str, event_name: &str) -> GhaClaims {
        GhaClaims {
            repository: "FerrLabs/FerrFlow".into(),
            repository_owner: "FerrLabs".into(),
            ref_: ref_.into(),
            event_name: event_name.into(),
            workflow: "Release".into(),
            run_id: "1".into(),
            actor: "octocat".into(),
            sub: "sub".into(),
            iss: "iss".into(),
            aud: Audience::None,
            exp: 0,
        }
    }

    #[test]
    fn untrusted_input_events_are_denied() {
        for event in [
            "pull_request",
            "pull_request_target",
            "workflow_run",
            "issue_comment",
            "pull_request_review",
            "pull_request_review_comment",
            "",
        ] {
            assert_eq!(
                check_event(&claims("refs/heads/main", event)),
                Err(Denial::Event)
            );
        }
    }

    #[test]
    fn release_events_pass() {
        for event in ["push", "workflow_dispatch", "schedule", "release"] {
            assert_eq!(check_event(&claims("refs/heads/main", event)), Ok(()));
        }
    }

    #[test]
    fn default_branch_and_tags_pass() {
        assert_eq!(
            check_ref(&claims("refs/heads/main", "push"), "main"),
            Ok(())
        );
        assert_eq!(
            check_ref(&claims("refs/tags/v1.0.0", "push"), "main"),
            Ok(())
        );
    }

    #[test]
    fn other_branches_are_denied() {
        assert_eq!(
            check_ref(&claims("refs/heads/feature/x", "workflow_dispatch"), "main"),
            Err(Denial::Ref)
        );
    }

    #[test]
    fn a_branch_named_like_the_default_prefix_is_denied() {
        assert_eq!(
            check_ref(&claims("refs/heads/main-evil", "push"), "main"),
            Err(Denial::Ref)
        );
        assert_eq!(
            check_ref(&claims("refs/pull/1/merge", "push"), "main"),
            Err(Denial::Ref)
        );
    }
}
