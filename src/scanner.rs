//! File traversal + ast-grep matching.
//!
//! [`scan`] walks every requested path with [`ignore::WalkBuilder`]
//! (so `.gitignore` and friends are respected by default), runs the
//! configured AST patterns on each supported file and produces a
//! [`Finding`] for every identifier that turns out to be Pinyin.
//!
//! File processing is parallelised via [`rayon`] so that large
//! directory trees benefit from multiple cores.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use ast_grep_language::{LanguageExt, SupportLang};
use ignore::WalkBuilder;
use rayon::prelude::*;
use regex::Regex;

use crate::lang;
use crate::output::{Finding, Position, Range};
use crate::patterns::{PatternSpec, default_patterns};
use crate::score::analyze;

/// Configuration for a single [`scan`] invocation.
pub struct ScanOptions<'a> {
    pub paths: &'a [PathBuf],
    /// Forces every file to be parsed as `lang`. When `None`, the language
    /// is inferred from the file extension.
    pub forced_lang: Option<SupportLang>,
    /// User-supplied patterns. When empty, the language's built-in
    /// defaults are used instead.
    pub user_patterns: Vec<PatternSpec>,
    /// Identifier-text regexes to skip.
    pub ignore_regexes: Vec<Regex>,
}

/// Walks `opts.paths` and returns every Finding produced by the AST + Pinyin
/// pipeline. Errors reading individual files are silently skipped — the
/// scan is best-effort, like ripgrep / ast-grep CLI.
pub fn scan(opts: &ScanOptions) -> Vec<Finding> {
    // ── Phase 1: collect every supported file (serial, IO-bound) ──────
    let mut files: Vec<(PathBuf, SupportLang)> = Vec::new();

    for root in opts.paths {
        let walker = WalkBuilder::new(root).build();
        for entry in walker {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let path = entry.path();
            let Some(detected_lang) = opts.forced_lang.or_else(|| lang::from_path(path)) else {
                continue;
            };
            files.push((path.to_path_buf(), detected_lang));
        }
    }

    // ── Phase 2: process files in parallel (CPU-bound) ───────────────
    let user = &opts.user_patterns;
    let ignore_regexes = &opts.ignore_regexes;

    files
        .par_iter()
        .flat_map(|(path, lang)| process_file(path, *lang, user, ignore_regexes))
        .collect()
}

/// Process a single file: parse, match patterns, analyse identifiers.
/// Returns zero or more [`Finding`]s for the file.
fn process_file(
    path: &Path,
    lang: SupportLang,
    user_patterns: &[PatternSpec],
    ignore_regexes: &[Regex],
) -> Vec<Finding> {
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let patterns: &[PatternSpec] = if user_patterns.is_empty() {
        default_patterns(lang)
    } else {
        user_patterns
    };
    if patterns.is_empty() {
        return Vec::new();
    }

    let mut findings = Vec::new();
    let mut seen: HashSet<(usize, usize)> = HashSet::new();

    let ast = lang.ast_grep(&src);
    let root_node = ast.root();

    for pat in patterns {
        for nm in root_node.find_all(pat.source) {
            let env = nm.get_env();
            let Some(node) = env.get_match(pat.meta_var) else {
                continue;
            };
            let identifier = node.text().to_string();

            if ignore_regexes.iter().any(|r| r.is_match(&identifier)) {
                continue;
            }

            let start = node.start_pos();
            let end = node.end_pos();
            let key = (start.line(), start.column(node));
            if !seen.insert(key) {
                continue;
            }

            if let Some(analysis) = analyze(&identifier) {
                findings.push(Finding {
                    file: Some(path.to_path_buf()),
                    range: Some(Range {
                        start: Position {
                            line: start.line(),
                            column: start.column(node),
                        },
                        end: Position {
                            line: end.line(),
                            column: end.column(node),
                        },
                    }),
                    identifier: analysis.identifier,
                    tokens: analysis.tokens,
                    score: analysis.score,
                    confidence: analysis.confidence,
                    ambiguous: analysis.ambiguous,
                });
            }
        }
    }
    findings
}

/// Convenience for the `--names` CLI mode: runs [`analyze`] over every
/// non-empty line of `lines` and produces matching Findings.
pub fn scan_names(lines: &[String]) -> Vec<Finding> {
    lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| analyze(l.trim()))
        .map(|a| Finding {
            file: None,
            range: None,
            identifier: a.identifier,
            tokens: a.tokens,
            score: a.score,
            confidence: a.confidence,
            ambiguous: a.ambiguous,
        })
        .collect()
}
