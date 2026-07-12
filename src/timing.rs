use std::time::{Duration, Instant};

pub struct Timing {
    enabled: bool,
    stages: Vec<Stage>,
}

enum Stage {
    Measured(&'static str, Duration),
    Skipped(&'static str, &'static str),
}

impl Timing {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            stages: Vec::new(),
        }
    }

    pub fn stage<T>(&mut self, name: &'static str, f: impl FnOnce() -> T) -> T {
        if !self.enabled {
            return f();
        }
        let start = Instant::now();
        let out = f();
        self.stages.push(Stage::Measured(name, start.elapsed()));
        out
    }

    pub fn record(&mut self, name: &'static str, elapsed: Duration) {
        if self.enabled {
            self.stages.push(Stage::Measured(name, elapsed));
        }
    }

    pub fn skip(&mut self, name: &'static str, reason: &'static str) {
        if self.enabled {
            self.stages.push(Stage::Skipped(name, reason));
        }
    }

    pub fn total(&self) -> Duration {
        self.stages
            .iter()
            .filter_map(|s| match s {
                Stage::Measured(_, d) => Some(*d),
                Stage::Skipped(..) => None,
            })
            .sum()
    }

    pub fn report(&self) {
        if !self.enabled || self.stages.is_empty() {
            return;
        }

        let label_width = self
            .stages
            .iter()
            .map(|s| match s {
                Stage::Measured(name, _) | Stage::Skipped(name, _) => name.len(),
            })
            .chain(std::iter::once("total".len()))
            .max()
            .unwrap_or(0);

        tracing::info!("");
        tracing::info!("Timing breakdown:");
        for stage in &self.stages {
            match stage {
                Stage::Measured(name, d) => {
                    tracing::info!("  {name:<label_width$}  {:>6} ms", d.as_millis());
                }
                Stage::Skipped(name, reason) => {
                    tracing::info!("  {name:<label_width$}  — ({reason})");
                }
            }
        }
        let rule = "─".repeat(label_width + 12);
        tracing::info!("  {rule}");
        tracing::info!(
            "  {:<label_width$}  {:>6} ms",
            "total",
            self.total().as_millis()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_records_nothing() {
        let mut t = Timing::new(false);
        let out = t.stage("noop", || 42);
        t.record("manual", Duration::from_millis(5));
        t.skip("phase", "dry-run");
        assert_eq!(out, 42);
        assert!(t.stages.is_empty());
        assert_eq!(t.total(), Duration::ZERO);
    }

    #[test]
    fn enabled_accumulates_measured_stages() {
        let mut t = Timing::new(true);
        t.record("a", Duration::from_millis(10));
        t.record("b", Duration::from_millis(20));
        t.skip("c", "dry-run");
        assert_eq!(t.total(), Duration::from_millis(30));
        assert_eq!(t.stages.len(), 3);
    }

    #[test]
    fn stage_returns_value_and_records_when_enabled() {
        let mut t = Timing::new(true);
        let out = t.stage("compute", || 7 * 6);
        assert_eq!(out, 42);
        assert_eq!(t.stages.len(), 1);
        assert!(t.total() < Duration::from_secs(1));
    }
}
