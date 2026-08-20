use anyhow::{Result, bail};
use chrono::{DateTime, Datelike, Utc};
use regex::Regex;

use crate::conventional_commits::BumpType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Var {
    Year,
    ShortYear,
    Month,
    PaddedMonth,
    Day,
    PaddedDay,
    Week,
    PaddedWeek,
    Quarter,
    Seq,
    Major,
    Minor,
    Patch,
}

impl Var {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "year" => Self::Year,
            "short_year" => Self::ShortYear,
            "month" => Self::Month,
            "padded_month" => Self::PaddedMonth,
            "day" => Self::Day,
            "padded_day" => Self::PaddedDay,
            "week" => Self::Week,
            "padded_week" => Self::PaddedWeek,
            "quarter" => Self::Quarter,
            "seq" => Self::Seq,
            "major" => Self::Major,
            "minor" => Self::Minor,
            "patch" => Self::Patch,
            _ => return None,
        })
    }

    fn is_calendar(self) -> bool {
        !matches!(self, Self::Seq | Self::Major | Self::Minor | Self::Patch)
    }

    fn render_calendar(self, now: DateTime<Utc>) -> String {
        match self {
            Self::Year => now.year().to_string(),
            Self::ShortYear => format!("{:02}", now.year() % 100),
            Self::Month => now.month().to_string(),
            Self::PaddedMonth => format!("{:02}", now.month()),
            Self::Day => now.day().to_string(),
            Self::PaddedDay => format!("{:02}", now.day()),
            Self::Week => now.iso_week().week().to_string(),
            Self::PaddedWeek => format!("{:02}", now.iso_week().week()),
            Self::Quarter => ((now.month() - 1) / 3 + 1).to_string(),
            _ => unreachable!(),
        }
    }
}

const VALID_VARS: &str = "year, short_year, month, padded_month, day, padded_day, \
week, padded_week, quarter, seq, major, minor, patch";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Literal(String),
    Var(Var),
}

#[derive(Debug, Clone)]
pub struct VersionTemplate {
    segments: Vec<Segment>,
}

impl VersionTemplate {
    pub fn parse(template: &str) -> Result<Self> {
        if template.trim().is_empty() {
            bail!("versionTemplate is empty");
        }

        let mut segments = Vec::new();
        let mut literal = String::new();
        let mut rest = template;

        while let Some(open) = rest.find('{') {
            literal.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            let Some(close) = after.find('}') else {
                bail!("versionTemplate has an unclosed '{{' in {template:?}");
            };
            let name = &after[..close];
            let Some(var) = Var::from_name(name) else {
                bail!("versionTemplate uses unknown variable {{{name}}}. Valid: {VALID_VARS}");
            };
            if !literal.is_empty() {
                segments.push(Segment::Literal(std::mem::take(&mut literal)));
            } else if matches!(segments.last(), Some(Segment::Var(_))) {
                bail!(
                    "versionTemplate puts {{{name}}} directly after another variable, which cannot be read back. Separate them with a literal, e.g. '.'"
                );
            }
            segments.push(Segment::Var(var));
            rest = &after[close + 1..];
        }
        literal.push_str(rest);
        if !literal.is_empty() {
            segments.push(Segment::Literal(literal));
        }

        if !segments.iter().any(|s| matches!(s, Segment::Var(_))) {
            bail!("versionTemplate {template:?} contains no variables");
        }
        if segments
            .iter()
            .filter(|s| matches!(s, Segment::Var(Var::Seq)))
            .count()
            > 1
        {
            bail!("versionTemplate uses {{seq}} more than once");
        }

        Ok(Self { segments })
    }

    fn vars(&self) -> impl Iterator<Item = Var> + '_ {
        self.segments.iter().filter_map(|s| match s {
            Segment::Var(v) => Some(*v),
            Segment::Literal(_) => None,
        })
    }

    fn read_back(&self, current: &str) -> Option<Vec<(Var, u64)>> {
        let mut pattern = String::from("^");
        for segment in &self.segments {
            match segment {
                Segment::Literal(text) => pattern.push_str(&regex::escape(text)),
                Segment::Var(_) => pattern.push_str(r"(\d+)"),
            }
        }
        pattern.push('$');

        let re = Regex::new(&pattern).ok()?;
        let caps = re.captures(current.trim_start_matches('v'))?;
        self.vars()
            .enumerate()
            .map(|(i, var)| caps.get(i + 1)?.as_str().parse().ok().map(|n| (var, n)))
            .collect()
    }

    pub fn render(&self, current: &str, bump: BumpType, now: DateTime<Utc>) -> String {
        let previous = self.read_back(current);

        let calendar_rolled = previous.as_ref().is_none_or(|vals| {
            vals.iter()
                .filter(|(v, _)| v.is_calendar())
                .any(|(v, n)| v.render_calendar(now) != n.to_string())
        });

        let previous_of = |want: Var| {
            previous
                .as_ref()
                .and_then(|vals| vals.iter().find(|(v, _)| *v == want).map(|(_, n)| *n))
        };

        let seq = if calendar_rolled {
            1
        } else {
            previous_of(Var::Seq).unwrap_or(0) + 1
        };

        let major = previous_of(Var::Major).unwrap_or(0);
        let minor = previous_of(Var::Minor).unwrap_or(0);
        let patch = previous_of(Var::Patch).unwrap_or(0);
        let (major, minor, patch) = match bump {
            BumpType::Major => (major + 1, 0, 0),
            BumpType::Minor => (major, minor + 1, 0),
            BumpType::Patch => (major, minor, patch + 1),
            BumpType::None => (major, minor, patch),
        };

        self.segments
            .iter()
            .map(|segment| match segment {
                Segment::Literal(text) => text.clone(),
                Segment::Var(Var::Seq) => seq.to_string(),
                Segment::Var(Var::Major) => major.to_string(),
                Segment::Var(Var::Minor) => minor.to_string(),
                Segment::Var(Var::Patch) => patch.to_string(),
                Segment::Var(var) => var.render_calendar(now),
            })
            .collect()
    }
}

pub fn validate(template: &str) -> Result<()> {
    VersionTemplate::parse(template).map(|_| ())
}

pub fn render(current: &str, bump: BumpType, template: &str) -> Result<String> {
    Ok(VersionTemplate::parse(template)?.render(current, bump, Utc::now()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
    }

    fn render_at(template: &str, current: &str, bump: BumpType, now: DateTime<Utc>) -> String {
        VersionTemplate::parse(template)
            .unwrap()
            .render(current, bump, now)
    }

    #[test]
    fn renders_calendar_variables() {
        let now = at(2026, 8, 5);
        assert_eq!(
            render_at("{year}.{month}.{day}", "0.0.0", BumpType::None, now),
            "2026.8.5"
        );
        assert_eq!(
            render_at(
                "{short_year}.{padded_month}.{padded_day}",
                "0.0.0",
                BumpType::None,
                now
            ),
            "26.08.05"
        );
        assert_eq!(
            render_at("{year}.Q{quarter}", "0.0", BumpType::None, now),
            "2026.Q3"
        );
    }

    #[test]
    fn seq_increments_while_the_calendar_part_holds() {
        let now = at(2026, 8, 20);
        assert_eq!(
            render_at("{year}.{month}.{seq}", "2026.8.3", BumpType::None, now),
            "2026.8.4"
        );
    }

    #[test]
    fn seq_resets_when_the_calendar_part_rolls() {
        let now = at(2026, 9, 1);
        assert_eq!(
            render_at("{year}.{month}.{seq}", "2026.8.7", BumpType::None, now),
            "2026.9.1"
        );
    }

    #[test]
    fn seq_resets_when_the_current_version_does_not_match_the_template() {
        let now = at(2026, 8, 20);
        assert_eq!(
            render_at("{year}.{month}.{seq}", "1.4.2-rc.1", BumpType::None, now),
            "2026.8.1"
        );
    }

    #[test]
    fn mixes_calendar_and_semver_parts() {
        let now = at(2026, 8, 20);
        assert_eq!(
            render_at("{year}.{minor}.{patch}", "2026.4.7", BumpType::Minor, now),
            "2026.5.0"
        );
        assert_eq!(
            render_at("{year}.{minor}.{patch}", "2026.4.7", BumpType::Patch, now),
            "2026.4.8"
        );
    }

    #[test]
    fn a_leading_v_on_the_current_version_still_reads_back() {
        let now = at(2026, 8, 20);
        assert_eq!(
            render_at("{year}.{month}.{seq}", "v2026.8.3", BumpType::None, now),
            "2026.8.4"
        );
    }

    #[test]
    fn rejects_an_unknown_variable() {
        let err = VersionTemplate::parse("{yaer}.{month}")
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown variable {yaer}"), "{err}");
        assert!(err.contains("short_year"), "{err}");
    }

    #[test]
    fn rejects_adjacent_variables_because_they_cannot_be_read_back() {
        let err = VersionTemplate::parse("{year}{month}")
            .unwrap_err()
            .to_string();
        assert!(err.contains("directly after another variable"), "{err}");
    }

    #[test]
    fn rejects_an_unclosed_brace() {
        assert!(VersionTemplate::parse("{year.{month}").is_err());
    }

    #[test]
    fn rejects_a_template_with_no_variables() {
        assert!(VersionTemplate::parse("1.0.0").is_err());
    }

    #[test]
    fn rejects_more_than_one_seq() {
        assert!(VersionTemplate::parse("{seq}.{seq}").is_err());
    }

    #[test]
    fn rejects_an_empty_template() {
        assert!(VersionTemplate::parse("   ").is_err());
    }
}
