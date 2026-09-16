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

/// For tool-error cases: stdout is empty (the error happens before any
/// printing) so there's nothing to parse as JSON -- just the exit code and
/// stderr matter.
fn run_manifest_raw(manifest: &str) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_oba"))
        .current_dir(manifest_dir())
        .args(["--targets", manifest, "--json"])
        .output()
        .expect("failed to run oba binary");
    (
        output
            .status
            .code()
            .expect("process exited via signal, not code"),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
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
    // Tightened per H1.1 review: the previous OR-of-two-verdicts version
    // would have silently accepted a regression where the declaration
    // scanner starts finding davis's flat-dotted option form but hits the
    // mysqlLocal alias gap instead (or vice versa) -- a golden this loose
    // can't tell "still exactly the documented gap" from "a different gap
    // now". Pin the exact, currently-true gate.
    let reports = run_golden();
    for name in ["davis-before", "davis-after"] {
        let r = target(&reports, name);
        assert_eq!(
            verdict_kind(r, "database.driver"),
            "OptionNotFound",
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

// --- H1.1 review fixes ---------------------------------------------------
//
// Four more correctness edge cases, again found by review of green code,
// not by this suite failing: gate 4's outcome filter silently dropped
// unresolvable test values into OBA001 (the exact fail-closed principle
// H1 applied to the *default* side, broken on the *test-value* side);
// the null-predicate outcome function was still comparing raw text
// instead of classifying the actual AST node, so a non-literal default
// or test value could be wrongly declared "definitely non-null"; an
// empty `watch = []` silently produced a clean PASS; and a missing
// module/test file (or any other manifest/I/O failure) exited through
// Rust's default error code, indistinguishable from a genuine FINDING.

// H1.1-1: an unresolvable test value must not silently vanish from gate 4's
// evidence search. qux's default is a *known* `false` (isolating this from
// the default-side bug below); the test assigns `builtins.elem "x" [ "x"
// "y" ]`, which is `true` at runtime but syntactically opaque.

#[test]
fn h1_1_unresolved_test_value_is_inconclusive_not_oba001() {
    let reports = run_golden();
    let r = target(&reports, "c11-unresolved-test-value");
    assert_eq!(
        verdict_kind(r, "qux"),
        "TestValueUnresolved",
        "an unresolvable matching test value must never be silently treated as 'no evidence'"
    );
}

// H1.1-2: predicate_outcome() for null predicates must classify the
// default's actual AST node, not compare its raw text to the string
// "null". baz's default is `if builtins.pathExists /etc/synth-baz then
// null else "/run/other.sock"` -- its text is never literally "null", but
// its ValueClass must still be Unknown (not "definitely non-null" from a
// naive text mismatch), so gate 3 must not silently resolve it.

#[test]
fn h1_1_null_predicate_default_is_ast_classified_not_textual() {
    let reports = run_golden();
    let r = target(&reports, "c10-null-expression-default");
    assert_eq!(
        verdict_kind(r, "baz"),
        "DefaultUnresolved",
        "a non-literal default under a null predicate must be Unknown, not guessed non-null from text"
    );
}

// H1.1-3: a target with watch = [] must never silently pass -- it checks
// nothing and would otherwise exit 0 with zero findings and zero
// inconclusive, the same "detector died, green light stayed on" failure
// mode `watch` itself exists to prevent, one level up the stack.

#[test]
fn h1_1_empty_watch_is_tool_error() {
    let (exit_code, stderr) = run_manifest_raw("targets/tool-error-empty-watch.toml");
    assert_eq!(
        exit_code, 3,
        "an empty watch list must be a tool/config error (3), never a passing analysis"
    );
    assert!(
        stderr.contains("watch"),
        "error message should name the actual problem; got: {stderr}"
    );
}

// H1.1-4: a missing module/test file must be TOOL_ERROR (3), never
// FINDING (1) -- before this fix, any `?`-propagated I/O error fell
// through to Rust's default error exit code, which happened to collide
// with this tool's own definition of "genuine OBA001 finding".

#[test]
fn h1_1_missing_file_is_tool_error_not_finding() {
    let (exit_code, stderr) = run_manifest_raw("targets/tool-error-missing-file.toml");
    assert_eq!(
        exit_code, 3,
        "a missing module/test file must be TOOL_ERROR (3), never FINDING (1)"
    );
    assert!(
        stderr.contains("this-file-does-not-exist"),
        "error should name the actual missing file; got: {stderr}"
    );
}

// --- H1.2 review fixes ---------------------------------------------------
//
// One more architectural gap, plus a clap exit-code contract leak and a
// verify-upstream.sh bug (not covered by Rust tests, checked separately).

/// Runs the binary with arbitrary raw CLI args (not the usual
/// `--targets <manifest> --json`) -- for testing clap-level parse failures
/// themselves, before any manifest is even looked at.
fn run_raw_args(args: &[&str]) -> i32 {
    let output = Command::new(env!("CARGO_BIN_EXE_oba"))
        .current_dir(manifest_dir())
        .args(args)
        .output()
        .expect("failed to run oba binary");
    output
        .status
        .code()
        .expect("process exited via signal, not code")
}

// H1.2-P0: a watched option that's never mentioned anywhere the walker CAN
// see must be TestConfigUnresolved when the test config also contains
// something the walker can't see into (imports, an alias) -- not silently
// treated as "not activated" just because nothing matching was *found*.
// The old code would OBA001 these exactly as if the option had genuinely
// never been touched, which is a different claim: "not observed in the
// part of the config I can read" vs "not activated". A census run over
// real modules would otherwise produce OBA001 findings that are actually
// just "config came in through `imports`".

#[test]
fn h1_2_imports_makes_result_inconclusive_not_oba001() {
    let reports = run_golden();
    let r = target(&reports, "c15-imports");
    assert_eq!(
        verdict_kind(r, "foo"),
        "TestConfigUnresolved",
        "an `imports` in the test config must not let OBA001 claim the option was never touched"
    );
}

#[test]
fn h1_2_instance_alias_makes_result_inconclusive_not_oba001() {
    let reports = run_golden();
    let r = target(&reports, "c16-instance-alias");
    assert_eq!(
        verdict_kind(r, "foo"),
        "TestConfigUnresolved",
        "`nodes.machine = someAlias;` must not let OBA001 claim the option was never touched"
    );
}

// Positive control: an explicit, provable opposite-outcome assignment
// found elsewhere in the same test file must win over an unrelated
// opacity site -- existential evidence beats incompleteness that has
// nothing to do with where the evidence was actually found.

#[test]
fn h1_2_explicit_opposite_evidence_wins_over_unrelated_import() {
    let reports = run_golden();
    let r = target(&reports, "c17-opposite-wins-over-import");
    assert_eq!(
        verdict_kind(r, "foo"),
        "PASS",
        "a provable transition must not be downgraded to inconclusive just because \
         an unrelated node elsewhere in the file also has an import"
    );
}

// H1.2-P1: clap's own Error::exit() (invoked internally by the old
// Cli::parse()) bypasses this tool's 4-state exit-code contract entirely --
// a bad CLI invocation exited with *clap's* code (2 for a usage error),
// indistinguishable from this tool's own INCONCLUSIVE (also 2), even
// though no analysis ever ran. Fixed with try_parse() mapping onto
// TOOL_ERROR (3) explicitly.

// Positive assertion for the opacity detector itself: c17 has both a real
// assignment AND an unrelated import, so `test_config_opacity` must
// actually be non-empty -- a detector that silently stopped finding
// opacity sites would make h1_2_explicit_opposite_evidence_wins_over_
// unrelated_import pass for the wrong reason (no opacity to lose to,
// rather than opposite evidence correctly outranking it).

#[test]
fn positive_assertions_c17_opacity_detector_found_the_import() {
    let reports = run_golden();
    let r = target(&reports, "c17-opposite-wins-over-import");
    let opacity = r["test_config_opacity"].as_array().unwrap();
    assert!(
        !opacity.is_empty(),
        "the opacity detector must have found the `other` node's import"
    );
    assert!(
        opacity.iter().any(|o| o["instance"] == "other"),
        "opacity should be attributed to the `other` instance; got {opacity:?}"
    );
}

#[test]
fn h1_2_cli_parse_failure_is_tool_error_not_inconclusive() {
    assert_eq!(
        run_raw_args(&["--bogus-flag-that-does-not-exist"]),
        3,
        "an unrecognized CLI flag must be TOOL_ERROR (3), not clap's own default exit code"
    );
    assert_eq!(
        run_raw_args(&[]),
        3,
        "a missing required --targets argument must be TOOL_ERROR (3)"
    );
}

// --- H1.3 review fixes ---------------------------------------------------
//
// The sharpest gap across all three review rounds: OBA001 only ever
// proved absence of *observed* evidence, not absence of activation. Two
// completely ordinary nixosTest patterns -- `imports`, and a `let`-bound
// alias for a node's config -- made the real assignment structurally
// invisible to the walker, which silently found nothing and OBA001'd
// exactly as if the option had never been touched. Confirmed against real
// nixpkgs, not just plausible synthetic scenarios: 559 files under
// nixos/tests use the nested `nodes = {` form, 95 use the
// `import ./make-test-python.nix (...)` wrapper -- a census would have
// produced industrial quantities of false findings. The walker is now an
// explicit three-context state machine (TestSpecRoot / ModuleRoot /
// ConfigTree, see the comment above `scan_test_assignments`) instead of
// one function accumulating special cases.

// Both forms of nodes/containers binding (flat, used everywhere else in
// this corpus, and nested `nodes = { machine = ...; };`) must produce
// identical verdicts for identical content.

#[test]
fn h1_3_nested_nodes_form_finds_opposite_evidence() {
    let reports = run_golden();
    let r = target(&reports, "c18-nested-nodes-opposite");
    assert_eq!(verdict_kind(r, "foo"), "PASS");
}

#[test]
fn h1_3_nested_nodes_form_still_oba001s_correctly() {
    let reports = run_golden();
    let r = target(&reports, "c19-nested-nodes-same");
    assert_eq!(verdict_kind(r, "foo"), "OBA001");
}

// An entirely unrecognized root wrapper must never let content it can't
// see into masquerade as "nothing happened here" -- TestConfigUnresolved,
// not OBA001, regardless of what's actually inside the wrapper.

#[test]
fn h1_3_unknown_root_wrapper_is_inconclusive_not_oba001() {
    let reports = run_golden();
    let r = target(&reports, "c20-unknown-root");
    assert_eq!(verdict_kind(r, "foo"), "TestConfigUnresolved");
}

// The dominant real-world nixosTest wrapper (`import ./make-test-python.nix
// (<lambda-or-attrset>)`) must be recognized and unwrapped, not treated
// as just another unknown root.

#[test]
fn h1_3_make_test_python_wrapper_is_unwrapped() {
    let reports = run_golden();
    let r = target(&reports, "c21-make-test-wrapper");
    assert_eq!(
        verdict_kind(r, "foo"),
        "PASS",
        "the import ./make-test-python.nix (...) wrapper must be transparently unwrapped, not treated as opaque"
    );
}

// Module-root `config = { ... };` must normalize into the SAME option-path
// namespace as top-level shorthand -- `services.synth.foo`, not
// `config.services.synth.foo`, which would never match anything.

#[test]
fn h1_3_module_root_config_key_normalizes_path() {
    let reports = run_golden();
    let r = target(&reports, "c22-module-root-config");
    assert_eq!(
        verdict_kind(r, "foo"),
        "PASS",
        "config = {{ ... }}'s contents must normalize into the same option-path namespace as top-level shorthand"
    );
}

// But `config = <opaque expression>;` (a function call, not a literal
// attrset) must still be treated as hiding anything -- `config` is only
// special because it's module-root syntax, not because the key happens
// to be named "config" anywhere in the tree.

#[test]
fn h1_3_module_root_opaque_config_is_inconclusive() {
    let reports = run_golden();
    let r = target(&reports, "c23-module-root-opaque-config");
    assert_eq!(verdict_kind(r, "foo"), "TestConfigUnresolved");
}

// `inherit` and a dynamic `${...}` attribute name are both genuinely
// untraceable by this walker -- neither should be silently skipped as if
// nothing was there.

#[test]
fn h1_3_inherit_and_dynamic_attrpath_are_opacity_not_silence() {
    let reports = run_golden();
    let r = target(&reports, "c24-inherit-and-dynamic");
    assert_eq!(
        verdict_kind(r, "foo"),
        "TestConfigUnresolved",
        "inherit / a dynamic attrpath must register as opacity, never silently skipped"
    );
    let opacity = r["test_config_opacity"].as_array().unwrap();
    assert!(
        opacity.len() >= 2,
        "both the inherit and the dynamic attrpath should be recorded; got {opacity:?}"
    );
}
