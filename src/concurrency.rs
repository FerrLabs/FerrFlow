use std::sync::OnceLock;

static MAX_JOBS: OnceLock<usize> = OnceLock::new();

pub fn init(jobs: Option<usize>) {
    if let Some(n) = jobs {
        let n = n.max(1);
        let _ = MAX_JOBS.set(n);
        if needs_pool(n) {
            let _ = rayon::ThreadPoolBuilder::new()
                .num_threads(n)
                .build_global();
        }
    }
}

fn needs_pool(jobs: usize) -> bool {
    jobs > 1
}

pub fn max_jobs() -> usize {
    MAX_JOBS.get().copied().unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_job_builds_no_pool() {
        assert!(!needs_pool(1));
    }

    #[test]
    fn several_jobs_build_a_pool() {
        assert!(needs_pool(2));
        assert!(needs_pool(32));
    }
}
