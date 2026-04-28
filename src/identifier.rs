//! Identifier word-splitting that copes with camelCase, PascalCase,
//! snake_case, kebab-case and SCREAMING_SNAKE_CASE.
//!
//! Output tokens are always lowercase ASCII so they can be passed straight
//! into [`crate::segment::segment`].

#[derive(Copy, Clone, PartialEq, Eq)]
enum CharKind {
    Lower,
    Upper,
    Other,
}

fn classify(c: char) -> CharKind {
    if c.is_ascii_lowercase() {
        CharKind::Lower
    } else if c.is_ascii_uppercase() {
        CharKind::Upper
    } else {
        CharKind::Other
    }
}

/// Splits `ident` into a sequence of lowercase ASCII tokens following the
/// "Unicode word break" conventions for code identifiers.
///
/// Rules:
/// * Non-alphabetic characters (including digits, `_`, `-`, `.`, `'`) act
///   as token separators and are dropped.
/// * Within an alphabetic run, a token break occurs at:
///   - lower→upper transitions (`huoQu` → `huo`, `qu`)
///   - acronym→word transitions where an uppercase run is followed by an
///     uppercase letter that itself precedes a lowercase letter
///     (`URLParser` → `url`, `parser`)
///
/// Empty input yields an empty vector.
pub fn tokenize(ident: &str) -> Vec<String> {
    let chars: Vec<char> = ident.chars().collect();
    let n = chars.len();
    let mut tokens: Vec<String> = Vec::new();
    let mut buf = String::new();

    let flush = |buf: &mut String, tokens: &mut Vec<String>| {
        if !buf.is_empty() {
            tokens.push(std::mem::take(buf).to_ascii_lowercase());
        }
    };

    let mut prev_kind: Option<CharKind> = None;
    for i in 0..n {
        let c = chars[i];
        let kind = classify(c);

        match kind {
            CharKind::Other => {
                flush(&mut buf, &mut tokens);
                prev_kind = Some(kind);
                continue;
            }
            CharKind::Lower => {
                buf.push(c);
            }
            CharKind::Upper => {
                let break_after_lower = matches!(prev_kind, Some(CharKind::Lower));
                let acronym_break = matches!(prev_kind, Some(CharKind::Upper))
                    && i + 1 < n
                    && classify(chars[i + 1]) == CharKind::Lower;
                if break_after_lower || acronym_break {
                    flush(&mut buf, &mut tokens);
                }
                buf.push(c);
            }
        }
        prev_kind = Some(kind);
    }
    flush(&mut buf, &mut tokens);

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_camel_case() {
        assert_eq!(tokenize("huoQuXinXi"), vec!["huo", "qu", "xin", "xi"]);
    }

    #[test]
    fn splits_pascal_case() {
        assert_eq!(tokenize("HuoQuXinXi"), vec!["huo", "qu", "xin", "xi"]);
    }

    #[test]
    fn splits_snake_case() {
        assert_eq!(tokenize("huo_qu_xin_xi"), vec!["huo", "qu", "xin", "xi"]);
    }

    #[test]
    fn splits_kebab_case() {
        assert_eq!(tokenize("huo-qu-xin-xi"), vec!["huo", "qu", "xin", "xi"]);
    }

    #[test]
    fn splits_screaming_snake() {
        assert_eq!(tokenize("HUO_QU_XIN_XI"), vec!["huo", "qu", "xin", "xi"]);
    }

    #[test]
    fn handles_acronyms() {
        assert_eq!(tokenize("URLParser"), vec!["url", "parser"]);
        assert_eq!(tokenize("getHTTPResponse"), vec!["get", "http", "response"]);
    }

    #[test]
    fn drops_digits_and_separators() {
        assert_eq!(tokenize("huo2qu_v3"), vec!["huo", "qu", "v"]);
    }

    #[test]
    fn mixed_case_with_trailing_acronym() {
        assert_eq!(tokenize("huoQu_xinXI"), vec!["huo", "qu", "xin", "xi"]);
    }

    #[test]
    fn empty_input() {
        assert_eq!(tokenize(""), Vec::<String>::new());
        assert_eq!(tokenize("____"), Vec::<String>::new());
    }
}
