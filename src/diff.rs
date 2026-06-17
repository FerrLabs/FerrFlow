const CONTEXT: usize = 1;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Op {
    Equal,
    Delete,
    Insert,
}

fn lcs_ops<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<(Op, &'a str)> {
    let n = old.len();
    let m = new.len();
    let mut table = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i][j] = if old[i] == new[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }

    let mut ops = Vec::with_capacity(n + m);
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if old[i] == new[j] {
            ops.push((Op::Equal, old[i]));
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            ops.push((Op::Delete, old[i]));
            i += 1;
        } else {
            ops.push((Op::Insert, new[j]));
            j += 1;
        }
    }
    while i < n {
        ops.push((Op::Delete, old[i]));
        i += 1;
    }
    while j < m {
        ops.push((Op::Insert, new[j]));
        j += 1;
    }
    ops
}

struct Hunk {
    old_start: usize,
    old_len: usize,
    new_start: usize,
    new_len: usize,
    lines: Vec<String>,
}

fn group_hunks(ops: &[(Op, &str)]) -> Vec<Hunk> {
    let change_idx: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter(|(_, (op, _))| *op != Op::Equal)
        .map(|(i, _)| i)
        .collect();
    if change_idx.is_empty() {
        return Vec::new();
    }

    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for &idx in &change_idx {
        let lo = idx.saturating_sub(CONTEXT);
        let hi = (idx + CONTEXT).min(ops.len() - 1);
        match ranges.last_mut() {
            Some(last) if lo <= last.1 + 1 => last.1 = last.1.max(hi),
            _ => ranges.push((lo, hi)),
        }
    }

    let mut hunks = Vec::with_capacity(ranges.len());
    for (lo, hi) in ranges {
        let old_start = 1 + ops[..lo].iter().filter(|(op, _)| *op != Op::Insert).count();
        let new_start = 1 + ops[..lo].iter().filter(|(op, _)| *op != Op::Delete).count();
        let (mut old_len, mut new_len) = (0usize, 0usize);
        let mut lines = Vec::new();
        for (op, text) in &ops[lo..=hi] {
            match op {
                Op::Equal => {
                    lines.push(format!("   {text}"));
                    old_len += 1;
                    new_len += 1;
                }
                Op::Delete => {
                    lines.push(format!("-  {text}"));
                    old_len += 1;
                }
                Op::Insert => {
                    lines.push(format!("+  {text}"));
                    new_len += 1;
                }
            }
        }
        hunks.push(Hunk {
            old_start,
            old_len,
            new_start,
            new_len,
            lines,
        });
    }
    hunks
}

pub fn unified_diff(old: &str, new: &str) -> String {
    if old == new {
        return String::new();
    }
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let ops = lcs_ops(&old_lines, &new_lines);
    let hunks = group_hunks(&ops);

    let mut out = String::new();
    for hunk in &hunks {
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk.old_start, hunk.old_len, hunk.new_start, hunk.new_len
        ));
        for line in &hunk.lines {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_change_yields_empty() {
        let s = "a\nb\nc\n";
        assert_eq!(unified_diff(s, s), "");
    }

    #[test]
    fn single_line_change_one_hunk() {
        let old = "  \"name\": \"api\",\n  \"version\": \"1.2.3\",\n  \"private\": true\n";
        let new = "  \"name\": \"api\",\n  \"version\": \"1.3.0\",\n  \"private\": true\n";
        let diff = unified_diff(old, new);
        let hunk_count = diff.matches("@@ ").count();
        assert_eq!(hunk_count, 1, "expected exactly one hunk:\n{diff}");
        assert!(diff.contains("-    \"version\": \"1.2.3\","));
        assert!(diff.contains("+    \"version\": \"1.3.0\","));
        assert!(diff.contains("     \"name\": \"api\","));
    }

    #[test]
    fn multi_line_change() {
        let old = "line1\nold-a\nold-b\nline4\n";
        let new = "line1\nnew-a\nnew-b\nline4\n";
        let diff = unified_diff(old, new);
        assert!(diff.contains("-  old-a"));
        assert!(diff.contains("-  old-b"));
        assert!(diff.contains("+  new-a"));
        assert!(diff.contains("+  new-b"));
        assert!(diff.contains("@@ -"));
    }

    #[test]
    fn pure_insertion() {
        let old = "a\nb\n";
        let new = "a\nb\nc\n";
        let diff = unified_diff(old, new);
        assert!(diff.contains("+  c"));
        assert!(!diff.contains("-  "));
    }

    #[test]
    fn pure_deletion() {
        let old = "a\nb\nc\n";
        let new = "a\nc\n";
        let diff = unified_diff(old, new);
        assert!(diff.contains("-  b"));
    }

    #[test]
    fn separate_changes_yield_separate_hunks() {
        let old = "a\nb\nc\nd\ne\nf\ng\nh\ni\n";
        let new = "a\nB\nc\nd\ne\nf\ng\nH\ni\n";
        let diff = unified_diff(old, new);
        let hunk_count = diff.matches("@@ ").count();
        assert_eq!(hunk_count, 2, "expected two hunks:\n{diff}");
    }

    #[test]
    fn hunk_header_offsets_are_one_based() {
        let old = "first\nsecond\nthird\n";
        let new = "first\nSECOND\nthird\n";
        let diff = unified_diff(old, new);
        assert!(
            diff.starts_with("@@ -1,3 +1,3 @@\n"),
            "unexpected header in:\n{diff}"
        );
    }
}
