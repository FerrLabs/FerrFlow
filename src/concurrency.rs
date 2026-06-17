use std::sync::OnceLock;

static MAX_JOBS: OnceLock<usize> = OnceLock::new();

pub fn init(jobs: Option<usize>) {
    if let Some(n) = jobs {
        let n = n.max(1);
        let _ = MAX_JOBS.set(n);
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global();
    }
}

pub fn max_jobs() -> usize {
    MAX_JOBS.get().copied().unwrap_or(usize::MAX)
}
