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
//! package-bump differential framework, or automatic nixpkgs-wide
//! scanning.
//!
//! K2a (done): the `doctrine/dbal` pin is no longer a hardcoded Rust
//! constant -- [`fetch_composer_lock`] + [`resolve_consumer_identity`]
//! derive it automatically, for real, from each app's own
//! `package.nix` -> `composer.lock`, and [`verify_identity_matches_vendored_fixture`]
//! checks the result against the vendored fixture's own recorded
//! provenance (`fixtures/integrity-lock.toml`) rather than a second,
//! redundant constant duplicating the same fact.
//!
//! K2b (done): [`ProducerEvidence`] generalizes HOW producer evidence was
//! acquired (`SentinelFlow`/`EvaluatedLiteral`) without touching consumer
//! provenance or [`compare_contract`].
//!
//! K2c (done, `fixtures/cdc/k2c-census/`): a real-corpus census measuring
//! how far `SentinelFlow`/`EvaluatedLiteral` reach across 14 real PHP web
//! apps -- found the flat-`DB_*`-env-var shape recurring 3 times
//! (agorakit, movim, snipe-it), the corpus-backed signal K2d acts on.
//!
//! K2d (done): [`ProducerEvidence::FlatEnvVars`] -- a third variant for
//! that flat-env shape, proven for real against agorakit/movim/snipe-it.
//! Deliberately does not attempt the `DB_HOST` -> Doctrine `host`
//! mapping for any of the three (that crosses a framework layer --
//! Laravel's own DB connector -- this module doesn't model); consumer
//! comparison for these apps stays honestly unsupported.
//!
//! K2a.1 (done): [`resolve_composer_lock`] adds a second, EXPLICIT
//! consumer-provenance source ([`ComposerLockOrigin::NixpkgsLocal`],
//! `pkgs.<attr>.composerVendor.composerLock`) alongside the original
//! source-based one K2a already had ([`fetch_composer_lock`], unchanged)
//! -- closes the locator gap K2c found recurring 3x
//! (flarum/baikal/postfixadmin).

use std::path::Path;
use std::process::Command;

use serde_json::Value as JsonValue;

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

/// K2a: automatic consumer provenance -- replaces the hand-verified
/// `DOCTRINE_DBAL_VERSION`/`DOCTRINE_DBAL_REV` constants above with a
/// real pipeline: nixpkgs revision -> the app's own package derivation's
/// `src` -> its vendored `composer.lock` -> the exact `doctrine/dbal`
/// entry. Deliberately narrow -- resolves ONE named package from an
/// already-fetched `composer.lock`; this does not become a
/// Composer/Packagist client. Resolution (pure, offline-testable) and
/// fetching (real Nix + network) are kept as separate functions on
/// purpose, not one that happens to also do I/O.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsumerIdentity {
    pub package: String,
    pub version: String,
    pub source_reference: String,
}

/// Pure: parses an already-fetched `composer.lock`'s JSON text and finds
/// the entry named EXACTLY `package_name` -- no fuzzy/prefix/similarity
/// matching anywhere near this trust boundary; a decoy package with a
/// similar name (`doctrine/dbal-foo`) must never be accepted in its
/// place (Rust `==` on `&str` already gives this for free -- verified by
/// a dedicated test with a decoy present, not just assumed). Fail-closed:
/// a missing `packages` array, zero or 2+ matching entries, or a missing
/// `version`/`source.reference` field on the match are all
/// `Inconclusive`, never a guessed identity.
pub fn resolve_consumer_identity(
    composer_lock_json: &str,
    package_name: &str,
) -> Result<ConsumerIdentity, CdcError> {
    let value: serde_json::Value = serde_json::from_str(composer_lock_json)
        .map_err(|e| CdcError::Inconclusive(format!("composer.lock is not valid JSON: {e}")))?;
    let packages = value.get("packages").and_then(|p| p.as_array()).ok_or_else(|| {
        CdcError::Inconclusive("composer.lock has no top-level \"packages\" array".to_string())
    })?;
    let matches: Vec<_> = packages
        .iter()
        .filter(|p| p.get("name").and_then(|n| n.as_str()) == Some(package_name))
        .collect();
    if matches.len() != 1 {
        return Err(CdcError::Inconclusive(format!(
            "composer.lock has {} entries named {package_name:?}, expected exactly 1",
            matches.len()
        )));
    }
    let pkg = matches[0];
    let version = pkg
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CdcError::Inconclusive(format!("{package_name} entry has no \"version\" string")))?
        .to_string();
    let source_reference = pkg
        .get("source")
        .and_then(|s| s.get("reference"))
        .and_then(|r| r.as_str())
        .ok_or_else(|| {
            CdcError::Inconclusive(format!("{package_name} entry has no \"source.reference\" string"))
        })?
        .to_string();
    Ok(ConsumerIdentity { package: package_name.to_string(), version, source_reference })
}

/// A resolved identity is only useful paired with a REAL source to
/// extract a contract from -- this refuses to let a vendored fixture
/// silently stand in for whatever `composer.lock` actually declared, if
/// the two ever disagree (a tampered/updated lockfile, a stale vendored
/// fixture, ...).
pub fn verify_identity_matches_vendored_fixture(
    identity: &ConsumerIdentity,
    vendored_source_reference: &str,
) -> Result<(), CdcError> {
    if identity.source_reference != vendored_source_reference {
        return Err(CdcError::Inconclusive(format!(
            "resolved {} source.reference {} does not match the vendored fixture's {} -- refusing \
             to extract a contract from a source that doesn't match the resolved identity",
            identity.package, identity.source_reference, vendored_source_reference
        )));
    }
    Ok(())
}

/// Fetching, kept separate from resolution on purpose (see this
/// section's own doc comment) -- real Nix evaluation asked directly for
/// the package's own vendored `composer.lock`, not a hand-rolled
/// imports/callPackage resolver walking `package.nix` itself.
/// `package_attr` is the top-level `pkgs.<attr>` name (`"kimai"`,
/// `"davis"`); `pkgs.<attr>.src` is that app's own `fetchFromGitHub`
/// result -- realizing it (a fixed-output derivation, cheap, and in
/// practice already cached on cache.nixos.org rather than a raw clone,
/// confirmed by trying it for real: ~2-3s each for kimai/davis) is what
/// makes `composer.lock`'s real bytes available to `readFile` at all.
pub fn fetch_composer_lock(rev: &str, package_attr: &str) -> Result<String, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        pkgs = import nixpkgsSrc {{ system = "x86_64-linux"; }};
        in builtins.readFile (pkgs.{package_attr}.src + "/composer.lock")"#
    );
    eval_nix_raw(&expr)
}

/// Reads the git commit `fixtures/integrity-lock.toml` already records
/// for the vendored Doctrine driver fixture -- the existing,
/// hand-verified provenance record from K1's own Phase A, reused here as
/// K2a's comparison oracle instead of a second Rust constant duplicating
/// the same fact under a different name (the removed
/// `DOCTRINE_DBAL_REV`).
fn vendored_doctrine_source_reference() -> Result<String, CdcError> {
    #[derive(serde::Deserialize)]
    struct Lock {
        fixture: Vec<Entry>,
    }
    #[derive(serde::Deserialize)]
    struct Entry {
        path: String,
        commit: String,
    }

    const VENDORED_PATH: &str = "fixtures/cdc/doctrine-dbal-3.10.6/PDO-MySQL-Driver.php";
    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/integrity-lock.toml");
    let text = std::fs::read_to_string(&lock_path)
        .map_err(|e| CdcError::ToolError(format!("reading {}: {e}", lock_path.display())))?;
    let lock: Lock = toml::from_str(&text)
        .map_err(|e| CdcError::ToolError(format!("parsing {}: {e}", lock_path.display())))?;
    lock.fixture
        .into_iter()
        .find(|e| e.path == VENDORED_PATH)
        .map(|e| e.commit)
        .ok_or_else(|| {
            CdcError::ToolError(format!(
                "fixtures/integrity-lock.toml has no entry for {VENDORED_PATH}"
            ))
        })
}

/// K2a.1: [`fetch_composer_lock`] above (unchanged, still used exactly as
/// before by every existing Kimai/Davis call site) only ever
/// looks at the package's OWN fetched source -- `pkgs.<attr>.src +
/// "/composer.lock"`. The K2c census found this fails outright for
/// `flarum`/`baikal`/`postfixadmin`: upstream doesn't ship a lock file
/// for these, so nixpkgs vendors its own, referenced from `package.nix`
/// via a `composerLock = ./composer.lock;` argument to
/// `buildComposerProject2`.
///
/// A second, EXPLICIT provenance source for that case -- deliberately
/// NOT "walk the directory next to `package.nix` and grab whatever
/// `composer.lock` is nearest" (exactly the kind of false-confidence
/// generator that would silently pick up an unrelated file with the
/// right name). Instead: `pkgs.<attr>.composerVendor.composerLock` is
/// the literal Nix path VALUE the package's own `package.nix` passed as
/// the `composerLock` argument -- confirmed by reading
/// `pkgs/build-support/php/builders/v2/build-composer-project.nix`
/// itself, not guessed: `composerVendor = args.composerVendor or
/// (php.mkComposerVendor { ...; composerLock; ...})`, and
/// `lib.extendMkDerivation`'s own merge semantics carry the caller's
/// `composerLock` argument through onto the resulting derivation's own
/// attribute set even though `mkComposerVendorOverride` never
/// re-exports it. So `.composerVendor.composerLock` isn't a guess about
/// where nixpkgs "usually" keeps a local lock -- it's asking Nix to hand
/// back the exact same path value the package expression itself
/// declared, resolved through the one relationship that actually
/// connects them. Real-verified for all three: `flarum` resolves to
/// `pkgs/by-name/fl/flarum/composer.lock` (contains a real
/// `doctrine/dbal` 2.13.9 entry -- Phase D for that specific version is
/// NOT vendored as part of K2a.1, same disclosed-not-silent deferral
/// K2c already used for `strichliste`'s 3.10.5); `baikal`/`postfixadmin`
/// resolve too, genuinely containing no `doctrine/dbal` entry at all
/// (matches K2c's "absent from the dependency closure" finding, now for
/// the right, confirmed reason rather than a blocked locator).
///
/// `kimai`/`davis`/`strichliste`/`agorakit`/`snipe-it` all resolve this
/// attribute to Nix `null` (the standard `buildComposerProject2` path
/// always forwards a `composerLock` argument, defaulting to `null` when
/// the package doesn't set one) -- `movim` is a genuine third case, the
/// attribute is entirely ABSENT rather than `null` (its `package.nix`
/// calls `php.mkComposerVendor` directly with its own argument list,
/// bypassing `buildComposerProject2`'s automatic forwarding, so
/// `composerLock` was never part of that call at all). The Nix `or null`
/// guard on the attribute select collapses both into the same "no local
/// lock via this relationship" outcome -- correct, since both really do
/// mean the same thing here, and movim isn't in K2a.1's target scope
/// (its own consumer support stays unsupported for the K2d reason:
/// Laravel's own DB connector, not a locator problem).
///
/// `None` is a real, valid outcome (most apps), not a failure --
/// [`resolve_composer_lock_origin`] decides what it means.
fn fetch_nixpkgs_local_composer_lock(rev: &str, package_attr: &str) -> Result<Option<String>, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        pkgs = import nixpkgsSrc {{ system = "x86_64-linux"; }};
        localLock = pkgs.{package_attr}.composerVendor.composerLock or null;
        in if localLock == null then null else builtins.readFile localLock"#
    );
    match eval_nix_json(&expr)? {
        JsonValue::Null => Ok(None),
        JsonValue::String(s) => Ok(Some(s)),
        other => Err(CdcError::ToolError(format!(
            "expected a JSON string or null from the nixpkgs-local composerLock probe, got {other}"
        ))),
    }
}

/// K2a.1's other half: the SAME source-based location `fetch_composer_lock`
/// already uses (`pkgs.<attr>.src + "/composer.lock"`), but existence-
/// checked via `builtins.pathExists` rather than letting a missing file
/// surface as a generic `readFile` failure -- the whole point of this
/// function is to give [`resolve_composer_lock_origin`] a clean `None`
/// for "genuinely absent" that doesn't require pattern-matching Nix's own
/// error text to distinguish from a real tool failure (network outage,
/// bad revision, etc.). Does NOT replace or call `fetch_composer_lock` --
/// that function and its existing Kimai/Davis callers are untouched by
/// K2a.1, on purpose.
fn fetch_source_composer_lock_if_exists(rev: &str, package_attr: &str) -> Result<Option<String>, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        pkgs = import nixpkgsSrc {{ system = "x86_64-linux"; }};
        lockPath = pkgs.{package_attr}.src + "/composer.lock";
        in if builtins.pathExists lockPath then builtins.readFile lockPath else null"#
    );
    match eval_nix_json(&expr)? {
        JsonValue::Null => Ok(None),
        JsonValue::String(s) => Ok(Some(s)),
        other => Err(CdcError::ToolError(format!(
            "expected a JSON string or null from the source composer.lock existence probe, got {other}"
        ))),
    }
}

/// Provenance -- not architectural decoration. Once
/// [`resolve_composer_lock`] returns, a Finding can honestly say which
/// of the two real sources its consumer contract actually came from,
/// rather than silently treating both as "the composer.lock". Wiring
/// this into an actual report/Finding type is future work (this module
/// still has no CLI/report integration at all, per its own top-level
/// doc comment) -- not attempted in K2a.1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComposerLockOrigin {
    PackageSource,
    NixpkgsLocal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedComposerLock {
    pub origin: ComposerLockOrigin,
    pub content: String,
}

/// Pure (no Nix/network) -- the actual decision the stop condition asked
/// to be provable, not a "first candidate wins" convention. `NixpkgsLocal`
/// wins whenever present, REGARDLESS of whether a source-based candidate
/// is also present: `build-composer-project.nix`'s own `composerVendor`
/// construction feeds a non-null `composerLock` straight to
/// `composer install` as the CommandUsed lock (confirmed by reading that
/// file, not assumed -- see [`fetch_nixpkgs_local_composer_lock`]'s doc
/// comment) -- when it's set, the real build never reads whatever
/// composer.lock might also happen to sit inside `src`, so there is no
/// actual ambiguity to break a tie on, only a fact to report correctly.
/// This is not exercised by a "both present" case in the current real
/// corpus (no surveyed app has both) -- covered by a dedicated offline
/// test using two deliberately DIFFERENT synthetic contents, so the
/// priority is checked by construction, not merely documented. Neither
/// candidate present is `Inconclusive`, never a guessed empty lock.
pub fn resolve_composer_lock_origin(
    nixpkgs_local: Option<String>,
    package_source: Option<String>,
) -> Result<ResolvedComposerLock, CdcError> {
    if let Some(content) = nixpkgs_local {
        return Ok(ResolvedComposerLock { origin: ComposerLockOrigin::NixpkgsLocal, content });
    }
    if let Some(content) = package_source {
        return Ok(ResolvedComposerLock { origin: ComposerLockOrigin::PackageSource, content });
    }
    Err(CdcError::Inconclusive(
        "no composer.lock resolved via either the nixpkgs-local composerLock relationship or the package's own fetched source".to_string(),
    ))
}

/// Real orchestration: fetches both candidates for real, then defers the
/// actual decision to the pure [`resolve_composer_lock_origin`]. The
/// locator abstraction ends HERE -- [`resolve_consumer_identity`] (K2a,
/// unchanged) takes `resolved.content` exactly as it always has, with no
/// idea which of the two sources it came from.
pub fn resolve_composer_lock(rev: &str, package_attr: &str) -> Result<ResolvedComposerLock, CdcError> {
    let nixpkgs_local = fetch_nixpkgs_local_composer_lock(rev, package_attr)?;
    let package_source = fetch_source_composer_lock_if_exists(rev, package_attr)?;
    resolve_composer_lock_origin(nixpkgs_local, package_source)
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

/// K2d: same semantic oracle as [`eval_nix_raw`], but for a Nix value
/// that isn't a plain string -- `FlatEnvVars` evidence is a whole
/// attrset (`services.<app>.config`/`.settings`), not one query-string
/// scalar, so `--json` (structured) replaces `--raw` (string-only) for
/// this call shape. A parse failure here is `ToolError` (the tool ran
/// but its own JSON decoding broke), distinct from `Inconclusive`
/// (the tool ran fine, the *evidence shape* is the problem) -- that
/// distinction is drawn in [`build_flat_env_vars_evidence`], not here.
fn eval_nix_json(expr: &str) -> Result<JsonValue, CdcError> {
    let out = Command::new("nix")
        .args(["eval", "--impure", "--json", "--expr", expr])
        .output()
        .map_err(|e| CdcError::ToolError(format!("spawning `nix`: {e}")))?;
    if !out.status.success() {
        return Err(CdcError::ToolError(format!(
            "nix eval failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    serde_json::from_slice(&out.stdout)
        .map_err(|e| CdcError::ToolError(format!("nix eval produced invalid JSON: {e}")))
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
/// K2d: no mutation testing is done against these three modules
/// (`module_override` is always `None` for them -- K2d proves evidence
/// *acquisition*, not a historical fix/before-after pair the way K1 did
/// for Kimai/Davis), but [`nixos_eval_expr`] still takes an
/// `upstream_path` argument, so these are recorded for documentation
/// parity with [`KIMAI_UPSTREAM_PATH`]/[`DAVIS_UPSTREAM_PATH`].
const AGORAKIT_UPSTREAM_PATH: &str = "nixos/modules/services/web-apps/agorakit.nix";
const SNIPEIT_UPSTREAM_PATH: &str = "nixos/modules/services/web-apps/snipe-it.nix";
const MOVIM_UPSTREAM_PATH: &str = "nixos/modules/services/web-apps/movim.nix";

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

/// K2b: generalizes HOW producer evidence was acquired without touching
/// consumer provenance (K2a) or `compare_contract` (Phase E) at all.
/// Kimai's `database.socket` is a real option, so sentinel-injection
/// proves the option->sink flow; Davis's socket path is a hardcoded
/// literal with nothing to inject a sentinel into, so the evidence is a
/// directly evaluated literal instead. Neither variant is "stronger" in
/// verdict semantics -- once `emitted_key()` is in hand, both go through
/// the IDENTICAL downstream pipeline (`ProducerEvidence::emitted_key()` ->
/// `compare_contract`), proven by a dedicated test using both variants
/// against the same `accepted_keys`, not just asserted in a doc comment.
/// Deliberately NOT a generic sink-discovery mechanism -- both variants
/// still know their own concrete, hand-picked sink (Kimai's
/// `kimai-init-<name>` script, Davis's `services.davis.config`); K2b
/// generalizes the SHAPE of the proof, not how sinks get found.
/// K2d adds `FlatEnvVars`: the shape the K2c census found recurring 3
/// times in the corpus (agorakit, movim, snipe-it) -- discrete `DB_HOST`/
/// `DB_PORT`/`DB_SOCKET`/etc. environment-variable keys rendered directly
/// as a Nix attrset, not one value embedded in a DSN query string. There
/// is no single `emitted_key` to compare here (that's the whole point --
/// a DSN's `unix_socket=...` query parameter and a bare `DB_SOCKET` env
/// var are not the same kind of fact), so this variant carries the full
/// set of rendered keys instead. Deliberately does NOT attempt to map
/// `DB_HOST` (or any other key) onto Doctrine's `host`/`unix_socket`
/// parameter names -- for every app this variant covers, that mapping
/// crosses a framework layer (Laravel's `Illuminate\Database` connector)
/// this module doesn't model at all; see the K2c census's own
/// "presence isn't wiring" finding. Acquiring producer evidence is the
/// only job this variant does -- consumer comparison for these apps is
/// allowed to stay honestly `unsupported`, not guessed.
#[derive(Debug, Clone, PartialEq)]
pub enum ProducerEvidence {
    SentinelFlow {
        option: String,
        sentinel: String,
        sink: String,
        rendered_value: String,
        emitted_key: String,
    },
    EvaluatedLiteral {
        sink: String,
        rendered_value: String,
        emitted_key: String,
    },
    FlatEnvVars {
        sink: String,
        emitted: Vec<String>,
    },
}

impl ProducerEvidence {
    /// `None` for `FlatEnvVars` -- there is no single comparable key for
    /// that shape (see the variant's own doc comment). Existing callers
    /// (`SentinelFlow`/`EvaluatedLiteral` via [`compare_contract`]) still
    /// get `Some`, unchanged.
    pub fn emitted_key(&self) -> Option<&str> {
        match self {
            ProducerEvidence::SentinelFlow { emitted_key, .. }
            | ProducerEvidence::EvaluatedLiteral { emitted_key, .. } => Some(emitted_key),
            ProducerEvidence::FlatEnvVars { .. } => None,
        }
    }
}

/// Pure (no Nix/network) -- the fail-closed producer-evidence mutations
/// asked for (sentinel absent, sentinel ambiguous) are `extract_key_for_value`
/// failures propagated here, covered by dedicated tests at THIS level,
/// not just at Phase C's.
pub fn build_sentinel_flow_evidence(
    option: &str,
    sink: &str,
    sentinel: &str,
    rendered_value: String,
) -> Result<ProducerEvidence, CdcError> {
    let emitted_key = extract_key_for_value(&rendered_value, sentinel)?;
    Ok(ProducerEvidence::SentinelFlow {
        option: option.to_string(),
        sentinel: sentinel.to_string(),
        sink: sink.to_string(),
        rendered_value,
        emitted_key,
    })
}

/// Pure, same fail-closed guarantee as above (a malformed DSN or an
/// unextractable key is `Inconclusive`, never a guessed literal).
pub fn build_evaluated_literal_evidence(
    sink: &str,
    rendered_value: String,
    known_literal: &str,
) -> Result<ProducerEvidence, CdcError> {
    let emitted_key = extract_key_for_value(&rendered_value, known_literal)?;
    Ok(ProducerEvidence::EvaluatedLiteral { sink: sink.to_string(), rendered_value, emitted_key })
}

const KIMAI_OPTION: &str = "services.kimai.sites.\"probe\".database.socket";
const KIMAI_SINK: &str = "systemd.services.\"kimai-init-probe\".script";
const DAVIS_SINK: &str = "services.davis.config.DATABASE_URL";
/// Davis's hardcoded MySQL unix-socket path -- not a sentinel, a known
/// constant this probe searches for directly (see
/// [`ProducerEvidence::EvaluatedLiteral`]'s own doc comment).
const DAVIS_KNOWN_SOCKET_LITERAL: &str = "/run/mysqld/mysqld.sock";

/// Real acquisition for Kimai: eval (Phase B, unchanged) + build (pure).
pub fn acquire_kimai_evidence(
    rev: &str,
    sentinel: &str,
    module_override: Option<&Path>,
) -> Result<ProducerEvidence, CdcError> {
    let rendered = eval_kimai_script(rev, sentinel, module_override)?;
    build_sentinel_flow_evidence(KIMAI_OPTION, KIMAI_SINK, sentinel, rendered)
}

/// Real acquisition for Davis: eval (Phase B, unchanged) + build (pure).
pub fn acquire_davis_evidence(rev: &str) -> Result<ProducerEvidence, CdcError> {
    let rendered = eval_davis_database_url(rev)?;
    build_evaluated_literal_evidence(DAVIS_SINK, rendered, DAVIS_KNOWN_SOCKET_LITERAL)
}

/// K2d, pure (no Nix/network) -- fail-closed on every shape that isn't a
/// non-empty JSON object: `rendered` not an object at all (`Inconclusive`,
/// evidence isn't even the right kind of value), or an object with zero
/// keys (`Inconclusive`, nothing to compare against later). A duplicate
/// key is not a failure mode modeled here -- unlike a raw DSN query
/// string (Phase C's problem), a Nix attrset and the JSON object it
/// evaluates to structurally cannot contain the same key twice, so there
/// is nothing to guard against at this layer.
pub fn build_flat_env_vars_evidence(
    sink: &str,
    rendered: JsonValue,
) -> Result<ProducerEvidence, CdcError> {
    let JsonValue::Object(map) = rendered else {
        return Err(CdcError::Inconclusive(format!(
            "rendered evidence at {sink} is not a JSON object: {rendered}"
        )));
    };
    if map.is_empty() {
        return Err(CdcError::Inconclusive(format!(
            "rendered evidence at {sink} is an empty object -- nothing to compare"
        )));
    }
    let mut emitted: Vec<String> = map.into_iter().map(|(k, _)| k).collect();
    emitted.sort();
    Ok(ProducerEvidence::FlatEnvVars { sink: sink.to_string(), emitted })
}

const AGORAKIT_SINK: &str = "services.agorakit.config";
const SNIPEIT_SINK: &str = "services.snipe-it.config";
const MOVIM_SINK: &str = "services.movim.settings";

/// Real acquisition for agorakit: `services.agorakit.config` is a plain
/// Nix attrset rendering discrete `DB_*` keys directly (no sentinel to
/// inject, no DSN string to parse -- this app never assembles one).
pub fn acquire_agorakit_evidence(rev: &str) -> Result<ProducerEvidence, CdcError> {
    let expr = nixos_eval_expr(
        rev,
        AGORAKIT_UPSTREAM_PATH,
        None,
        r#"services.agorakit = { enable = true; appKeyFile = "/dev/null"; database.createLocally = true; };"#,
    );
    let rendered = eval_nix_json(&format!("({expr}).config.services.agorakit.config"))?;
    build_flat_env_vars_evidence(AGORAKIT_SINK, rendered)
}

/// Real acquisition for snipe-it: same flat-env shape as agorakit, but
/// with a dedicated `DB_SOCKET` key alongside `DB_HOST`/`DB_PORT` --
/// recorded in the K2c census as the one corpus service with a
/// socket-equivalent field in this shape.
pub fn acquire_snipeit_evidence(rev: &str) -> Result<ProducerEvidence, CdcError> {
    let expr = nixos_eval_expr(
        rev,
        SNIPEIT_UPSTREAM_PATH,
        None,
        r#"services.snipe-it = { enable = true; appKeyFile = "/dev/null"; database.createLocally = true; };"#,
    );
    let rendered = eval_nix_json(&format!("({expr}).config.services.snipe-it.config"))?;
    build_flat_env_vars_evidence(SNIPEIT_SINK, rendered)
}

/// Real acquisition for movim: same flat-env shape, rendered via
/// `services.movim.settings` rather than a `.config` attribute. Uses
/// `database.type = "postgresql"`, not `"mariadb"` -- the `mariadb` path
/// hits a real, independently confirmed nixpkgs bug at
/// `movim.nix:628` (`config.services.${cfg.database.type}.settings.port`
/// interpolates the enum value "mariadb" directly as a `services.<x>`
/// attribute name, but the actual NixOS service the module registers a
/// few lines later is `services.mysql`, not `services.mariadb` -- so the
/// attribute lookup fails outright). Not fixed here (a real nixpkgs bug,
/// but a different one from this module's own defect class, and out of
/// scope for K2d); worked around by using `postgresql`, a correctly-wired
/// sibling path in the same module, to prove the acquisition mechanism.
pub fn acquire_movim_evidence(rev: &str) -> Result<ProducerEvidence, CdcError> {
    let expr = nixos_eval_expr(
        rev,
        MOVIM_UPSTREAM_PATH,
        None,
        r#"services.movim = { enable = true; domain = "movim.example.com"; database.createLocally = true; database.type = "postgresql"; };"#,
    );
    let rendered = eval_nix_json(&format!("({expr}).config.services.movim.settings"))?;
    build_flat_env_vars_evidence(MOVIM_SINK, rendered)
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

    // --- K2b: producer evidence generalization, offline (both builder
    // functions are pure -- only acquire_kimai_evidence/
    // acquire_davis_evidence touch nix/network, covered separately
    // below) ---

    #[test]
    fn sentinel_flow_absent_sentinel_is_inconclusive() {
        assert!(matches!(
            build_sentinel_flow_evidence("opt", "sink", "/needle", "no match here".to_string()),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn sentinel_flow_ambiguous_sentinel_is_inconclusive() {
        let rendered = "a=/needle&b=/needle".to_string();
        assert!(matches!(
            build_sentinel_flow_evidence("opt", "sink", "/needle", rendered),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn sentinel_flow_success_carries_the_full_evidence() {
        let rendered = "mysql://h?charset=utf8&unix_socket=/needle".to_string();
        let evidence =
            build_sentinel_flow_evidence("services.kimai...socket", "kimai-init.script", "/needle", rendered)
                .unwrap();
        match evidence {
            ProducerEvidence::SentinelFlow { option, sentinel, sink, emitted_key, .. } => {
                assert_eq!(option, "services.kimai...socket");
                assert_eq!(sentinel, "/needle");
                assert_eq!(sink, "kimai-init.script");
                assert_eq!(emitted_key, "unix_socket");
            }
            other => panic!("expected SentinelFlow, got {other:?}"),
        }
    }

    #[test]
    fn evaluated_literal_malformed_dsn_is_inconclusive() {
        // literal present but not preceded by '=' -- not a DSN key/value pair.
        let rendered = "not a dsn at all, just contains /run/mysqld/mysqld.sock somewhere".to_string();
        assert!(matches!(
            build_evaluated_literal_evidence("sink", rendered, "/run/mysqld/mysqld.sock"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn evaluated_literal_unextractable_key_is_inconclusive() {
        // literal appears twice -- key cannot be attributed unambiguously.
        let rendered = "a=/run/mysqld/mysqld.sock&b=/run/mysqld/mysqld.sock".to_string();
        assert!(matches!(
            build_evaluated_literal_evidence("sink", rendered, "/run/mysqld/mysqld.sock"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn evaluated_literal_success_carries_the_full_evidence() {
        let rendered = "mysql://h?unix_socket=/run/mysqld/mysqld.sock".to_string();
        let evidence =
            build_evaluated_literal_evidence("davis.config", rendered, "/run/mysqld/mysqld.sock")
                .unwrap();
        match evidence {
            ProducerEvidence::EvaluatedLiteral { sink, emitted_key, .. } => {
                assert_eq!(sink, "davis.config");
                assert_eq!(emitted_key, "unix_socket");
            }
            other => panic!("expected EvaluatedLiteral, got {other:?}"),
        }
    }

    /// The invariant explicitly required: neither `ProducerEvidence`
    /// variant is "stronger" in verdict semantics. Two DIFFERENT
    /// variants that both unambiguously resolved the SAME emitted_key
    /// must produce the IDENTICAL verdict against the same accepted
    /// keys -- checked by construction (both go through
    /// `compare_contract(evidence.emitted_key(), ...)`), not by
    /// convention.
    #[test]
    fn sentinel_flow_and_evaluated_literal_are_never_treated_differently_by_comparison() {
        let accepted = vec!["host".to_string(), "unix_socket".to_string()];

        let sentinel_flow =
            build_sentinel_flow_evidence("opt", "sink", "/s", "unix_socket=/s".to_string()).unwrap();
        let evaluated_literal =
            build_evaluated_literal_evidence("sink", "unix_socket=/lit".to_string(), "/lit").unwrap();

        assert_eq!(sentinel_flow.emitted_key(), evaluated_literal.emitted_key());
        let sentinel_flow_key = sentinel_flow.emitted_key().unwrap();
        let evaluated_literal_key = evaluated_literal.emitted_key().unwrap();
        assert_eq!(
            compare_contract(sentinel_flow_key, &accepted),
            compare_contract(evaluated_literal_key, &accepted)
        );
        assert_eq!(compare_contract(sentinel_flow_key, &accepted), CdcVerdict::Pass);
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

    // --- K2a: automatic consumer provenance, offline (resolution is
    // pure -- these never touch nix/network, only fetch_composer_lock
    // does, covered separately below) ---

    fn synthetic_lock(packages_json: &str) -> String {
        format!(r#"{{"packages": [{packages_json}]}}"#)
    }

    const REAL_DOCTRINE_ENTRY: &str = r#"{"name": "doctrine/dbal", "version": "3.10.6", "source": {"type": "git", "reference": "c95589d775a0b2e543467d40f8c3ecccf586f2b4"}}"#;

    #[test]
    fn resolves_the_real_shaped_doctrine_entry() {
        let lock = synthetic_lock(REAL_DOCTRINE_ENTRY);
        let identity = resolve_consumer_identity(&lock, "doctrine/dbal").unwrap();
        assert_eq!(
            identity,
            ConsumerIdentity {
                package: "doctrine/dbal".to_string(),
                version: "3.10.6".to_string(),
                source_reference: "c95589d775a0b2e543467d40f8c3ecccf586f2b4".to_string(),
            }
        );
    }

    #[test]
    fn missing_packages_array_is_inconclusive() {
        assert!(matches!(
            resolve_consumer_identity(r#"{"not-packages": []}"#, "doctrine/dbal"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn removed_entry_is_inconclusive_not_a_stale_pass() {
        let lock = synthetic_lock(r#"{"name": "some/other-package", "version": "1.0.0"}"#);
        assert!(matches!(
            resolve_consumer_identity(&lock, "doctrine/dbal"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    /// A decoy package with a similar name sitting right next to the
    /// real one -- proves exact-string matching, not just the absence of
    /// a fuzzy matcher.
    #[test]
    fn similarly_named_decoy_package_is_never_accepted_in_place_of_the_real_one() {
        let lock = synthetic_lock(&format!(
            r#"{{"name": "doctrine/dbal-foo", "version": "9.9.9", "source": {{"reference": "deadbeef"}}}}, {REAL_DOCTRINE_ENTRY}"#
        ));
        let identity = resolve_consumer_identity(&lock, "doctrine/dbal").unwrap();
        assert_eq!(identity.version, "3.10.6");
        assert_eq!(identity.source_reference, "c95589d775a0b2e543467d40f8c3ecccf586f2b4");
    }

    #[test]
    fn duplicate_real_entries_is_inconclusive_not_first_match() {
        let lock = synthetic_lock(&format!("{REAL_DOCTRINE_ENTRY}, {REAL_DOCTRINE_ENTRY}"));
        assert!(matches!(
            resolve_consumer_identity(&lock, "doctrine/dbal"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn missing_source_reference_is_inconclusive() {
        let lock = synthetic_lock(r#"{"name": "doctrine/dbal", "version": "3.10.6"}"#);
        assert!(matches!(
            resolve_consumer_identity(&lock, "doctrine/dbal"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    /// The provenance mutation explicitly asked for: a tampered
    /// `source.reference` must never be silently paired with the
    /// vendored fixture as if nothing changed.
    #[test]
    fn tampered_source_reference_fails_the_vendored_fixture_match() {
        let lock = synthetic_lock(r#"{"name": "doctrine/dbal", "version": "3.10.6", "source": {"reference": "0000000000000000000000000000000000000000"}}"#);
        let identity = resolve_consumer_identity(&lock, "doctrine/dbal").unwrap();
        assert!(matches!(
            verify_identity_matches_vendored_fixture(
                &identity,
                "c95589d775a0b2e543467d40f8c3ecccf586f2b4"
            ),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn matching_source_reference_passes_the_vendored_fixture_match() {
        let lock = synthetic_lock(REAL_DOCTRINE_ENTRY);
        let identity = resolve_consumer_identity(&lock, "doctrine/dbal").unwrap();
        assert!(verify_identity_matches_vendored_fixture(
            &identity,
            "c95589d775a0b2e543467d40f8c3ecccf586f2b4"
        )
        .is_ok());
    }

    // --- K2a.1: composer.lock locator, offline (resolve_composer_lock_origin
    // is pure -- only fetch_nixpkgs_local_composer_lock/
    // fetch_source_composer_lock_if_exists touch nix/network, covered
    // separately below) ---

    #[test]
    fn composer_lock_locator_neither_candidate_present_is_inconclusive() {
        assert!(matches!(
            resolve_composer_lock_origin(None, None),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn composer_lock_locator_only_nixpkgs_local_present() {
        let resolved = resolve_composer_lock_origin(Some("local content".to_string()), None).unwrap();
        assert_eq!(resolved.origin, ComposerLockOrigin::NixpkgsLocal);
        assert_eq!(resolved.content, "local content");
    }

    #[test]
    fn composer_lock_locator_only_package_source_present() {
        let resolved = resolve_composer_lock_origin(None, Some("source content".to_string())).unwrap();
        assert_eq!(resolved.origin, ComposerLockOrigin::PackageSource);
        assert_eq!(resolved.content, "source content");
    }

    /// The actual point of the stop condition: NOT "first candidate
    /// wins" -- deliberately different content on each side proves the
    /// result really is `NixpkgsLocal`'s content, not an accident of
    /// which one happened to be checked first.
    #[test]
    fn composer_lock_locator_both_present_nixpkgs_local_wins_by_construction() {
        let resolved = resolve_composer_lock_origin(
            Some("local content".to_string()),
            Some("source content".to_string()),
        )
        .unwrap();
        assert_eq!(resolved.origin, ComposerLockOrigin::NixpkgsLocal);
        assert_eq!(resolved.content, "local content");
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

    // --- K2d: FlatEnvVars, offline (build_flat_env_vars_evidence is pure
    // -- only acquire_{agorakit,snipeit,movim}_evidence touch nix/network,
    // covered separately below) ---

    #[test]
    fn flat_env_vars_non_object_is_inconclusive() {
        assert!(matches!(
            build_flat_env_vars_evidence("sink", serde_json::json!(["not", "an", "object"])),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn flat_env_vars_scalar_is_inconclusive() {
        assert!(matches!(
            build_flat_env_vars_evidence("sink", serde_json::json!("just a string")),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn flat_env_vars_empty_object_is_inconclusive() {
        assert!(matches!(
            build_flat_env_vars_evidence("sink", serde_json::json!({})),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn flat_env_vars_success_carries_every_key_sorted_and_has_no_emitted_key() {
        let rendered = serde_json::json!({
            "DB_HOST": "localhost",
            "DB_PORT": 3306,
            "DB_DATABASE": "app",
        });
        let evidence = build_flat_env_vars_evidence("services.app.config", rendered).unwrap();
        match &evidence {
            ProducerEvidence::FlatEnvVars { sink, emitted } => {
                assert_eq!(sink, "services.app.config");
                assert_eq!(emitted, &vec!["DB_DATABASE".to_string(), "DB_HOST".to_string(), "DB_PORT".to_string()]);
            }
            other => panic!("expected FlatEnvVars, got {other:?}"),
        }
        assert_eq!(evidence.emitted_key(), None);
    }

    // --- K2a real transition tests: opt-in only, same as the K1 golden
    // proof below. Auto-resolves EACH app's consumer identity
    // independently (never sharing one lookup between kimai/davis --
    // invariant #2) via the real fetch_composer_lock -> real
    // resolve_consumer_identity pipeline, for BOTH the before and after
    // revision (kimai/davis's own package.nix is unrelated to the
    // module-level fix, but this is checked, not assumed), and checks
    // the result against the vendored fixture's own recorded provenance
    // via verify_identity_matches_vendored_fixture -- the same function
    // production use would call, exercised here end-to-end rather than
    // only against synthetic strings. This comparison is what PROVED the
    // now-removed DOCTRINE_DBAL_VERSION/REV constants were redundant,
    // before they were deleted -- see git history for the version of
    // this file where the comparison ran against those constants
    // directly, kept green through the swap. ---

    fn assert_auto_resolved_matches_vendored_fixture(rev: &str, package_attr: &str) {
        let lock = fetch_composer_lock(rev, package_attr).unwrap();
        let identity = resolve_consumer_identity(&lock, "doctrine/dbal").unwrap();
        let vendored = vendored_doctrine_source_reference().unwrap();
        verify_identity_matches_vendored_fixture(&identity, &vendored).unwrap();
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn kimai_auto_resolved_provenance_matches_vendored_fixture_before() {
        assert_auto_resolved_matches_vendored_fixture(BEFORE_REV, "kimai");
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn kimai_auto_resolved_provenance_matches_vendored_fixture_after() {
        assert_auto_resolved_matches_vendored_fixture(AFTER_REV, "kimai");
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn davis_auto_resolved_provenance_matches_vendored_fixture_before() {
        assert_auto_resolved_matches_vendored_fixture(BEFORE_REV, "davis");
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn davis_auto_resolved_provenance_matches_vendored_fixture_after() {
        assert_auto_resolved_matches_vendored_fixture(AFTER_REV, "davis");
    }

    // --- K2a.1: composer.lock locator, real (opt-in only, same as every
    // other real test in this module). Closes the exact gap K2c found:
    // flarum/baikal/postfixadmin's `pkgs.<attr>.src + "/composer.lock"`
    // fails outright (no lock file there at all), so the NixpkgsLocal
    // path is the only one that can resolve them. ---

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn flarum_composer_lock_resolves_via_nixpkgs_local_and_finds_doctrine_dbal() {
        let resolved = resolve_composer_lock(AFTER_REV, "flarum").unwrap();
        assert_eq!(resolved.origin, ComposerLockOrigin::NixpkgsLocal);
        let identity = resolve_consumer_identity(&resolved.content, "doctrine/dbal").unwrap();
        assert_eq!(identity.version, "2.13.9");
        assert_eq!(identity.source_reference, "c480849ca3ad6706a39c970cdfe6888fa8a058b8");
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn baikal_composer_lock_resolves_via_nixpkgs_local_with_no_doctrine_dbal() {
        let resolved = resolve_composer_lock(AFTER_REV, "baikal").unwrap();
        assert_eq!(resolved.origin, ComposerLockOrigin::NixpkgsLocal);
        assert!(matches!(
            resolve_consumer_identity(&resolved.content, "doctrine/dbal"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn postfixadmin_composer_lock_resolves_via_nixpkgs_local_with_no_doctrine_dbal() {
        let resolved = resolve_composer_lock(AFTER_REV, "postfixadmin").unwrap();
        assert_eq!(resolved.origin, ComposerLockOrigin::NixpkgsLocal);
        assert!(matches!(
            resolve_consumer_identity(&resolved.content, "doctrine/dbal"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    /// Regression, all three previously-supported apps: the NEW unified
    /// resolver must still land on `PackageSource`, never accidentally
    /// report `NixpkgsLocal` for an app that never set a local lock.
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn kimai_composer_lock_still_resolves_via_package_source() {
        let resolved = resolve_composer_lock(AFTER_REV, "kimai").unwrap();
        assert_eq!(resolved.origin, ComposerLockOrigin::PackageSource);
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn davis_composer_lock_still_resolves_via_package_source() {
        let resolved = resolve_composer_lock(AFTER_REV, "davis").unwrap();
        assert_eq!(resolved.origin, ComposerLockOrigin::PackageSource);
    }

    /// `strichliste` never became a code-level caller of
    /// `fetch_composer_lock` before this round (K2c verified it manually
    /// via a one-off shell transcript, recorded in the census, not as a
    /// `cdc::` test) -- this is the first automated real test for it,
    /// proving BOTH that the old function is untouched (called directly)
    /// AND that the new unified resolver agrees with it.
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn strichliste_composer_lock_old_path_and_new_resolver_agree() {
        let old_path_content = fetch_composer_lock(AFTER_REV, "strichliste").unwrap();
        let identity = resolve_consumer_identity(&old_path_content, "doctrine/dbal").unwrap();
        assert_eq!(identity.version, "3.10.5");

        let resolved = resolve_composer_lock(AFTER_REV, "strichliste").unwrap();
        assert_eq!(resolved.origin, ComposerLockOrigin::PackageSource);
        assert_eq!(resolved.content, old_path_content);
    }

    // --- Real end-to-end: needs a real `nix` binary + network, opt-in
    // only (`cargo test --ignored -- cdc::`). These are the K1 golden
    // proof and mutation proof, now routed through ProducerEvidence
    // (K2b) -- Kimai's SentinelFlow and Davis's EvaluatedLiteral both
    // funnel through the SAME verdict_for_evidence below; branching ends
    // the moment ProducerEvidence exists, not at comparison time. ---

    fn accepted_doctrine_keys() -> Vec<String> {
        let doctrine_src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/cdc/doctrine-dbal-3.10.6/PDO-MySQL-Driver.php"),
        )
        .unwrap();
        extract_accepted_keys(&doctrine_src).unwrap()
    }

    /// THE unified downstream step -- identical for both `ProducerEvidence`
    /// variants that carry a comparable key. Its signature (`&ProducerEvidence`,
    /// not `&SentinelFlow` or two separate functions) is what actually
    /// proves branching ends before comparison, not a doc comment's claim
    /// about it. Only ever called with Kimai/Davis evidence (`SentinelFlow`/
    /// `EvaluatedLiteral`) -- `unwrap()` on `emitted_key()` is deliberate
    /// here, not a K2d oversight: `FlatEnvVars` has no consumer-side
    /// verdict path yet (see [`ProducerEvidence::FlatEnvVars`]'s own doc
    /// comment), so this helper is never asked to compare one.
    fn verdict_for_evidence(evidence: &ProducerEvidence) -> CdcVerdict {
        compare_contract(evidence.emitted_key().unwrap(), &accepted_doctrine_keys())
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn golden_kimai_before_is_finding() {
        let evidence =
            acquire_kimai_evidence(BEFORE_REV, "/__OBA_CONTRACT_socket_k1a__/mysql.sock", None)
                .unwrap();
        assert!(matches!(evidence, ProducerEvidence::SentinelFlow { .. }));
        assert!(matches!(verdict_for_evidence(&evidence), CdcVerdict::Finding { .. }));
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn golden_kimai_after_is_pass() {
        let evidence =
            acquire_kimai_evidence(AFTER_REV, "/__OBA_CONTRACT_socket_k1b__/mysql.sock", None)
                .unwrap();
        assert!(matches!(evidence, ProducerEvidence::SentinelFlow { .. }));
        assert_eq!(verdict_for_evidence(&evidence), CdcVerdict::Pass);
    }

    /// Davis is no longer a special `if app == davis` case at the
    /// verdict level -- it acquires `ProducerEvidence::EvaluatedLiteral`
    /// and goes through the exact same `verdict_for_evidence` Kimai does.
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn golden_davis_before_is_finding() {
        let evidence = acquire_davis_evidence(BEFORE_REV).unwrap();
        assert!(matches!(evidence, ProducerEvidence::EvaluatedLiteral { .. }));
        assert!(matches!(verdict_for_evidence(&evidence), CdcVerdict::Finding { .. }));
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn golden_davis_after_is_pass() {
        let evidence = acquire_davis_evidence(AFTER_REV).unwrap();
        assert!(matches!(evidence, ProducerEvidence::EvaluatedLiteral { .. }));
        assert_eq!(verdict_for_evidence(&evidence), CdcVerdict::Pass);
    }

    /// Mutation proof: takes the REAL fixed module source, patches ONLY
    /// the `unix_socket` literal, and runs it through the EXACT SAME
    /// `acquire_kimai_evidence` + module-override mechanism the golden
    /// proof uses -- not a separate mutation-only fixture/parser.
    fn mutation_verdict(mutated_key: &str) -> CdcVerdict {
        let fixed_source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/kimai/after/module.nix"),
        )
        .unwrap();
        let mutated = fixed_source.replace("unix_socket", mutated_key);
        let tmp = std::env::temp_dir().join(format!("oba-cdc-mutation-{mutated_key}.nix"));
        std::fs::write(&tmp, mutated).unwrap();

        let sentinel = "/__OBA_CONTRACT_socket_mut__/mysql.sock";
        let evidence = acquire_kimai_evidence(AFTER_REV, sentinel, Some(&tmp)).unwrap();
        verdict_for_evidence(&evidence)
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

    // --- K2d real acquisition: opt-in only, same as K1/K2a/K2b above.
    // One unified assertion helper for all three apps -- no app-name
    // branch in what's checked, only in which acquire_* function and
    // which expected keys are passed in (the K2d stop condition). ---

    fn assert_flat_env_vars(
        evidence: &ProducerEvidence,
        expected_sink: &str,
        must_contain: &[&str],
    ) {
        match evidence {
            ProducerEvidence::FlatEnvVars { sink, emitted } => {
                assert_eq!(sink, expected_sink);
                for key in must_contain {
                    assert!(emitted.contains(&key.to_string()), "missing {key} in {emitted:?}");
                }
            }
            other => panic!("expected FlatEnvVars, got {other:?}"),
        }
        assert_eq!(evidence.emitted_key(), None);
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn agorakit_real_evidence_is_flat_env_vars_with_no_socket_key() {
        let evidence = acquire_agorakit_evidence(AFTER_REV).unwrap();
        assert_flat_env_vars(
            &evidence,
            AGORAKIT_SINK,
            &["DB_HOST", "DB_PORT", "DB_DATABASE", "DB_USERNAME", "DB_PASSWORD"],
        );
        if let ProducerEvidence::FlatEnvVars { emitted, .. } = &evidence {
            assert!(!emitted.contains(&"DB_SOCKET".to_string()));
        }
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn snipeit_real_evidence_is_flat_env_vars_with_a_dedicated_socket_key() {
        let evidence = acquire_snipeit_evidence(AFTER_REV).unwrap();
        assert_flat_env_vars(
            &evidence,
            SNIPEIT_SINK,
            &["DB_HOST", "DB_PORT", "DB_DATABASE", "DB_USERNAME", "DB_PASSWORD", "DB_SOCKET"],
        );
    }

    /// Uses `database.type = "postgresql"` inside `acquire_movim_evidence`
    /// (see that function's own doc comment for the real, independently
    /// confirmed `mariadb`-path nixpkgs bug this works around).
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn movim_real_evidence_is_flat_env_vars() {
        let evidence = acquire_movim_evidence(AFTER_REV).unwrap();
        assert_flat_env_vars(
            &evidence,
            MOVIM_SINK,
            &["DB_HOST", "DB_PORT", "DB_DATABASE", "DB_USERNAME", "DB_PASSWORD", "DB_DRIVER"],
        );
    }
}
