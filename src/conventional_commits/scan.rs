const CONVENTIONAL_TYPES: [&str; 10] = [
    "feat", "fix", "refactor", "perf", "build", "chore", "docs", "style", "test", "ci",
];

const FOOTER_LEN: usize = "BREAKING CHANGE: ".len();

fn strip_conventional_type(subject: &str) -> Option<&str> {
    CONVENTIONAL_TYPES
        .iter()
        .find_map(|ty| subject.strip_prefix(ty))
}

fn split_scope(rest: &str) -> Option<(&str, &str)> {
    let inner = rest.strip_prefix('(')?;
    let close = inner.find(['(', ')'])?;
    if inner.as_bytes()[close] != b')' {
        return None;
    }
    Some((&inner[..close], &inner[close + 1..]))
}

pub(super) fn is_breaking_header(subject: &str) -> bool {
    let Some(rest) = strip_conventional_type(subject) else {
        return false;
    };
    if rest.starts_with("!:") {
        return true;
    }
    rest.starts_with('(')
        && rest
            .as_bytes()
            .get(2..)
            .is_some_and(|tail| tail.windows(3).any(|w| w == b")!:"))
}

pub(super) fn is_scope_bang_header(subject: &str) -> bool {
    let Some(rest) = strip_conventional_type(subject) else {
        return false;
    };
    split_scope(rest).is_some_and(|(scope, after)| scope.ends_with('!') && after.starts_with(':'))
}

pub(super) fn is_refactor_header(subject: &str) -> bool {
    let Some(head) = subject.as_bytes().get(..8) else {
        return false;
    };
    if !head.eq_ignore_ascii_case(b"refactor") {
        return false;
    }
    let rest = &subject[8..];
    let after_scope = split_scope(rest).map_or(rest, |(_, after)| after);
    after_scope.starts_with([':', '/'])
}

fn footer_prefix_len(line: &str) -> Option<usize> {
    let b = line.as_bytes();
    let matches = b.len() >= FOOTER_LEN
        && b[..8].eq_ignore_ascii_case(b"BREAKING")
        && matches!(b[8], b' ' | b'-')
        && b[9..15].eq_ignore_ascii_case(b"CHANGE")
        && &b[15..FOOTER_LEN] == b": ";
    matches.then_some(FOOTER_LEN)
}

pub(super) fn breaking_footer_end(message: &str) -> Option<usize> {
    let mut start = 0;
    for line in message.split('\n') {
        if let Some(len) = footer_prefix_len(line) {
            return Some(start + len);
        }
        start += line.len() + 1;
    }
    None
}

pub(super) struct RawHeader<'a> {
    pub commit_type: &'a str,
    pub scope: Option<&'a str>,
    pub bang: bool,
    pub description: &'a str,
}

pub(super) fn parse_raw_header(subject: &str) -> Option<RawHeader<'_>> {
    let type_len = subject
        .bytes()
        .take_while(|b| b.is_ascii_lowercase())
        .count();
    if type_len == 0 {
        return None;
    }
    let (commit_type, mut rest) = subject.split_at(type_len);
    let mut scope = None;
    if rest.starts_with('(') {
        let (inner, after) = split_scope(rest)?;
        if inner.is_empty() {
            return None;
        }
        scope = Some(inner);
        rest = after;
    }
    let bang = rest.starts_with('!');
    let rest = rest.strip_prefix('!').unwrap_or(rest).strip_prefix(':')?;
    Some(RawHeader {
        commit_type,
        scope,
        bang,
        description: rest.trim_start(),
    })
}
