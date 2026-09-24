//! S5-F3: real, frozen regression fixture for NixOS/nixpkgs#462487
//! (`nixos/guacamole: client option defined in server module`), plus
//! the two mandatory real hard-removal negative controls (sshd
//! `#509507`, gollum `#466806`) that must remain UNCHANGED.
//!
//! Root cause: `services.guacamole-server.logbackXml` (a real
//! declaration, base side, with a real, never-tested `!= null`
//! predicate -- a genuine OBA001 finding) is relocated to
//! `services.guacamole-client.logbackXml` at head, via a real, static
//! `lib.mkRenamedOptionModule [ "services" "guacamole-server"
//! "logbackXml" ] [ "services" "guacamole-client" "logbackXml" ]` call
//! in the server module's own `imports = [...]`. Before this fix,
//! neither `mkRenamedOptionModule` nor `mkRemovedOptionModule` had any
//! handler anywhere in this codebase, so `run_target`'s own gate-1 miss
//! fell straight through to the generic `Verdict::OptionNotFound`,
//! rendered as "FINDING BECAME INCONCLUSIVE / no mkOption declaration
//! found / declaration: not found" -- materially misleading, since the
//! source already contained explicit, machine-readable rename
//! provenance.
//!
//! See `fixtures/s5-f3/investigation.md` for the full root-cause trace.

use serde_json::Value;
use std::process::Command;

const FIXTURE: &str = "fixtures/synthetic/f3-guacamole-logbackxml-relocation";

fn oba(args: &[&str]) -> (Value, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_oba"))
        .args(args)
        .output()
        .expect("failed to run oba binary");
    let code = out.status.code().expect("process exited via signal");
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout not JSON ({e}): {}", String::from_utf8_lossy(&out.stdout)));
    (v, code)
}

#[test]
fn real_guacamole_pr462487_base_has_a_real_oba001_finding() {
    let (v, code) = oba(&[
        "check",
        "--root",
        &format!("{FIXTURE}/base"),
        "--targets",
        &format!("{FIXTURE}/targets.toml"),
        "--json",
    ]);
    assert_eq!(code, 1, "full output: {v}");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OBA001");
    assert_eq!(verdict["option"], "logbackXml");
}

#[test]
fn real_guacamole_pr462487_head_is_relocated_not_a_bare_option_not_found() {
    let (v, code) = oba(&[
        "check",
        "--root",
        &format!("{FIXTURE}/head"),
        "--targets",
        &format!("{FIXTURE}/targets.toml"),
        "--json",
    ]);
    assert_eq!(code, 2, "full output: {v}");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OptionRelocated");
    assert_eq!(verdict["option"], "logbackXml");
    assert_eq!(
        verdict["to"],
        serde_json::json!(["services", "guacamole-client", "logbackXml"]),
        "destination must be the real, complete path -- full verdict: {verdict}"
    );
    assert_eq!(
        verdict["destination_confirmed"], true,
        "the real destination declaration genuinely exists in guacamole-client.nix and must be found across files; full verdict: {verdict}"
    );
    assert_eq!(verdict["helper_form"], "mkRenamedOptionModule");
    assert_eq!(verdict["migration_source_file"], "nixos/modules/services/web-apps/guacamole-server.nix");
}

#[test]
fn real_guacamole_pr462487_audit_diff_renders_relocation_not_generic_inconclusive() {
    let dir = std::env::temp_dir().join("oba-f3-guac-audit-diff");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let summary_path = dir.join("summary.md");

    let out = Command::new(env!("CARGO_BIN_EXE_oba"))
        .args([
            "audit-diff",
            "--base-root",
            &format!("{FIXTURE}/base"),
            "--head-root",
            &format!("{FIXTURE}/head"),
            "--targets",
            &format!("{FIXTURE}/targets.toml"),
            "--format",
            "json",
            "--summary-path",
        ])
        .arg(&summary_path)
        .output()
        .expect("failed to run oba binary");
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");

    // The bucket itself, and its own count, are UNCHANGED -- this is
    // still, mechanically, a Finding -> Inconclusive-class transition.
    // Only the underlying evidence and the rendered heading carry the
    // new, honest distinction.
    assert_eq!(v["summary"]["finding_became_inconclusive"], 1);
    assert_eq!(
        v["summary"]["oba_verdict_transitions"]["oba001->option_relocated"], 1,
        "full summary: {}", v["summary"]
    );

    let notable = &v["summary"]["notable"][0];
    assert_eq!(notable["bucket"], "finding_became_inconclusive");
    assert_eq!(notable["code"], "OBA-RELOCATED");

    let markdown = std::fs::read_to_string(&summary_path).expect("summary.md written");
    assert!(
        markdown.contains("FINDING'S OPTION WAS RENAMED/RELOCATED"),
        "expected the relocation-specific heading, got:\n{markdown}"
    );
    assert!(
        !markdown.contains("no mkOption declaration found"),
        "the old, materially misleading OptionNotFound message must not appear here anymore:\n{markdown}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Mandatory negative control (sshd `#509507`, real, previously
/// adjudicated as a CORRECT hard removal -- `mkRemovedOptionModule`, no
/// replacement path). Must remain plain `OptionNotFound`, byte-for-byte
/// unchanged from before this round -- never reclassified as a
/// relocation.
#[test]
fn real_sshd_pr509507_banner_hard_removal_stays_option_not_found() {
    let (v, code) = oba(&[
        "check",
        "--root",
        "fixtures/synthetic/f3-removal-negative-controls/sshd/head",
        "--targets",
        "fixtures/synthetic/f3-removal-negative-controls/sshd/targets.toml",
        "--json",
    ]);
    assert_eq!(code, 2, "full output: {v}");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OptionNotFound", "full verdict: {verdict}");
    assert_eq!(
        verdict.as_object().unwrap().len(),
        2,
        "OptionNotFound's own JSON shape (option + verdict tag only) must stay byte-for-byte unchanged; full verdict: {verdict}"
    );
}

/// Mandatory negative control (gollum `#466806`, real, previously
/// adjudicated as a CORRECT hard removal). Same requirement as sshd
/// above.
#[test]
fn real_gollum_pr466806_local_time_hard_removal_stays_option_not_found() {
    let (v, code) = oba(&[
        "check",
        "--root",
        "fixtures/synthetic/f3-removal-negative-controls/gollum/head",
        "--targets",
        "fixtures/synthetic/f3-removal-negative-controls/gollum/targets.toml",
        "--json",
    ]);
    assert_eq!(code, 2, "full output: {v}");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OptionNotFound", "full verdict: {verdict}");
    assert_eq!(
        verdict.as_object().unwrap().len(),
        2,
        "OptionNotFound's own JSON shape (option + verdict tag only) must stay byte-for-byte unchanged; full verdict: {verdict}"
    );
}
