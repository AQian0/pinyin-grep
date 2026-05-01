//! Confidence scoring for identifiers that may contain Pinyin.
//!
//! [`analyze`] turns a raw identifier into a structured [`Analysis`] with
//! per-token segmentations, an integer score, a [`Confidence`] bucket and
//! an `ambiguous` flag. Returns `None` for identifiers with no Pinyin
//! signal at all (so callers can avoid emitting empty results).

use std::sync::OnceLock;

use serde::Serialize;

use crate::identifier::tokenize;
use crate::segment::segment;

/// Common English short words that happen to overlap with valid Pinyin
/// syllables. Used to dampen false positives for single-token identifiers.
const ENGLISH_WORDS: &[&str] = &[
    "an", "me", "you", "men", "ban", "can", "fan", "man", "pan", "ran", "tan", "han", "ha", "he",
    "ma", "pa", "la", "ya", "yo", "die", "tie", "pie", "lie", "pin", "bin", "den", "pen", "hen",
    "pi", "mu", "ping", "ming",
];

fn english_word_set() -> &'static std::collections::HashSet<&'static str> {
    static SET: OnceLock<std::collections::HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| ENGLISH_WORDS.iter().copied().collect())
}

/// Confidence bucket assigned to an [`Analysis`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::Low => "low",
            Confidence::Medium => "medium",
            Confidence::High => "high",
        }
    }

    /// Parses a CLI value (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "low" => Some(Confidence::Low),
            "medium" | "mid" => Some(Confidence::Medium),
            "high" => Some(Confidence::High),
            _ => None,
        }
    }
}

/// Per-token analysis: lowercase token text plus zero or more candidate
/// syllable segmentations sorted by ascending syllable count.
#[derive(Debug, Clone, Serialize)]
pub struct TokenAnalysis {
    pub text: String,
    pub syllables: Vec<Vec<&'static str>>,
}

/// Result of [`analyze`] for a single identifier.
#[derive(Debug, Clone, Serialize)]
pub struct Analysis {
    pub identifier: String,
    pub tokens: Vec<TokenAnalysis>,
    pub score: i32,
    pub confidence: Confidence,
    pub ambiguous: bool,
}

/// Analyzes `identifier` for Pinyin content.
///
/// Returns `None` if the identifier has no plausible Pinyin signal (i.e.
/// no token can be segmented into syllables). Otherwise the returned
/// [`Analysis`] always carries a confidence — callers are expected to
/// filter by `--min-confidence`.
pub fn analyze(identifier: &str) -> Option<Analysis> {
    let raw_tokens = tokenize(identifier);
    if raw_tokens.is_empty() {
        return None;
    }

    let token_analyses: Vec<TokenAnalysis> = raw_tokens
        .iter()
        .map(|t| TokenAnalysis {
            text: t.clone(),
            syllables: segment(t),
        })
        .collect();

    let valid_count = token_analyses
        .iter()
        .filter(|t| !t.syllables.is_empty())
        .count();
    if valid_count == 0 {
        return None;
    }

    let total = token_analyses.len();

    // Sum of preferred-segmentation syllable counts across valid tokens.
    let total_syllables: usize = token_analyses
        .iter()
        .filter_map(|t| t.syllables.first().map(|s| s.len()))
        .sum();

    // Total preferred-segmentation character coverage (used for avg length).
    let total_syllable_chars: usize = token_analyses
        .iter()
        .filter_map(|t| t.syllables.first())
        .flat_map(|s| s.iter().map(|x| x.len()))
        .sum();
    let avg_len = if total_syllables == 0 {
        0.0
    } else {
        total_syllable_chars as f64 / total_syllables as f64
    };

    // Suppress single, very-short tokens (`an`, `me`, …) outright — they
    // create too much noise when the identifier has no other context.
    if total == 1 && token_analyses[0].text.len() < 3 {
        return None;
    }

    // ─── scoring ────────────────────────────────────────────
    let mut score: i32 = 0;

    // 1 point per token that segmented into a valid candidate.
    score += valid_count as i32;

    let all_valid = valid_count == total;
    if all_valid {
        // Baseline bonus: every token is recognised as Pinyin.
        score += 1;
        // Long sequences (≥ 3 syllables) are very characteristic.
        if total_syllables >= 3 {
            score += 1;
        }
        // Pinyin syllables average ~2.5–3 chars; lookalike segmentations
        // (e.g. "name" → na+me) average closer to 2.0.
        if avg_len >= 2.5 {
            score += 1;
        }
        // Strong signal: multi-token identifier with every token Pinyin.
        if total >= 2 {
            score += 2;
        }
    } else {
        // Mixed identifier (some tokens are English-like). Penalise unless
        // the matched parts have characteristically Pinyin-shaped syllables.
        if avg_len < 2.5 {
            score -= 2;
        }
    }

    // English-blacklist penalty for single-token identifiers whose only
    // segmentation is a single syllable.
    if total == 1 {
        let token = &token_analyses[0];
        if let Some(seg) = token.syllables.first()
            && seg.len() == 1
            && english_word_set().contains(token.text.as_str())
        {
            score -= 3;
        }
    }

    let confidence = if score >= 5 {
        Confidence::High
    } else if score >= 3 {
        Confidence::Medium
    } else {
        Confidence::Low
    };

    let ambiguous = token_analyses.iter().any(|t| t.syllables.len() > 1);

    Some(Analysis {
        identifier: identifier.to_string(),
        tokens: token_analyses,
        score,
        confidence,
        ambiguous,
    })
}

/// Returns the preferred (longest-match) segmentation joined with `·`,
/// e.g. `huo·qu·xin·xi`. Tokens that failed to segment fall back to their
/// original text.
#[allow(dead_code)] // exposed as a library-style helper, used in tests
pub fn pretty_syllables(analysis: &Analysis) -> String {
    let mut parts = Vec::new();
    for tok in &analysis.tokens {
        if let Some(seg) = tok.syllables.first() {
            for s in seg {
                parts.push(*s);
            }
        } else {
            parts.push(&tok.text);
        }
    }
    parts.join("·")
}

/// Returns the total syllable count using the preferred segmentation per
/// token. Tokens that failed to segment count as zero.
#[allow(dead_code)] // exposed as a library-style helper, used in tests
pub fn syllable_count(analysis: &Analysis) -> usize {
    analysis
        .tokens
        .iter()
        .filter_map(|t| t.syllables.first().map(|s| s.len()))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_classic_pinyin_identifier_high() {
        let a = analyze("huoQuYongHuXinXi").unwrap();
        assert_eq!(a.confidence, Confidence::High);
        assert_eq!(a.tokens.len(), 6);
        assert_eq!(syllable_count(&a), 6);
    }

    #[test]
    fn detects_two_token_pinyin_high() {
        let a = analyze("huoQu").unwrap();
        assert_eq!(a.confidence, Confidence::High);
    }

    #[test]
    fn detects_long_single_token_medium() {
        let a = analyze("huoquxinxi").unwrap();
        assert_eq!(a.confidence, Confidence::Medium);
    }

    #[test]
    fn english_two_letter_pinyin_filtered() {
        // "name" is 1 token, 2 syllables, avg length 2.0
        let a = analyze("name").unwrap();
        assert_eq!(a.confidence, Confidence::Low);
    }

    #[test]
    fn pure_english_returns_none() {
        assert!(analyze("loadFile").is_none());
        assert!(analyze("getUser").is_none());
        assert!(analyze("hello").is_none());
    }

    #[test]
    fn very_short_single_token_returns_none() {
        assert!(analyze("an").is_none());
        assert!(analyze("me").is_none());
    }

    #[test]
    fn english_blacklist_penalty() {
        // "men" is 3 chars so passes the length gate but is in the blacklist;
        // it should land in Low.
        let a = analyze("men").unwrap();
        assert_eq!(a.confidence, Confidence::Low);
    }

    #[test]
    fn ambiguity_is_flagged() {
        let a = analyze("xian").unwrap();
        assert!(a.ambiguous);
    }

    #[test]
    fn pretty_syllables_preferred_segmentation() {
        let a = analyze("huoQuXinXi").unwrap();
        assert_eq!(pretty_syllables(&a), "huo·qu·xin·xi");
    }
}
