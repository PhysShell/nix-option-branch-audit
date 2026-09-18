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
    // root) sneaking into the envelope would show up here.
    let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["mode", "schema_version", "summary", "targets", "tool"]);
}
