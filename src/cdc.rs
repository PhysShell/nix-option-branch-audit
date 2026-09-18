//! K1: a minimal, deliberately narrow vertical spike testing OBA's
//! original premise on a real production defect -- NOT a general
//! Contract Drift Checker (CDC) framework. Scope, fixed on purpose:
//! Kimai + Davis, the MySQL DSN `unix_socket` query parameter, the one
//! historical nixpkgs bug (buggy: `PhysShell/nixpkgs` commit
//! [`BEFORE_REV`], fixed: [`AFTER_REV`]), against the exact
//! `doctrine/dbal` version each app's own `composer.lock` pins (verified
//! independently for each app, not assumed shared -- both happen to be
//! 3.10.6 at the same upstream commit).
//!
//! Real Nix evaluation is the semantic oracle (`eval_nix_raw`), not a
//! hand-rolled interpreter -- interpolation, `mkIf`/`mkMerge`, and option
//! evaluation are exactly the things a real `nixosSystem` eval already
//! does correctly. `rnix` plays no role here at all; there is no source
//! AST anywhere in this module. No Git integration, no CLI: this is a
//! pure, unit-testable pipeline, exercised by `#[cfg(test)]` (offline,
//! synthetic-string tests) and a handful of `#[ignore]`-by-default tests
//! that shell out to a real `nix` binary and the network (run with
//! `cargo test --ignored -- cdc::`).
//!
//! Not built, on purpose (K2 territory if this spike holds up): generic
//! env-var/CLI-flag/OpenAPI/JSON-Schema contracts, a real PHP parser, a
//! package-bump differential framework, automatic nixpkgs-wide scanning,
//! or deriving the `doctrine/dbal` pin automatically from
//! `package.nix` -> `composer.lock` (hardcoded here -- see
//! [`DOCTRINE_DBAL_VERSION`]'s own doc comment).

use std::path::Path;
use std::process::Command;

/// Tree state immediately before the fix -- a real, but otherwise
/// unrelated, upstream nixpkgs commit (the bug itself predates it; see
/// the fix commit's own message). Verified byte-identical to
/// `fixtures/{kimai,davis}/before/module.nix` (locked in
/// `fixtures/integrity-lock.toml`).
pub const BEFORE_REV: &str = "5530e24f2100f4c2ca766050a805a12d7541662f";
/// Current tip of `PhysShell/nixpkgs@fix/doctrine-unix-socket-param-name`.
/// Originally recorded as `37f81efa4abf623009e474fa563a094658c25be6`;
/// repointed here after that commit was amended (an added trailer) plus a
/// later test-only follow-up -- see `fixtures/integrity-lock.toml`'s own
/// note for the verification that the MODULE content never changed
/// across that rewrite.
pub const AFTER_REV: &str = "d81d88f4354b2c9d8a7492c9b72cd3add62a34e0";
/// Independently confirmed (not just trusted from the fix commit's own
/// message) via each app's own `composer.lock` at its nixpkgs-pinned
/// version (kimai 2.66.0, davis v5.4.4): both resolve `doctrine/dbal` to
/// this exact version and git commit. Hardcoded as a K1 corpus fact, not
/// derived at runtime from `package.nix` -> upstream `composer.lock` --
/// generalizing that chain is explicitly K2 scope, not K1's.
pub const DOCTRINE_DBAL_VERSION: &str = "3.10.6";
pub const DOCTRINE_DBAL_REV: &str = "c95589d775a0b2e543467d40f8c3ecccf586f2b4";

#[derive(Debug)]
pub enum CdcError {
    /// The pipeline itself couldn't run (missing `nix`, network failure,
    /// a `nix eval` error, a vendored fixture missing) -- distinct from
    /// [`CdcError::Inconclusive`]: this is "never got to analyze it".
    ToolError(String),
    /// Ran, but evidence was ambiguous or absent on either the producer
    /// or consumer side. Fail-closed: NEVER silently promoted to `Pass`.
    Inconclusive(String),
}

impl std::fmt::Display for CdcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CdcError::ToolError(m) => write!(f, "TOOL_ERROR: {m}"),
            CdcError::Inconclusive(m) => write!(f, "INCONCLUSIVE: {m}"),
        }
    }
}

/// Real Nix evaluation as the semantic oracle. `--impure` (for
/// `builtins.fetchTarball` without a pinned hash -- acceptable for a
/// spike verifying a fixed historical revision; a production version
/// would pin the tarball hash) `--raw` (so a Nix string comes back as
/// plain bytes, not JSON-quoted).
fn eval_nix_raw(expr: &str) -> Result<String, CdcError> {
    let out = Command::new("nix")
        .args(["eval", "--impure", "--raw", "--expr", expr])
        .output()
        .map_err(|e| CdcError::ToolError(format!("spawning `nix`: {e}")))?;
    if !out.status.success() {
        return Err(CdcError::ToolError(format!(
            "nix eval failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    String::from_utf8(out.stdout)
        .map_err(|e| CdcError::ToolError(format!("nix eval produced non-UTF8 output: {e}")))
}

/// Builds the shared `nixosSystem` prelude for one nixpkgs revision.
/// `module_override`, when given, disables the real nixpkgs module at
/// `upstream_path` and substitutes a local file instead -- the mechanism
/// mutation testing uses to run a controlled text change through the
/// EXACT SAME evaluation path as the historical corpus, not a separate
/// mutation-only code path. Matching by PATH VALUE (not a string) is
/// load-bearing: `disabledModules = [ "nixos/modules/.../kimai.nix" ]`
/// (a string) silently fails to match nixpkgs's own internal module key
/// and the real module loads anyway, colliding with the substitute
/// ("option already declared") -- confirmed by trying the string form
/// first. `disabledModules = [ (nixpkgsSrc + "/nixos/modules/.../kimai.nix") ]`
/// (a path built from the SAME `nixpkgsSrc`) matches correctly.
fn nixos_eval_expr(
    rev: &str,
    upstream_path: &str,
    module_override: Option<&Path>,
    configuration_body: &str,
) -> String {
    let disable_and_import = match module_override {
        Some(p) => format!(
            r#"disabledModules = [ (nixpkgsSrc + "/{upstream_path}") ]; imports = [ {} ];"#,
            p.display()
        ),
        None => String::new(),
    };
    format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz"; in
        (import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            {disable_and_import}
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
            {configuration_body}
          }};
        }})"#
    )
}

const KIMAI_UPSTREAM_PATH: &str = "nixos/modules/services/web-apps/kimai.nix";
const DAVIS_UPSTREAM_PATH: &str = "nixos/modules/services/web-apps/davis.nix";

/// Phase B for Kimai: `services.kimai.sites.<name>.database.socket` is a
/// real option, so a unique sentinel is injected and the rendered
/// `kimai-init-<name>` systemd service script is evaluated (a plain Nix
/// string attribute -- no build, no VM). `module_override` is the
/// mutation-testing hook (see [`nixos_eval_expr`]).
pub fn eval_kimai_script(
    rev: &str,
    sentinel: &str,
    module_override: Option<&Path>,
) -> Result<String, CdcError> {
    let expr = nixos_eval_expr(
        rev,
        KIMAI_UPSTREAM_PATH,
        module_override,
        &format!(
            r#"services.kimai.sites."probe" = {{ database.createLocally = true; database.socket = "{sentinel}"; }};"#
        ),
    );
    eval_nix_raw(&format!(
        r#"({expr}).config.systemd.services."kimai-init-probe".script"#
    ))
}

/// Phase B for Davis: `database.socket` is NOT an option here -- the
/// socket path is a hardcoded string literal in the module
/// (`/run/mysqld/mysqld.sock`) regardless of configuration, so there is
/// no sentinel to inject. Real Nix evaluation still does the real work
/// (option resolution, `mkIf`/`createLocally` branch selection) -- this
/// reads `config.services.davis.config.DATABASE_URL` directly rather
/// than tracing to a systemd sink the way Kimai's probe does, since
/// there's no sentinel-driven value to trace further and the option
/// itself already IS the final pre-secret value. Documented divergence
/// from Kimai's probe, not a shortcut around it -- see the K1 README
/// section.
pub fn eval_davis_database_url(rev: &str) -> Result<String, CdcError> {
    let expr = nixos_eval_expr(
        rev,
        DAVIS_UPSTREAM_PATH,
        None,
        r#"services.davis = { enable = true; hostname = "davis.example.com"; database.driver = "mysql"; database.createLocally = true; adminLogin = "admin"; appSecretFile = "/dev/null"; adminPasswordFile = "/dev/null"; };"#,
    );
    eval_nix_raw(&format!(
        r#"({expr}).config.services.davis.config.DATABASE_URL"#
    ))
}

/// Phase C: the DSN-query-specific model the task calls for -- not a
/// general URI parser, not raw-text DSN comparison. Fail-closed: `Err`
/// (never a guessed key) unless `needle` appears in `rendered` EXACTLY
/// once, immediately after a `=`, with a key made only of characters a
/// DSN query key could plausibly be.
pub fn extract_key_for_value(rendered: &str, needle: &str) -> Result<String, CdcError> {
    let hits: Vec<_> = rendered.match_indices(needle).collect();
    if hits.len() != 1 {
        return Err(CdcError::Inconclusive(format!(
            "sentinel/value {needle:?} found {} times in rendered output, expected exactly 1",
            hits.len()
        )));
    }
    let before = &rendered[..hits[0].0];
    let Some(before_eq) = before.strip_suffix('=') else {
        return Err(CdcError::Inconclusive(format!(
            "value {needle:?} is not immediately preceded by '=' in rendered output"
        )));
    };
    let key_start = before_eq.rfind(['&', '?']).map(|i| i + 1).unwrap_or(0);
    let key = &before_eq[key_start..];
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(CdcError::Inconclusive(format!(
            "extracted key {key:?} doesn't look like a DSN query parameter name"
        )));
    }
    Ok(key.to_string())
}

/// Phase D: the accepted-keys extractor. Deliberately a bounded literal
/// scan over `isset($params['<key>'])`, not a PHP parser -- this is the
/// exact, narrow shape `constructPdoDsn()` actually uses (verified
/// against the real vendored source, see `fixtures/cdc/`), and a bounded
/// extractor over an exactly-pinned file is more trustworthy for THIS
/// spike than a general PHP AST library would be. Fail-closed: `Err`
/// (never an empty-but-confident `Ok(vec![])`) if the pattern isn't
/// found at all -- that means this extractor no longer understands the
/// source shape, not that the driver accepts nothing.
pub fn extract_accepted_keys(php_source: &str) -> Result<Vec<String>, CdcError> {
    const NEEDLE: &str = "isset($params['";
    let mut keys = Vec::new();
    let mut rest = php_source;
    while let Some(pos) = rest.find(NEEDLE) {
        let after = &rest[pos + NEEDLE.len()..];
        let Some(end) = after.find("'])") else {
            return Err(CdcError::Inconclusive(
                "found `isset($params['` without a matching `'])` -- source shape changed"
                    .to_string(),
            ));
        };
        keys.push(after[..end].to_string());
        rest = &after[end..];
    }
    if keys.is_empty() {
        return Err(CdcError::Inconclusive(
            "no `isset($params['<key>'])` pattern found -- extractor doesn't understand this source"
                .to_string(),
        ));
    }
    Ok(keys)
}

#[derive(Debug, PartialEq)]
pub enum CdcVerdict {
    Pass,
    /// "CDC001": ExternalContractKeyMismatch. `probable_candidate` is a
    /// diagnostic hint ONLY (case/separator-insensitive containment
    /// match against `accepted_keys`) -- never used to decide
    /// Pass/Finding, only attached to a Finding that already exists on
    /// its own evidence.
    Finding { emitted_key: String, accepted_keys: Vec<String>, probable_candidate: Option<String> },
}

/// Phase E: boring on purpose. `emitted_key` and `accepted_keys` come
/// from evidence gathered upstream (`Err` from either extraction step
/// must never reach here as a fabricated `Pass`) -- fail-closed policy
/// lives in the caller, not here.
pub fn compare_contract(emitted_key: &str, accepted_keys: &[String]) -> CdcVerdict {
    if accepted_keys.iter().any(|k| k == emitted_key) {
        return CdcVerdict::Pass;
    }
    let normalize = |s: &str| s.to_lowercase().replace(['_', '-'], "");
    let emitted_norm = normalize(emitted_key);
    let probable_candidate = accepted_keys
        .iter()
        .find(|k| {
            let k_norm = normalize(k);
            k_norm == emitted_norm || k_norm.contains(&emitted_norm) || emitted_norm.contains(&k_norm)
        })
        .cloned();
    CdcVerdict::Finding {
        emitted_key: emitted_key.to_string(),
        accepted_keys: accepted_keys.to_vec(),
        probable_candidate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Phase C: offline, no nix/network needed ---

    #[test]
    fn extracts_key_immediately_preceding_the_value() {
        let rendered = "mysql://u@h:3306/db?charset=utf8&serverVersion=$v&unix_socket=/tmp/x.sock";
        assert_eq!(extract_key_for_value(rendered, "/tmp/x.sock").unwrap(), "unix_socket");
    }

    #[test]
    fn value_appearing_zero_times_is_inconclusive() {
        assert!(matches!(
            extract_key_for_value("no needle here", "/tmp/x.sock"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn value_appearing_twice_is_inconclusive_not_first_match() {
        let rendered = "a=/tmp/x.sock&b=/tmp/x.sock";
        assert!(matches!(
            extract_key_for_value(rendered, "/tmp/x.sock"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn value_not_preceded_by_equals_is_inconclusive() {
        assert!(matches!(
            extract_key_for_value("path/tmp/x.sock/extra", "/tmp/x.sock"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    // --- Phase D: offline, against the real vendored pinned source ---

    #[test]
    fn extracts_accepted_keys_from_the_real_pinned_driver() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/cdc/doctrine-dbal-3.10.6/PDO-MySQL-Driver.php"),
        )
        .unwrap();
        let keys = extract_accepted_keys(&source).unwrap();
        assert_eq!(keys, vec!["host", "port", "dbname", "unix_socket", "charset"]);
    }

    #[test]
    fn unrecognized_source_shape_is_inconclusive_not_empty_pass() {
        assert!(matches!(
            extract_accepted_keys("<?php // nothing recognizable here"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn truncated_isset_is_inconclusive() {
        assert!(matches!(
            extract_accepted_keys("isset($params['unix_socket'"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    // --- Phase E: boring comparison, offline ---

    #[test]
    fn matching_key_is_pass() {
        let accepted = vec!["host".to_string(), "unix_socket".to_string()];
        assert_eq!(compare_contract("unix_socket", &accepted), CdcVerdict::Pass);
    }

    #[test]
    fn kimai_camel_case_mismatch_is_finding_with_a_probable_candidate() {
        let accepted = vec!["host".to_string(), "unix_socket".to_string()];
        let verdict = compare_contract("unixSocket", &accepted);
        match verdict {
            CdcVerdict::Finding { emitted_key, probable_candidate, .. } => {
                assert_eq!(emitted_key, "unixSocket");
                assert_eq!(probable_candidate.as_deref(), Some("unix_socket"));
            }
            other => panic!("expected Finding, got {other:?}"),
        }
    }

    #[test]
    fn davis_bare_socket_mismatch_is_finding_with_a_probable_candidate() {
        let accepted = vec!["host".to_string(), "unix_socket".to_string()];
        let verdict = compare_contract("socket", &accepted);
        match verdict {
            CdcVerdict::Finding { probable_candidate, .. } => {
                assert_eq!(probable_candidate.as_deref(), Some("unix_socket"));
            }
            other => panic!("expected Finding, got {other:?}"),
        }
    }

    #[test]
    fn unrelated_key_is_finding_with_no_probable_candidate() {
        let accepted = vec!["host".to_string(), "unix_socket".to_string()];
        let verdict = compare_contract("totally_unrelated", &accepted);
        match verdict {
            CdcVerdict::Finding { probable_candidate, .. } => assert_eq!(probable_candidate, None),
            other => panic!("expected Finding, got {other:?}"),
        }
    }

    // --- Real end-to-end: needs a real `nix` binary + network, opt-in
    // only (`cargo test --ignored -- cdc::`). These are the K1 golden
    // proof and mutation proof, run through the exact same
    // eval_kimai_script/eval_davis_database_url functions the offline
    // tests above only exercise the pure-Rust half of. ---

    fn require_verdict(rev: &str, sentinel: &str) -> CdcVerdict {
        let doctrine_src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/cdc/doctrine-dbal-3.10.6/PDO-MySQL-Driver.php"),
        )
        .unwrap();
        let accepted = extract_accepted_keys(&doctrine_src).unwrap();
        let rendered = eval_kimai_script(rev, sentinel, None).unwrap();
        let emitted = extract_key_for_value(&rendered, sentinel).unwrap();
        compare_contract(&emitted, &accepted)
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn golden_kimai_before_is_finding() {
        assert!(matches!(
            require_verdict(BEFORE_REV, "/__OBA_CONTRACT_socket_k1a__/mysql.sock"),
            CdcVerdict::Finding { .. }
        ));
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn golden_kimai_after_is_pass() {
        assert_eq!(
            require_verdict(AFTER_REV, "/__OBA_CONTRACT_socket_k1b__/mysql.sock"),
            CdcVerdict::Pass
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn golden_davis_before_is_finding() {
        let doctrine_src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/cdc/doctrine-dbal-3.10.6/PDO-MySQL-Driver.php"),
        )
        .unwrap();
        let accepted = extract_accepted_keys(&doctrine_src).unwrap();
        let rendered = eval_davis_database_url(BEFORE_REV).unwrap();
        let emitted = extract_key_for_value(&rendered, "/run/mysqld/mysqld.sock").unwrap();
        assert!(matches!(compare_contract(&emitted, &accepted), CdcVerdict::Finding { .. }));
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn golden_davis_after_is_pass() {
        let doctrine_src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/cdc/doctrine-dbal-3.10.6/PDO-MySQL-Driver.php"),
        )
        .unwrap();
        let accepted = extract_accepted_keys(&doctrine_src).unwrap();
        let rendered = eval_davis_database_url(AFTER_REV).unwrap();
        let emitted = extract_key_for_value(&rendered, "/run/mysqld/mysqld.sock").unwrap();
        assert_eq!(compare_contract(&emitted, &accepted), CdcVerdict::Pass);
    }

    /// Mutation proof: takes the REAL fixed module source, patches ONLY
    /// the `unix_socket` literal, and runs it through the EXACT SAME
    /// `eval_kimai_script` + module-override mechanism the golden proof
    /// uses -- not a separate mutation-only fixture/parser.
    fn mutation_verdict(mutated_key: &str) -> CdcVerdict {
        let fixed_source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/kimai/after/module.nix"),
        )
        .unwrap();
        let mutated = fixed_source.replace("unix_socket", mutated_key);
        let tmp = std::env::temp_dir().join(format!("oba-cdc-mutation-{mutated_key}.nix"));
        std::fs::write(&tmp, mutated).unwrap();

        let doctrine_src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/cdc/doctrine-dbal-3.10.6/PDO-MySQL-Driver.php"),
        )
        .unwrap();
        let accepted = extract_accepted_keys(&doctrine_src).unwrap();
        let sentinel = "/__OBA_CONTRACT_socket_mut__/mysql.sock";
        let rendered = eval_kimai_script(AFTER_REV, sentinel, Some(&tmp)).unwrap();
        let emitted = extract_key_for_value(&rendered, sentinel).unwrap();
        compare_contract(&emitted, &accepted)
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn mutation_unix_socket_to_unixsocket_is_finding() {
        assert!(matches!(mutation_verdict("unixSocket"), CdcVerdict::Finding { .. }));
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn mutation_unix_socket_to_socket_is_finding() {
        assert!(matches!(mutation_verdict("socket"), CdcVerdict::Finding { .. }));
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn mutation_unix_socket_to_unix_dash_socket_is_finding() {
        assert!(matches!(mutation_verdict("unix-socket"), CdcVerdict::Finding { .. }));
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn mutation_control_unix_socket_to_itself_is_pass() {
        assert_eq!(mutation_verdict("unix_socket"), CdcVerdict::Pass);
    }
}
