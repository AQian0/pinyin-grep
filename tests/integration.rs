//! End-to-end tests: drive the `pinyin-grep` binary against fixture files
//! under `tests/samples/`.

use assert_cmd::Command;
use serde_json::Value;

fn run_ndjson(args: &[&str]) -> Vec<Value> {
    let out = Command::cargo_bin("pinyin-grep")
        .unwrap()
        .args(args)
        .output()
        .expect("binary should run");
    assert!(
        out.status.success() || out.status.code() == Some(0),
        "exit code: {:?}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("ndjson output should be utf-8")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("each line should parse as JSON"))
        .collect()
}

fn identifiers(findings: &[Value]) -> Vec<String> {
    findings
        .iter()
        .filter_map(|f| f["identifier"].as_str().map(|s| s.to_string()))
        .collect()
}

#[test]
fn scans_typescript_fixture() {
    let findings = run_ndjson(&[
        "tests/samples/foo.ts",
        "--format",
        "ndjson",
        "--min-confidence",
        "medium",
    ]);
    let idents = identifiers(&findings);

    // Pinyin identifiers should be found.
    for expected in [
        "xinXi",
        "huoQuYongHu",
        "GuanLiYuan",
        "YongHuXinXi",
    ] {
        assert!(
            idents.contains(&expected.to_string()),
            "expected {expected} in {idents:?}"
        );
    }

    // English-only identifiers should NOT show up at medium confidence.
    for unwanted in ["userName", "loadFile", "Person", "path"] {
        assert!(
            !idents.contains(&unwanted.to_string()),
            "unexpected {unwanted} in {idents:?}"
        );
    }
}

#[test]
fn scans_rust_fixture() {
    let findings = run_ndjson(&[
        "tests/samples/bar.rs",
        "--format",
        "ndjson",
        "--min-confidence",
        "medium",
    ]);
    let idents = identifiers(&findings);

    for expected in [
        "huo_qu_yong_hu",
        "GuanLiYuan",
        "ZhuangTai",
        "SHU_LIANG",
    ] {
        assert!(
            idents.contains(&expected.to_string()),
            "expected {expected} in {idents:?}"
        );
    }

    for unwanted in ["load_file", "path"] {
        assert!(
            !idents.contains(&unwanted.to_string()),
            "unexpected {unwanted} in {idents:?}"
        );
    }
}

#[test]
fn ndjson_schema_is_stable() {
    let findings = run_ndjson(&[
        "tests/samples/foo.ts",
        "--format",
        "ndjson",
        "--min-confidence",
        "medium",
    ]);
    assert!(!findings.is_empty());
    let f = &findings[0];

    // Required top-level fields.
    for field in ["identifier", "tokens", "score", "confidence", "ambiguous"] {
        assert!(f.get(field).is_some(), "missing field {field}: {f}");
    }
    // File scans always carry file + range.
    assert!(f.get("file").is_some());
    assert!(f.get("range").is_some());

    // Range has nested 0-based start/end positions with line and column.
    let range = &f["range"];
    let start = &range["start"];
    assert!(start["line"].is_u64());
    assert!(start["column"].is_u64());

    // Tokens is an array of {text, syllables}.
    let tokens = f["tokens"].as_array().expect("tokens should be array");
    assert!(!tokens.is_empty());
    for t in tokens {
        assert!(t["text"].is_string());
        assert!(t["syllables"].is_array());
    }
}

#[test]
fn names_mode_reads_stdin() {
    let out = Command::cargo_bin("pinyin-grep")
        .unwrap()
        .args(["--names", "--format", "ndjson", "--min-confidence", "low"])
        .write_stdin("huoQuYongHu\nloadFile\nxinxi\n")
        .output()
        .expect("binary should run");
    assert!(out.status.success());

    let findings: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let idents = identifiers(&findings);

    assert!(idents.contains(&"huoQuYongHu".to_string()));
    assert!(idents.contains(&"xinxi".to_string()));
    assert!(!idents.contains(&"loadFile".to_string()));

    // --names findings have no file/range fields.
    for f in &findings {
        assert!(f.get("file").is_none() || f["file"].is_null());
        assert!(f.get("range").is_none() || f["range"].is_null());
    }
}

#[test]
fn min_confidence_high_filters_more() {
    let medium = run_ndjson(&[
        "tests/samples/foo.ts",
        "--format",
        "ndjson",
        "--min-confidence",
        "medium",
    ]);
    let high = run_ndjson(&[
        "tests/samples/foo.ts",
        "--format",
        "ndjson",
        "--min-confidence",
        "high",
    ]);
    assert!(
        high.len() <= medium.len(),
        "high ({}) should not exceed medium ({})",
        high.len(),
        medium.len()
    );
}

#[test]
fn ignore_regex_skips_identifier() {
    let baseline = run_ndjson(&[
        "tests/samples/foo.ts",
        "--format",
        "ndjson",
        "--min-confidence",
        "medium",
    ]);
    let filtered = run_ndjson(&[
        "tests/samples/foo.ts",
        "--format",
        "ndjson",
        "--min-confidence",
        "medium",
        "--ignore",
        "^GuanLiYuan$",
    ]);
    assert!(identifiers(&baseline).contains(&"GuanLiYuan".to_string()));
    assert!(!identifiers(&filtered).contains(&"GuanLiYuan".to_string()));
}
