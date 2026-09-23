//! S5-F2 hostile controls for `option_prefix_is_instance_keyed_submodule`
//! / `path_matches_prefix`'s own new `allow_instance_key` tolerance.
//! Synthetic fixtures (not the real cgit PR), each isolating exactly one
//! semantic situation the mandate requires distinguished. See
//! `fixtures/s5-f2/investigation.md` for the full root-cause trace this
//! fix is built from.

use serde_json::Value;
use std::process::Command;

/// Writes `module.nix`/`test.nix`/`targets.toml` under a fresh temp dir
/// named `name`, runs `check --root <dir> --targets <dir>/targets.toml
/// --json`, returns the parsed JSON and the process exit code.
fn run_check(name: &str, module: &str, test: &str, targets_toml: &str) -> (Value, i32) {
    let dir = std::env::temp_dir().join(format!("oba-f2-hostile-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("module.nix"), module).unwrap();
    std::fs::write(dir.join("test.nix"), test).unwrap();
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

/// Hostile control (2): a DIFFERENT concrete instance of the same
/// submodule type that never touches the watched leaf at all must not
/// prevent an instance that DOES from witnessing it.
#[test]
fn different_untouched_instance_does_not_block_witness_from_the_one_that_matters() {
    let module = r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
        in
        {
          options = {
            services.widget = lib.mkOption {
              type = lib.types.attrsOf (
                lib.types.submodule (
                  { ... }: {
                    options = {
                      foo.enable = lib.mkOption { type = lib.types.bool; default = true; };
                    };
                  }
                )
              );
              default = { };
            };
          };
          config.assertions = lib.optionalAttrs cfg.foo.enable [ ];
        }
    "#;
    let test = r#"
        { pkgs, ... }:
        {
          name = "widget-test";
          nodes.machine = { ... }: {
            services.widget."untouched-instance" = {
            };
            services.widget."flipping-instance" = {
              foo.enable = false;
            };
          };
          testScript = "";
        }
    "#;
    let targets = r#"
        [[target]]
        name = "widget"
        module = "module.nix"
        test = "test.nix"
        cfg_ident = "cfg"
        option_prefix = ["services", "widget"]
        watch = ["foo.enable"]
    "#;
    let (v, code) = run_check("different-untouched-instance", module, test, targets);
    assert_eq!(code, 0, "full output: {v}");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "PASS", "the flipping instance's own real assignment must witness the predicate regardless of the OTHER, untouched instance; full verdict: {verdict}");
}

/// Hostile control (2b), the honestly-documented limitation: TWO
/// DIFFERENT concrete instances of the same submodule type, WITHIN THE
/// SAME nixosTest node, BOTH explicitly assign the SAME relative option
/// to DIFFERENT values. `evaluate_predicate_witness`'s own
/// `instances_assigning_x` bookkeeping is deduped by the nixosTest NODE
/// (`TestAssignment.instance`), not by the cgit-style submodule instance
/// key `path_matches_prefix`'s own new tolerance skips over -- so when
/// two DIFFERENT submodule instances within ONE node both match, only
/// the FIRST one in source-scan order is actually used (the same
/// `.find()`-first-match precedent this project already uses elsewhere,
/// e.g. `run_target`'s own gate-1 lookup). This is NOT solved by S5-F2,
/// and NOT claimed to be: this test asserts the ACTUAL, current,
/// disclosed behavior, not an idealized one -- exactly the distinction
/// the S5-F2 mandate's own hostile control (2) asks to have documented
/// rather than assumed.
#[test]
fn two_instances_disagreeing_on_the_same_leaf_use_first_source_order_match_documented_limitation() {
    let module = r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
        in
        {
          options = {
            services.widget = lib.mkOption {
              type = lib.types.attrsOf (
                lib.types.submodule (
                  { ... }: {
                    options = {
                      foo.enable = lib.mkOption { type = lib.types.bool; default = true; };
                    };
                  }
                )
              );
              default = { };
            };
          };
          config.assertions = lib.optionalAttrs cfg.foo.enable [ ];
        }
    "#;
    // "a-first-instance" (alphabetically and textually first) flips the
    // predicate (false, default is true); "z-second-instance" does not
    // (true, same as default). Source order in the test file: a-first
    // is written before z-second.
    let test = r#"
        { pkgs, ... }:
        {
          name = "widget-test";
          nodes.machine = { ... }: {
            services.widget."a-first-instance" = {
              foo.enable = false;
            };
            services.widget."z-second-instance" = {
              foo.enable = true;
            };
          };
          testScript = "";
        }
    "#;
    let targets = r#"
        [[target]]
        name = "widget"
        module = "module.nix"
        test = "test.nix"
        cfg_ident = "cfg"
        option_prefix = ["services", "widget"]
        watch = ["foo.enable"]
    "#;
    let (v, _code) = run_check("two-instances-disagree", module, test, targets);
    let verdict = &v["targets"][0]["verdicts"][0];
    // Documented current behavior: the first matching assignment in
    // source-scan order (a-first-instance, foo.enable = false, which
    // DOES flip the predicate away from its true default) is the one
    // used -- so this specific ordering witnesses PASS. Swapping the two
    // instances' own source order would flip this result; that
    // source-order sensitivity for cross-instance ambiguity is the
    // documented limitation itself, not a bug this test is hiding.
    assert_eq!(
        verdict["verdict"], "PASS",
        "documents that source-order-first assignment wins when multiple submodule instances \
         within one node disagree -- full verdict: {verdict}"
    );
}

/// Hostile control (3): an assignment to an UNRELATED option with the
/// same terminal leaf name ("enable") must not witness a DIFFERENT
/// predicate.
#[test]
fn unrelated_option_with_same_leaf_name_does_not_witness() {
    let module = r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
        in
        {
          options = {
            services.widget = lib.mkOption {
              type = lib.types.attrsOf (
                lib.types.submodule (
                  { ... }: {
                    options = {
                      enable = lib.mkOption { type = lib.types.bool; default = true; };
                      foo.enable = lib.mkOption { type = lib.types.bool; default = true; };
                    };
                  }
                )
              );
              default = { };
            };
          };
          config.assertions = lib.optionalAttrs cfg.foo.enable [ ];
        }
    "#;
    let test = r#"
        { pkgs, ... }:
        {
          name = "widget-test";
          nodes.machine = { ... }: {
            services.widget."instance" = {
              enable = false;
            };
          };
          testScript = "";
        }
    "#;
    let targets = r#"
        [[target]]
        name = "widget"
        module = "module.nix"
        test = "test.nix"
        cfg_ident = "cfg"
        option_prefix = ["services", "widget"]
        watch = ["foo.enable"]
    "#;
    let (v, _code) = run_check("unrelated-same-leaf", module, test, targets);
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_ne!(
        verdict["verdict"], "PASS",
        "the top-level 'enable' assignment must NOT witness the DIFFERENT 'foo.enable' predicate \
         merely because both end in the same leaf name; full verdict: {verdict}"
    );
}

/// Hostile control (4): an assignment at the same relative suffix under
/// an entirely unrelated service must not witness this target's own
/// predicate.
#[test]
fn same_relative_path_under_unrelated_service_does_not_witness() {
    let module = r#"
        { config, lib, ... }:
        let
          cfg = config.services.widgetA;
        in
        {
          options = {
            services.widgetA = lib.mkOption {
              type = lib.types.attrsOf (
                lib.types.submodule (
                  { ... }: {
                    options = {
                      foo.enable = lib.mkOption { type = lib.types.bool; default = true; };
                    };
                  }
                )
              );
              default = { };
            };
            services.widgetB = lib.mkOption {
              type = lib.types.attrsOf (
                lib.types.submodule (
                  { ... }: {
                    options = {
                      foo.enable = lib.mkOption { type = lib.types.bool; default = true; };
                    };
                  }
                )
              );
              default = { };
            };
          };
          config.assertions = lib.optionalAttrs cfg.foo.enable [ ];
        }
    "#;
    let test = r#"
        { pkgs, ... }:
        {
          name = "widget-test";
          nodes.machine = { ... }: {
            services.widgetB."instance" = {
              foo.enable = false;
            };
          };
          testScript = "";
        }
    "#;
    let targets = r#"
        [[target]]
        name = "widgetA"
        module = "module.nix"
        test = "test.nix"
        cfg_ident = "cfg"
        option_prefix = ["services", "widgetA"]
        watch = ["foo.enable"]
    "#;
    let (v, _code) = run_check("unrelated-service", module, test, targets);
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_ne!(
        verdict["verdict"], "PASS",
        "widgetB's own assignment must never witness widgetA's own predicate, even at the identical \
         relative suffix; full verdict: {verdict}"
    );
}

/// Hostile control (5): a concrete assignment that leaves the predicate
/// at its own DEFAULT outcome is not evidence of flipping it.
#[test]
fn assignment_matching_the_default_outcome_is_not_a_witness() {
    let module = r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
        in
        {
          options = {
            services.widget = lib.mkOption {
              type = lib.types.attrsOf (
                lib.types.submodule (
                  { ... }: {
                    options = {
                      foo.enable = lib.mkOption { type = lib.types.bool; default = true; };
                    };
                  }
                )
              );
              default = { };
            };
          };
          config.assertions = lib.optionalAttrs cfg.foo.enable [ ];
        }
    "#;
    let test = r#"
        { pkgs, ... }:
        {
          name = "widget-test";
          nodes.machine = { ... }: {
            services.widget."instance" = {
              foo.enable = true;
            };
          };
          testScript = "";
        }
    "#;
    let targets = r#"
        [[target]]
        name = "widget"
        module = "module.nix"
        test = "test.nix"
        cfg_ident = "cfg"
        option_prefix = ["services", "widget"]
        watch = ["foo.enable"]
    "#;
    let (v, _code) = run_check("matches-default", module, test, targets);
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_ne!(
        verdict["verdict"], "PASS",
        "an assignment equal to the predicate's own default outcome is real evidence but not a \
         witnessed TRANSITION -- must not PASS; full verdict: {verdict}"
    );
}

/// Hostile control (6): a syntactically present assignment whose own
/// value cannot be statically classified (a function call, not a
/// literal) must remain fail-closed -- Unresolved, never silently
/// treated as either "no evidence" or a witness.
#[test]
fn statically_unclassifiable_value_stays_unresolved_not_silently_dropped() {
    let module = r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
        in
        {
          options = {
            services.widget = lib.mkOption {
              type = lib.types.attrsOf (
                lib.types.submodule (
                  { ... }: {
                    options = {
                      foo.enable = lib.mkOption { type = lib.types.bool; default = true; };
                    };
                  }
                )
              );
              default = { };
            };
          };
          config.assertions = lib.optionalAttrs cfg.foo.enable [ ];
        }
    "#;
    let test = r#"
        { pkgs, ... }:
        {
          name = "widget-test";
          nodes.machine = { ... }: {
            services.widget."instance" = {
              foo.enable = someHelperFunction cfg.something;
            };
          };
          testScript = "";
        }
    "#;
    let targets = r#"
        [[target]]
        name = "widget"
        module = "module.nix"
        test = "test.nix"
        cfg_ident = "cfg"
        option_prefix = ["services", "widget"]
        watch = ["foo.enable"]
    "#;
    let (v, code) = run_check("unclassifiable-value", module, test, targets);
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_ne!(
        verdict["verdict"], "PASS",
        "a value this walker cannot statically classify must never itself count as a witness; full verdict: {verdict}"
    );
    assert_eq!(
        code, 2,
        "an unresolved predicate (not a clean finding, not a clean pass) must surface as \
         inconclusive/unavailable, matching this project's own existing fail-closed exit-code \
         convention; full output: {v}"
    );
}

/// Hostile control (7): a quoted attribute component containing a
/// literal dot ("no-git-http-backend.localhost"-shaped) is ONE path
/// segment, not several -- isolated here independent of the real cgit
/// fixture, so this property stays covered even if that fixture's own
/// content ever changes.
#[test]
fn quoted_instance_key_with_internal_dots_is_one_segment_not_several() {
    let module = r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
        in
        {
          options = {
            services.widget = lib.mkOption {
              type = lib.types.attrsOf (
                lib.types.submodule (
                  { ... }: {
                    options = {
                      foo.enable = lib.mkOption { type = lib.types.bool; default = true; };
                    };
                  }
                )
              );
              default = { };
            };
          };
          config.assertions = lib.optionalAttrs cfg.foo.enable [ ];
        }
    "#;
    let test = r#"
        { pkgs, ... }:
        {
          name = "widget-test";
          nodes.machine = { ... }: {
            services.widget."no-git-http-backend.localhost" = {
              foo.enable = false;
            };
          };
          testScript = "";
        }
    "#;
    let targets = r#"
        [[target]]
        name = "widget"
        module = "module.nix"
        test = "test.nix"
        cfg_ident = "cfg"
        option_prefix = ["services", "widget"]
        watch = ["foo.enable"]
    "#;
    let (v, code) = run_check("quoted-dotted-instance-key", module, test, targets);
    assert_eq!(code, 0, "full output: {v}");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(verdict["verdict"], "PASS", "the quoted, dotted instance key must be treated as ONE opaque segment; full verdict: {verdict}");
    let evidence_path = &verdict["evidence"][0]["path"];
    assert_eq!(
        evidence_path,
        &serde_json::json!(["services", "widget", "no-git-http-backend.localhost", "foo", "enable"]),
        "the quoted key must appear as a SINGLE path element, not split on its internal dots; got {evidence_path}"
    );
}

/// Predicate-form breadth (mandate requirement): the real cgit fixture
/// exercises `PredicateKind::Truthy` (`lib.optionalAttrs cfg.foo.enable
/// {...}`, an H1 unary form). `allow_instance_key` is computed ONCE per
/// target in `run_target` and threaded to every `path_matches_prefix`
/// call site uniformly -- H1's own gate-4 site (`pred.kind` ranges over
/// `Truthy`/`NegTruthy`/`NullEq`/`NullNeq`) and every H2 site inside
/// `evaluate_predicate_witness` (whose `pred.ir` is already normalized,
/// at lowering time, from `cfg.foo` / `!cfg.foo` / `cfg.foo == v` /
/// `cfg.foo != v` alike into the same `Pred::Eq`/`Pred::Not(Eq)` IR --
/// see `lower_pred_chained`) call the identical function with the
/// identical flag. There is no form-specific branch anywhere in the
/// fix, so the tolerance is form-agnostic BY CONSTRUCTION -- this test
/// gives that structural argument a real, independent empirical check
/// on the ONE form (`if !cfg.foo then ...`, `NegTruthy`) the real cgit
/// fixture doesn't itself exercise.
#[test]
fn negated_predicate_form_on_instance_keyed_submodule_is_also_witnessed() {
    let module = r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
        in
        {
          options = {
            services.widget = lib.mkOption {
              type = lib.types.attrsOf (
                lib.types.submodule (
                  { ... }: {
                    options = {
                      foo.enable = lib.mkOption { type = lib.types.bool; default = false; };
                    };
                  }
                )
              );
              default = { };
            };
          };
          config.assertions = if !cfg.foo.enable then [ ] else [ ];
        }
    "#;
    let test = r#"
        { pkgs, ... }:
        {
          name = "widget-test";
          nodes.machine = { ... }: {
            services.widget."an-instance" = {
              foo.enable = true;
            };
          };
          testScript = "";
        }
    "#;
    let targets = r#"
        [[target]]
        name = "widget"
        module = "module.nix"
        test = "test.nix"
        cfg_ident = "cfg"
        option_prefix = ["services", "widget"]
        watch = ["foo.enable"]
    "#;
    let (v, code) = run_check("negtruthy-instance-keyed", module, test, targets);
    assert_eq!(code, 0, "full output: {v}");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(
        verdict["verdict"], "PASS",
        "a `!cfg.foo` (NegTruthy) predicate must witness through the same instance-keyed \
         submodule tolerance as the Truthy form cgit itself uses; full verdict: {verdict}"
    );
}

/// Predicate-form breadth, continued: an H2 equality form (`cfg.foo ==
/// "value"`, lowered to `Pred::Eq`) on an instance-keyed submodule.
/// Same structural argument as above, exercised on the H2 side instead
/// of H1.
#[test]
fn equality_predicate_form_on_instance_keyed_submodule_is_also_witnessed() {
    let module = r#"
        { config, lib, ... }:
        let
          cfg = config.services.widget;
        in
        {
          options = {
            services.widget = lib.mkOption {
              type = lib.types.attrsOf (
                lib.types.submodule (
                  { ... }: {
                    options = {
                      foo.mode = lib.mkOption { type = lib.types.str; default = "off"; };
                    };
                  }
                )
              );
              default = { };
            };
          };
          config.assertions = lib.optionalAttrs (cfg.foo.mode == "on") [ ];
        }
    "#;
    let test = r#"
        { pkgs, ... }:
        {
          name = "widget-test";
          nodes.machine = { ... }: {
            services.widget."an-instance" = {
              foo.mode = "on";
            };
          };
          testScript = "";
        }
    "#;
    let targets = r#"
        [[target]]
        name = "widget"
        module = "module.nix"
        test = "test.nix"
        cfg_ident = "cfg"
        option_prefix = ["services", "widget"]
        watch = ["foo.mode"]
    "#;
    let (v, code) = run_check("eq-instance-keyed", module, test, targets);
    assert_eq!(code, 0, "full output: {v}");
    let verdict = &v["targets"][0]["verdicts"][0];
    assert_eq!(
        verdict["verdict"], "PASS",
        "a `cfg.foo == value` (H2 Eq) predicate must witness through the same instance-keyed \
         submodule tolerance as the Truthy/NegTruthy forms; full verdict: {verdict}"
    );
}
