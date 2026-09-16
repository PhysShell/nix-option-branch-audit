//! Golden + adversarial acceptance test for the Layer-1 OBA spike.
//!
//! Runs the actual compiled binary against the real commits from
//! PhysShell/nixpkgs branch fix/doctrine-unix-socket-param-name
//! (5530e24f2 = the historical bug, 37f81efa4 = the first fix commit) plus
//! four adversarial mutations, and asserts the exact verdict each one must
//! produce. This is the "does the analyzer die silently" backstop: every
//! assertion checks a *specific* verdict for a *specific* target, never
//! just "did it not crash".

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_golden() -> Vec<Value> {
    let output = Command::new(env!("CARGO_BIN_EXE_oba"))
        .current_dir(manifest_dir())
        .args(["--targets", "targets/golden.toml", "--json"])
        .output()
        .expect("failed to run oba binary");
    // exit code is expected to be 1 (some targets are OBA001 by design), so
    // don't assert on status -- only that stdout parses.
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "oba did not emit valid JSON: {e}\nstderr: {}\nstdout: {stdout}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn target<'a>(reports: &'a [Value], name: &str) -> &'a Value {
    reports
        .iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no report for target {name}"))
}

fn verdict_kind(report: &Value, option: &str) -> String {
    report["verdicts"]
        .as_array()
        .expect("verdicts array")
        .iter()
        .find(|v| v["option"] == option)
        .unwrap_or_else(|| panic!("no verdict for option {option} in {}", report["name"]))["verdict"]
        .as_str()
        .expect("verdict tag is a string")
        .to_string()
}

// --- Positive assertions: the scanner must actually have looked, not just
// returned nothing everywhere. A detector that silently stopped finding
// anything must fail loudly here, not read as "zero findings = all clean".

#[test]
fn positive_assertions_kimai_after_found_real_things() {
    let reports = run_golden();
    let r = target(&reports, "kimai-after");

    let options: Vec<String> = r["discovered_options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["path"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect::<Vec<_>>().join("."))
        .collect();
    assert!(
        options.contains(&"database.socket".to_string()),
        "scanner must discover the database.socket option declaration; got {options:?}"
    );

    let predicates: Vec<String> = r["discovered_predicates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["path"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect::<Vec<_>>().join("."))
        .collect();
    assert!(
        predicates.contains(&"database.socket".to_string()),
        "scanner must discover the `cfg.database.socket != null` predicate; got {predicates:?}"
    );

    let assignments: Vec<String> = r["matched_test_assignments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| format!("{}[{}]", a["path"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect::<Vec<_>>().join("."), a["instance"].as_str().unwrap_or("?")))
        .collect();
    assert!(
        assignments.iter().any(|a| a.contains("database.socket") && a.contains("socketMachine")),
        "scanner must match socketMachine's database.socket assignment; got {assignments:?}"
    );
}

// --- A: reproduces the historical finding on the exact parent commit.

#[test]
fn golden_a_parent_commit_shows_oba001() {
    let reports = run_golden();
    let r = target(&reports, "kimai-before");
    assert_eq!(verdict_kind(r, "database.socket"), "OBA001");
}

// --- B: clears on the exact fix commit, with real evidence attached.

#[test]
fn golden_b_fix_commit_shows_pass_with_evidence() {
    let reports = run_golden();
    let r = target(&reports, "kimai-after");
    assert_eq!(verdict_kind(r, "database.socket"), "PASS");

    let v = r["verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["option"] == "database.socket")
        .unwrap();
    let evidence = v["evidence"].as_array().expect("PASS verdict must carry evidence");
    assert!(!evidence.is_empty(), "PASS must not have empty evidence");
    assert_eq!(evidence[0]["instance"], "socketMachine");
}

// --- davis: honest PREDICATE_NOT_FOUND, not a false PASS, on both commits
// (the branch is gated by the `mysqlLocal` let-alias, out of MVP scope).

#[test]
fn davis_predicate_is_honestly_reported_as_not_found() {
    let reports = run_golden();
    for name in ["davis-before", "davis-after"] {
        let r = target(&reports, name);
        assert_eq!(
            verdict_kind(r, "database.driver"),
            "PredicateNotFound",
            "target {name} must not silently PASS or silently omit the watched option"
        );
    }
}

// --- C: survives adversarial mutations against the fixed module. Every one
// of these must stay OBA001 -- if any flips to PASS, the matcher is doing
// something dumber than it looks (naive grep-shaped false positive).

#[test]
fn golden_c_mutations_all_stay_oba001() {
    let reports = run_golden();
    for name in [
        "c2-remove-scenario",
        "c3-explicit-null",
        "c4-wrong-prefix",
        "c5-wrong-suffix",
    ] {
        let r = target(&reports, name);
        assert_eq!(
            verdict_kind(r, "database.socket"),
            "OBA001",
            "mutation {name} must not produce a false PASS"
        );
    }
}
