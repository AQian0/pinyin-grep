//! DP-based segmentation of a lowercase ASCII token into Pinyin syllables.
//!
//! For an input token like `xian`, returns every valid decomposition into
//! syllables drawn from [`crate::syllables::syllable_set`] — for example:
//! `[["xian"], ["xi", "an"]]`. Tokens that cannot be fully segmented yield
//! an empty vector.

use crate::syllables::{max_syllable_len, syllable_set};

/// Maximum number of segmentations retained per token to bound complexity.
const MAX_SEGMENTATIONS: usize = 8;

/// Produces all valid Pinyin segmentations of `token` (lowercase ASCII),
/// sorted by syllable count ascending (longest-match preferred). At most
/// [`MAX_SEGMENTATIONS`] candidates are returned.
pub fn segment(token: &str) -> Vec<Vec<&'static str>> {
    if token.is_empty() || !token.is_ascii() {
        return Vec::new();
    }
    let bytes = token.as_bytes();
    let n = bytes.len();
    let max_len = max_syllable_len();
    let set = syllable_set();

    // dp[i] = list of segmentations covering token[0..i]
    let mut dp: Vec<Vec<Vec<&'static str>>> = vec![Vec::new(); n + 1];
    dp[0].push(Vec::new());

    for i in 1..=n {
        let lower = i.saturating_sub(max_len);
        for j in lower..i {
            if dp[j].is_empty() {
                continue;
            }
            let slice = &token[j..i];
            if let Some(syl) = set.get(slice).copied() {
                let prev = dp[j].clone();
                for seg in prev {
                    let mut next = seg;
                    next.push(syl);
                    dp[i].push(next);
                    if dp[i].len() >= MAX_SEGMENTATIONS * 2 {
                        // local cap to prevent quadratic blowup; we'll
                        // truncate again after the loop
                        break;
                    }
                }
            }
        }
        // Trim aggressive growth: prefer fewer-syllable splits.
        if dp[i].len() > MAX_SEGMENTATIONS {
            dp[i].sort_by_key(|s| s.len());
            dp[i].truncate(MAX_SEGMENTATIONS);
        }
    }

    let mut out = std::mem::take(&mut dp[n]);
    out.sort_by_key(|s| s.len());
    out.truncate(MAX_SEGMENTATIONS);
    out
}

/// Convenience: true if `token` admits at least one valid segmentation.
#[allow(dead_code)] // exposed as a library-style helper, used in tests
pub fn is_pinyin_token(token: &str) -> bool {
    !segment(token).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(token: &str) -> Vec<Vec<&'static str>> {
        segment(token)
    }

    #[test]
    fn empty_token_yields_nothing() {
        assert!(segment("").is_empty());
    }

    #[test]
    fn non_ascii_yields_nothing() {
        assert!(segment("拼音").is_empty());
    }

    #[test]
    fn single_syllable_preferred_first() {
        // "huo" is genuinely ambiguous: huo OR hu+o; we just require the
        // longest-match candidate to come first.
        let huo = collect("huo");
        assert!(!huo.is_empty());
        assert_eq!(huo[0], vec!["huo"]);
        assert_eq!(collect("qu")[0], vec!["qu"]);
    }

    #[test]
    fn multi_syllable_preferred_first() {
        let huoqu = collect("huoqu");
        assert!(huoqu.contains(&vec!["huo", "qu"]));
        assert_eq!(huoqu[0], vec!["huo", "qu"]);
        assert!(collect("buzhi").contains(&vec!["bu", "zhi"]));
    }

    #[test]
    fn ambiguous_xian() {
        let segs = collect("xian");
        assert!(segs.contains(&vec!["xian"]));
        assert!(segs.contains(&vec!["xi", "an"]));
    }

    #[test]
    fn longest_match_first() {
        let segs = collect("xian");
        // The first (preferred) segmentation should have the fewest syllables.
        assert_eq!(segs.first().unwrap().len(), 1);
    }

    #[test]
    fn rejects_non_pinyin() {
        assert!(segment("hello").is_empty());
        assert!(segment("xyz").is_empty());
    }

    #[test]
    fn is_pinyin_token_basic() {
        assert!(is_pinyin_token("xinxi"));
        assert!(is_pinyin_token("yonghu"));
        assert!(!is_pinyin_token("hello"));
    }
}
