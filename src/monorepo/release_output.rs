use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseOutputFormat {
    Github,
    Dotenv,
    Json,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct PublishedRelease {
    pub name: String,
    pub version: String,
    pub tag: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReleaseOutputTarget {
    format: ReleaseOutputFormat,
    path: PathBuf,
}

#[derive(Serialize)]
struct Payload<'a> {
    released: bool,
    releases: &'a [PublishedRelease],
}

impl ReleaseOutputTarget {
    pub fn resolve(
        format: ReleaseOutputFormat,
        file: Option<PathBuf>,
        github_output: Option<OsString>,
    ) -> Result<Self> {
        let path = match (file, format) {
            (Some(path), _) => path,
            (None, ReleaseOutputFormat::Github) => match github_output {
                Some(path) if !path.is_empty() => PathBuf::from(path),
                _ => bail!("--output-format github needs --output-file or GITHUB_OUTPUT"),
            },
            (None, ReleaseOutputFormat::Dotenv) => {
                bail!("--output-format dotenv needs --output-file")
            }
            (None, ReleaseOutputFormat::Json) => bail!("--output-format json needs --output-file"),
        };
        Ok(Self { format, path })
    }

    pub fn write(&self, releases: &[PublishedRelease]) -> Result<()> {
        let content = render(self.format, releases)?;
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(self.format == ReleaseOutputFormat::Github)
            .truncate(self.format != ReleaseOutputFormat::Github)
            .open(&self.path)
            .with_context(|| format!("opening release output file {}", self.path.display()))?;
        file.write_all(content.as_bytes())
            .with_context(|| format!("writing release output file {}", self.path.display()))
    }
}

fn render(format: ReleaseOutputFormat, releases: &[PublishedRelease]) -> Result<String> {
    let released = !releases.is_empty();
    Ok(match format {
        ReleaseOutputFormat::Github => format!(
            "released={released}\nreleases={}\n",
            serde_json::to_string(releases)?
        ),
        ReleaseOutputFormat::Dotenv => format!(
            "FERRFLOW_RELEASED={released}\nFERRFLOW_RELEASES={}\n",
            serde_json::to_string(releases)?
        ),
        ReleaseOutputFormat::Json => {
            format!(
                "{}\n",
                serde_json::to_string(&Payload { released, releases })?
            )
        }
    })
}

#[cfg(test)]
mod tests;
