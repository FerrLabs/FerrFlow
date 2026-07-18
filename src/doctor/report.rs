use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Info,
    Warn,
    Error,
}

impl Status {
    fn glyph(self) -> colored::ColoredString {
        match self {
            Status::Ok => "✓".green(),
            Status::Info => "·".dimmed(),
            Status::Warn => "⚠".yellow(),
            Status::Error => "✗".red(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub name: String,
    pub status: Status,
    pub detail: Option<String>,
}

impl Check {
    pub fn new(name: impl Into<String>, status: Status, detail: Option<String>) -> Self {
        Self {
            name: name.into(),
            status,
            detail,
        }
    }

    pub fn ok(name: impl Into<String>, detail: Option<String>) -> Self {
        Self::new(name, Status::Ok, detail)
    }

    pub fn info(name: impl Into<String>, detail: Option<String>) -> Self {
        Self::new(name, Status::Info, detail)
    }

    pub fn warn(name: impl Into<String>, detail: Option<String>) -> Self {
        Self::new(name, Status::Warn, detail)
    }

    pub fn error(name: impl Into<String>, detail: Option<String>) -> Self {
        Self::new(name, Status::Error, detail)
    }
}

#[derive(Debug, Serialize)]
pub struct Section {
    pub title: String,
    pub checks: Vec<Check>,
}

impl Section {
    pub fn new(title: impl Into<String>, checks: Vec<Check>) -> Self {
        Self {
            title: title.into(),
            checks,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub status: Status,
    pub exit_code: i32,
    pub sections: Vec<Section>,
}

impl Report {
    pub fn build(sections: Vec<Section>) -> Self {
        let mut errors = 0usize;
        let mut warnings = 0usize;
        for section in &sections {
            for check in &section.checks {
                match check.status {
                    Status::Error => errors += 1,
                    Status::Warn => warnings += 1,
                    Status::Ok | Status::Info => {}
                }
            }
        }
        let (status, exit_code) = if errors > 0 {
            (Status::Error, 2)
        } else if warnings > 0 {
            (Status::Warn, 1)
        } else {
            (Status::Ok, 0)
        };
        Self {
            status,
            exit_code,
            sections,
        }
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn print_human(&self) {
        tracing::info!("");
        tracing::info!("{}", "ferrflow doctor".bold());

        for section in &self.sections {
            tracing::info!("");
            tracing::info!("{}", section.title.bold());
            for check in &section.checks {
                match &check.detail {
                    Some(detail) => {
                        tracing::info!("  {} {}: {}", check.status.glyph(), check.name, detail)
                    }
                    None => tracing::info!("  {} {}", check.status.glyph(), check.name),
                }
            }
        }

        tracing::info!("");
        let summary = match self.status {
            Status::Error => "problems found — see the errors above".red().bold(),
            Status::Warn => "ready, with warnings".yellow().bold(),
            Status::Ok | Status::Info => "all checks passed".green().bold(),
        };
        tracing::info!("  {summary}");
        tracing::info!("");
    }
}
