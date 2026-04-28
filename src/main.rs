//! `pinyin-grep` CLI entry point.

use std::io::{self, BufRead, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{ArgAction, Parser};
use regex::Regex;

mod identifier;
mod lang;
mod output;
mod patterns;
mod scanner;
mod score;
mod segment;
mod syllables;

use crate::output::{Finding, Format, write_findings};
use crate::patterns::PatternSpec;
use crate::scanner::{ScanOptions, scan, scan_names};
use crate::score::Confidence;

/// Detect Pinyin-named identifiers in source code using ast-grep as an
/// embedded library.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Paths to scan. Defaults to the current directory.
    paths: Vec<PathBuf>,

    /// Force a language (`ts`, `tsx`, `rs`). Required when `--pattern` is
    /// used; otherwise the language is inferred from the file extension.
    #[arg(long, value_name = "LANG")]
    lang: Option<String>,

    /// One or more ast-grep patterns. May be repeated. Replaces the
    /// language's built-in pattern set.
    #[arg(long, value_name = "PATTERN", action = ArgAction::Append)]
    pattern: Vec<String>,

    /// Name of the metavariable to extract from each match. Applies to
    /// every `--pattern` provided. Default: `NAME`.
    #[arg(long, value_name = "NAME", default_value = "NAME")]
    meta_var: String,

    /// Read identifier names from stdin (one per line) instead of
    /// scanning files. Mutually exclusive with positional paths.
    #[arg(long)]
    names: bool,

    /// Output format. `auto` picks NDJSON when stdout is piped and Text
    /// when it is a terminal.
    #[arg(long, value_name = "FORMAT", default_value = "auto",
          value_parser = ["auto", "text", "ndjson"])]
    format: String,

    /// Minimum confidence to display. Default: `medium`.
    #[arg(long, value_name = "LEVEL", default_value = "medium",
          value_parser = ["low", "medium", "high"])]
    min_confidence: String,

    /// Skip identifiers whose text matches this regex. May be repeated.
    #[arg(long, value_name = "REGEX", action = ArgAction::Append)]
    ignore: Vec<String>,

    /// Show every finding regardless of confidence (overrides
    /// `--min-confidence`).
    #[arg(long)]
    show_all: bool,
}

fn main() -> ExitCode {
    match real_main() {
        Ok(_) => ExitCode::from(0),
        Err(err) => {
            eprintln!("pinyin-grep: {err:#}");
            ExitCode::from(2)
        }
    }
}

fn real_main() -> Result<bool> {
    let cli = Cli::parse();

    let format = resolve_format(&cli.format);
    let min_confidence = Confidence::parse(&cli.min_confidence)
        .context("invalid --min-confidence value")?;

    let ignore_regexes = compile_ignore_regexes(&cli.ignore)?;

    let findings = if cli.names {
        if !cli.paths.is_empty() {
            bail!("--names is mutually exclusive with positional paths");
        }
        if !cli.pattern.is_empty() {
            bail!("--pattern has no effect in --names mode");
        }
        let stdin = io::stdin();
        let lines: Vec<String> = stdin
            .lock()
            .lines()
            .map_while(|l| l.ok())
            .collect();
        let mut findings = scan_names(&lines);
        findings.retain(|f| !ignore_regexes.iter().any(|r| r.is_match(&f.identifier)));
        findings
    } else {
        let paths = if cli.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            cli.paths.clone()
        };

        let forced_lang = cli
            .lang
            .as_deref()
            .map(|s| {
                lang::from_cli(s)
                    .with_context(|| format!("unsupported --lang: {s}"))
            })
            .transpose()?;

        let user_patterns = if cli.pattern.is_empty() {
            Vec::new()
        } else {
            if forced_lang.is_none() {
                bail!("--pattern requires --lang");
            }
            // Leak the source/meta_var strings so they get 'static lifetime,
            // matching the built-in pattern array's signature.
            let meta_var: &'static str = Box::leak(cli.meta_var.clone().into_boxed_str());
            cli.pattern
                .iter()
                .map(|src| PatternSpec {
                    source: Box::leak(src.clone().into_boxed_str()),
                    meta_var,
                })
                .collect()
        };

        let opts = ScanOptions {
            paths: &paths,
            forced_lang,
            user_patterns,
            ignore_regexes,
        };
        scan(&opts)
    };

    let filtered = filter_findings(findings, min_confidence, cli.show_all);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    write_findings(&mut out, &filtered, format)?;

    Ok(!filtered.is_empty())
}

fn resolve_format(value: &str) -> Format {
    match value {
        "ndjson" => Format::Ndjson,
        "text" => Format::Text,
        _ => {
            // auto: NDJSON when piped, text when on a TTY.
            if io::stdout().is_terminal() {
                Format::Text
            } else {
                Format::Ndjson
            }
        }
    }
}

fn compile_ignore_regexes(patterns: &[String]) -> Result<Vec<Regex>> {
    patterns
        .iter()
        .map(|p| Regex::new(p).with_context(|| format!("invalid --ignore regex: {p}")))
        .collect()
}

fn filter_findings(
    findings: Vec<Finding>,
    min: Confidence,
    show_all: bool,
) -> Vec<Finding> {
    if show_all {
        return findings;
    }
    findings
        .into_iter()
        .filter(|f| f.confidence >= min)
        .collect()
}
