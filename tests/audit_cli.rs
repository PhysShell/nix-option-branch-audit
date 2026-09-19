//! P3a/P3b: `oba audit`/`oba audit-diff` CLI acceptance tests.
//!
//! Deliberately does NOT re-test OBA's own `analyze()`/`compare()`
//! semantics (`tests/golden.rs`/`tests/diff_cli.rs` already cover that
//! exhaustively) or CDC's own comparison semantics
//! (`src/cdc.rs`'s own `mod ce12_tests`), or CDC's own diff algebra
//! (`compare_cdc`/`compare_cdc_result`, exhaustively covered offline in
//! `src/main.rs`'s own `mod tests`). This file exercises only the NEW
//! surface each command itself adds at the real CLI level: the unified
//! schema, manifest validation for `[[cdc_target]]`, and the real
//! end-to-end wiring of both engines through one command (P3a), plus
//! `audit-diff`'s own real base/head wiring and exit-code philosophy
//! (P3b). Tests that need a real CDC candidate evaluated are
//! `#[ignore]`d (real network access, `fetchTarball`), matching this
//! whole project's own convention; everything reachable offline
//! (manifest validation, the OBA-only path against local fixtures) runs
//! in the default tier.

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

/// A real, local nixpkgs checkout -- fetched via the same real
/// `fetchTarball` technique `src/cdc.rs`'s own
/// `real_nixpkgs_source_local_path_produces_the_identical_real_result_as_pinned_rev`
/// test uses, so this test file needs no separately-vendored checkout.
/// Real network access; only called from `#[ignore]`d tests.
fn real_local_nixpkgs_checkout() -> String {
    let out = Command::new("nix")
        .args([
            "eval",
            "--impure",
            "--raw",
            "--expr",
            r#"builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/68740713a1d5904edf9ba92a998a522b1b6ce080.tar.gz""#,
        ])
        .output()
        .expect("spawning nix eval to materialize a real local nixpkgs checkout");
    assert!(out.status.success(), "nix eval failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

// --- offline: manifest validation, zero network/nix-eval needed ---

#[test]
fn an_unknown_cdc_candidate_name_is_a_real_tool_error_before_any_nix_eval() {
    let out = oba(&[
        "audit",
        "--root",
        ".",
        "--targets",
        "fixtures/synthetic/audit-cdc/unknown-candidate.toml",
    ]);
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not a known GeneratedConfigArtifact candidate"), "stderr: {stderr}");
}

#[test]
fn a_manifest_with_neither_target_nor_cdc_target_is_a_tool_error() {
    let out = oba(&["audit", "--root", ".", "--targets", "fixtures/synthetic/audit-cdc/empty.toml"]);
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no [[target]] or [[cdc_target]] entries"), "stderr: {stderr}");
}

#[test]
fn a_duplicate_cdc_target_name_is_a_tool_error() {
    let out = oba(&[
        "audit",
        "--root",
        ".",
        "--targets",
        "fixtures/synthetic/audit-cdc/duplicate-cdc-name.toml",
    ]);
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("duplicate [[cdc_target]] name"), "stderr: {stderr}");
}

/// An OBA-only manifest (zero `[[cdc_target]]`) against the real,
/// already-vendored kimai/before fixture -- the same real historical
/// OBA001 this whole project's own golden corpus already establishes,
/// reached through `audit` instead of `check`, with zero real network
/// access needed (the OBA half never touches CDC's own nixpkgs-tree
/// requirement).
#[test]
fn oba_only_audit_reaches_the_real_historical_oba001() {
    let out = oba(&[
        "audit",
        "--root",
        "fixtures/kimai/before",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
        "--format",
        "json",
    ]);
    assert_eq!(exit_code(&out), 1, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["mode"], "audit");
    assert_eq!(v["summary"]["finding"], 1);
    assert_eq!(v["results"][0]["engine"], "oba");
    assert_eq!(v["results"][0]["code"], "OBA001");
    assert_eq!(v["results"][0]["verdict"], "finding");
    assert!(v["results"][0]["cdc_evidence"].is_null());
}

#[test]
fn oba_only_audit_reaches_the_real_historical_pass_after_the_fix() {
    let out = oba(&[
        "audit",
        "--root",
        "fixtures/kimai/after",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
        "--format",
        "json",
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["summary"]["pass"], 1);
    assert_eq!(v["results"][0]["verdict"], "pass");
    assert!(v["results"][0]["code"].is_null(), "a Pass must carry no problem code");
}

#[test]
fn human_and_json_output_agree_on_the_same_oba_only_audit() {
    let json_out = oba(&[
        "audit",
        "--root",
        "fixtures/kimai/before",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
        "--format",
        "json",
    ]);
    let human_out = oba(&[
        "audit",
        "--root",
        "fixtures/kimai/before",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
    ]);
    assert_eq!(exit_code(&json_out), exit_code(&human_out));
    let v: Value = serde_json::from_slice(&json_out.stdout).unwrap();
    let human = String::from_utf8_lossy(&human_out.stdout);
    assert!(human.contains(&format!("finding={}", v["summary"]["finding"])));
    assert!(human.contains("OBA001"));
}

#[test]
fn audit_json_is_byte_stable_across_repeated_runs() {
    let args = [
        "audit",
        "--root",
        "fixtures/kimai/before",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
        "--format",
        "json",
    ];
    let a = oba(&args);
    let b = oba(&args);
    assert_eq!(a.stdout, b.stdout);
}

// --- real: needs a real `nix` binary and network access ---

#[test]
#[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
fn cdc_only_audit_reaches_a_real_clean_pass() {
    let local_root = real_local_nixpkgs_checkout();
    let out = oba(&[
        "audit",
        "--root",
        &local_root,
        "--targets",
        "fixtures/synthetic/audit-cdc/cdc-only.toml",
        "--format",
        "json",
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["summary"]["pass"], 1);
    assert_eq!(v["results"][0]["engine"], "cdc");
    assert_eq!(v["results"][0]["target"], "spacecookie");
    assert_eq!(v["results"][0]["cdc_evidence"]["binding"]["proof_depth"], "byte_exact");
    assert!(v["results"][0]["oba_evidence"].is_null());
}

/// The real end-to-end proof this whole round exists for: ONE command,
/// ONE `--root` (a real local nixpkgs checkout), ONE manifest mixing a
/// real OBA target and a real CDC target, unified under one schema.
#[test]
#[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
fn combined_audit_unifies_a_real_oba_result_and_a_real_cdc_result() {
    let local_root = real_local_nixpkgs_checkout();
    let out = oba(&[
        "audit",
        "--root",
        &local_root,
        "--targets",
        "fixtures/synthetic/audit-cdc/combined.toml",
        "--format",
        "json",
    ]);
    // real, current upstream nixpkgs' own kimai test may or may not
    // still activate the watched branch -- either way this must exit
    // 0/1/2, never 3 (a real tool error), and must carry exactly one
    // real "oba" result plus one real "cdc" result.
    assert_ne!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    let results = v["results"].as_array().expect("results is an array");
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|r| r["engine"] == "oba" && r["target"] == "database.socket"));
    let cdc_result =
        results.iter().find(|r| r["engine"] == "cdc").expect("a real cdc result is present");
    assert_eq!(cdc_result["target"], "unpackerr");
    assert_eq!(cdc_result["verdict"], "pass");
}

#[test]
#[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
fn cdc_finding_reaches_the_cli_with_a_real_provenance_chain() {
    // akkoma's own real, stable Finding (C-E1.2b/c) -- re-verified once
    // more here, through the CLI surface itself rather than a direct
    // `run_cdc_candidate` call.
    let local_root = real_local_nixpkgs_checkout();
    let manifest = "[[cdc_target]]\nname = \"akkoma\"\n";
    let manifest_path = std::env::temp_dir().join("oba-audit-cli-akkoma-test.toml");
    std::fs::write(&manifest_path, manifest).unwrap();
    let out = oba(&[
        "audit",
        "--root",
        &local_root,
        "--targets",
        manifest_path.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(exit_code(&out), 1, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["results"][0]["code"], "CDC001");
    assert_eq!(v["results"][0]["verdict"], "finding");
    let provenance = v["results"][0]["provenance"].as_array().unwrap();
    assert!(provenance.iter().any(|p| p.as_str().unwrap().contains("upload_dir")));
    let _ = std::fs::remove_file(&manifest_path);
}

// =======================================================================
// P3b: `oba audit-diff` CLI acceptance tests.
// =======================================================================

/// Invariant 1 (`audit-diff A A` -> only Unchanged), OBA half, fully
/// offline (local fixtures, no CDC target).
#[test]
fn oba_only_self_diff_is_all_unchanged() {
    let out = oba(&[
        "audit-diff",
        "--base-root",
        "fixtures/kimai/before",
        "--head-root",
        "fixtures/kimai/before",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
        "--format",
        "json",
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["mode"], "audit-diff");
    assert_eq!(v["summary"]["unchanged"], 1);
    assert_eq!(v["summary"]["changed"], 0);
    assert_eq!(v["oba"][0]["diff"]["kind"], "unchanged");
}

/// The real historical transition (the same one `tests/diff_cli.rs`
/// already proves for `oba diff` itself), reached through `audit-diff`
/// instead -- confirms the unified command reuses OBA's own real
/// `compare()` correctly, not a reimplementation that happens to look
/// similar.
#[test]
fn oba_only_diff_reaches_the_real_historical_oba001_to_pass_transition() {
    let out = oba(&[
        "audit-diff",
        "--base-root",
        "fixtures/kimai/before",
        "--head-root",
        "fixtures/kimai/after",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
        "--format",
        "json",
    ]);
    // invariant 9: a real Changed/Finding-shaped transition must NEVER
    // make this exit 1 -- `audit-diff` only ever exits 0/2/3.
    assert_eq!(exit_code(&out), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["summary"]["changed"], 1);
    assert_eq!(v["summary"]["oba_verdict_transitions"]["oba001->pass"], 1);
    assert_eq!(v["oba"][0]["diff"]["kind"], "changed");
    assert_eq!(v["oba"][0]["diff"]["changes"][0], "verdict_changed");
}

#[test]
fn audit_diff_exit_code_is_2_when_either_side_is_inconclusive_never_1() {
    // golden.toml's own real davis-before/after targets are honestly
    // OptionNotFound (inconclusive-shaped) on both sides -- comparing
    // it against itself must still succeed (a real Changed is not a
    // diff failure) but exit 2, matching `oba diff`'s own already-
    // established precedent for the exact same real manifest.
    let out = oba(&[
        "audit-diff",
        "--base-root",
        ".",
        "--head-root",
        ".",
        "--targets",
        "targets/golden.toml",
    ]);
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn an_unknown_cdc_candidate_name_is_a_real_tool_error_for_audit_diff_too() {
    // manifest validation is shared with `audit` -- reused, not
    // reimplemented.
    let out = oba(&[
        "audit-diff",
        "--base-root",
        ".",
        "--head-root",
        ".",
        "--targets",
        "fixtures/synthetic/audit-cdc/unknown-candidate.toml",
    ]);
    assert_eq!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn audit_diff_human_and_json_output_agree() {
    let json_out = oba(&[
        "audit-diff",
        "--base-root",
        "fixtures/kimai/before",
        "--head-root",
        "fixtures/kimai/after",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
        "--format",
        "json",
    ]);
    let human_out = oba(&[
        "audit-diff",
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
    assert!(human.contains(&format!("changed={}", v["summary"]["changed"])));
    assert!(human.contains("CHANGED"));
}

#[test]
fn audit_diff_json_is_byte_stable_across_repeated_runs() {
    // invariant 2 (order/determinism), exercised at the real CLI level.
    let args = [
        "audit-diff",
        "--base-root",
        "fixtures/kimai/before",
        "--head-root",
        "fixtures/kimai/after",
        "--targets",
        "fixtures/synthetic/diff-kimai/targets.toml",
        "--format",
        "json",
    ];
    let a = oba(&args);
    let b = oba(&args);
    assert_eq!(a.stdout, b.stdout);
}

/// Invariant 1, CDC half: `audit-diff` with the SAME real root on both
/// sides for a real CDC candidate must be all Unchanged.
#[test]
#[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
fn cdc_only_self_diff_is_unchanged() {
    let local_root = real_local_nixpkgs_checkout();
    let out = oba(&[
        "audit-diff",
        "--base-root",
        &local_root,
        "--head-root",
        &local_root,
        "--targets",
        "fixtures/synthetic/audit-cdc/cdc-only.toml",
        "--format",
        "json",
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["summary"]["unchanged"], 1);
    assert_eq!(v["cdc"][0]["diff"]["kind"], "unchanged");
}

/// The real combined proof: ONE `audit-diff` invocation, real OBA half
/// (local fixtures via `--targets`) and real CDC half (against the SAME
/// real freshly-materialized nixpkgs checkout on both sides) unified
/// under one report.
#[test]
#[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
fn combined_audit_diff_unifies_real_oba_and_real_cdc_self_diffs() {
    let local_root = real_local_nixpkgs_checkout();
    let out = oba(&[
        "audit-diff",
        "--base-root",
        &local_root,
        "--head-root",
        &local_root,
        "--targets",
        "fixtures/synthetic/audit-cdc/combined.toml",
        "--format",
        "json",
    ]);
    assert_ne!(exit_code(&out), 3, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    // same root on both sides -> both halves must be entirely unchanged.
    assert_eq!(v["oba"][0]["diff"]["kind"], "unchanged");
    assert_eq!(v["cdc"][0]["diff"]["kind"], "unchanged");
}
