//! Built-in ast-grep pattern set used when the user does not pass any
//! `--pattern` of their own. Each pattern declares which metavariable
//! captures the identifier we want to inspect.

use ast_grep_language::SupportLang;

/// A single pattern entry: source string + the name of the metavariable
/// that captures the identifier (typically `NAME`).
#[derive(Debug, Clone, Copy)]
pub struct PatternSpec {
    pub source: &'static str,
    pub meta_var: &'static str,
}

const TS_PATTERNS: &[PatternSpec] = &[
    PatternSpec { source: "const $NAME = $$$VAL", meta_var: "NAME" },
    PatternSpec { source: "let $NAME = $$$VAL", meta_var: "NAME" },
    PatternSpec { source: "var $NAME = $$$VAL", meta_var: "NAME" },
    PatternSpec { source: "function $NAME($$$ARGS) { $$$BODY }", meta_var: "NAME" },
    PatternSpec { source: "class $NAME { $$$BODY }", meta_var: "NAME" },
    PatternSpec { source: "interface $NAME { $$$BODY }", meta_var: "NAME" },
    PatternSpec { source: "type $NAME = $$$BODY", meta_var: "NAME" },
    PatternSpec { source: "enum $NAME { $$$BODY }", meta_var: "NAME" },
];

const RUST_PATTERNS: &[PatternSpec] = &[
    PatternSpec { source: "fn $NAME($$$ARGS) { $$$BODY }", meta_var: "NAME" },
    PatternSpec { source: "fn $NAME($$$ARGS) -> $RET { $$$BODY }", meta_var: "NAME" },
    PatternSpec { source: "struct $NAME { $$$BODY }", meta_var: "NAME" },
    PatternSpec { source: "struct $NAME($$$FIELDS);", meta_var: "NAME" },
    PatternSpec { source: "struct $NAME;", meta_var: "NAME" },
    PatternSpec { source: "enum $NAME { $$$BODY }", meta_var: "NAME" },
    PatternSpec { source: "trait $NAME { $$$BODY }", meta_var: "NAME" },
    PatternSpec { source: "let $NAME = $$$VAL", meta_var: "NAME" },
    PatternSpec { source: "let mut $NAME = $$$VAL", meta_var: "NAME" },
    PatternSpec { source: "const $NAME: $T = $$$VAL", meta_var: "NAME" },
    PatternSpec { source: "static $NAME: $T = $$$VAL", meta_var: "NAME" },
    PatternSpec { source: "type $NAME = $$$BODY", meta_var: "NAME" },
];

/// Returns the default pattern set for `lang`, or an empty slice when the
/// language is not yet supported.
pub fn default_patterns(lang: SupportLang) -> &'static [PatternSpec] {
    match lang {
        SupportLang::TypeScript | SupportLang::Tsx => TS_PATTERNS,
        SupportLang::Rust => RUST_PATTERNS,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typescript_patterns_non_empty() {
        assert!(!default_patterns(SupportLang::TypeScript).is_empty());
        assert!(!default_patterns(SupportLang::Tsx).is_empty());
    }

    #[test]
    fn rust_patterns_non_empty() {
        assert!(!default_patterns(SupportLang::Rust).is_empty());
    }

    #[test]
    fn unsupported_language_yields_empty() {
        assert!(default_patterns(SupportLang::Python).is_empty());
    }

    #[test]
    fn every_pattern_uses_dollar_name() {
        for lang in [SupportLang::TypeScript, SupportLang::Tsx, SupportLang::Rust] {
            for p in default_patterns(lang) {
                assert!(p.source.contains("$NAME"), "pattern missing $NAME: {:?}", p);
                assert_eq!(p.meta_var, "NAME");
            }
        }
    }
}
