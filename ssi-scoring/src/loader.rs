//! JSONL corpus loader for the trusted/grader side.
//!
//! The corpus is `patterns.jsonl`: one JSON object per line, each a full
//! symmetrized CSC sparsity pattern. `pattern_from_jsonl_line` is the SINGLE
//! definition of "corpus line -> Pattern"; both the harness and the grader
//! parse through it, so there is no second parser that can silently disagree
//! (Invariant 2 at the parsing boundary).
//!
//! VALUES NEVER ENTER. The corpus format is pattern-only — a line carries `n`,
//! `indptr`, and `indices`, with no value column anywhere — so NARROW INPUT
//! (proposal §3.1) holds by construction: there is nothing for the loader to
//! drop. The diagonal IS present in the stored CSC and is stripped here to
//! match the contract's `Pattern` (off-diagonal both-triangle storage).

use crate::pattern::Pattern;
use std::fmt;
use std::path::Path;

/// Failure to parse a JSONL corpus line into a `Pattern`.
#[derive(Debug)]
pub enum LoadError {
    /// A `patterns.jsonl` line was malformed or structurally invalid.
    Json(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Json(m) => write!(f, "failed to parse JSONL pattern: {m}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// Parse ONE line of `patterns.jsonl` into `(source, Pattern)`.
///
/// The pipeline (`corpus-generation`) emits a full *symmetrized* CSC pattern
/// that INCLUDES the diagonal (column `j`'s indices contain `j`). The contract
/// `Pattern` is off-diagonal, both-triangle, so we drop `i == j` and rebuild
/// via `Pattern::from_adjacency` (which sorts, dedups, and validates symmetry).
///
/// This is the SINGLE definition of "corpus line -> Pattern". The harness loads
/// the whole corpus through it; a future grader can address one line by index
/// and route it through this same core (Invariant 2 at the parsing boundary).
pub fn pattern_from_jsonl_line(line: &str) -> Result<(String, Pattern), LoadError> {
    let err = |m: &str| LoadError::Json(format!("{m} in line: {}", truncate(line, 80)));

    let n = parse_usize_field(line, "\"n\"").ok_or_else(|| err("missing/invalid \"n\""))?;
    let indptr = parse_int_array(line, "\"indptr\"").ok_or_else(|| err("missing \"indptr\""))?;
    let indices = parse_int_array(line, "\"indices\"").ok_or_else(|| err("missing \"indices\""))?;
    let source = parse_string_field(line, "\"source\"").unwrap_or_default();

    if indptr.len() != n + 1 {
        return Err(err("indptr length != n+1"));
    }
    if indptr.first() != Some(&0) {
        return Err(err("indptr[0] != 0"));
    }
    if indptr.last().copied() != Some(indices.len()) {
        return Err(err("indptr[n] != indices.len()"));
    }

    // Expand CSC -> per-column adjacency, dropping the diagonal.
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for j in 0..n {
        let (lo, hi) = (indptr[j], indptr[j + 1]);
        if lo > hi || hi > indices.len() {
            return Err(err("non-monotone indptr"));
        }
        for &i in &indices[lo..hi] {
            if i >= n {
                return Err(err("row index out of range"));
            }
            if i != j {
                adj[j].push(i);
            }
        }
    }
    let pattern = Pattern::from_adjacency(n, &mut adj);
    if let Err(reason) = pattern_invariants(&pattern) {
        return Err(err(reason));
    }
    Ok((source, pattern))
}

/// Constructors document symmetry and sorted columns. `from_adjacency` sorts
/// and dedups but does not check that `(i, j)` exists whenever `(j, i)` does,
/// so a one-triangle corpus line used to become a silently asymmetric `Pattern`.
fn pattern_invariants(p: &Pattern) -> Result<(), &'static str> {
    if p.col_ptr.len() != p.n + 1 || p.col_ptr.first() != Some(&0) {
        return Err("broken column pointers");
    }
    for j in 0..p.n {
        let (lo, hi) = (p.col_ptr[j], p.col_ptr[j + 1]);
        if lo > hi || hi > p.row_idx.len() {
            return Err("broken column pointers");
        }
        let col = &p.row_idx[lo..hi];
        for w in col.windows(2) {
            if w[0] >= w[1] {
                return Err("column indices are not strictly increasing");
            }
        }
        for &i in col {
            if i >= p.n || i == j {
                return Err("row index out of range");
            }
            let other = &p.row_idx[p.col_ptr[i]..p.col_ptr[i + 1]];
            if other.binary_search(&j).is_err() {
                return Err("pattern is not symmetric");
            }
        }
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let end = s.floor_char_boundary(max);
        format!("{}…", &s[..end])
    }
}

/// Index just after a JSON key, where the value (after optional whitespace and
/// `:`) begins. A bare `find` is wrong: `"n"` is a substring of a source value
/// `"n"`, and `"indices"` is a substring of a source value `"indices"`.
fn find_json_key(line: &str, key: &str) -> Option<usize> {
    let mut from = 0;
    while from < line.len() {
        let rel = line[from..].find(key)?;
        let abs = from + rel;
        let after = abs + key.len();
        let Some(rest) = line.get(after..) else {
            return None;
        };
        let preceded_by_separator = line[..abs]
            .trim_end()
            .chars()
            .next_back()
            .is_some_and(|c| c == '{' || c == ',');
        if preceded_by_separator && rest.trim_start().starts_with(':') {
            return Some(after);
        }
        from = abs + 1;
    }
    None
}

fn value_after_key<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let after = find_json_key(line, key)?;
    let rest = line.get(after..)?.trim_start();
    rest.strip_prefix(':').map(str::trim_start)
}

/// Find `"key":<digits>` and parse the integer. Returns None if absent/invalid.
fn parse_usize_field(line: &str, key: &str) -> Option<usize> {
    let rest = value_after_key(line, key)?;
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    rest[..end].parse().ok()
}

/// Find `"key":"value"` and return the string value. Handles the escapes a
/// corpus source name can carry (`\"`, `\\`, and the single-character forms).
fn parse_string_field(line: &str, key: &str) -> Option<String> {
    let rest = value_after_key(line, key)?;
    parse_json_string(rest)
}

fn parse_json_string(rest: &str) -> Option<String> {
    let rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                other => out.push(other),
            },
            other => out.push(other),
        }
    }
    None
}

/// Find `"key":[<comma-separated ints>]` and parse into a Vec<usize>.
fn parse_int_array(line: &str, key: &str) -> Option<Vec<usize>> {
    let rest = value_after_key(line, key)?;
    let rest = rest.strip_prefix('[')?;
    let end = rest.find(']')?;
    let body = &rest[..end];
    if body.trim().is_empty() {
        return Some(Vec::new());
    }
    body.split(',')
        .map(|t| t.trim().parse::<usize>().ok())
        .collect()
}

/// Load an entire `patterns.jsonl` corpus into `(source, Pattern)` pairs, in
/// file order. Blank lines are skipped. Used by the local harness.
pub fn load_corpus_jsonl(path: &Path) -> Result<Vec<(String, Pattern)>, LoadError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| LoadError::Json(format!("{}: {e}", path.display())))?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let (source, pat) = pattern_from_jsonl_line(line)
            .map_err(|e| LoadError::Json(format!("{}:{}: {e}", path.display(), i)))?;
        out.push((source, pat));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonl_line_parses_and_drops_diagonal() {
        // n=4 line from the real corpus (source st_e09). indices include the
        // diagonal (each column j lists j); the contract Pattern must drop it.
        let line = r#"{"n":4,"nnz":12,"indptr":[0,3,6,8,12],"indices":[0,1,3,0,1,3,2,3,0,1,2,3],"hash":"9cce0c0e","source":"st_e09"}"#;
        let (source, p) = pattern_from_jsonl_line(line).unwrap();
        assert_eq!(source, "st_e09");
        assert_eq!(p.n, 4);
        // Column 0 raw = {0,1,3}; diagonal 0 dropped -> {1,3}.
        assert_eq!(p.col(0), &[1, 3]);
        // Column 2 raw = {2,3}; diagonal 2 dropped -> {3}.
        assert_eq!(p.col(2), &[3]);
        // Column 3 raw = {0,1,2,3}; diagonal 3 dropped -> {0,1,2}.
        assert_eq!(p.col(3), &[0, 1, 2]);
        // 12 raw entries - 4 diagonal = 8 off-diagonal.
        assert_eq!(p.nnz(), 8);
    }

    #[test]
    fn jsonl_line_rejects_malformed() {
        assert!(pattern_from_jsonl_line("not json").is_err());
        assert!(pattern_from_jsonl_line(r#"{"n":2}"#).is_err());
    }

    #[test]
    fn load_corpus_parses_all_lines() {
        let jsonl = "\
{\"n\":2,\"nnz\":4,\"indptr\":[0,2,4],\"indices\":[0,1,0,1],\"hash\":\"a\",\"source\":\"m0\"}
{\"n\":4,\"nnz\":12,\"indptr\":[0,3,6,8,12],\"indices\":[0,1,3,0,1,3,2,3,0,1,2,3],\"hash\":\"b\",\"source\":\"m1\"}
";
        let dir = std::env::temp_dir().join("ssi-scoring-jsonl-io-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("patterns.jsonl");
        std::fs::write(&path, jsonl).unwrap();

        let corpus = load_corpus_jsonl(&path).unwrap();
        assert_eq!(corpus.len(), 2);
        assert_eq!(corpus[0].0, "m0");
        assert_eq!(corpus[1].0, "m1");
        assert_eq!(corpus[1].1.n, 4);
    }

    #[test]
    fn source_value_that_matches_a_key_does_not_steal_the_field() {
        // A first-match scan of `"n"` or `"indices"` used to bind the source
        // value and then reject a well-formed line.
        let line =
            r#"{"source":"indices","n":2,"nnz":4,"indptr":[0,2,4],"indices":[0,1,0,1],"hash":"n"}"#;
        let (source, p) = pattern_from_jsonl_line(line).unwrap();
        assert_eq!(source, "indices");
        assert_eq!(p.n, 2);
        assert_eq!(p.col(0), &[1]);
        assert_eq!(p.col(1), &[0]);
    }

    #[test]
    fn whitespace_around_keys_is_accepted() {
        let line =
            r#"{ "n" : 2, "indptr" : [0, 2, 4], "indices" : [0, 1, 0, 1], "source" : "spaced" }"#;
        let (source, p) = pattern_from_jsonl_line(line).unwrap();
        assert_eq!(source, "spaced");
        assert_eq!(p.nnz(), 2);
    }

    #[test]
    fn escaped_quote_in_source_is_preserved() {
        let line = r#"{"source":"a\"b","n":2,"indptr":[0,2,4],"indices":[0,1,0,1]}"#;
        let (source, p) = pattern_from_jsonl_line(line).unwrap();
        assert_eq!(source, "a\"b");
        assert_eq!(p.n, 2);
    }

    #[test]
    fn asymmetric_pattern_is_rejected() {
        // Column 0 lists row 1, but column 1 does not list row 0.
        let line = r#"{"n":2,"indptr":[0,1,1],"indices":[1],"source":"asym"}"#;
        let err = pattern_from_jsonl_line(line).unwrap_err();
        assert!(err.to_string().contains("not symmetric"), "{err}");
    }

    #[test]
    fn malformed_multibyte_line_reports_an_error_instead_of_panicking() {
        let line = format!("{} not-json", "é".repeat(40));
        let err = pattern_from_jsonl_line(&line).unwrap_err();
        let rendered = err.to_string();
        assert!(rendered.contains("failed to parse"));
        assert!(rendered.contains('…'));
    }
}
