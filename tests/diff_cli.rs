//! PR D2: `oba diff` CLI acceptance tests.
//!
//! Deliberately does NOT re-test compare()'s own semantics -- D1's
//! `#[cfg(test)]` corpus (the 49-way VerdictKind matrix, the mirror-
//! under-swap property, etc.) already covers that exhaustively. This
//! file only exercises the NEW surface D2 actually adds: two independent
//! roots, the security boundary applied to each, --targets staying
//! cwd-relative, CompareError reaching the CLI as a real exit code, human
//! and JSON output agreeing, and one real end-to-end proof against the
//! actual historical kimai OBA001 -> PASS transition.

use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn oba(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oba"))
        .current_dir(manifest_dir())
        .args(args)
        .output()
        .expect("failed to run oba binary")
}

fn exit_code(out: &Output) -> i32 {
    out.status.code().expect("process exited via signal")
}

/// The real, historical fixture: fixtures/kimai/before -> fixtures/kimai/after
/// is the actual PhysShell/nixpkgs 5530e24f2 -> 37f81efa4 fix, already
/// pinned by tests/golden.rs as two SEPARATE static targets. Here the
/// exact same transition is reached through two independent --*-root
/// analyses of ONE manifest instead -- proving the two roots are actually
/// analyzed independently, not e.g. both accidentally pointed at the same
/// tree.
#[test]
fn two_roots_are_analyzed_independently_real_kimai_transition() {
    let out = oba(&[
        "diff",
        "--base-root",
        "fixtures/kimai/before",
        "--head-root",
        "fixtures/kimai/after",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
        "--json",
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["mode"], "diff");
    assert_eq!(v["summary"]["changed"], 1);
    assert_eq!(v["summary"]["unchanged"], 0);
    assert_eq!(v["summary"]["verdict_transitions"]["oba001->pass"], 1);
    assert_eq!(v["targets"][0]["diff"]["kind"], "changed");
}

/// --base-root gets the identical `resolve_within_root` boundary `check
/// --root` already has -- reusing the exact same fixtures PR B/D1 used to
/// prove that for `check`, not a new escape corpus. Base fails before
/// head is ever analyzed, so --head-root is irrelevant here on purpose.
#[test]
fn base_root_absolute_path_is_tool_error() {
    let out = oba(&[
        "diff",
        "--base-root",
        "fixtures/synthetic",
        "--head-root",
        ".",
        "--targets",
        "fixtures/synthetic/root-escape/t-absolute.toml",
    ]);
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stderr).contains("must be relative to --root"));
}

#[test]
fn base_root_dotdot_escape_is_tool_error() {
    let out = oba(&[
        "diff",
        "--base-root",
        "fixtures/synthetic",
        "--head-root",
        ".",
        "--targets",
        "fixtures/synthetic/root-escape/t-dotdot.toml",
    ]);
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stderr).contains("escapes --root"));
}

/// The head-specific case, isolated: base/module.nix + base/test.nix are
/// real, valid files -- --base-root's own analysis succeeds cleanly, so
/// this specifically proves --head-root enforces the same boundary
/// independently, not merely "whichever root happens to be checked
/// first". head/module.nix is a checked-in relative symlink resolving
/// outside --head-root's own subtree.
#[test]
fn head_root_symlink_escape_is_tool_error_even_when_base_root_is_clean() {
    let out = oba(&[
        "diff",
        "--base-root",
        "fixtures/synthetic/diff-root-escape/base",
        "--head-root",
        "fixtures/synthetic/diff-root-escape/head",
        "--targets",
        "fixtures/synthetic/diff-root-escape/targets.toml",
    ]);
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stderr).contains("escapes --root"));
}

/// A missing module/test file under either root must produce exactly the
/// same TOOL_ERROR class `check` already has -- `analyze()`'s own `?`,
/// not special diff-only handling.
#[test]
fn missing_file_under_a_root_is_tool_error_same_as_check() {
    let out = oba(&[
        "diff",
        "--base-root",
        ".",
        "--head-root",
        ".",
        "--targets",
        "targets/tool-error-missing-file.toml",
    ]);
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
}

/// Duplicate target identity (two different target blocks that happen to
/// collide once matched by (module, test, cfg_ident, option_prefix,
/// watched_path)) must reach the CLI as a real TOOL_ERROR, not a crash or
/// a silently-guessed pairing.
#[test]
fn duplicate_identity_is_a_tool_error_at_the_cli() {
    let out = oba(&[
        "diff",
        "--base-root",
        "fixtures/kimai/before",
        "--head-root",
        "fixtures/kimai/before",
        "--targets",
        "fixtures/synthetic/diff-duplicate/targets.toml",
    ]);
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stderr).contains("duplicate target identity"));
}

/// --targets stays resolved relative to cwd even when both roots are
/// elsewhere and absolute -- the one deliberate asymmetry from the design
/// note, exercised for real (not just documented).
#[test]
fn targets_manifest_is_cwd_relative_not_root_relative() {
    let root = manifest_dir();
    let out = Command::new(env!("CARGO_BIN_EXE_oba"))
        .current_dir(std::env::temp_dir())
        .args([
            "diff",
            "--base-root",
            root.join("fixtures/kimai/before").to_str().unwrap(),
            "--head-root",
            root.join("fixtures/kimai/after").to_str().unwrap(),
            "--targets",
            root.join("fixtures/synthetic/diff-kimai/targets.toml").to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("failed to run oba binary");
    assert_eq!(exit_code(&out), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["summary"]["changed"], 1);
}

#[test]
fn diff_json_is_byte_stable_across_repeated_runs() {
    let args = [
        "diff",
        "--base-root",
        "fixtures/kimai/before",
        "--head-root",
        "fixtures/kimai/after",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
        "--json",
    ];
    let a = oba(&args);
    let b = oba(&args);
    assert_eq!(a.stdout, b.stdout);
}

/// Human and JSON rendering must describe the SAME `ComparisonReport`,
/// not independently recompute what "changed" means -- checked by
/// cross-referencing the human summary line's counts against the JSON
/// summary for the identical invocation.
#[test]
fn human_and_json_output_agree_on_the_same_comparison() {
    let json_out = oba(&[
        "diff",
        "--base-root",
        "fixtures/kimai/before",
        "--head-root",
        "fixtures/kimai/after",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
        "--json",
    ]);
    let human_out = oba(&[
        "diff",
        "--base-root",
        "fixtures/kimai/before",
        "--head-root",
        "fixtures/kimai/after",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
    ]);
    assert_eq!(exit_code(&json_out), exit_code(&human_out));

    let v: Value = serde_json::from_slice(&json_out.stdout).unwrap();
    let human = String::from_utf8_lossy(&human_out.stdout);
    assert!(human.contains(&format!("unchanged={}", v["summary"]["unchanged"])));
    assert!(human.contains(&format!("added={}", v["summary"]["added"])));
    assert!(human.contains(&format!("removed={}", v["summary"]["removed"])));
    assert!(human.contains(&format!("changed={}", v["summary"]["changed"])));
    assert!(
        human.contains("CHANGED  database.socket  oba001 -> pass"),
        "human output: {human}"
    );
}

/// The real end-to-end claim: filesystem -> analyze() x2 -> compare() ->
/// rendering, with base == head, must report every entry Unchanged and
/// exit 0 -- the CLI-level counterpart to D1's own
/// `compare_of_identical_report_is_all_unchanged` unit test.
#[test]
fn diff_of_identical_roots_is_all_unchanged() {
    let out = oba(&[
        "diff",
        "--base-root",
        ".",
        "--head-root",
        ".",
        "--targets",
        "targets/clean.toml",
        "--json",
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["summary"]["added"], 0);
    assert_eq!(v["summary"]["removed"], 0);
    assert_eq!(v["summary"]["changed"], 0);
    assert!(v["summary"]["unchanged"].as_u64().unwrap() > 0);
}

/// `golden.toml` mixes a real OBA001 finding with davis's honest
/// OptionNotFound (inconclusive-shaped) on both sides -- comparing it
/// against itself must still succeed (a real Changed is not a diff
/// failure), but exit 2, not 0, because both sides' own analysis has an
/// inconclusive verdict. Exit 1 must never appear for `diff` at all.
#[test]
fn inconclusive_analysis_on_either_side_yields_exit_2_never_1() {
    let out = oba(&[
        "diff",
        "--base-root",
        ".",
        "--head-root",
        ".",
        "--targets",
        "targets/golden.toml",
    ]);
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
}
