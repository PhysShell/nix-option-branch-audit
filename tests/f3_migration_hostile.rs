//! S5-F3 hostile controls for `scan_migrations`/`resolve_unambiguous_rename`/
//! `locate_migration_destination`. Synthetic fixtures (not the real
//! guacamole/sshd/gollum PRs -- see `tests/f3_guacamole.rs` for those),
//! each isolating exactly one semantic situation the mandate requires
//! distinguished. See `fixtures/s5-f3/investigation.md` for the full
//! root-cause trace this fix is built from.

use serde_json::Value;
use std::process::Command;

/// Writes an arbitrary set of files under a fresh temp dir named `name`
/// (each `(relative_path, content)` pair), runs `check --root <dir>
/// --targets <dir>/targets.toml --json`, returns the parsed JSON and
/// exit code.
fn run_check(name: &str, files: &[(&str, &str)], targets_toml: &str) -> (Value, i32) {
    let dir = std::env::temp_dir().join(format!("oba-f3-hostile-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (rel, content) in files {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
    }
    let targets_path = dir.join("targets.toml");
    std::fs::write(&targets_path, targets_toml).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_oba"))
        .args([
            "check",
            "--root",
            dir.to_str().unwrap(),
            "--targets",
            targets_path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("failed to run oba binary");
    let code = out.status.code().expect("process exited via signal");
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout not JSON ({e}): {}", String::from_utf8_lossy(&out.stdout)));
    let _ = std::fs::remove_dir_all(&dir);
    (v, code)
}

const WIDGET_TARGETS: &str = r#"
    [[target]]
    name = "widget"
    module = "module.nix"
    test = "test.nix"
    cfg_ident = "cfg"
    option_prefix = ["services", "widget"]
    watch = ["oldLeaf"]
"#;

/// Hostile control (1): exact rename, old full path -> new full path,
/// correctly classified as a relocation.
#[test]
fn exact_rename_full_path_is_classified_as_relocation() {
    let module = r#"
        { config, lib, ... }:
        {
          imports = [
            (lib.mkRenamedOptionModule [ "services" "widget" "oldLeaf" ] [ "services" "elsewhere" "newLeaf" ])
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
          };
        }
    "#;
    let test = "{ nodes = {}; testScript = \"\"; }";
    let (v, code) = run_check("exact-rename", &[("module.nix", module), ("test.nix", test)], WIDGET_TARGETS);
    assert_eq!(code, 2, "full output: {v}");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OptionRelocated", "full verdict: {verdict}");
    assert_eq!(verdict["to"], serde_json::json!(["services", "elsewhere", "newLeaf"]));
    // No real declaration exists anywhere under this root at the
    // destination path -- must NEVER be claimed confirmed.
    assert_eq!(verdict["destination_confirmed"], false, "full verdict: {verdict}");
}

/// Hostile control (2): same LEAF name ("oldLeaf"), unrelated rename
/// (different full source path) -- must NOT affect the watched path.
/// Full-path equality is required, never a leaf-only match.
#[test]
fn same_leaf_unrelated_rename_does_not_match() {
    let module = r#"
        { config, lib, ... }:
        {
          imports = [
            (lib.mkRenamedOptionModule [ "services" "unrelated" "oldLeaf" ] [ "services" "unrelated" "newLeaf" ])
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
          };
        }
    "#;
    let test = "{ nodes = {}; testScript = \"\"; }";
    let (v, _code) = run_check("same-leaf-unrelated", &[("module.nix", module), ("test.nix", test)], WIDGET_TARGETS);
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(
        verdict["verdict"], "OptionNotFound",
        "a rename for services.unrelated.oldLeaf must never be mistaken for services.widget.oldLeaf merely because both end in 'oldLeaf'; full verdict: {verdict}"
    );
}

/// Hostile control (3): same PREFIX ("services.widget"), different leaf
/// -- must not match.
#[test]
fn same_prefix_different_leaf_does_not_match() {
    let module = r#"
        { config, lib, ... }:
        {
          imports = [
            (lib.mkRenamedOptionModule [ "services" "widget" "differentLeaf" ] [ "services" "elsewhere" "newLeaf" ])
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
          };
        }
    "#;
    let test = "{ nodes = {}; testScript = \"\"; }";
    let (v, _code) = run_check("same-prefix-different-leaf", &[("module.nix", module), ("test.nix", test)], WIDGET_TARGETS);
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OptionNotFound", "full verdict: {verdict}");
}

/// Hostile control (4): reverse direction. A `new -> old` edge must
/// never satisfy a query for the FORWARD `old -> new` relationship --
/// migration edges are directional, matched purely by `from_path`.
#[test]
fn reverse_direction_edge_does_not_satisfy_forward_query() {
    let module = r#"
        { config, lib, ... }:
        {
          imports = [
            (lib.mkRenamedOptionModule [ "services" "elsewhere" "newLeaf" ] [ "services" "widget" "oldLeaf" ])
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
          };
        }
    "#;
    let test = "{ nodes = {}; testScript = \"\"; }";
    let (v, _code) = run_check("reverse-direction", &[("module.nix", module), ("test.nix", test)], WIDGET_TARGETS);
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(
        verdict["verdict"], "OptionNotFound",
        "an edge whose TO_PATH happens to equal the watched path must never be mistaken for a FROM_PATH match; full verdict: {verdict}"
    );
}

/// Hostile control (5): hard removal (`mkRemovedOptionModule`) must
/// never be classified as a relocation.
#[test]
fn hard_removal_is_not_classified_as_relocation() {
    let module = r#"
        { config, lib, ... }:
        {
          imports = [
            (lib.mkRemovedOptionModule [ "services" "widget" "oldLeaf" ] "no replacement")
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
          };
        }
    "#;
    let test = "{ nodes = {}; testScript = \"\"; }";
    let (v, _code) = run_check("hard-removal", &[("module.nix", module), ("test.nix", test)], WIDGET_TARGETS);
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OptionNotFound", "full verdict: {verdict}");
    assert_eq!(verdict.as_object().unwrap().len(), 2, "OptionNotFound's own shape must stay exactly {{option, verdict}}; full verdict: {verdict}");
}

/// Hostile control (6): plain disappearance -- no declaration and no
/// migration directive of any kind -- stays ordinary
/// `OptionNotFound`/inconclusive behavior, unaffected by this round.
#[test]
fn plain_disappearance_stays_ordinary_option_not_found() {
    let module = r#"
        { config, lib, ... }:
        {
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
          };
        }
    "#;
    let test = "{ nodes = {}; testScript = \"\"; }";
    let (v, _code) = run_check("plain-disappearance", &[("module.nix", module), ("test.nix", test)], WIDGET_TARGETS);
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OptionNotFound", "full verdict: {verdict}");
}

/// Hostile control (7): cross-file destination. A guacamole-shaped
/// rename to a declaration in ANOTHER module file under the same root
/// must be found and confirmed.
#[test]
fn cross_file_destination_is_confirmed() {
    let module_a = r#"
        { config, lib, ... }:
        {
          imports = [
            (lib.mkRenamedOptionModule [ "services" "widget" "oldLeaf" ] [ "services" "otherModule" "newLeaf" ])
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
          };
        }
    "#;
    // Nested `options = { ... }` form -- matching the real
    // guacamole-client.nix's own shape (and `scan_options`'s own
    // generic per-entry `walk_options_block` path, which doesn't require
    // a local `cfg` alias unlike the flat-dotted `options.a.b = {...};`
    // form's own `resolve_cfg_root` guard).
    let module_b = r#"
        { config, lib, ... }:
        {
          options = {
            services.otherModule = {
              newLeaf = lib.mkOption {
                type = lib.types.bool;
                default = false;
              };
            };
          };
        }
    "#;
    let test = "{ nodes = {}; testScript = \"\"; }";
    let (v, code) = run_check(
        "cross-file-destination",
        &[("module.nix", module_a), ("other-module.nix", module_b), ("test.nix", test)],
        WIDGET_TARGETS,
    );
    assert_eq!(code, 2, "full output: {v}");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OptionRelocated", "full verdict: {verdict}");
    assert_eq!(verdict["to"], serde_json::json!(["services", "otherModule", "newLeaf"]));
    assert_eq!(
        verdict["destination_confirmed"], true,
        "a real declaration exists in a DIFFERENT file under the same root and must be found; full verdict: {verdict}"
    );
}

/// Hostile control (8): a quoted path component containing an unusual
/// character (a literal dot) stays ONE path component, not several --
/// `extract_literal_string_list` walks NODE_LIST children directly
/// (never splits string content), so this is really confirming that
/// property end-to-end through the real migration path.
#[test]
fn quoted_path_component_with_internal_dot_is_one_component() {
    let module = r#"
        { config, lib, ... }:
        {
          imports = [
            (lib.mkRenamedOptionModule [ "services" "widget" "oldLeaf" ] [ "services" "widget.v2" "newLeaf" ])
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
          };
        }
    "#;
    let test = "{ nodes = {}; testScript = \"\"; }";
    let (v, _code) = run_check("quoted-dotted-component", &[("module.nix", module), ("test.nix", test)], WIDGET_TARGETS);
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OptionRelocated", "full verdict: {verdict}");
    assert_eq!(
        verdict["to"],
        serde_json::json!(["services", "widget.v2", "newLeaf"]),
        "the quoted 'widget.v2' component must appear as ONE element containing a literal dot, not split; full verdict: {verdict}"
    );
}

/// Hostile control (9): a dynamic/computed migration path (not a
/// literal string list) stays unresolved -- never guessed. The edge is
/// simply never extracted at all, so the watched option falls through
/// to ordinary `OptionNotFound`.
#[test]
fn dynamic_migration_path_stays_unresolved_never_guessed() {
    let module = r#"
        { config, lib, ... }:
        let
          computedPath = [ "services" "widget" "oldLeaf" ];
        in
        {
          imports = [
            (lib.mkRenamedOptionModule computedPath [ "services" "elsewhere" "newLeaf" ])
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
          };
        }
    "#;
    let test = "{ nodes = {}; testScript = \"\"; }";
    let (v, _code) = run_check("dynamic-path", &[("module.nix", module), ("test.nix", test)], WIDGET_TARGETS);
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(
        verdict["verdict"], "OptionNotFound",
        "a computed (non-literal-list) migration path must never be guessed at; full verdict: {verdict}"
    );
}

/// Hostile control (10): multiple unrelated migrations in the same
/// file -- the correct edge is selected by exact `from_path`, never by
/// source order (each of these edges has a DIFFERENT `from_path`, so
/// there is no real ambiguity here -- this specifically guards against
/// an implementation that accidentally picks "the first migration edge
/// in the file" rather than filtering by identity first).
#[test]
fn multiple_unrelated_migrations_the_correct_edge_is_selected_by_exact_from_path() {
    let module = r#"
        { config, lib, ... }:
        {
          imports = [
            (lib.mkRenamedOptionModule [ "services" "other1" "leaf" ] [ "services" "other1" "newLeaf" ])
            (lib.mkRenamedOptionModule [ "services" "widget" "oldLeaf" ] [ "services" "elsewhere" "newLeaf" ])
            (lib.mkRenamedOptionModule [ "services" "other2" "leaf" ] [ "services" "other2" "newLeaf" ])
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
          };
        }
    "#;
    let test = "{ nodes = {}; testScript = \"\"; }";
    let (v, code) = run_check("multiple-unrelated-migrations", &[("module.nix", module), ("test.nix", test)], WIDGET_TARGETS);
    assert_eq!(code, 2, "full output: {v}");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OptionRelocated", "full verdict: {verdict}");
    assert_eq!(verdict["to"], serde_json::json!(["services", "elsewhere", "newLeaf"]));
}

/// Hostile control (11), explicit ambiguity: TWO real rename edges
/// share the EXACT SAME `from_path` but point to DIFFERENT
/// destinations. Must fail closed to plain `OptionNotFound` --
/// never silently pick the first one in source order.
#[test]
fn ambiguous_same_from_path_different_destinations_fails_closed() {
    let module = r#"
        { config, lib, ... }:
        {
          imports = [
            (lib.mkRenamedOptionModule [ "services" "widget" "oldLeaf" ] [ "services" "destA" "leaf" ])
            (lib.mkRenamedOptionModule [ "services" "widget" "oldLeaf" ] [ "services" "destB" "leaf" ])
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
          };
        }
    "#;
    let test = "{ nodes = {}; testScript = \"\"; }";
    let (v, _code) = run_check("ambiguous-multi-edge", &[("module.nix", module), ("test.nix", test)], WIDGET_TARGETS);
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(
        verdict["verdict"], "OptionNotFound",
        "two edges disagreeing on the same from_path's own destination must fail closed, never pick a source-order winner; full verdict: {verdict}"
    );
}

/// `mkRenamedOptionModuleWith { from = [...]; to = [...]; ... }` --
/// current real nixpkgs form (e.g. `nixos/modules/virtualisation/oci-options.nix`),
/// with an extra, irrelevant field (`sinceRelease`) that must be
/// ignored, not required or validated.
#[test]
fn mk_renamed_option_module_with_is_supported() {
    let module = r#"
        { config, lib, ... }:
        {
          imports = [
            (lib.mkRenamedOptionModuleWith {
              sinceRelease = 2411;
              from = [ "services" "widget" "oldLeaf" ];
              to = [ "services" "elsewhere" "newLeaf" ];
            })
          ];
          options.services.widget = {
            enable = lib.mkEnableOption "widget";
          };
        }
    "#;
    let test = "{ nodes = {}; testScript = \"\"; }";
    let (v, code) = run_check("renamed-with", &[("module.nix", module), ("test.nix", test)], WIDGET_TARGETS);
    assert_eq!(code, 2, "full output: {v}");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "OptionRelocated", "full verdict: {verdict}");
    assert_eq!(verdict["to"], serde_json::json!(["services", "elsewhere", "newLeaf"]));
    assert_eq!(verdict["helper_form"], "mkRenamedOptionModuleWith");
}
