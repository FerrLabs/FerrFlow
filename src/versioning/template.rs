use anyhow::{Result, bail};
use chrono::{DateTime, Datelike, Utc};
use regex::Regex;

use super::strategies::{bump_core, overflow};
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

    fn is_week(self) -> bool {
        matches!(self, Self::Week | Self::PaddedWeek)
    }

    fn calendar_value(self, now: DateTime<Utc>, iso_year: bool) -> i64 {
        let year = if iso_year {
            now.iso_week().year()
        } else {
            now.year()
        };
        match self {
            Self::Year => year.into(),
            Self::ShortYear => year.rem_euclid(100).into(),
            Self::Month | Self::PaddedMonth => now.month().into(),
            Self::Day | Self::PaddedDay => now.day().into(),
            Self::Week | Self::PaddedWeek => now.iso_week().week().into(),
            Self::Quarter => ((now.month() - 1) / 3 + 1).into(),
            _ => unreachable!(),
        }
    }

    fn render_calendar(self, now: DateTime<Utc>, iso_year: bool) -> String {
        let value = self.calendar_value(now, iso_year);
        match self {
            Self::ShortYear | Self::PaddedMonth | Self::PaddedDay | Self::PaddedWeek => {
                format!("{value:02}")
            }
            _ => value.to_string(),
        }
    }
}

const VALID_VARS: &str = "year, short_year, month, padded_month, day, padded_day, \
week, padded_week, quarter, seq, major, minor, patch";

const GIT_RESERVED: &[char] = &['~', '^', ':', '?', '*', '[', '\\'];

fn check_literal(literal: &str, template: &str) -> Result<()> {
    for c in literal.chars() {
        let what = if c.is_control() {
            "a control character"
        } else if c.is_whitespace() {
            "whitespace"
        } else if GIT_RESERVED.contains(&c) {
            "a character git reserves in a ref name"
        } else {
            continue;
        };
        bail!(
            "versionTemplate {template:?} has {what} ({c:?}) in a literal. \
             The version becomes a git tag, which cannot carry it."
        );
    }

    if literal.contains("..") {
        bail!(
            "versionTemplate {template:?} has \"..\" in a literal. \
             The version becomes a git tag, which cannot carry two consecutive dots."
        );
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Literal(String),
    Var(Var),
}

fn split_segments(template: &str) -> Result<Vec<Segment>> {
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
    Ok(segments)
}

fn validate_segments(segments: &[Segment], template: &str) -> Result<()> {
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

    for segment in segments {
        if let Segment::Literal(text) = segment {
            check_literal(text, template)?;
        }
    }

    if let Some(Segment::Literal(last)) = segments.last() {
        if last.ends_with(".lock") {
            bail!(
                "versionTemplate {template:?} ends with \".lock\". \
                 The version becomes a git tag, which cannot end with it."
            );
        }
        if last.ends_with('.') {
            bail!(
                "versionTemplate {template:?} ends with a dot. \
                 The version becomes a git tag, which cannot end with one."
            );
        }
    }
    Ok(())
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

        let segments = split_segments(template)?;
        validate_segments(&segments, template)?;
        Ok(Self { segments })
    }

    fn vars(&self) -> impl Iterator<Item = Var> + '_ {
        self.segments.iter().filter_map(|s| match s {
            Segment::Var(v) => Some(*v),
            Segment::Literal(_) => None,
        })
    }

    fn uses_iso_year(&self) -> bool {
        self.vars().any(Var::is_week)
    }

    fn read_back(&self, current: &str) -> Result<Option<Vec<(Var, u64)>>> {
        let mut pattern = String::from("^");
        for segment in &self.segments {
            match segment {
                Segment::Literal(text) => pattern.push_str(&regex::escape(text)),
                Segment::Var(_) => pattern.push_str(r"([0-9]+)"),
            }
        }
        pattern.push('$');

        let re = Regex::new(&pattern)?;
        let bare = current.trim_start_matches('v');
        let prefixed = format!("v{bare}");
        let Some(caps) = re
            .captures(current)
            .or_else(|| re.captures(bare))
            .or_else(|| re.captures(&prefixed))
        else {
            return Ok(None);
        };
        self.vars()
            .zip(caps.iter().skip(1).flatten())
            .map(|(var, m)| Ok((var, m.as_str().parse().map_err(|_| overflow())?)))
            .collect::<Result<_>>()
            .map(Some)
    }

    pub fn render(&self, current: &str, bump: BumpType, now: DateTime<Utc>) -> Result<String> {
        let previous = self.read_back(current)?;

        let iso_year = self.uses_iso_year();
        let calendar_rolled = previous.as_ref().is_none_or(|vals| {
            vals.iter()
                .filter(|(v, _)| v.is_calendar())
                .any(|(v, n)| u64::try_from(v.calendar_value(now, iso_year)) != Ok(*n))
        });

        let previous_of = |want: Var| {
            previous
                .as_ref()
                .and_then(|vals| vals.iter().find(|(v, _)| *v == want).map(|(_, n)| *n))
        };

        let seq = if calendar_rolled {
            1
        } else {
            previous_of(Var::Seq)
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(overflow)?
        };

        let major = previous_of(Var::Major).unwrap_or(0);
        let minor = previous_of(Var::Minor).unwrap_or(0);
        let patch = previous_of(Var::Patch).unwrap_or(0);
        let (major, minor, patch) = bump_core(major, minor, patch, bump).ok_or_else(overflow)?;

        Ok(self
            .segments
            .iter()
            .map(|segment| match segment {
                Segment::Literal(text) => text.clone(),
                Segment::Var(Var::Seq) => seq.to_string(),
                Segment::Var(Var::Major) => major.to_string(),
                Segment::Var(Var::Minor) => minor.to_string(),
                Segment::Var(Var::Patch) => patch.to_string(),
                Segment::Var(var) => var.render_calendar(now, iso_year),
            })
            .collect())
    }
}

const UNORDERED_AROUND_NEW_YEAR: &str = "combines a week variable with month, day or quarter. \
These versions are not ordered around New Year: the year follows the ISO week-numbering year while \
month, day and quarter stay on the calendar.";

pub fn ordering_warning(template: &str) -> Option<&'static str> {
    let template = VersionTemplate::parse(template).ok()?;
    let has_week = template.vars().any(Var::is_week);
    let has_calendar_unit = template.vars().any(|v| {
        matches!(
            v,
            Var::Month | Var::PaddedMonth | Var::Day | Var::PaddedDay | Var::Quarter
        )
    });
    (has_week && has_calendar_unit).then_some(UNORDERED_AROUND_NEW_YEAR)
}

pub fn validate(template: &str) -> Result<()> {
    VersionTemplate::parse(template).map(|_| ())
}

pub fn render(current: &str, bump: BumpType, template: &str) -> Result<String> {
    VersionTemplate::parse(template)?.render(current, bump, Utc::now())
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
            .unwrap()
    }

    fn render_err(template: &str, current: &str, bump: BumpType) -> String {
        VersionTemplate::parse(template)
            .unwrap()
            .render(current, bump, at(2026, 8, 5))
            .unwrap_err()
            .to_string()
    }

    const MAX: &str = "18446744073709551615";

    #[test]
    fn seq_at_u64_max_errors() {
        let current = format!("2026.8.{MAX}");
        let err = render_err("{year}.{month}.{seq}", &current, BumpType::None);
        assert!(err.contains("overflow"), "{err}");
    }

    #[test]
    fn major_at_u64_max_errors_on_major_bump() {
        let current = format!("{MAX}.0.0");
        let err = render_err("{major}.{minor}.{patch}", &current, BumpType::Major);
        assert!(err.contains("overflow"), "{err}");
    }

    #[test]
    fn minor_at_u64_max_errors_on_minor_bump() {
        let current = format!("1.{MAX}.0");
        let err = render_err("{major}.{minor}.{patch}", &current, BumpType::Minor);
        assert!(err.contains("overflow"), "{err}");
    }

    #[test]
    fn patch_at_u64_max_errors_on_patch_bump() {
        let current = format!("1.2.{MAX}");
        let err = render_err("{major}.{minor}.{patch}", &current, BumpType::Patch);
        assert!(err.contains("overflow"), "{err}");
    }

    #[test]
    fn a_component_past_u64_max_errors_instead_of_resetting() {
        let over = "18446744073709551616";
        for (template, current) in [
            ("{major}.{minor}.{patch}", format!("{over}.0.0")),
            ("{major}.{minor}.{patch}", format!("1.{over}.0")),
            ("{year}.{month}.{seq}", format!("2026.8.{over}")),
        ] {
            for bump in [BumpType::Major, BumpType::Patch, BumpType::None] {
                let err = render_err(template, &current, bump);
                assert!(
                    err.contains("overflow"),
                    "{template} {current} {bump:?}: {err}"
                );
            }
        }
    }

    #[test]
    fn non_ascii_digits_are_not_read_back() {
        assert_eq!(
            render_at(
                "{major}.{minor}.{patch}",
                "\u{661}.2.3",
                BumpType::Major,
                at(2026, 8, 5)
            ),
            "1.0.0"
        );
    }

    #[test]
    fn a_component_at_u64_max_that_is_not_incremented_still_renders() {
        let now = at(2026, 8, 5);
        let template = "{major}.{minor}.{patch}";
        assert_eq!(
            render_at(template, &format!("1.2.{MAX}"), BumpType::Minor, now),
            "1.3.0"
        );
        assert_eq!(
            render_at(template, &format!("1.{MAX}.{MAX}"), BumpType::Major, now),
            "2.0.0"
        );
        assert_eq!(
            render_at(template, &format!("{MAX}.1.1"), BumpType::Patch, now),
            format!("{MAX}.1.2")
        );
        assert_eq!(
            render_at(template, &format!("{MAX}.{MAX}.{MAX}"), BumpType::None, now),
            format!("{MAX}.{MAX}.{MAX}")
        );
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
    fn week_templates_use_the_iso_year_at_new_year() {
        let template = "{year}.{week}.{seq}";
        assert_eq!(
            render_at(template, "0.0.0", BumpType::None, at(2025, 12, 29)),
            "2026.1.1"
        );
        assert_eq!(
            render_at(template, "0.0.0", BumpType::None, at(2027, 1, 1)),
            "2026.53.1"
        );
        assert_eq!(
            render_at(
                "{short_year}.{padded_week}",
                "0.0",
                BumpType::None,
                at(2027, 1, 1)
            ),
            "26.53"
        );
        assert_eq!(
            render_at(template, "0.0.0", BumpType::None, at(2026, 8, 5)),
            "2026.32.1"
        );
    }

    #[test]
    fn week_templates_keep_seq_ordering_across_new_year() {
        let template = "{year}.{week}.{seq}";
        assert_eq!(
            render_at(template, "2026.1.1", BumpType::None, at(2025, 12, 30)),
            "2026.1.2"
        );
        assert_eq!(
            render_at(template, "2026.53.2", BumpType::None, at(2027, 1, 2)),
            "2026.53.3"
        );
        assert_eq!(
            render_at(template, "2026.53.2", BumpType::None, at(2027, 1, 4)),
            "2027.1.1"
        );
        assert_eq!(
            render_at(template, "2026.32.4", BumpType::None, at(2026, 8, 6)),
            "2026.32.5"
        );
    }

    #[test]
    fn seq_increments_within_a_period_for_padded_variables() {
        for (template, current, now, expected) in [
            (
                "{year}.{padded_week}.{seq}",
                "2001.01.1",
                at(2001, 1, 2),
                "2001.01.2",
            ),
            (
                "{short_year}.{week}.{seq}",
                "01.1.1",
                at(2001, 1, 2),
                "01.1.2",
            ),
            (
                "{year}.{padded_month}.{seq}",
                "2026.01.1",
                at(2026, 1, 20),
                "2026.01.2",
            ),
            (
                "{year}.{month}.{padded_day}.{seq}",
                "2026.8.05.3",
                at(2026, 8, 5),
                "2026.8.05.4",
            ),
        ] {
            assert_eq!(
                render_at(template, current, BumpType::None, now),
                expected,
                "{template} {current}"
            );
        }
    }

    #[test]
    fn seq_resets_when_a_padded_variable_rolls() {
        assert_eq!(
            render_at(
                "{year}.{padded_week}.{seq}",
                "2026.09.4",
                BumpType::None,
                at(2026, 3, 4)
            ),
            "2026.10.1"
        );
    }

    #[test]
    fn ordering_warning_fires_for_every_week_and_calendar_unit_pair() {
        for week in ["{week}", "{padded_week}"] {
            for unit in [
                "{month}",
                "{padded_month}",
                "{day}",
                "{padded_day}",
                "{quarter}",
            ] {
                let template = format!("{{year}}.{unit}.{week}.{{seq}}");
                assert!(ordering_warning(&template).is_some(), "{template}");
            }
        }
    }

    #[test]
    fn ordering_warning_is_silent_for_week_only_and_calendar_only_templates() {
        for template in [
            "{year}.{week}.{seq}",
            "{short_year}.{padded_week}",
            "{year}.{month}.{day}",
            "{year}.{quarter}.{seq}",
            "{major}.{minor}.{patch}",
            "{year}.{week}.{patch}",
        ] {
            assert!(ordering_warning(template).is_none(), "{template}");
        }
    }

    #[test]
    fn ordering_warning_ignores_an_invalid_template() {
        assert!(ordering_warning("{week}{month}").is_none());
    }

    #[test]
    fn templates_without_a_week_keep_the_calendar_year() {
        assert_eq!(
            render_at(
                "{year}.{month}.{seq}",
                "2025.12.1",
                BumpType::None,
                at(2025, 12, 29)
            ),
            "2025.12.2"
        );
        assert_eq!(
            render_at(
                "{year}.{month}.{day}",
                "0.0.0",
                BumpType::None,
                at(2027, 1, 1)
            ),
            "2027.1.1"
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

    #[test]
    fn a_control_character_in_a_literal_is_refused() {
        for template in ["\t{patch}.21", "{major}.{minor}\u{0}", "{year}\n.{seq}"] {
            let err = validate(template).unwrap_err().to_string();
            assert!(err.contains("a control character"), "{template:?}: {err}");
        }
    }

    #[test]
    fn whitespace_in_a_literal_is_refused() {
        let err = validate(" {major}.{minor}.{patch}")
            .unwrap_err()
            .to_string();
        assert!(err.contains("whitespace"), "{err}");
        assert!(
            err.contains("git tag"),
            "the message must say why it cannot be there: {err}"
        );
    }

    #[test]
    fn a_character_git_reserves_in_a_ref_is_refused() {
        for template in ["{major}:{minor}", "{major}^{minor}", "{major}?{minor}"] {
            let err = validate(template).unwrap_err().to_string();
            assert!(err.contains("git reserves"), "{template:?}: {err}");
        }
    }

    #[test]
    fn the_message_names_the_offending_character() {
        let err = validate("{major}.{minor}\t").unwrap_err().to_string();
        assert!(err.contains(r"'\t'"), "{err}");
    }

    #[test]
    fn the_templates_people_actually_write_still_parse() {
        for template in [
            "v{major}.{minor}.{patch}",
            "{year}.{padded_month}.{seq}",
            "{major}.{minor}.{patch}-rc{seq}",
            "{major}.{minor}.{patch}+build",
            "{short_year}.{quarter}_{seq}",
        ] {
            assert!(validate(template).is_ok(), "{template:?} must stay valid");
        }
    }

    #[test]
    fn a_validated_template_cannot_render_a_version_a_tag_would_refuse() {
        let rendered = render("1.2.3", BumpType::Patch, "{major}.{minor}.{patch}").unwrap();
        assert_eq!(rendered, "1.2.4");
        assert!(
            !rendered
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
        );
    }

    #[test]
    fn consecutive_dots_in_a_literal_are_refused() {
        let err = validate("{major}..{minor}").unwrap_err().to_string();
        assert!(err.contains("two consecutive dots"), "{err}");
    }

    #[test]
    fn a_template_ending_in_a_dot_is_refused() {
        let err = validate("{major}.{minor}.").unwrap_err().to_string();
        assert!(err.contains("ends with a dot"), "{err}");
    }

    #[test]
    fn a_template_ending_in_dot_lock_is_refused() {
        let err = validate("{major}.{minor}.lock").unwrap_err().to_string();
        assert!(err.contains(".lock"), "{err}");
    }

    #[test]
    fn a_dot_that_is_not_last_and_not_doubled_is_fine() {
        assert!(validate("{major}.{minor}.{patch}").is_ok());
        assert!(validate("v{major}.{minor}.{patch}-rc{seq}").is_ok());
        assert!(validate("{year}.{padded_month}.{seq}").is_ok());
    }

    #[test]
    fn a_v_in_the_template_reads_the_previous_version_back() {
        assert_eq!(
            render("v1.2.3", BumpType::Patch, "v{major}.{minor}.{patch}").unwrap(),
            "v1.2.4"
        );
        assert_eq!(
            render("v1.2.3", BumpType::Minor, "v{major}.{minor}.{patch}").unwrap(),
            "v1.3.0"
        );
        assert_eq!(
            render("v1.2.3", BumpType::Major, "v{major}.{minor}.{patch}").unwrap(),
            "v2.0.0"
        );
    }

    #[test]
    fn a_v_template_advances_release_after_release() {
        let mut current = "v1.0.0".to_string();
        for expected in ["v1.0.1", "v1.0.2", "v1.0.3"] {
            current = render(&current, BumpType::Patch, "v{major}.{minor}.{patch}").unwrap();
            assert_eq!(current, expected, "a v-prefixed template must not reset");
        }
    }

    #[test]
    fn a_v_template_rolls_its_seq_like_a_bare_one() {
        let now = at(2026, 8, 20);
        assert_eq!(
            render_at("v{year}.{month}.{seq}", "v2026.8.3", BumpType::None, now),
            "v2026.8.4"
        );
    }

    #[test]
    fn a_bare_current_version_reads_back_into_a_v_template() {
        assert_eq!(
            render("1.2.3", BumpType::Patch, "v{major}.{minor}.{patch}").unwrap(),
            "v1.2.4",
            "a version file holding a bare version must not cost one reset tag"
        );
    }

    #[test]
    fn a_bare_current_version_reads_back_into_a_v_seq_template() {
        let now = at(2026, 8, 20);
        assert_eq!(
            render_at("v{year}.{month}.{seq}", "2026.8.3", BumpType::None, now),
            "v2026.8.4"
        );
    }
}
