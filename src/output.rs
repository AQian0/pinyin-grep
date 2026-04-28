//! Output rendering.
//!
//! Two stable formats are supported:
//!
//! * **NDJSON** (machine / AI friendly): one JSON object per line, schema
//!   fixed by serde derives. Line/column are 0-based — matching ast-grep's
//!   own JSON output — and nested under `range.start` / `range.end`.
//!
//! * **Text** (human friendly): one finding per line plus an indented
//!   syllable line, columns 1-based, syllables separated by `·`.

use std::io::{self, Write};
use std::path::PathBuf;

use serde::Serialize;

use crate::score::{Confidence, TokenAnalysis};

/// Zero-based line/column position. Mirrors ast-grep CLI's JSON format.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// Closed-open range, lines and columns 0-based.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// One pinyin-grep result. The schema is stable across versions: fields
/// are added in a backwards-compatible way and ordering is fixed.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    pub identifier: String,
    pub tokens: Vec<TokenAnalysis>,
    pub score: i32,
    pub confidence: Confidence,
    pub ambiguous: bool,
}

/// Output flavour selected by the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Ndjson,
    Text,
}

/// Writes every finding to `out` in the chosen format. Returns once all
/// findings are written; flushes the underlying writer on completion.
pub fn write_findings<W: Write>(
    out: &mut W,
    findings: &[Finding],
    format: Format,
) -> io::Result<()> {
    match format {
        Format::Ndjson => write_ndjson(out, findings),
        Format::Text => write_text(out, findings),
    }
}

fn write_ndjson<W: Write>(out: &mut W, findings: &[Finding]) -> io::Result<()> {
    for f in findings {
        let line = serde_json::to_string(f).map_err(io::Error::other)?;
        writeln!(out, "{line}")?;
    }
    out.flush()
}

fn write_text<W: Write>(out: &mut W, findings: &[Finding]) -> io::Result<()> {
    for f in findings {
        // Header line: file:line:col<TAB>confidence<TAB>identifier
        let location = match (&f.file, f.range) {
            (Some(path), Some(r)) => format!(
                "{}:{}:{}",
                path.display(),
                r.start.line + 1,
                r.start.column + 1
            ),
            _ => "<stdin>".to_string(),
        };
        writeln!(
            out,
            "{location}\t{}\t{}",
            f.confidence.as_str(),
            f.identifier
        )?;

        // Syllable line(s).
        let primary = preferred_syllables(&f.tokens);
        let count = primary.split('·').filter(|s| !s.is_empty()).count();
        let amb_marker = if f.ambiguous { " (ambiguous)" } else { "" };
        writeln!(out, "  └─ {primary}  ({count} syllables{amb_marker})")?;

        // Alternative segmentations get a compact second line.
        let alts = alternative_segmentations(&f.tokens);
        if !alts.is_empty() {
            writeln!(out, "     alt: {}", alts.join(" | "))?;
        }
    }
    out.flush()
}

/// Joins the preferred segmentation per token with `·`, falling back to
/// the raw token text when segmentation failed.
fn preferred_syllables(tokens: &[TokenAnalysis]) -> String {
    let mut parts = Vec::new();
    for tok in tokens {
        if let Some(seg) = tok.syllables.first() {
            for s in seg {
                parts.push(s.clone());
            }
        } else {
            parts.push(tok.text.clone());
        }
    }
    parts.join("·")
}

/// Returns at most three alternative segmentations (skipping the preferred
/// one) for tokens with multiple candidates.
fn alternative_segmentations(tokens: &[TokenAnalysis]) -> Vec<String> {
    let mut alts: Vec<String> = Vec::new();
    for tok in tokens {
        if tok.syllables.len() <= 1 {
            continue;
        }
        // Skip the first (preferred) candidate, take up to 3 others.
        for seg in tok.syllables.iter().skip(1).take(3) {
            alts.push(format!("{}={}", tok.text, seg.join("·")));
        }
    }
    alts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::analyze;

    fn finding_for(ident: &str) -> Finding {
        let a = analyze(ident).unwrap();
        Finding {
            file: Some(PathBuf::from("src/foo.ts")),
            range: Some(Range {
                start: Position { line: 10, column: 6 },
                end: Position { line: 10, column: 6 + ident.len() },
            }),
            identifier: a.identifier,
            tokens: a.tokens,
            score: a.score,
            confidence: a.confidence,
            ambiguous: a.ambiguous,
        }
    }

    #[test]
    fn ndjson_emits_one_line_per_finding() {
        let findings = vec![finding_for("huoQuYongHuXinXi"), finding_for("huoQu")];
        let mut buf = Vec::new();
        write_findings(&mut buf, &findings, Format::Ndjson).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert_eq!(text.lines().count(), 2);
        for line in text.lines() {
            assert!(line.starts_with('{'));
            assert!(line.ends_with('}'));
        }
    }

    #[test]
    fn ndjson_uses_zero_based_columns() {
        let findings = vec![finding_for("huoQu")];
        let mut buf = Vec::new();
        write_findings(&mut buf, &findings, Format::Ndjson).unwrap();
        let text = String::from_utf8(buf).unwrap();
        // Source position { line: 10, column: 6 } stays as-is.
        assert!(text.contains("\"line\":10"));
        assert!(text.contains("\"column\":6"));
    }

    #[test]
    fn text_uses_one_based_columns() {
        let findings = vec![finding_for("huoQu")];
        let mut buf = Vec::new();
        write_findings(&mut buf, &findings, Format::Text).unwrap();
        let text = String::from_utf8(buf).unwrap();
        // 0-based 10:6 should display as 11:7 in text mode.
        assert!(text.lines().next().unwrap().contains(":11:7\t"));
    }

    #[test]
    fn text_renders_dot_separated_syllables() {
        let findings = vec![finding_for("huoQuXinXi")];
        let mut buf = Vec::new();
        write_findings(&mut buf, &findings, Format::Text).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("huo·qu·xin·xi"));
        assert!(text.contains("(4 syllables"));
    }
}
