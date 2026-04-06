#[derive(Debug, Clone, Copy)]
pub enum HookPoint {
    PreBump,
    PostBump,
    PreCommit,
    PrePublish,
    PostPublish,
}

impl HookPoint {
    pub fn label(self) -> &'static str {
        match self {
            Self::PreBump => "pre_bump",
            Self::PostBump => "post_bump",
            Self::PreCommit => "pre_commit",
            Self::PrePublish => "pre_publish",
            Self::PostPublish => "post_publish",
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
        assert_eq!(HookPoint::PrePublish.label(), "pre_publish");
        assert_eq!(HookPoint::PostPublish.label(), "post_publish");
    }
}
