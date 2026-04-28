//! Bridges between file extensions / CLI input and ast-grep's
//! [`SupportLang`]. v1 supports TypeScript, TSX and Rust.

use std::path::Path;

use ast_grep_language::SupportLang;

/// Returns the language matching `path`'s extension, or `None` if the file
/// type is not supported.
pub fn from_path(path: &Path) -> Option<SupportLang> {
    let ext = path.extension()?.to_str()?;
    from_extension(ext)
}

/// Returns the language for an extension (without the dot), e.g. `"ts"`.
pub fn from_extension(ext: &str) -> Option<SupportLang> {
    match ext.to_ascii_lowercase().as_str() {
        "ts" | "mts" | "cts" => Some(SupportLang::TypeScript),
        "tsx" => Some(SupportLang::Tsx),
        "rs" => Some(SupportLang::Rust),
        _ => None,
    }
}

/// Parses a CLI `--lang` value (case-insensitive). Accepts both extension
/// shorthands (`ts`, `tsx`, `rs`) and human names (`typescript`, `rust`).
pub fn from_cli(value: &str) -> Option<SupportLang> {
    match value.to_ascii_lowercase().as_str() {
        "ts" | "typescript" => Some(SupportLang::TypeScript),
        "tsx" => Some(SupportLang::Tsx),
        "rs" | "rust" => Some(SupportLang::Rust),
        _ => None,
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extension_mapping() {
        assert_eq!(from_extension("ts"), Some(SupportLang::TypeScript));
        assert_eq!(from_extension("TSX"), Some(SupportLang::Tsx));
        assert_eq!(from_extension("rs"), Some(SupportLang::Rust));
        assert_eq!(from_extension("py"), None);
    }

    #[test]
    fn path_mapping() {
        assert_eq!(
            from_path(&PathBuf::from("src/foo.ts")),
            Some(SupportLang::TypeScript)
        );
        assert_eq!(
            from_path(&PathBuf::from("src/foo.tsx")),
            Some(SupportLang::Tsx)
        );
        assert_eq!(
            from_path(&PathBuf::from("src/foo.rs")),
            Some(SupportLang::Rust)
        );
        assert_eq!(from_path(&PathBuf::from("README.md")), None);
    }

    #[test]
    fn cli_mapping() {
        assert_eq!(from_cli("ts"), Some(SupportLang::TypeScript));
        assert_eq!(from_cli("Rust"), Some(SupportLang::Rust));
        assert_eq!(from_cli("python"), None);
    }
}
