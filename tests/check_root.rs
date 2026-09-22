//! PR B commits 2-3: `oba check --root` and the versioned JSON envelope.
//!
//! Three things this file exists to prove, black-box (real subprocess,
//! real filesystem -- no unit-level shortcuts): (1) `--root` is a real
//! security boundary, not a decorative `PathBuf::join`, so a
//! `module`/`test` path that's absolute, escapes via `../`, or escapes
//! via a symlink all become a TOOL_ERROR (exit 3) rather than a read from
//! wherever they resolve to; (2) the legacy flat `--targets` invocation
//! and the new `check --root . --targets` invocation are the exact same
//! analysis, not two independently-maintained paths that happen to agree
//! today; (3) `schema_version: 1`'s envelope shape is what it claims to
//! be.

use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn oba(args: &[&str], cwd: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oba"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("failed to run oba binary")
}

fn exit_code(out: &Output) -> i32 {
    out.status.code().expect("process exited via signal")
}

#[test]
fn absolute_module_path_is_tool_error() {
    let out = oba(
        &[
            "check",
            "--root",
            "fixtures/synthetic",
            "--targets",
            "fixtures/synthetic/root-escape/t-absolute.toml",
        ],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("must be relative to --root"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn dotdot_escape_is_tool_error() {
    let out = oba(
        &[
            "check",
            "--root",
            "fixtures/synthetic",
            "--targets",
            "fixtures/synthetic/root-escape/t-dotdot.toml",
        ],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("escapes --root"), "unexpected stderr: {stderr}");
}

#[test]
fn symlink_escape_is_tool_error() {
    // fixtures/synthetic/root-escape/root/evil -> .. (a real, checked-in
    // relative symlink) -- --root is the `root/` subdirectory specifically
    // so canonicalizing `evil/decoy.nix` resolves outside it.
    let out = oba(
        &[
            "check",
            "--root",
            "fixtures/synthetic/root-escape/root",
            "--targets",
            "fixtures/synthetic/root-escape/t-symlink.toml",
        ],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("escapes --root"), "unexpected stderr: {stderr}");
}

#[test]
fn missing_root_is_tool_error_not_inconclusive() {
    let out = oba(
        &[
            "check",
            "--root",
            "this-root-does-not-exist-anywhere",
            "--targets",
            "targets/golden.toml",
        ],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
}

/// The actual equivalence claim: legacy `--targets` and `check --root .
/// --targets` must produce the byte-identical JSON report, not just "the
/// same verdicts under manual inspection" -- run against the real golden
/// manifest (mixed PASS/OBA001/inconclusive), not a trivial fixture.
#[test]
fn legacy_invocation_equals_check_root_dot() {
    let legacy = oba(&["--targets", "targets/golden.toml", "--json"], &manifest_dir());
    let check = oba(
        &["check", "--root", ".", "--targets", "targets/golden.toml", "--json"],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&legacy), exit_code(&check));
    let legacy_json: Value = serde_json::from_slice(&legacy.stdout).expect("legacy stdout is JSON");
    let check_json: Value = serde_json::from_slice(&check.stdout).expect("check stdout is JSON");
    assert_eq!(
        legacy_json, check_json,
        "legacy --targets and `check --root . --targets` must be the same analysis"
    );
}

/// `--root` must be a genuine alternative to cwd, not merely tolerated
/// when it happens to equal cwd: running from a completely different
/// directory with an explicit (absolute) --root must reproduce the exact
/// same report as running from inside the repo.
#[test]
fn check_works_from_a_different_cwd_with_explicit_root() {
    let root = manifest_dir();
    let targets = root.join("targets/golden.toml");
    let elsewhere = std::env::temp_dir();

    let from_repo = oba(&["--targets", "targets/golden.toml", "--json"], &root);
    let from_elsewhere = oba(
        &[
            "check",
            "--root",
            root.to_str().unwrap(),
            "--targets",
            targets.to_str().unwrap(),
            "--json",
        ],
        &elsewhere,
    );

    assert_eq!(exit_code(&from_repo), exit_code(&from_elsewhere));
    let a: Value = serde_json::from_slice(&from_repo.stdout).unwrap();
    let b: Value = serde_json::from_slice(&from_elsewhere.stdout).unwrap();
    assert_eq!(a, b);
}

/// Two runs of the same invocation must produce byte-identical stdout --
/// no HashMap/HashSet iteration order, no timestamp, no PID, leaking into
/// the report. Cheap insurance against a nondeterminism regression that
/// would otherwise only show up as CI flakiness much later.
#[test]
fn repeated_runs_are_byte_stable() {
    let a = oba(&["check", "--root", ".", "--targets", "targets/golden.toml", "--json"], &manifest_dir());
    let b = oba(&["check", "--root", ".", "--targets", "targets/golden.toml", "--json"], &manifest_dir());
    assert_eq!(a.stdout, b.stdout);
}

/// `--census` is explicitly out of scope for this PR -- still reachable
/// only through the legacy flat invocation, unaffected by `check`/`--root`.
#[test]
fn census_is_unaffected_by_the_check_subcommand() {
    let out = oba(&["--census", "fixtures/real"], &manifest_dir());
    assert_eq!(exit_code(&out), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
}

/// The frozen part of `schema_version: 1`, pinned explicitly rather than
/// left to be implied by the other tests only diffing whole-document
/// equality: envelope keys, `mode: "check"`, `tool.name`, and that
/// `tool.version` actually matches the binary's own crate version (not
/// hand-copied/stale). `summary.pass` + `summary.finding` +
/// `summary.inconclusive` must also add up to the total verdict count --
/// the envelope's own internal consistency, not just "the fields exist".
#[test]
fn schema_v1_envelope_shape_is_what_it_claims() {
    let out = oba(
        &["check", "--root", ".", "--targets", "targets/golden.toml", "--json"],
        &manifest_dir(),
    );
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");

    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["mode"], "check");
    assert_eq!(v["tool"]["name"], "oba");
    assert_eq!(v["tool"]["version"], env!("CARGO_PKG_VERSION"));

    let pass = v["summary"]["pass"].as_u64().unwrap();
    let finding = v["summary"]["finding"].as_u64().unwrap();
    let inconclusive = v["summary"]["inconclusive"].as_u64().unwrap();
    let total_verdicts: u64 = v["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["verdicts"].as_array().unwrap().len() as u64)
        .sum();
    assert_eq!(pass + finding + inconclusive, total_verdicts);

    // No stray top-level keys -- CI cruft (timestamp/hostname/pid/absolute
    // root) sneaking into the envelope would show up here. S4-F1 added
    // `unavailable_targets` (purely additive to schema_version: 1, always
    // present, empty here since targets/golden.toml has nothing missing --
    // see the s4f1_* tests below for its non-empty shape).
    let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        ["mode", "schema_version", "summary", "targets", "tool", "unavailable_targets"]
    );
    assert_eq!(v["unavailable_targets"].as_array().unwrap().len(), 0);
    assert_eq!(v["summary"]["unavailable"].as_u64().unwrap(), 0);
}

// ---------------------------------------------------------------------
// S4-F1: `oba check`'s multi-target partial-failure semantics.
//
// The real incident this whole section exists to fix: nixpkgs PR
// #543492 deleted a whole greeter submodule; the S4 evaluation's own
// combined manifest named that now-deleted target ALONGSIDE an
// unrelated, still-valid one, and `oba check --root head-root
// --targets targets.toml` aborted the WHOLE invocation (TOOL_ERROR,
// exit 3, EMPTY stdout) -- discarding the unrelated target's own real
// PASS result too. Fixed via `partition_targets_by_availability`
// (reuses `resolve_within_root_if_exists`, already built for exactly
// this "genuinely does not exist" distinction by `run_audit_diff`'s own
// base/head partitioning -- not a new mechanism, an existing one given
// a second caller). The core invariant every test below is checking one
// facet of: ONE target being unavailable must never destroy the
// results of every OTHER target in the same manifest -- and every
// OTHER resolution failure (an absolute path, a `../`/symlink escape,
// `--root` itself unreadable, a malformed manifest) must stay exactly
// as fatal as it was before this fix.
// ---------------------------------------------------------------------

/// The real, historical case, reproduced with real content: PR #543492's
/// own real `nixos/modules/services/x11/display-managers/lightdm.nix` +
/// `nixos/tests/lightdm.nix` at its real head SHA
/// (`aa970620a51824a5361565d16a18882fcf1a3759`, fetched via `gh api`),
/// alongside the real, byte-for-byte manifest that PR actually named
/// `lightdm-enso-greeter`'s module under -- a file that genuinely does
/// not exist at that commit (the PR itself deleted it; not a fixture
/// mistake). Before S4-F1: exit 3, empty stdout. After: `lightdm` is
/// analyzed and reported with its real PASS verdict, unaffected by its
/// unrelated sibling target's own unavailability.
#[test]
fn s4f1_reproduces_543492_lightdm_survives_enso_greeter_deletion() {
    let out = oba(
        &[
            "check",
            "--root",
            "fixtures/regressions/543492-lightdm-enso-greeter-removed/root",
            "--targets",
            "fixtures/regressions/543492-lightdm-enso-greeter-removed/targets.toml",
            "--json",
        ],
        &manifest_dir(),
    );
    assert_eq!(
        exit_code(&out),
        2,
        "an unavailable target alongside a valid one must be INCONCLUSIVE-class (2), not TOOL_ERROR (3); stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty(), "stderr should be empty on a clean partial run: {}", String::from_utf8_lossy(&out.stderr));

    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    let targets = v["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 1, "only the genuinely analyzable target should appear in targets");
    assert_eq!(targets[0]["name"], "lightdm");
    let verdicts: Vec<&str> = targets[0]["verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|verdict| verdict["verdict"].as_str().unwrap())
        .collect();
    assert_eq!(verdicts, ["PASS"], "lightdm's own enable verdict must survive unaffected");

    let unavailable = v["unavailable_targets"].as_array().unwrap();
    assert_eq!(unavailable.len(), 1);
    assert_eq!(unavailable[0]["name"], "lightdm-enso-greeter");
    assert!(
        unavailable[0]["reason"]
            .as_str()
            .unwrap()
            .contains("lightdm-greeters/enso-os.nix"),
        "reason should name the real missing path: {}",
        unavailable[0]["reason"]
    );
    assert_eq!(v["summary"]["unavailable"].as_u64().unwrap(), 1);
}

/// The general (non-#543492-specific) positive case, with a mainline
/// fixture: one real target, one whose module is missing.
#[test]
fn s4f1_one_deleted_target_does_not_lose_the_other_targets_result() {
    let out = oba(
        &[
            "check",
            "--root",
            ".",
            "--targets",
            "targets/s4f1-mixed-available-unavailable.toml",
            "--json",
        ],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["targets"].as_array().unwrap().len(), 1);
    assert_eq!(v["targets"][0]["name"], "kimai-after");
    assert_eq!(v["unavailable_targets"].as_array().unwrap().len(), 1);
    assert_eq!(v["unavailable_targets"][0]["name"], "deleted-target");
    assert!(
        v["unavailable_targets"][0]["reason"].as_str().unwrap().contains("module:"),
        "reason should name module, not test, as the missing half"
    );
}

/// Ordering control: deleted target first, two real targets follow.
#[test]
fn s4f1_deleted_target_position_first_is_unaffected() {
    let out = oba(
        &["check", "--root", ".", "--targets", "targets/s4f1-deleted-first.toml", "--json"],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    let mut names: Vec<&str> = v["targets"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    names.sort();
    assert_eq!(names, ["davis-before", "kimai-after"]);
    assert_eq!(v["unavailable_targets"].as_array().unwrap().len(), 1);
    assert_eq!(v["unavailable_targets"][0]["name"], "deleted-target");
}

/// Ordering control: deleted target in the middle.
#[test]
fn s4f1_deleted_target_position_middle_is_unaffected() {
    let out = oba(
        &["check", "--root", ".", "--targets", "targets/s4f1-deleted-middle.toml", "--json"],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    let mut names: Vec<&str> = v["targets"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    names.sort();
    assert_eq!(names, ["davis-before", "kimai-after"]);
    assert_eq!(v["unavailable_targets"].as_array().unwrap().len(), 1);
    assert_eq!(v["unavailable_targets"][0]["name"], "deleted-target");
}

/// Ordering control: deleted target last.
#[test]
fn s4f1_deleted_target_position_last_is_unaffected() {
    let out = oba(
        &["check", "--root", ".", "--targets", "targets/s4f1-deleted-last.toml", "--json"],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    let mut names: Vec<&str> = v["targets"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    names.sort();
    assert_eq!(names, ["davis-before", "kimai-after"]);
    assert_eq!(v["unavailable_targets"].as_array().unwrap().len(), 1);
    assert_eq!(v["unavailable_targets"][0]["name"], "deleted-target");
}

/// Multiple different unavailable targets, alongside one real target --
/// every unavailable target must be recorded, not just the first found.
#[test]
fn s4f1_multiple_deleted_targets_are_all_recorded() {
    let out = oba(
        &["check", "--root", ".", "--targets", "targets/s4f1-multiple-deleted.toml", "--json"],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["targets"].as_array().unwrap().len(), 1);
    assert_eq!(v["targets"][0]["name"], "kimai-after");
    let mut unavailable_names: Vec<&str> = v["unavailable_targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["name"].as_str().unwrap())
        .collect();
    unavailable_names.sort();
    assert_eq!(unavailable_names, ["deleted-target-one", "deleted-target-two"]);
    assert_eq!(v["summary"]["unavailable"].as_u64().unwrap(), 2);
}

/// Negative control: EVERY target in a multi-target manifest is
/// unavailable -- must stay the pre-S4-F1 hard failure (TOOL_ERROR,
/// exit 3), since there is no other target's result left to preserve.
/// Generalizes `h1_1_missing_file_is_tool_error_not_finding` (a
/// single-target manifest) to the multi-target case.
#[test]
fn s4f1_all_targets_unavailable_stays_a_hard_tool_error() {
    let out = oba(
        &["check", "--root", ".", "--targets", "targets/s4f1-all-deleted.toml", "--json"],
        &manifest_dir(),
    );
    assert_eq!(
        exit_code(&out),
        3,
        "a manifest where NOTHING is analyzable must stay TOOL_ERROR (3): stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty(), "no partial report should be emitted when nothing was analyzable");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("deleted-target-one") && stderr.contains("deleted-target-two"),
        "error should name both unavailable targets: {stderr}"
    );
}

/// The single-target case, named explicitly for S4-F1's own record
/// (the same invariant `h1_1_missing_file_is_tool_error_not_finding`
/// already covers under its own, older name) -- confirms this exact
/// pre-existing fixture's behavior is genuinely unchanged by S4-F1, not
/// merely un-asserted.
#[test]
fn s4f1_single_target_manifest_with_missing_module_stays_a_hard_tool_error() {
    let out = oba(
        &["check", "--root", ".", "--targets", "targets/tool-error-missing-file.toml", "--json"],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(out.stdout.is_empty());
}

/// `test` missing (module present) must be caught and reported the same
/// way `module` missing is, with a reason that correctly names `test:`.
#[test]
fn s4f1_missing_test_file_is_also_a_soft_unavailable_result() {
    let out = oba(
        &["check", "--root", ".", "--targets", "targets/s4f1-test-file-missing.toml", "--json"],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["targets"].as_array().unwrap().len(), 1);
    assert_eq!(v["targets"][0]["name"], "kimai-after");
    assert_eq!(v["unavailable_targets"].as_array().unwrap().len(), 1);
    assert_eq!(v["unavailable_targets"][0]["name"], "test-missing-target");
    assert!(
        v["unavailable_targets"][0]["reason"].as_str().unwrap().starts_with("test:"),
        "reason should name test, not module, as the missing half: {}",
        v["unavailable_targets"][0]["reason"]
    );
}

/// The most important negative control: a genuinely-missing module
/// (soft, on its own) alongside a SEPARATE target whose module is an
/// ABSOLUTE path -- a real `--root` security-boundary violation. This
/// must stay a hard, whole-invocation TOOL_ERROR (3), exactly as before
/// S4-F1: a security-relevant resolution failure must never be silently
/// downgraded into a partial success merely because an ordinary missing
/// module happens to sit in the same manifest.
#[test]
fn s4f1_negative_control_absolute_path_escape_stays_fatal_even_alongside_a_deleted_target() {
    let out = oba(
        &[
            "check",
            "--root",
            ".",
            "--targets",
            "targets/s4f1-negative-control-escape.toml",
            "--json",
        ],
        &manifest_dir(),
    );
    assert_eq!(
        exit_code(&out),
        3,
        "an absolute-path escape must stay fatal even alongside an ordinary deleted target: stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("must be relative to --root"),
        "the real security error must still be the one reported: {stderr}"
    );
}

/// A second negative control in the same spirit: the manifest file
/// itself is not valid TOML. Entirely upstream of
/// `partition_targets_by_availability` (which never runs) -- must stay
/// exactly as fatal as before S4-F1.
#[test]
fn s4f1_negative_control_malformed_manifest_stays_fatal() {
    let out = oba(
        &["check", "--root", ".", "--targets", "targets/s4f1-malformed-manifest.toml", "--json"],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("parsing targets manifest"),
        "error should name the real cause (a manifest parse failure), not something else: {stderr}"
    );
}

/// The human (non-JSON) renderer must show unavailable targets too, not
/// only the JSON envelope.
#[test]
fn s4f1_human_renderer_shows_unavailable_targets() {
    let out = oba(
        &["check", "--root", ".", "--targets", "targets/s4f1-mixed-available-unavailable.toml"],
        &manifest_dir(),
    );
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("unavailable=1"), "human summary line should show the unavailable count: {stdout}");
    assert!(stdout.contains("deleted-target"), "human output should name the unavailable target: {stdout}");
    assert!(stdout.contains("UNAVAILABLE"), "human output should mark it distinctly from an analyzed target: {stdout}");
    assert!(stdout.contains("kimai-after"), "the real, analyzed target must still be rendered: {stdout}");
}

/// Deterministic ordering: two runs of the same partial-failure manifest
/// must produce byte-identical stdout, same discipline
/// `repeated_runs_are_byte_stable` already applies to the all-clean case.
#[test]
fn s4f1_partial_run_is_byte_stable_across_repeats() {
    let a = oba(
        &["check", "--root", ".", "--targets", "targets/s4f1-multiple-deleted.toml", "--json"],
        &manifest_dir(),
    );
    let b = oba(
        &["check", "--root", ".", "--targets", "targets/s4f1-multiple-deleted.toml", "--json"],
        &manifest_dir(),
    );
    assert_eq!(a.stdout, b.stdout);
}
