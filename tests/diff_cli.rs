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

// =============================================================================
// S5-F1: real angrr PR #471312 false-"Unchanged" defect (S5's own
// confirmed defect 2, fixtures/s5-live-pr-shadow/s5-confirmed-defects.md)
// and its hostile controls. Root cause: `scan_options`'s nested-form
// walk gave EVERY top-level `options = {...}` block it found a fresh
// empty starting path, with no check for whether the block actually
// belonged to `option_prefix`'s own real tree -- a `let`/`rec`-bound
// auxiliary submodule's own leaves (never nested inside a tracked
// `mkOption {...}` call) got recorded with the SAME bare path as a
// genuinely `option_prefix`-anchored declaration elsewhere, and
// `compare()`'s `TargetIdentity` (keyed purely on the watched STRING,
// never on which declaration it resolved to) then compared them as
// "the same identity" -- both landing on the same `VerdictKind`
// produced `Unchanged`, inventing continuity nobody ever established.
// =============================================================================

/// The real reproducer: NixOS/nixpkgs PR #471312, base
/// ca696276378b844d888872c9045d89e9b58cc85a -> head
/// 62ea5b9ae7d4329475089f60232ee1426d9547a9, module/test content
/// fetched verbatim from GitHub at those exact SHAs. Before the fix:
/// `unchanged: 1`. After the fix: a real `verdict_changed`
/// (predicate_not_found -> option_not_found), since the real top-level
/// `services.angrr.period` (removed via `mkRemovedOptionModule`) no
/// longer falsely matches the unrelated, structurally distinct
/// `services.angrr.settings.temporary-root-policies.<name>.period`
/// nested inside the new, named `let`-bound `temporaryRootPolicyOptions`
/// submodule.
#[test]
fn real_angrr_pr471312_false_unchanged_is_now_a_real_verdict_change() {
    let out = oba(&[
        "diff",
        "--base-root",
        "fixtures/synthetic/f1-angrr-period-collision/base",
        "--head-root",
        "fixtures/synthetic/f1-angrr-period-collision/head",
        "--targets",
        "fixtures/synthetic/f1-angrr-period-collision/targets.toml",
        "--json",
    ]);
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["summary"]["unchanged"], 0, "the real defect: this used to be 1");
    assert_eq!(v["summary"]["changed"], 1);
    assert_eq!(
        v["summary"]["verdict_transitions"]["predicate_not_found->option_not_found"],
        1
    );
    assert_eq!(v["targets"][0]["diff"]["base"]["verdict"]["verdict"], "PredicateNotFound");
    assert_eq!(v["targets"][0]["diff"]["head"]["verdict"]["verdict"], "OptionNotFound");
}

/// Hostile control (b): the same bug SHAPE with entirely different,
/// generic names -- proves the fix is a real structural check, not
/// hardcoded to angrr's own identifiers ("period",
/// "temporaryRootPolicyOptions", ...). A real top-level `widgetTimeout`
/// is removed; an unrelated, newly-introduced `widgetTimeout` survives
/// nested inside a named `let`-bound `unrelatedHelperOptions` submodule
/// that has nothing to do with `option_prefix`.
#[test]
fn synthetic_generic_collision_removed_field_plus_unrelated_survivor_is_not_unchanged() {
    let base_dir = std::env::temp_dir().join("oba-f1-generic-collision-base");
    let head_dir = std::env::temp_dir().join("oba-f1-generic-collision-head");
    std::fs::create_dir_all(&base_dir).unwrap();
    std::fs::create_dir_all(&head_dir).unwrap();

    std::fs::write(
        base_dir.join("module.nix"),
        r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
        in
        {
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
            widgetTimeout = lib.mkOption {
              type = lib.types.int;
              default = 30;
            };
          };
        }
        "#,
    )
    .unwrap();
    std::fs::write(base_dir.join("test.nix"), "{ nodes = {}; testScript = \"\"; }").unwrap();

    std::fs::write(
        head_dir.join("module.nix"),
        r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
          unrelatedHelperOptions = {
            options = {
              widgetTimeout = lib.mkOption {
                type = lib.types.nullOr lib.types.int;
                default = null;
              };
            };
          };
        in
        {
          imports = [
            (lib.mkRemovedOptionModule [ "services" "widget" "widgetTimeout" ] "use services.widget.profiles instead")
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
            profiles = lib.mkOption {
              type = lib.types.attrsOf (lib.types.submodule unrelatedHelperOptions);
              default = { };
            };
          };
        }
        "#,
    )
    .unwrap();
    std::fs::write(head_dir.join("test.nix"), "{ nodes = {}; testScript = \"\"; }").unwrap();

    let targets = base_dir.join("targets.toml");
    std::fs::write(
        &targets,
        r#"
        [[target]]
        name = "widget"
        module = "module.nix"
        test = "test.nix"
        cfg_ident = "cfg"
        option_prefix = ["services", "widget"]
        watch = ["widgetTimeout"]
        "#,
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_oba"))
        .args([
            "diff",
            "--base-root",
            base_dir.to_str().unwrap(),
            "--head-root",
            head_dir.to_str().unwrap(),
            "--targets",
            targets.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("failed to run oba binary");
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["summary"]["unchanged"], 0, "generic collision must not be hidden either");
    assert_eq!(v["summary"]["changed"], 1);
    assert_eq!(v["targets"][0]["diff"]["head"]["verdict"]["verdict"], "OptionNotFound");

    let _ = std::fs::remove_dir_all(&base_dir);
    let _ = std::fs::remove_dir_all(&head_dir);
}

/// Hostile control (c): a genuinely unchanged field, same structural
/// identity on both sides -- must STILL report `Unchanged`. This fix
/// must never turn a real non-event into a false positive.
#[test]
fn genuine_unchanged_field_under_same_structural_identity_stays_unchanged() {
    let dir = std::env::temp_dir().join("oba-f1-genuine-unchanged");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("module.nix"),
        r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
        in
        {
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
            widgetTimeout = lib.mkOption {
              type = lib.types.int;
              default = 30;
            };
          };
        }
        "#,
    )
    .unwrap();
    std::fs::write(dir.join("test.nix"), "{ nodes = {}; testScript = \"\"; }").unwrap();
    let targets = dir.join("targets.toml");
    std::fs::write(
        &targets,
        r#"
        [[target]]
        name = "widget"
        module = "module.nix"
        test = "test.nix"
        cfg_ident = "cfg"
        option_prefix = ["services", "widget"]
        watch = ["widgetTimeout"]
        "#,
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_oba"))
        .args([
            "diff",
            "--base-root",
            dir.to_str().unwrap(),
            "--head-root",
            dir.to_str().unwrap(),
            "--targets",
            targets.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("failed to run oba binary");
    // Exit 2, not 0: widgetTimeout is never set by test.nix on either
    // side, so its own verdict is inconclusive-class (matching this
    // file's own `inconclusive_analysis_on_either_side_yields_exit_2_never_1`)
    // -- orthogonal to the base/head COMPARISON itself, which is what
    // this control actually checks below and is genuinely `unchanged`.
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["summary"]["unchanged"], 1);
    assert_eq!(v["summary"]["changed"], 0);

    let _ = std::fs::remove_dir_all(&dir);
}

/// Hostile control (d): a real `mkRenamedOptionModule` relocation (the
/// SAME general shape as the already-known, separately-tracked
/// guacamole defect, #462487 -- explicitly OUT OF SCOPE for this fix)
/// must be completely UNAFFECTED by this change. This fix targets only
/// the named-`let`/`rec`-bound-auxiliary-submodule class; it does not
/// add, and must not accidentally add, any rename/move detection. The
/// tool's own `TargetIdentity` doc comment states this is deliberate:
/// "no rename/move tracking ... surfaces as Removed + Added, never a
/// detected 'same target, different path'".
#[test]
fn mk_renamed_option_module_relocation_stays_unaffected_by_this_fix() {
    let base_dir = std::env::temp_dir().join("oba-f1-rename-base");
    let head_dir = std::env::temp_dir().join("oba-f1-rename-head");
    std::fs::create_dir_all(&base_dir).unwrap();
    std::fs::create_dir_all(&head_dir).unwrap();

    std::fs::write(
        base_dir.join("module.nix"),
        r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
        in
        {
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
            oldName = lib.mkOption {
              type = lib.types.int;
              default = 30;
            };
          };
        }
        "#,
    )
    .unwrap();
    std::fs::write(base_dir.join("test.nix"), "{ nodes = {}; testScript = \"\"; }").unwrap();

    std::fs::write(
        head_dir.join("module.nix"),
        r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
        in
        {
          imports = [
            (lib.mkRenamedOptionModule [ "services" "widget" "oldName" ] [ "services" "widget" "newName" ])
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
            newName = lib.mkOption {
              type = lib.types.int;
              default = 30;
            };
          };
        }
        "#,
    )
    .unwrap();
    std::fs::write(head_dir.join("test.nix"), "{ nodes = {}; testScript = \"\"; }").unwrap();

    let targets = base_dir.join("targets.toml");
    std::fs::write(
        &targets,
        r#"
        [[target]]
        name = "widget"
        module = "module.nix"
        test = "test.nix"
        cfg_ident = "cfg"
        option_prefix = ["services", "widget"]
        watch = ["oldName"]
        "#,
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_oba"))
        .args([
            "diff",
            "--base-root",
            base_dir.to_str().unwrap(),
            "--head-root",
            head_dir.to_str().unwrap(),
            "--targets",
            targets.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("failed to run oba binary");
    assert_eq!(exit_code(&out), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    // Still not detected as a rename/move -- this fix does not claim to
    // fix that (guacamole's own defect is a separate, unauthorized-here
    // class). `oldName` genuinely resolves to OptionNotFound at head,
    // same as before this fix -- the point is this stays UNCHANGED
    // behavior, not a new capability.
    assert_eq!(v["targets"][0]["diff"]["head"]["verdict"]["verdict"], "OptionNotFound");

    let _ = std::fs::remove_dir_all(&base_dir);
    let _ = std::fs::remove_dir_all(&head_dir);
}
