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

struct GoldenRun {
    full: Value,
    exit_code: i32,
}

fn run_manifest(manifest: &str) -> GoldenRun {
    let output = Command::new(env!("CARGO_BIN_EXE_oba"))
        .current_dir(manifest_dir())
        .args(["--targets", manifest, "--json"])
        .output()
        .expect("failed to run oba binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let full: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "oba did not emit valid JSON: {e}\nstderr: {}\nstdout: {stdout}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    GoldenRun {
        full,
        exit_code: output
            .status
            .code()
            .expect("process exited via signal, not code"),
    }
}

fn run_golden() -> Vec<Value> {
    run_manifest("targets/golden.toml")
        .full
        .get("targets")
        .expect("top-level JSON must have a targets array")
        .as_array()
        .expect("targets is an array")
        .clone()
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
        .unwrap_or_else(|| panic!("no verdict for option {option} in {}", report["name"]))
        ["verdict"]
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
        .map(|o| {
            o["path"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap())
                .collect::<Vec<_>>()
                .join(".")
        })
        .collect();
    assert!(
        options.contains(&"database.socket".to_string()),
        "scanner must discover the database.socket option declaration; got {options:?}"
    );

    let predicates: Vec<String> = r["discovered_predicates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            p["path"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap())
                .collect::<Vec<_>>()
                .join(".")
        })
        .collect();
    assert!(
        predicates.contains(&"database.socket".to_string()),
        "scanner must discover the `cfg.database.socket != null` predicate; got {predicates:?}"
    );

    let assignments: Vec<String> = r["matched_test_assignments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            format!(
                "{}[{}]",
                a["path"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join("."),
                a["instance"].as_str().unwrap_or("?")
            )
        })
        .collect();
    assert!(
        assignments
            .iter()
            .any(|a| a.contains("database.socket") && a.contains("socketMachine")),
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
    let evidence = v["evidence"]
        .as_array()
        .expect("PASS verdict must carry evidence");
    assert!(!evidence.is_empty(), "PASS must not have empty evidence");
    assert_eq!(evidence[0]["instance"], "socketMachine");
}

// --- davis: honest OptionNotFound, not a false PASS, on both commits.
// davis.nix uses the flat `options.services.davis = { ... };` attrpath
// form, which the (deliberately narrow) declaration scanner doesn't
// recognize -- so it fails at gate 1 (declaration) before ever reaching
// gate 2 (the `mysqlLocal` let-alias predicate, also out of MVP scope).
// Either gate failing first is fine; what matters is it's never PASS.

#[test]
fn davis_predicate_is_honestly_reported_as_not_found() {
    let reports = run_golden();
    for name in ["davis-before", "davis-after"] {
        let r = target(&reports, name);
        let kind = verdict_kind(r, "database.driver");
        assert!(
            kind == "OptionNotFound" || kind == "PredicateNotFound",
            "target {name} must not silently PASS or silently omit the watched option; got {kind}"
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

// --- H1 review fixes ---------------------------------------------------
//
// Three real correctness issues, found by review of the pre-H1 code, not
// by this test suite (it was green the whole time): PredicateNotFound
// exited 0, is_default_class() checked textual equality to the default
// instead of predicate *outcome* (silently correct for kimai's null
// default only, wrong the moment a default is non-null), and PASS was
// reachable without the declaration scanner having found anything. Every
// fix below is pinned to the specific fixture that demonstrates it, not
// just re-asserted against the existing corpus.

// H1-1: PredicateNotFound (and friends) must make the whole run exit
// non-zero, distinctly from a genuine OBA001 finding -- a detector that
// couldn't evaluate something must never look like "all clear" to CI.

#[test]
fn h1_exit_codes_distinguish_clean_finding_and_inconclusive() {
    let clean = run_manifest("targets/clean.toml");
    assert_eq!(clean.exit_code, 0);
    assert_eq!(clean.full["summary"]["status"], "PASS");

    let findings = run_manifest("targets/findings-only.toml");
    assert_eq!(findings.exit_code, 1);
    assert_eq!(findings.full["summary"]["status"], "FINDING");
    assert_eq!(findings.full["summary"]["findings"], 1);
    assert_eq!(findings.full["summary"]["inconclusive"], 0);

    // golden.toml mixes real OBA001 findings with davis's honest
    // OptionNotFound -- inconclusive must take precedence over finding in
    // both the exit code and the summary status, per the explicit ask:
    // a run that couldn't fully evaluate everything has no business
    // reporting itself as merely "found some bugs, otherwise clean".
    let mixed = run_manifest("targets/golden.toml");
    assert_eq!(mixed.exit_code, 2);
    assert_eq!(mixed.full["summary"]["status"], "INCONCLUSIVE");
    assert!(mixed.full["summary"]["findings"].as_u64().unwrap() > 0);
    assert!(mixed.full["summary"]["inconclusive"].as_u64().unwrap() > 0);
}

// H1-1b: parse errors specifically must fail the whole target closed --
// no per-watch verdict gets computed off a tree rnix patched together
// around the damage.

#[test]
fn h1_parse_errors_fail_closed() {
    let run = run_manifest("targets/parse-error.toml");
    assert_eq!(run.exit_code, 2);
    assert_eq!(run.full["summary"]["status"], "INCONCLUSIVE");
    let targets = run.full["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 1);
    let parse_errors = targets[0]["parse_errors"].as_array().unwrap();
    assert!(
        !parse_errors.is_empty(),
        "malformed module.nix must produce parse_errors"
    );
    // and no verdict should be a PASS or OBA001 computed off the broken tree
    for v in targets[0]["verdicts"].as_array().unwrap() {
        assert!(
            v["verdict"] != "PASS" && v["verdict"] != "OBA001",
            "a target with parse errors must never produce a scan-derived verdict"
        );
    }
}

// H1-2: the outcome-transition model, not textual equality to the
// default. c6a is the exact scenario from the review: a non-null default
// (`"/run/default.sock"`), a test assignment of the *same* string. The
// old code read this as "not equal to null => non-default evidence" and
// would have reported PASS despite proving no branch transition at all.
// c6b is the positive control in the same module: assigning `null`
// (genuinely the opposite predicate outcome) must still produce PASS, so
// the fix isn't just "never PASS again".

#[test]
fn h1_outcome_transition_not_textual_equality() {
    let reports = run_golden();

    let same_as_default = target(&reports, "c6a-same-as-nonnull-default");
    assert_eq!(
        verdict_kind(same_as_default, "foo"),
        "OBA001",
        "assigning the option's own non-null default must not count as activation evidence"
    );

    let real_transition = target(&reports, "c6b-transitions-from-nonnull-default");
    assert_eq!(
        verdict_kind(real_transition, "foo"),
        "PASS",
        "assigning null against a non-null default IS a genuine predicate-outcome transition"
    );
}

// H1-2b: a default whose outcome can't be statically classified (not a
// literal null/true/false) must report DefaultUnresolved, not silently
// treat "unknown" as either "counts" or "doesn't count".

#[test]
fn h1_unresolvable_default_is_inconclusive_not_guessed() {
    let reports = run_golden();
    let r = target(&reports, "c7-unresolved-default");
    assert_eq!(verdict_kind(r, "bar"), "DefaultUnresolved");
}

// H1-3: PASS must be structurally unreachable without the declaration
// scanner having found the option first -- watching a path with no
// mkOption at all must report OptionNotFound, the new first gate.

#[test]
fn h1_declaration_is_a_mandatory_gate() {
    let reports = run_golden();
    let r = target(&reports, "c8-option-not-found");
    assert_eq!(verdict_kind(r, "doesNotExist"), "OptionNotFound");
}
