//! S5-F2: real, frozen regression fixture for NixOS/nixpkgs#475112
//! (`nixos/cgit: add gitHttpBackend options`).
//!
//! Root cause: `services.cgit = mkOption { type = attrsOf (submodule
//! ({...}: {options = {...};})); };` -- `option_prefix = [services,
//! cgit]` is concrete (no wildcard), but every real test assignment to
//! any of the submodule's own leaves necessarily carries a concrete
//! instance key (`services.cgit."no-git-http-backend.localhost"...`)
//! the predicate's own reference (`cfg.gitHttpBackend.enable`, generic
//! across every instance) never mentions. `path_matches_prefix`'s own
//! exact-length check (`full.len() == prefix.len() + suffix.len()`)
//! rejected the extra segment outright, so the real, deliberate
//! `gitHttpBackend.enable = false;` assignment (and the testScript's own
//! real exercise of the disabled branch) was silently invisible to
//! witness matching -- a false OBA001 ("uncovered option branch") on a
//! real declaration the test corpus genuinely, deliberately covers.
//!
//! Fixed by `option_prefix_is_instance_keyed_submodule` (verifies,
//! structurally, that `option_prefix`'s own true-root declaration really
//! is `attrsOf`/`listOf (submodule ...)`-shaped) gating
//! `path_matches_prefix`'s own new `allow_instance_key` tolerance for
//! exactly one extra, concrete segment between `option_prefix` and the
//! watched suffix.
//!
//! Head-only test (matches how the historical defect was actually
//! observed: `check --root head-root`, not a diff -- the base commit
//! predates `gitHttpBackend` entirely, so a base-side check would only
//! show `OptionNotFound`, not exercise the witness-matching fix at all).

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

const FIXTURE: &str = "fixtures/synthetic/f2-cgit-githttpbackend-instance-keyed-witness";

#[test]
fn real_cgit_pr475112_githttpbackend_enable_is_witnessed_not_a_false_finding() {
    let out = oba(&[
        "check",
        "--root",
        &format!("{FIXTURE}/head"),
        "--targets",
        &format!("{FIXTURE}/targets.toml"),
        "--json",
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["summary"]["pass"], 1, "expected the predicate to PASS (witnessed); full output: {v}");
    assert_eq!(v["summary"]["finding"], 0, "the false OBA001 must be gone; full output: {v}");

    let target = &v["targets"][0];
    let verdict = &target["verdicts"][0];
    assert_eq!(verdict["verdict"], "PASS");
    assert_eq!(verdict["option"], "gitHttpBackend.enable");

    // Real evidence, not merely a passing verdict with no basis: the
    // exact assignment from the real test file, at its own real path
    // and span, carrying the concrete instance key.
    let evidence = &verdict["evidence"][0];
    assert_eq!(
        evidence["path"],
        serde_json::json!(["services", "cgit", "no-git-http-backend.localhost", "gitHttpBackend", "enable"]),
        "evidence must be the real, concrete-instance-qualified assignment path; full verdict: {verdict}"
    );
    assert_eq!(evidence["value_source"], "false");
    assert_eq!(evidence["span"]["line"], 58);

    let attempt = &verdict["predicate_attempts"][0];
    assert_eq!(attempt["witnessed"], true, "the predicate_attempts entry itself must be witnessed=true; full verdict: {verdict}");
}

/// The base commit predates `gitHttpBackend` entirely -- confirms the
/// fixture's own baseline shape (OptionNotFound is the CORRECT,
/// unaffected result there; this fix is entirely about the head side's
/// own witness matching, not declaration discovery).
#[test]
fn real_cgit_pr475112_base_commit_has_no_githttpbackend_option_yet() {
    let out = oba(&[
        "check",
        "--root",
        &format!("{FIXTURE}/base"),
        "--targets",
        &format!("{FIXTURE}/targets.toml"),
        "--json",
    ]);
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OptionNotFound");
}
