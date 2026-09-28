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
        for line in self.report_lines() {
            tracing::info!("{line}");
        }
    }

    fn report_lines(&self) -> Vec<String> {
        if !self.enabled || self.stages.is_empty() {
            return Vec::new();
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

        let mut out = vec![String::new(), "Timing breakdown:".to_string()];
        for stage in &self.stages {
            out.push(match stage {
                Stage::Measured(name, d) => {
                    format!("  {name:<label_width$}  {:>6} ms", d.as_millis())
                }
                Stage::Skipped(name, reason) => format!("  {name:<label_width$}  — ({reason})"),
            });
        }
        let rule = "─".repeat(label_width + 12);
        out.push(format!("  {rule}"));
        out.push(format!(
            "  {:<label_width$}  {:>6} ms",
            "total",
            self.total().as_millis()
        ));
        out
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

    #[test]
    fn nothing_is_reported_when_disabled_or_empty() {
        let mut off = Timing::new(false);
        off.record("scan", Duration::from_millis(5));
        assert!(off.report_lines().is_empty());
        assert!(Timing::new(true).report_lines().is_empty());
    }

    #[test]
    fn the_report_aligns_stages_skips_and_the_total() {
        let mut t = Timing::new(true);
        t.record("collect_commits", Duration::from_millis(1234));
        t.skip("git", "dry-run");
        t.record("bump", Duration::from_millis(7));
        let lines = t.report_lines();

        assert_eq!(
            lines,
            vec![
                String::new(),
                "Timing breakdown:".to_string(),
                "  collect_commits    1234 ms".to_string(),
                "  git              — (dry-run)".to_string(),
                "  bump                  7 ms".to_string(),
                format!("  {}", "─".repeat("collect_commits".len() + 12)),
                "  total              1241 ms".to_string(),
            ]
        );
    }

    #[test]
    fn short_stage_names_are_padded_to_the_total_label() {
        let mut t = Timing::new(true);
        t.record("a", Duration::from_millis(3));
        let lines = t.report_lines();
        assert_eq!(lines[2], "  a           3 ms");
        assert_eq!(lines[4], "  total       3 ms");
    }
}
