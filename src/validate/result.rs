use serde::Serialize;

#[derive(Debug, PartialEq)]
pub enum ValidationLevel {
    Error,
    Warning,
    Suggestion,
}

#[derive(Debug)]
pub struct ValidationEntry {
    pub level: ValidationLevel,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub config_file: Option<String>,
    pub package_count: usize,
    pub errors: Vec<EntryOutput>,
    pub warnings: Vec<EntryOutput>,
    pub suggestions: Vec<EntryOutput>,
}

#[derive(Debug, Serialize)]
pub struct EntryOutput {
    pub path: String,
    pub message: String,
}

impl ValidationResult {
    pub fn from_entries(entries: Vec<ValidationEntry>) -> Self {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();
        for entry in entries {
            let output = EntryOutput {
                path: entry.path,
                message: entry.message,
            };
            match entry.level {
                ValidationLevel::Error => errors.push(output),
                ValidationLevel::Warning => warnings.push(output),
                ValidationLevel::Suggestion => suggestions.push(output),
            }
        }
        let valid = errors.is_empty();
        Self {
            valid,
            config_file: None,
            package_count: 0,
            errors,
            warnings,
            suggestions,
        }
    }
}
