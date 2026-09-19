//! P3a: `oba audit` CLI acceptance tests.
//!
//! Deliberately does NOT re-test OBA's own `analyze()`/`compare()`
//! semantics (`tests/golden.rs`/`tests/diff_cli.rs` already cover that
//! exhaustively) or CDC's own comparison semantics
//! (`src/cdc.rs`'s own `mod ce12_tests`). This file exercises only the
//! NEW surface `audit` itself adds: the unified schema, manifest
//! validation for `[[cdc_target]]`, and the real end-to-end wiring of
//! both engines through one command. Tests that need a real CDC
//! candidate evaluated are `#[ignore]`d (real network access,
//! `fetchTarball`), matching this whole project's own convention;
//! everything reachable offline (manifest validation, the OBA-only
//! path against local fixtures) runs in the default tier.

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
