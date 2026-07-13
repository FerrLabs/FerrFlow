#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookPoint {
    PreBump,
    PostBump,
    PreCommit,
    PostCommit,
    PreTag,
    PostTag,
    PrePublish,
    PostPublish,
    PreRelease,
    OnSuccess,
    OnError,
}

impl HookPoint {
    pub fn label(self) -> &'static str {
        match self {
            Self::PreBump => "pre_bump",
            Self::PostBump => "post_bump",
            Self::PreCommit => "pre_commit",
            Self::PostCommit => "post_commit",
            Self::PreTag => "pre_tag",
            Self::PostTag => "post_tag",
            Self::PrePublish => "pre_publish",
            Self::PostPublish => "post_publish",
            Self::PreRelease => "pre_release",
            Self::OnSuccess => "on_success",
            Self::OnError => "on_error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_point_labels() {
        assert_eq!(HookPoint::PreBump.label(), "pre_bump");
        assert_eq!(HookPoint::PostBump.label(), "post_bump");
        assert_eq!(HookPoint::PreCommit.label(), "pre_commit");
        assert_eq!(HookPoint::PostCommit.label(), "post_commit");
        assert_eq!(HookPoint::PreTag.label(), "pre_tag");
        assert_eq!(HookPoint::PostTag.label(), "post_tag");
        assert_eq!(HookPoint::PrePublish.label(), "pre_publish");
        assert_eq!(HookPoint::PostPublish.label(), "post_publish");
        assert_eq!(HookPoint::PreRelease.label(), "pre_release");
        assert_eq!(HookPoint::OnSuccess.label(), "on_success");
        assert_eq!(HookPoint::OnError.label(), "on_error");
    }
}
