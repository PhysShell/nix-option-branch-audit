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
//!
//! K2e (done, `fixtures/cdc/k2c-census/`): consumer reachability census --
//! for every app where `doctrine/dbal` is present, traced the real chain
//! to whatever actually consumes the Nix-emitted key. Found
//! agorakit/movim/snipe-it's `doctrine/dbal` is confirmed vestigial;
//! `Illuminate\Database` is the real consumer via `config/database.php`.
//! Census-only, zero code here.
//!
//! K2f (done): [`ConsumerRoute`]/[`ConsumerContractEvidence`] give that
//! reachability chain an explicit type, and
//! [`acquire_illuminate_consumer_contract`] proves it for real against
//! agorakit/movim/snipe-it -- ONE adapter, branching only on
//! [`IlluminateDriver`] (mysql vs postgres), never on app identity.
//! `doctrine/dbal`/`composer.lock` are never consulted anywhere in this
//! path.
//!
//! K2g (done): Phase D vendoring for `strichliste`/`part-db`, the two
//! real corpus cases K2e already qualified as needing no new semantic
//! model. [`acquire_strichliste_evidence`]/[`acquire_part_db_evidence`]
//! reuse the entire existing K1 pipeline (Phase C/D/E, K2a/K2b)
//! unchanged. Real find: `strichliste`'s pinned doctrine/dbal 3.10.5
//! driver is byte-identical to the already-vendored 3.10.6 one -- no
//! duplicate fixture. `part-db`'s Postgres driver has no `unix_socket`
//! key at all (`host=` doubles as socket path there) -- the first
//! non-MySQL-dialect Doctrine fixture in this project.
//!
//! K3a (done): [`ContractDiff`]/[`diff_contracts`] -- a SEPARATE
//! experiment, package-bump drift, not an extension of K1-K2g's
//! current-contract validation. Pure set difference between two
//! [`ConsumerContract`]s (library+dialect+version+accepted_keys), no
//! `ProducerEvidence` involved at all yet. Fail-closed across mismatched
//! consumer families (the `part-db` dialect lesson) and duplicate keys.
//!
//! K3b (done, `fixtures/cdc/k3b-drift-census/`): a real historical
//! contract-drift census across the 4 qualified PHP/Doctrine/Illuminate
//! families -- found real drift exists (`doctrine/dbal`'s Postgres
//! `default_dbname` removal, an added `use_db_after_connecting` on
//! Illuminate MySQL) but no removed/renamed token inside any
//! currently-qualified consumer's own observable window. K3b.1 (done,
//! same directory) then did a bounded ~20-candidate search for a real
//! nixpkgs consumer straddling that Postgres removal -- not found.
//! FROZEN as "bounded search exhausted": the PHP/Doctrine/Illuminate
//! differential corpus is closed for now, not abandoned.
//!
//! K4a (done): [`CliContract`]/[`CliFlagDiff`]/[`diff_cli_contracts`] --
//! CLI-flag contracts, the next interface family, picked over further
//! Doctrine excavation and over env vars (argv is typically a more
//! direct producer<->consumer boundary than a framework-mediated env
//! var). Pure model only, deliberately not reusing
//! `ConsumerContract`/`ContractDiff`.
//!
//! K4b (done, `fixtures/cdc/k4b-cli-drift-census/`): a real historical
//! CLI-flag drift census across a bounded ~21-candidate corpus (Rust/
//! Go/Python) -- found a real, source-verified rename on a real
//! nixpkgs-crossed pair: `mimir` 2.14.0 -> 3.2.1,
//! `-querier.prefer-availability-zone` -> `-querier.prefer-availability-zones`.
//!
//! K4c (done): [`evaluate_mimir_cli_drift`] correlates that ONE
//! qualified case with the real Nix producer -- [`CliDriftRelevance`]
//! distinguishes "producer still emits the removed flag" (an actual
//! bug) from "producer adopted the new flag" (handled, no finding) from
//! "producer never emitted either" (drift real, producer irrelevant).
//! Deliberately Mimir-specific throughout -- no generic CLI extractor
//! framework, no `krill` work this round. Real result: Outcome C --
//! `mimir`'s own NixOS module never emits either flag name by default,
//! proving producer-relevance filtering matters, not just that upstream
//! changed something. K4 (K4a+K4b+K4c) FROZEN as a proven, parked
//! vertical.
//!
//! K5a (done): [`EnvContract`]/[`EnvContractDiff`]/[`diff_env_contracts`]
//! -- environment-variable contracts, the next interface family, chosen
//! because the producer side already exists and is already proven
//! ([`ProducerEvidence::FlatEnvVars`], K2d). Pure model only,
//! deliberately not reusing `CliContract`/`ConsumerContract`.
//!
//! K5b (done, `fixtures/cdc/k5b-env-drift-census/`): a real historical
//! env-var drift census across a bounded ~29-candidate corpus -- found a
//! real, source-verified removal on a real nixpkgs-crossed pair:
//! `grafana` `12.3.3` -> `13.1.4`, the whole `[auth.passwordless]`
//! section (`GF_AUTH_PASSWORDLESS_ENABLED`/
//! `GF_AUTH_PASSWORDLESS_CODE_EXPIRATION`) removed, no replacement added.
//!
//! K5c (done): [`evaluate_grafana_env_drift`] correlates that removal
//! with the real Nix producer -- [`EnvDriftRelevance`] mirrors K4c's A/B/C
//! split in spirit but is a genuinely separate type (no full `EnvContract`
//! exists for grafana on either side; K5b's confirmed removed/added sets
//! are taken as given, not re-derived through `diff_env_contracts`). Real
//! result: Outcome C, and a categorically stronger one than mimir's --
//! grafana's own NixOS module never used the `GF_*` env-var interface at
//! all, on either side of the bump (`environment` evaluates to only
//! systemd's own default `PATH`, `EnvironmentFile` is unset); it
//! configures entirely via a generated `config.ini` passed with
//! `-config`. Second independent interface family confirming the same
//! architectural point K4c already made: drift is not a finding until
//! producer reachability is proven.

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
/// for a given vendored fixture path -- the existing, hand-verified
/// provenance record from K1's own Phase A, reused as K2a's (and now
/// K2g's) comparison oracle instead of a second Rust constant
/// duplicating the same fact under a different name (the removed
/// `DOCTRINE_DBAL_REV`). Parameterized by `vendored_path` (K2g) --
/// originally hardcoded to the one K1/K2a fixture; generalizing it is
/// the ENTIRE change, callers/behavior for that one path are unchanged.
fn vendored_fixture_source_reference(vendored_path: &str) -> Result<String, CdcError> {
    #[derive(serde::Deserialize)]
    struct Lock {
        fixture: Vec<Entry>,
    }
    #[derive(serde::Deserialize)]
    struct Entry {
        path: String,
        commit: String,
    }

    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/integrity-lock.toml");
    let text = std::fs::read_to_string(&lock_path)
        .map_err(|e| CdcError::ToolError(format!("reading {}: {e}", lock_path.display())))?;
    let lock: Lock = toml::from_str(&text)
        .map_err(|e| CdcError::ToolError(format!("parsing {}: {e}", lock_path.display())))?;
    lock.fixture
        .into_iter()
        .find(|e| e.path == vendored_path)
        .map(|e| e.commit)
        .ok_or_else(|| {
            CdcError::ToolError(format!(
                "fixtures/integrity-lock.toml has no entry for {vendored_path}"
            ))
        })
}

fn vendored_doctrine_source_reference() -> Result<String, CdcError> {
    vendored_fixture_source_reference("fixtures/cdc/doctrine-dbal-3.10.6/PDO-MySQL-Driver.php")
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

/// K2f: `FlatEnvVars` evidence (K2d) never had a consumer-side verdict
/// path -- for agorakit/movim/snipe-it, K2e proved WHY: `doctrine/dbal`
/// is present in all three but is confirmed NOT the real consumer of
/// these `DB_*` keys, `Illuminate\Database` is. This section gives that
/// route an explicit type, not a hidden assumption inside an extractor:
/// `ConsumerRoute` names every real hop between a Nix-emitted key and
/// its actual terminal consumer, so a `Finding` can explain not just
/// "was this key accepted" but "why Illuminate is even the right
/// library to check against."
///
/// Deliberately does NOT touch `doctrine/dbal`, `composer.lock`, or any
/// K1/K2a/K2b/K2d code path at all -- this is a parallel pipeline, not a
/// modification of the existing one. `ConsumerRoute::Direct` exists for
/// completeness (the Kimai/Davis/Symfony-bundle shape K1/K2e already
/// cover) but nothing in this module currently constructs it; K2f only
/// ever builds `Mediated`.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsumerHop {
    pub layer: String,
    pub from_key: String,
    pub to_key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConsumerRoute {
    Direct,
    Mediated { hops: Vec<ConsumerHop> },
}

impl ConsumerRoute {
    /// The key to actually compare against a consumer's accepted-keys
    /// list -- the original emitted key for `Direct`, the LAST hop's
    /// `to_key` for `Mediated` (falling back to `original` only if
    /// `hops` is somehow empty, which no K2f constructor ever produces).
    pub fn resolved_key<'a>(&'a self, original: &'a str) -> &'a str {
        match self {
            ConsumerRoute::Direct => original,
            ConsumerRoute::Mediated { hops } => {
                hops.last().map(|h| h.to_key.as_str()).unwrap_or(original)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsumerContractEvidence {
    pub route: ConsumerRoute,
    pub consumer_library: String,
    pub accepted_keys: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IlluminateDriver {
    MySql,
    Postgres,
}

impl IlluminateDriver {
    fn consumer_library_name(self) -> &'static str {
        match self {
            IlluminateDriver::MySql => "Illuminate\\Database\\Connectors\\MySqlConnector",
            IlluminateDriver::Postgres => "Illuminate\\Database\\Connectors\\PostgresConnector",
        }
    }
}

/// Pure. Reuse survey: same bounded-literal-scan discipline Phase D
/// already established for Doctrine's driver source (a general PHP
/// parser would be strictly less trustworthy for this one exact, narrow,
/// pinned shape, not more) -- this isn't a new precedent, it's the same
/// one applied to a second real source shape.
///
/// Scans `config/database.php`-shaped source for `'<key>' =>
/// env('<env_var>' ...)`, tolerating one simple cast immediately before
/// `env(` (`(int)`/`(bool)` etc. -- both `(int) env(` and `(int)env(`,
/// both seen in real vendored fixtures below). `Ok(None)` means
/// `env_var` doesn't appear in this file AT ALL -- a real, valid
/// outcome (not every `FlatEnvVars` key is part of the DB connection
/// surface), never an error. Two or more DISTINCT resolved keys, or an
/// occurrence that doesn't backward-parse into the expected shape, is
/// `Inconclusive` -- checked on DISTINCT values, not raw occurrence
/// count, because the real corpus has a case that would otherwise be a
/// false ambiguity: snipe-it's `config/database.php` maps `DB_SOCKET`
/// -> `unix_socket` identically in BOTH its `mysql` and `mariadb`
/// connection blocks (verified against the real file, not assumed).
pub fn extract_config_key_for_env_var(
    php_source: &str,
    env_var: &str,
) -> Result<Option<String>, CdcError> {
    let needle = format!("env('{env_var}'");
    let mut resolved: Vec<String> = Vec::new();
    let mut search_from = 0;
    let mut found_any = false;
    while let Some(rel) = php_source[search_from..].find(&needle) {
        found_any = true;
        let pos = search_from + rel;
        let before = strip_optional_php_cast(php_source[..pos].trim_end()).trim_end();
        let Some(before_arrow) = before.strip_suffix("=>") else {
            return Err(CdcError::Inconclusive(format!(
                "env('{env_var}'...) at byte {pos} is not immediately preceded by '=>' \
                 (after an optional cast) -- unrecognized shape"
            )));
        };
        let Some(before_quote) = before_arrow.trim_end().strip_suffix('\'') else {
            return Err(CdcError::Inconclusive(format!(
                "env('{env_var}'...) at byte {pos}: preceding key is not single-quoted -- \
                 unrecognized shape"
            )));
        };
        let Some(key_start) = before_quote.rfind('\'') else {
            return Err(CdcError::Inconclusive(format!(
                "env('{env_var}'...) at byte {pos}: no matching opening quote for the \
                 preceding key"
            )));
        };
        let key = &before_quote[key_start + 1..];
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(CdcError::Inconclusive(format!(
                "env('{env_var}'...) at byte {pos}: extracted key {key:?} doesn't look like \
                 a PHP array key"
            )));
        }
        if !resolved.iter().any(|k| k == key) {
            resolved.push(key.to_string());
        }
        search_from = pos + needle.len();
    }
    if !found_any {
        return Ok(None);
    }
    match resolved.len() {
        1 => Ok(Some(resolved.into_iter().next().unwrap())),
        _ => Err(CdcError::Inconclusive(format!(
            "env('{env_var}'...) resolves to {} distinct keys ({resolved:?}), expected exactly 1",
            resolved.len()
        ))),
    }
}

/// Strips one trailing simple PHP cast (`(int)`, `(bool)`, ...) if
/// present immediately at the end of `s` -- a bare `(<alpha-only>)`
/// group, nothing fancier. Not a general expression parser: if what
/// precedes `env(` isn't `=>` (with or without one such cast in
/// between), [`extract_config_key_for_env_var`] already refuses via its
/// own `unrecognized shape` error, so a false-positive "strip" here
/// can't silently manufacture an incorrect key.
fn strip_optional_php_cast(s: &str) -> &str {
    let Some(stripped) = s.strip_suffix(')') else { return s };
    let Some(open) = stripped.rfind('(') else { return s };
    let inner = &stripped[open + 1..];
    if !inner.is_empty() && inner.chars().all(|c| c.is_ascii_alphabetic()) {
        stripped[..open].trim_end()
    } else {
        s
    }
}

/// Pure. Does the vendored Illuminate connector source (the caller
/// concatenates the driver-specific connector with the shared base
/// `Connector.php`, since `username`/`password` are read there, common
/// to every driver) reference `key` at all -- via EITHER of the two real
/// syntactic shapes actually observed in the pinned source (verified by
/// reading it, not assumed): array-keyed (`$config['host']`, MySQL's
/// shape -- checked as a plain bounded substring since the key sits
/// between two literal `'` delimiters with no room for a false match)
/// or boundary-checked bare-variable (`$host`, Postgres's shape after
/// its own `extract($config, EXTR_SKIP)` call -- boundary-checked so
/// `$host` doesn't false-match inside a longer identifier like
/// `$hostname`). Deliberately NOT an `isset(...)`-only scan:
/// `host`/`database` are read UNCONDITIONALLY by both drivers (no
/// `isset` gate at all, confirmed by reading the real vendored source),
/// so requiring the `isset(...)` wrapper specifically would wrongly
/// report them as unaccepted.
pub fn illuminate_connector_accepts_key(source: &str, key: &str) -> bool {
    source.contains(&format!("$config['{key}']")) || php_bare_var_referenced(source, key)
}

fn php_bare_var_referenced(source: &str, key: &str) -> bool {
    let needle = format!("${key}");
    let mut search_from = 0;
    while let Some(rel) = source[search_from..].find(&needle) {
        let pos = search_from + rel;
        let after = &source[pos + needle.len()..];
        let boundary_ok =
            after.chars().next().is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
        if boundary_ok {
            return true;
        }
        search_from = pos + needle.len();
    }
    false
}

/// Pure: combines the two extractors above into one
/// `ConsumerContractEvidence` for ONE specific Nix-emitted key.
/// `database_php`/`connector_source` are the caller's already-fetched
/// real bytes -- no I/O here. Fail-closed like every other K2f piece: an
/// unresolved config mapping propagates as `Inconclusive`, never a
/// guessed route -- this is also the honest, correct outcome for a key
/// that genuinely isn't part of this app's DB config surface at all
/// (e.g. `DB_SOCKET` against movim's `config/database.php`, which never
/// mentions it -- Postgres via Illuminate has no unix-socket concept at
/// all, confirmed by reading `PostgresConnector.php` directly).
pub fn build_consumer_contract_evidence(
    nix_key: &str,
    layer_name: &str,
    database_php: &str,
    connector_source: &str,
    driver: IlluminateDriver,
) -> Result<ConsumerContractEvidence, CdcError> {
    let Some(mapped_key) = extract_config_key_for_env_var(database_php, nix_key)? else {
        return Err(CdcError::Inconclusive(format!(
            "{nix_key} does not appear in {layer_name} at all -- not part of this consumer's \
             DB config surface"
        )));
    };
    let accepted = illuminate_connector_accepts_key(connector_source, &mapped_key);
    let route = ConsumerRoute::Mediated {
        hops: vec![ConsumerHop {
            layer: layer_name.to_string(),
            from_key: nix_key.to_string(),
            to_key: mapped_key.clone(),
        }],
    };
    Ok(ConsumerContractEvidence {
        route,
        consumer_library: driver.consumer_library_name().to_string(),
        accepted_keys: if accepted { vec![mapped_key] } else { Vec::new() },
    })
}

/// Phase E (frozen, K1) reused UNCHANGED -- `compare_contract`'s
/// signature only ever sees `&str`/`&[String]`, never `ConsumerRoute`
/// itself, same "no type-level room for one path to become stronger
/// than another" discipline K2b established for `ProducerEvidence`.
pub fn verdict_for_consumer_contract(evidence: &ConsumerContractEvidence, nix_key: &str) -> CdcVerdict {
    compare_contract(evidence.route.resolved_key(nix_key), &evidence.accepted_keys)
}

/// Real: reads the vendored, integrity-locked Illuminate connector
/// fixtures (see `fixtures/integrity-lock.toml`) -- NOT a live fetch.
/// `doctrine/dbal`'s presence in any of these three apps' `composer.lock`
/// is never consulted anywhere in this function or its callers --
/// requirement (d), satisfied structurally, not by omission.
fn illuminate_fixture_source(driver: IlluminateDriver) -> Result<String, CdcError> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let base_path = manifest_dir.join("fixtures/cdc/illuminate-database/Connectors/Connector.php");
    let driver_path = manifest_dir.join(match driver {
        IlluminateDriver::MySql => "fixtures/cdc/illuminate-database/Connectors/MySqlConnector.php",
        IlluminateDriver::Postgres => {
            "fixtures/cdc/illuminate-database/Connectors/PostgresConnector.php"
        }
    });
    let base_src = std::fs::read_to_string(&base_path)
        .map_err(|e| CdcError::ToolError(format!("reading {}: {e}", base_path.display())))?;
    let driver_src = std::fs::read_to_string(&driver_path)
        .map_err(|e| CdcError::ToolError(format!("reading {}: {e}", driver_path.display())))?;
    Ok(format!("{driver_src}\n{base_src}"))
}

/// Real: `pkgs.<attr>.src + "/config/database.php"`, the same
/// `pkgs.<attr>.src` oracle `fetch_composer_lock` (K2a) already uses --
/// asking Nix for the app's own fetched source, not a second fetch
/// mechanism.
fn fetch_config_database_php(rev: &str, package_attr: &str) -> Result<String, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        pkgs = import nixpkgsSrc {{ system = "x86_64-linux"; }};
        in builtins.readFile (pkgs.{package_attr}.src + "/config/database.php")"#
    );
    eval_nix_raw(&expr)
}

/// Real acquisition: the ONE shared adapter -- no `if app == ...`
/// anywhere in this function or anything it calls. Branches only on
/// `driver` (a property of how the deployment configures its DB
/// connection, not of which app it is), exactly the boundary the design
/// review asked for.
pub fn acquire_illuminate_consumer_contract(
    rev: &str,
    package_attr: &str,
    nix_key: &str,
    driver: IlluminateDriver,
) -> Result<ConsumerContractEvidence, CdcError> {
    let database_php = fetch_config_database_php(rev, package_attr)?;
    let connector_source = illuminate_fixture_source(driver)?;
    build_consumer_contract_evidence(
        nix_key,
        "config/database.php",
        &database_php,
        &connector_source,
        driver,
    )
}

/// K2g: Phase D vendoring for `strichliste`/`part-db` -- the two real
/// corpus cases K2e already qualified as needing ZERO new semantic
/// model (`strichliste`: Symfony's bundle calls Doctrine's own
/// `DsnParser`, unchanged contract; `part-db`: same mechanism, Postgres
/// dialect). Reuses the ENTIRE existing K1 pipeline unchanged --
/// `extract_key_for_value` (Phase C), `extract_accepted_keys` (Phase D,
/// below), `compare_contract` (Phase E), `build_sentinel_flow_evidence`/
/// `resolve_consumer_identity`/`fetch_composer_lock` (K2a/K2b) -- this
/// section only adds two new real acquisition paths and two new
/// vendored consumer fixtures, no new comparison semantics.
const STRICHLISTE_UPSTREAM_PATH: &str = "nixos/modules/services/web-apps/strichliste.nix";
const STRICHLISTE_OPTION: &str = "services.strichliste.environment.DATABASE_URL";
const STRICHLISTE_SINK: &str = "systemd.services.\"strichliste-migrate\".environment.DATABASE_URL";

/// Real: Phase B-shaped -- `environment.DATABASE_URL` is a real option,
/// sentinel-injected exactly like Kimai's `database.socket`.
/// `module_override` is the same mutation-testing hook `nixos_eval_expr`
/// already provides.
pub fn eval_strichliste_database_url(
    rev: &str,
    sentinel: &str,
    module_override: Option<&Path>,
) -> Result<String, CdcError> {
    let expr = nixos_eval_expr(
        rev,
        STRICHLISTE_UPSTREAM_PATH,
        module_override,
        &format!(
            r#"services.strichliste = {{ enable = true; domain = "strichliste.example.com"; environment.DATABASE_URL = "mysql://u@localhost/db?charset=utf8&unix_socket={sentinel}"; }};"#
        ),
    );
    eval_nix_raw(&format!(
        r#"({expr}).config.systemd.services."strichliste-migrate".environment.DATABASE_URL"#
    ))
}

pub fn acquire_strichliste_evidence(
    rev: &str,
    sentinel: &str,
    module_override: Option<&Path>,
) -> Result<ProducerEvidence, CdcError> {
    let rendered = eval_strichliste_database_url(rev, sentinel, module_override)?;
    build_sentinel_flow_evidence(STRICHLISTE_OPTION, STRICHLISTE_SINK, sentinel, rendered)
}

const PART_DB_UPSTREAM_PATH: &str = "nixos/modules/services/web-apps/part-db.nix";
const PART_DB_OPTION: &str = "services.part-db.settings.DATABASE_URL";
const PART_DB_SINK: &str = "systemd.services.\"part-db-setup\".restartTriggers[0]";

/// Real: part-db's `settings.DATABASE_URL` renders into `envFile`, a
/// real `pkgs.writeText` derivation -- reading its content requires
/// REALIZING that (tiny, cheap) derivation, a genuine (if minor)
/// difference from Kimai's purely-evaluated string; confirmed fast in
/// practice, no network needed for a `writeText` FOD. A real Postgres
/// nuance: `host=` doubles as BOTH a TCP hostname and a unix-socket
/// directory path (confirmed by reading `part-db.nix`'s own default,
/// `host=/run/postgresql`) -- there is no separate `unix_socket`
/// parameter to inject a sentinel into, so the sentinel goes into the
/// `host=` query parameter instead. `extract_key_for_value` (Phase C,
/// unchanged) correctly resolves this to `emitted_key = "host"`, not
/// `"unix_socket"` -- proving Phase C was never hardcoded to look for
/// one specific key name; that's the exact generality this reuses.
pub fn eval_part_db_env_file(
    rev: &str,
    sentinel: &str,
    module_override: Option<&Path>,
) -> Result<String, CdcError> {
    let expr = nixos_eval_expr(
        rev,
        PART_DB_UPSTREAM_PATH,
        module_override,
        &format!(
            r#"services.part-db = {{ enable = true; settings.DATABASE_URL = "postgresql://u@localhost/db?serverVersion=16.6&charset=utf8&host={sentinel}"; }}; services.postgresql.enable = true;"#
        ),
    );
    eval_nix_raw(&format!(
        r#"builtins.readFile (builtins.head ({expr}).config.systemd.services."part-db-setup".restartTriggers)"#
    ))
}

pub fn acquire_part_db_evidence(
    rev: &str,
    sentinel: &str,
    module_override: Option<&Path>,
) -> Result<ProducerEvidence, CdcError> {
    let rendered = eval_part_db_env_file(rev, sentinel, module_override)?;
    build_sentinel_flow_evidence(PART_DB_OPTION, PART_DB_SINK, sentinel, rendered)
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

/// K3a: Differential Consumer Contracts -- a SEPARATE experiment from
/// current-contract validation (K1-K2g), not an extension of
/// `compare_contract`. Current-contract asks "producer <-> consumer at
/// revision X"; package-bump drift asks "consumer contract at BASE <->
/// consumer contract at HEAD, and does Nix still emit a name the
/// consumer no longer accepts." This phase deliberately does not
/// reference `ProducerEvidence`/`ConsumerRoute` at all -- it's pure set
/// difference over two already-extracted accepted-key lists, nothing
/// more. No "breaking"/"regression"/"safe rename" classification and no
/// fuzzy matching live here -- K3a reports facts only; anything
/// diagnostic (a probable old-name/new-name correlation) is explicitly
/// K3c's job, once a real historical drift case actually exists to
/// correlate against.
///
/// `part-db`'s own K2g finding sets a hard constraint on this type:
/// a consumer contract's identity must carry its library AND its
/// dialect/driver, never be compared as a bare set of strings --
/// diffing Doctrine's MySQL driver against its own Postgres driver
/// would manufacture "drift" that's really just two different
/// libraries-in-effect sharing one package name (confirmed for real in
/// K2g: Doctrine's Postgres driver has no `unix_socket` key at all,
/// which a dialect-blind diff would have wrongly reported as removed).
#[derive(Debug, Clone, PartialEq)]
pub struct ConsumerContract {
    pub library: String,
    pub dialect: String,
    pub version: String,
    pub accepted_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContractDiff {
    pub removed: Vec<String>,
    pub added: Vec<String>,
    pub retained: Vec<String>,
}

/// Pure. Fail-closed on the two ways this could otherwise silently lie:
/// comparing across different consumer families (library/dialect
/// mismatch -- see this section's own doc comment), or a malformed
/// input contract carrying a duplicate accepted key (never silently
/// deduped -- a real `extract_accepted_keys`-style extractor should
/// never produce one, so seeing one here means something upstream is
/// already wrong, not a case to paper over). Results are sorted, so
/// input ordering never affects the outcome.
pub fn diff_contracts(
    base: &ConsumerContract,
    head: &ConsumerContract,
) -> Result<ContractDiff, CdcError> {
    if base.library != head.library || base.dialect != head.dialect {
        return Err(CdcError::Inconclusive(format!(
            "cannot diff different consumer families: {}/{} (base) vs {}/{} (head)",
            base.library, base.dialect, head.library, head.dialect
        )));
    }
    let base_keys = require_no_duplicate_keys(&base.accepted_keys, "base")?;
    let head_keys = require_no_duplicate_keys(&head.accepted_keys, "head")?;

    let mut removed: Vec<String> =
        base_keys.iter().filter(|k| !head_keys.contains(k)).cloned().collect();
    let mut added: Vec<String> =
        head_keys.iter().filter(|k| !base_keys.contains(k)).cloned().collect();
    let mut retained: Vec<String> =
        base_keys.iter().filter(|k| head_keys.contains(k)).cloned().collect();
    removed.sort();
    added.sort();
    retained.sort();
    Ok(ContractDiff { removed, added, retained })
}

fn require_no_duplicate_keys(keys: &[String], side: &str) -> Result<Vec<String>, CdcError> {
    let mut seen = std::collections::HashSet::new();
    for k in keys {
        if !seen.insert(k.as_str()) {
            return Err(CdcError::Inconclusive(format!(
                "{side} contract has a duplicate accepted key {k:?} -- refusing to diff an \
                 ambiguous contract"
            )));
        }
    }
    Ok(keys.to_vec())
}

/// K4: CLI-flag contracts -- a new interface family for differential
/// consumer-contract validation, picked over expanding the PHP/Doctrine
/// corpus further (K3b.1: closed for now, not abandoned) and over env
/// vars (ranked second). Reasoning: argv is typically a direct
/// producer<->consumer boundary --
/// `ExecStart = "${pkg}/bin/foo --socket ...";` on the Nix side, the
/// upstream program's own argument parser on the consumer side -- with
/// less framework mediation than K2e/K2f/K2g repeatedly found sitting
/// between a Nix-emitted env var and its actual consumer.
///
/// K4a (this section): the pure model ONLY, mirroring K3a's own scope
/// exactly -- no extraction, no real nixpkgs research (that's K4b's
/// job, not started). Deliberately does NOT reuse
/// `ConsumerContract`/`ContractDiff` even though the shape rhymes --
/// forcing a shared abstraction here would blur what's actually being
/// compared (a CLI program's flag surface is not a PHP library's DSN
/// parameter contract) for no real current benefit; a shared
/// abstraction is worth building once/if it falls out naturally from a
/// second real use, not speculatively now because two `enum`s look
/// similar.
///
/// K4a deliberately only diffs FLAG NAMES, nothing else. K3b's own
/// `movim` finding (a same-named key can change consuming MECHANISM
/// without changing its name) applies here in principle too, but is
/// explicitly out of scope until K4 has caught at least one real
/// name-level drift case first -- the same "scope has to end somewhere"
/// discipline K3a itself was built under.
///
/// **Extraction-source ranking, recorded here as a design principle for
/// K4b -- NOT implemented as code this round**: (1) structured parser
/// metadata/source (e.g. a Rust `clap` derive's `#[arg(long = "...")]`,
/// a Go `cobra`/`flag` definition, a Python `argparse`/`click` call) --
/// the closest analogue to Phase D's own bounded literal scan over
/// vendored source, same trust level; (2) `--help` output, only if
/// demonstrably stable across the versions being compared; (3) other
/// source literals/definitions not cleanly "structured metadata" but
/// still the real vendored source; (4) documentation, fallback/
/// provenance-hint only, never the primary oracle -- docs drift from
/// implementation more easily than any of the above. Good first
/// families for K4b's own census, per this ranking: Rust `clap`, Go
/// `cobra`/`flag`, Python `argparse`/`click`.
#[derive(Debug, Clone, PartialEq)]
pub struct CliContract {
    pub program: String,
    pub version: String,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CliFlagDiff {
    pub removed: Vec<String>,
    pub added: Vec<String>,
    pub retained: Vec<String>,
}

/// Pure. Fail-closed on a `program` mismatch (comparing `foo --socket`
/// against `bar --socket` is exactly as meaningless as K3a's own
/// library/dialect mismatch -- same principle, independently enforced
/// here rather than shared, per this section's own doc comment) and on
/// a duplicate flag in either input contract (never silently deduped).
/// Results are sorted, so input ordering never affects the outcome.
pub fn diff_cli_contracts(
    base: &CliContract,
    head: &CliContract,
) -> Result<CliFlagDiff, CdcError> {
    if base.program != head.program {
        return Err(CdcError::Inconclusive(format!(
            "cannot diff different CLI programs: {:?} (base) vs {:?} (head)",
            base.program, head.program
        )));
    }
    let base_flags = require_no_duplicate_flags(&base.flags, "base")?;
    let head_flags = require_no_duplicate_flags(&head.flags, "head")?;

    let mut removed: Vec<String> =
        base_flags.iter().filter(|f| !head_flags.contains(f)).cloned().collect();
    let mut added: Vec<String> =
        head_flags.iter().filter(|f| !base_flags.contains(f)).cloned().collect();
    let mut retained: Vec<String> =
        base_flags.iter().filter(|f| head_flags.contains(f)).cloned().collect();
    removed.sort();
    added.sort();
    retained.sort();
    Ok(CliFlagDiff { removed, added, retained })
}

fn require_no_duplicate_flags(flags: &[String], side: &str) -> Result<Vec<String>, CdcError> {
    let mut seen = std::collections::HashSet::new();
    for f in flags {
        if !seen.insert(f.as_str()) {
            return Err(CdcError::Inconclusive(format!(
                "{side} CLI contract has a duplicate flag {f:?} -- refusing to diff an \
                 ambiguous contract"
            )));
        }
    }
    Ok(flags.to_vec())
}

/// K4c: correlates the ONE real, K4b-qualified CLI drift case
/// (`mimir` 2.14.0 -> 3.2.1, `-querier.prefer-availability-zone` ->
/// `-querier.prefer-availability-zones`) with the real Nix producer.
/// Deliberately narrow and Mimir-specific throughout -- no generic CLI
/// extractor framework, no `krill` work, per explicit instruction. This
/// is the first K3/K4-series round that reuses `diff_cli_contracts`
/// (K4a) against a REAL pair rather than a synthetic test fixture.
///
/// Reuses `eval_nix_raw` (K1) for the producer side, unchanged.
///
/// Fail-closed distinction the design review required: three outcomes,
/// only one of which is an actual bug. `ProducerStillEmitsRemoved` (A)
/// is the only finding-worthy case; `ProducerAdoptedNew` (B) proves the
/// checker can tell "drift happened, nixpkgs already adapted" from a
/// real bug; `ProducerIrrelevant` (C) proves producer-relevance
/// filtering works at all -- upstream contract drift is real (K4b) but
/// simply doesn't reach this particular Nix module's own emitted argv.
#[derive(Debug, Clone, PartialEq)]
pub enum CliDriftRelevance {
    ProducerStillEmitsRemoved,
    ProducerAdoptedNew,
    ProducerIrrelevant,
}

/// Pure. Deliberately NOT a substring search -- unsound here
/// specifically, since `prefer-availability-zone` is a literal PREFIX
/// of `prefer-availability-zones`; a naive `argv.contains(needle)`
/// could not tell "old flag present" from "new flag present" apart.
/// Checks both single- and double-dash spellings (Go's `flag` package
/// treats them as equivalent) and requires the character immediately
/// after the matched flag name to be `=`, whitespace, or end-of-string.
fn argv_contains_flag(argv: &str, flag_name: &str) -> bool {
    for prefix in ["--", "-"] {
        let needle = format!("{prefix}{flag_name}");
        let mut search_from = 0;
        while let Some(rel) = argv[search_from..].find(&needle) {
            let pos = search_from + rel;
            let before_ok = pos == 0
                || argv.as_bytes().get(pos - 1).is_some_and(|b| b.is_ascii_whitespace());
            let after = &argv[pos + needle.len()..];
            let after_ok =
                after.is_empty() || after.starts_with('=') || after.starts_with(char::is_whitespace);
            if before_ok && after_ok {
                return true;
            }
            search_from = pos + needle.len();
        }
    }
    false
}

/// Pure. `removed_flag`/`added_flag` are the specific names
/// `diff_cli_contracts` (K4a) already confirmed for this pair -- this
/// function doesn't recompute the diff, only classifies real argv
/// against it.
pub fn classify_cli_drift_relevance(
    removed_flag: &str,
    added_flag: &str,
    argv: &str,
) -> CliDriftRelevance {
    if argv_contains_flag(argv, removed_flag) {
        CliDriftRelevance::ProducerStillEmitsRemoved
    } else if argv_contains_flag(argv, added_flag) {
        CliDriftRelevance::ProducerAdoptedNew
    } else {
        CliDriftRelevance::ProducerIrrelevant
    }
}

/// Pure. Reuse survey: same bounded-literal-scan discipline Phase
/// D/K2f already established, applied to a THIRD real source shape
/// (Go's `flag.FlagSet` convention) -- not a general Go parser, not a
/// new precedent. Scoped to this file's own consistent convention:
/// every directly-named flag registration is
/// `f.<Method>(&cfg.<Field>, "<literal>", ...)`. Recognizes ONLY the
/// literal form -- a flag registered via a named Go constant (e.g.
/// `f.DurationVar(&cfg.X, queryStoreAfterFlag, ...)`) is NOT captured,
/// a real, disclosed limitation, not a silent gap. Confirmed for real
/// this affects exactly two flags in this file
/// (`querier.streaming-chunks-per-{ingester,store-gateway}-buffer-size`,
/// literals in 2.14.0, refactored to named constants resolving to the
/// IDENTICAL strings in 3.2.1 -- verified by reading the constant
/// declarations directly, not assumed) -- confirmed NOT a second real
/// rename before this extractor's raw output was trusted for the
/// actual finding below.
pub fn extract_go_flagset_literal_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    const NEEDLE: &str = "&cfg.";
    let mut search_from = 0;
    while let Some(rel) = source[search_from..].find(NEEDLE) {
        let pos = search_from + rel;
        let after_field = &source[pos + NEEDLE.len()..];
        let ident_end = after_field
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
            .unwrap_or(after_field.len());
        let after_ident = after_field[ident_end..].trim_start();
        if let Some(rest) = after_ident.strip_prefix(',') {
            let rest = rest.trim_start();
            if let Some(lit_rest) = rest.strip_prefix('"') {
                if let Some(end) = lit_rest.find('"') {
                    names.push(lit_rest[..end].to_string());
                }
            }
        }
        search_from = pos + NEEDLE.len();
    }
    names
}

/// Real: reads the vendored, integrity-locked `querier.go` fixture (see
/// `fixtures/integrity-lock.toml`) -- NOT a live fetch, same discipline
/// as every other vendored consumer fixture in this project.
fn mimir_querier_go_contract(vendored_path: &str, version: &str) -> Result<CliContract, CdcError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(vendored_path);
    let source = std::fs::read_to_string(&path)
        .map_err(|e| CdcError::ToolError(format!("reading {}: {e}", path.display())))?;
    let flags = extract_go_flagset_literal_names(&source);
    if flags.is_empty() {
        return Err(CdcError::Inconclusive(format!(
            "no literal flag names extracted from {vendored_path} -- extractor no longer \
             understands this source shape"
        )));
    }
    Ok(CliContract { program: "mimir".to_string(), version: version.to_string(), flags })
}

const MIMIR_2_14_0_FIXTURE: &str = "fixtures/cdc/mimir-2.14.0/querier.go";
const MIMIR_3_2_1_FIXTURE: &str = "fixtures/cdc/mimir-3.2.1/querier.go";
const MIMIR_REMOVED_FLAG: &str = "querier.prefer-availability-zone";
const MIMIR_ADDED_FLAG: &str = "querier.prefer-availability-zones";

/// Real: `pkgs.mimir`'s own NixOS module, evaluated with the module's
/// OWN default configuration (`extraFlags = []`, its own real default,
/// not overridden) -- the same "ask Nix, don't hand-interpret the
/// module" discipline this whole project has used since K1.
fn eval_mimir_exec_start(rev: &str) -> Result<String, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            services.mimir = {{ enable = true; configuration = {{ target = "all"; }}; }};
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        in eval.config.systemd.services.mimir.serviceConfig.ExecStart"#
    );
    eval_nix_raw(&expr)
}

/// Real: the full K4c pipeline for the one qualified case -- extract
/// both vendored contracts, confirm the K4b-identified rename via the
/// unchanged K4a `diff_cli_contracts`, evaluate the real producer argv,
/// classify. Fails closed (via the `?` propagation from every real
/// step above) rather than guessing if extraction or evaluation don't
/// behave as expected -- this function never falls back to asserting
/// the rename by name alone.
pub fn evaluate_mimir_cli_drift(rev: &str) -> Result<CliDriftRelevance, CdcError> {
    let base = mimir_querier_go_contract(MIMIR_2_14_0_FIXTURE, "2.14.0")?;
    let head = mimir_querier_go_contract(MIMIR_3_2_1_FIXTURE, "3.2.1")?;
    let diff = diff_cli_contracts(&base, &head)?;
    if !diff.removed.iter().any(|f| f == MIMIR_REMOVED_FLAG) {
        return Err(CdcError::ToolError(format!(
            "expected the real consumer diff to show {MIMIR_REMOVED_FLAG} as removed -- \
             extraction or a vendored fixture has drifted from what K4b confirmed"
        )));
    }
    let argv = eval_mimir_exec_start(rev)?;
    Ok(classify_cli_drift_relevance(MIMIR_REMOVED_FLAG, MIMIR_ADDED_FLAG, &argv))
}

/// K5: environment-variable contracts -- the next interface family,
/// chosen over `krill`'s structural CLI-drift rabbit hole because this
/// project's producer-side acquisition for env vars already exists and
/// is already proven on three real apps
/// ([`ProducerEvidence::FlatEnvVars`], K2d, real-verified on
/// agorakit/movim/snipe-it) -- K5 tests a NEW consumer family against
/// an ALREADY-PROVEN producer side, not both halves at once the way K4
/// had to.
///
/// K5a (this section): the pure model ONLY, mirroring K3a/K4a's own
/// scope exactly -- no extraction, no real nixpkgs research (K5b's job,
/// not started). Deliberately a genuinely separate type from
/// `CliContract`/`ConsumerContract`, not a shared abstraction forced
/// because all three are "a name and a diff" -- same discipline the
/// design review required for K4a. Diffs variable NAMES only this
/// round -- deliberately does not model `required?`/`default?`/`parse
/// kind?` yet, even though a real env var clearly has more shape than a
/// bare name; K3a's/K4a's own "scope must end somewhere" discipline
/// applies here too.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvContract {
    pub consumer: String,
    pub version: String,
    pub variables: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvContractDiff {
    pub removed: Vec<String>,
    pub added: Vec<String>,
    pub retained: Vec<String>,
}

/// Pure. Fail-closed on a `consumer` mismatch (the env-var analogue of
/// K3a's library/dialect check and K4c's `program` check) and on a
/// duplicate variable name in either input contract, never silently
/// deduped. Results are sorted, so input ordering never affects the
/// outcome.
pub fn diff_env_contracts(
    base: &EnvContract,
    head: &EnvContract,
) -> Result<EnvContractDiff, CdcError> {
    if base.consumer != head.consumer {
        return Err(CdcError::Inconclusive(format!(
            "cannot diff different env-var consumers: {:?} (base) vs {:?} (head)",
            base.consumer, head.consumer
        )));
    }
    let base_vars = require_no_duplicate_vars(&base.variables, "base")?;
    let head_vars = require_no_duplicate_vars(&head.variables, "head")?;

    let mut removed: Vec<String> =
        base_vars.iter().filter(|v| !head_vars.contains(v)).cloned().collect();
    let mut added: Vec<String> =
        head_vars.iter().filter(|v| !base_vars.contains(v)).cloned().collect();
    let mut retained: Vec<String> =
        base_vars.iter().filter(|v| head_vars.contains(v)).cloned().collect();
    removed.sort();
    added.sort();
    retained.sort();
    Ok(EnvContractDiff { removed, added, retained })
}

fn require_no_duplicate_vars(vars: &[String], side: &str) -> Result<Vec<String>, CdcError> {
    let mut seen = std::collections::HashSet::new();
    for v in vars {
        if !seen.insert(v.as_str()) {
            return Err(CdcError::Inconclusive(format!(
                "{side} env contract has a duplicate variable {v:?} -- refusing to diff an \
                 ambiguous contract"
            )));
        }
    }
    Ok(vars.to_vec())
}

/// K5c: correlates the real, K5b-confirmed env-var removal (`grafana`
/// `12.3.3` -> `13.1.4`, `GF_AUTH_PASSWORDLESS_ENABLED`/
/// `GF_AUTH_PASSWORDLESS_CODE_EXPIRATION`) with the real Nix-evaluated
/// producer -- the same "drift is not a finding until producer
/// reachability is proven" correlation K4c already did for CLI flags,
/// applied to a SECOND, independent interface family. Deliberately does
/// NOT reuse [`CliDriftRelevance`]: K5b never built (and was explicitly
/// told not to build) a generic Grafana env-var extractor the way K4a
/// vendored and parsed mimir's full Go source, so there is no full
/// `EnvContract` for grafana on either side to run through
/// [`diff_env_contracts`] -- the removed/added sets below are exactly
/// what K5b already proved by direct source diff at both real tags,
/// taken as given, not re-derived. `diff_env_contracts`/`EnvContractDiff`
/// (K5a) stay completely untouched by this section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvDriftRelevance {
    /// The real HEAD producer still emits at least one of the removed variables.
    ProducerStillEmitsRemoved,
    /// The real HEAD producer emits at least one of the added (replacement)
    /// variables. Never observed for grafana's own case -- K5b found a
    /// pure removal, no replacement -- kept for family completeness only,
    /// the same way K4c's own `ProducerAdoptedNew` stays real even though
    /// THIS correlation can never reach it.
    ProducerUsesAdded,
    /// The producer's real emitted-variable evidence intersects neither
    /// the removed nor the added set.
    ProducerIrrelevant,
}

/// The exact pair K5b confirmed by direct source diff at both real tags
/// (`v12.3.3`/`v13.1.4`) -- see `fixtures/cdc/k5b-env-drift-census/census.md`.
/// A pure removal: the `[auth.passwordless]` section has no successor in
/// `13.1.4`'s config surface, so the added set is genuinely empty, not
/// merely unresearched.
const GRAFANA_REMOVED_ENV_VARS: [&str; 2] =
    ["GF_AUTH_PASSWORDLESS_ENABLED", "GF_AUTH_PASSWORDLESS_CODE_EXPIRATION"];
const GRAFANA_ADDED_ENV_VARS: [&str; 0] = [];

/// `12.3.3`, the oldest real `PhysShell/nixpkgs` commit touching
/// `pkgs/by-name/gr/grafana/package.nix` -- K5b's own base revision,
/// resolved to its full SHA here (K5b's census recorded only the short
/// form).
const GRAFANA_BASE_REV: &str = "3f222263343f664103f13efc3a3591191e89a425";

/// Pure. Exact set-membership check -- unlike K4c's `argv_contains_flag`,
/// no boundary logic is needed here: these are whole environment-variable
/// NAMES compared by exact string equality, not a substring scan over a
/// combined argv string, so there is no `-zone`/`-zones`-style prefix-
/// collision risk to guard against.
pub fn classify_env_drift_relevance(
    removed: &[&str],
    added: &[&str],
    producer_emitted: &std::collections::BTreeSet<String>,
) -> EnvDriftRelevance {
    if removed.iter().any(|v| producer_emitted.contains(*v)) {
        EnvDriftRelevance::ProducerStillEmitsRemoved
    } else if added.iter().any(|v| producer_emitted.contains(*v)) {
        EnvDriftRelevance::ProducerUsesAdded
    } else {
        EnvDriftRelevance::ProducerIrrelevant
    }
}

/// Real: `pkgs.grafana`'s own NixOS module, evaluated with its own
/// default configuration (`services.grafana.enable = true;`, no
/// `environment`/`EnvironmentFile` override) -- the same "ask Nix, don't
/// hand-interpret the module" discipline K1/K4c already used. Producer
/// evidence is the real KEYS the module sets on the systemd unit's
/// `environment=` (`systemd.services.<name>.environment`, a NixOS-level
/// option that always exists, default `{}`) -- not a source grep.
///
/// Fails closed (`Inconclusive`, never silently absorbed as "no drift")
/// if the module sets `serviceConfig.EnvironmentFile`: that would pull
/// additional variables in from a file this function doesn't read, so
/// the real emitted-variable set would be genuinely incomplete evidence,
/// not merely inconvenient to fetch. Grafana's real HEAD module does not
/// set it (verified: `hasEnvironmentFile = false`), so this branch is
/// disclosed, not exercised, for the one case this project actually has.
///
/// Sets `services.grafana.settings.security.secret_key` to a fixed probe
/// value -- a real requirement of the OLDER (`12.3.3`) module, whose
/// `serviceConfig` cannot be evaluated at all (even for unrelated keys
/// like `EnvironmentFile`) without it once that module dropped its own
/// hardcoded default; confirmed to not affect `environment`/
/// `EnvironmentFile` on the HEAD module either (a config-file value the
/// env-var interface has nothing to do with).
fn eval_grafana_producer_environment(
    rev: &str,
) -> Result<std::collections::BTreeSet<String>, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            services.grafana.enable = true;
            services.grafana.settings.security.secret_key = "OBA_K5C_PROBE_ONLY";
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        in {{
          environmentKeys = builtins.attrNames eval.config.systemd.services.grafana.environment;
          hasEnvironmentFile = (eval.config.systemd.services.grafana.serviceConfig.EnvironmentFile or null) != null;
        }}"#
    );
    let value = eval_nix_json(&expr)?;
    let has_environment_file = value
        .get("hasEnvironmentFile")
        .and_then(JsonValue::as_bool)
        .ok_or_else(|| {
            CdcError::ToolError("nix eval result missing hasEnvironmentFile".to_string())
        })?;
    if has_environment_file {
        return Err(CdcError::Inconclusive(
            "grafana's real serviceConfig.EnvironmentFile is set -- the real emitted-variable \
             set may include variables from that file, which this function does not read"
                .to_string(),
        ));
    }
    let keys = value
        .get("environmentKeys")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing environmentKeys".to_string()))?;
    keys.iter()
        .map(|k| {
            k.as_str().map(str::to_string).ok_or_else(|| {
                CdcError::ToolError("environmentKeys contained a non-string entry".to_string())
            })
        })
        .collect()
}

/// Real: the full K5c pipeline -- evaluate grafana's real HEAD producer
/// environment, classify against the K5b-confirmed removed/added sets.
/// Fails closed via `?` propagation, same as [`evaluate_mimir_cli_drift`]
/// -- never falls back to guessing if evaluation doesn't behave as
/// expected.
pub fn evaluate_grafana_env_drift(rev: &str) -> Result<EnvDriftRelevance, CdcError> {
    let producer_emitted = eval_grafana_producer_environment(rev)?;
    Ok(classify_env_drift_relevance(
        &GRAFANA_REMOVED_ENV_VARS,
        &GRAFANA_ADDED_ENV_VARS,
        &producer_emitted,
    ))
}

// ---------------------------------------------------------------------
// C-E1.2a: GeneratedConfigArtifact, the CDC vertical qualified by
// C-E1.1's own real A->B->C->D executable-fit audit (12/14 real
// holdout candidates, 93% blind inter-rater agreement --
// fixtures/c-e1.1-generated-config-audit/census.md). Not a new format-
// specific family per format (`YamlEvidence`/`TomlEvidence`/... were
// explicitly rejected in that audit's own protocol before any evidence
// was looked at) -- ONE evidence model, format and binding mechanism
// are locator details feeding it, never separate architectures.
//
// Four deliberately heterogeneous real anchor proofs, chosen exactly
// per the user's own instruction, not the easiest four:
//   - unpackerr (Go, TOML via `pkgs.formats.toml`, direct `--config=`
//     ExecStart flag)
//   - unbound (C, a hand-rolled `toConf` serializer -- NOT a
//     `pkgs.formats.*` call -- bound via an `environment.etc`
//     activation-time symlink, not a literal ExecStart path)
//   - mobilizon (Elixir, `pkgs.formats.elixirConf`, a `makeWrapper`-
//     injected env var)
//   - nebula-lighthouse-service (Python, `pkgs.formats.yaml`, a FULLY
//     IMPLICIT binding -- zero CLI args, the consumer's own real source
//     hardcodes the exact same default path the Nix module targets)
//
// Pipeline, deliberately split into named stages (per the user's own
// explicit instruction) so every real complication ends BEFORE
// `compare_config_contract`, which stays boring on purpose, the same
// "weirdness ends before compare()" discipline `ProducerEvidence`/
// `ConsumerRoute` already established:
//   acquire generated artifact -> prove artifact binding
//     -> parse producer paths -> resolve exact consumer
//     -> extract accepted paths -> compare_config_contract()
// ---------------------------------------------------------------------

/// The same real `PhysShell/nixpkgs` tree C-E1.1 itself audited all 14
/// candidates against (`fixtures/c-e1.1-generated-config-audit/
/// census.md`) -- reusing the exact pin, not re-resolving a fresh one,
/// so C-E1.2a's own real evidence is directly comparable to what that
/// audit already cited for these same four modules.
const CE12_REV: &str = "68740713a1d5904edf9ba92a998a522b1b6ce080";

/// A real Nix-generated config artifact's own content, in the form
/// leaf-path extraction actually reads. `StructuredValue` covers any
/// producer using a real `pkgs.formats.*` generator (unpackerr,
/// mobilizon, nebula-lighthouse-service all qualify) -- the real
/// Nix-evaluated JSON representation of the value that generator
/// serialized IS the semantic source of truth, so extraction reads
/// THAT directly rather than re-parsing rendered TOML/YAML/JSON text
/// (which would need a real format-specific parser per format, exactly
/// the "five near-identical adapters" this whole family was designed
/// to avoid). `RenderedText` covers a hand-rolled serializer with no
/// single structured Nix value the file's content maps to 1:1 (unbound's
/// own real `toConf`) -- extraction here genuinely does need a real,
/// bounded, disclosed text-format reader.
#[derive(Debug, Clone, PartialEq)]
enum ArtifactContent {
    StructuredValue(JsonValue),
    RenderedText(String),
}

/// Real config-format identity -- a DESCRIPTIVE/provenance field only.
/// Never branched on inside `compare_config_contract` or any shared
/// comparison logic (stop condition 6) -- only the two `ArtifactContent`
/// extraction paths above care about format, and even those only care
/// about "structured value available, yes/no", not the specific format
/// name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigFormat {
    Toml,
    Yaml,
    ElixirConf,
    HandRolled,
    /// C-E1.2b: transmission's/spacecookie's own real format -- plain
    /// JSON, a real, distinct format tag from `Yaml`/`Toml` even though
    /// `ArtifactContent::StructuredValue` handles it identically
    /// (format stays descriptive-only, never branched on).
    Json,
    /// C-E1.2b: i2pd's own real format -- `boost::program_options`'
    /// own real INI-style parser, sections included.
    Ini,
}

/// The real, mechanically-distinct ways C-E1.1 found a generated
/// artifact actually bound to its consumer process -- B's own evidence
/// shape, never assumed from A alone. Four real variants, one per
/// anchor; a future candidate needing a fifth gets a fifth variant
/// here, not a workaround forced into an existing one.
#[derive(Debug, Clone, PartialEq)]
enum ArtifactBindingEvidence {
    /// unpackerr's own real shape: the artifact's path appears literally
    /// in `ExecStart`'s own argv, behind a named flag.
    DirectArgv { flag: String, argv: String },
    /// unbound's own real shape: `environment.etc.<path>.source` is the
    /// SAME real artifact, and `ExecStart` references the fixed `/etc`
    /// path that activation-time mechanism populates -- never a literal
    /// store path on the command line at all.
    EnvironmentEtcSymlink { etc_path: String },
    /// mobilizon's own real shape: a `makeWrapper`-generated launcher
    /// sets an env var to the artifact's own real store path before
    /// exec'ing the real binary.
    WrapperScriptEnvVar { var_name: String },
    /// nebula-lighthouse-service's own real shape: `ExecStart` takes NO
    /// arguments referencing the artifact at all -- binding is proved
    /// only by the consumer's own real source hardcoding the identical
    /// default path the Nix module's `environment.etc` target uses.
    ImplicitDefaultPath { path: String },
    /// C-E1.2b: privoxy's/spacecookie's own real shape (found
    /// independently in both, the census's own repeating-pattern case) --
    /// the artifact's own real store path appears as a bare POSITIONAL
    /// `ExecStart` argument, with no named flag at all (unlike
    /// `DirectArgv`, which always has a `--flag`/`-f`-shaped name).
    DirectPositionalArg { argv: String },
    /// C-E1.2b: misskey's own real shape -- a real `ExecStartPre` step
    /// installs the artifact to a FIXED runtime path (not a literal
    /// store path) before the main process starts, and a `systemd`-level
    /// `environment` entry points at that same fixed path. A real,
    /// general NixOS secret-substitution idiom (activation happens at
    /// every service start, not just Nix activation time), distinct from
    /// `EnvironmentEtcSymlink` (that's a Nix-level `environment.etc`
    /// symlink, populated once at system activation, never a per-service-
    /// start imperative install step).
    ExecStartPreInstalledEnvVar { var_name: String, install_path: String },
}

/// One producer's full real evidence -- A (via `content`) + B (via
/// `binding`) + the D-facing half of C-E1.1's own pipeline, D's actual
/// extraction (`emitted_paths`/`opaque_paths`) done by
/// `parse_producer_paths`, a separate named stage, not folded into
/// acquisition itself.
#[derive(Debug, Clone, PartialEq)]
struct GeneratedConfigArtifactEvidence {
    producer: String,
    format: ConfigFormat,
    content: ArtifactContent,
    binding: ArtifactBindingEvidence,
    /// Real, normalized, dotted leaf paths -- e.g. `"radarr"` is never
    /// in here (it's a real list in unpackerr's own corpus, see
    /// `opaque_paths` below), but a real scalar like `"debug"` is.
    emitted_paths: Vec<String>,
    /// Real paths this bounded v1 extraction couldn't reach -- a JSON/
    /// YAML array, a repeated key under the same section in a
    /// rendered-text artifact, ... Disclosed, never silently dropped:
    /// stop condition 7 (missing/ambiguous -> inconclusive, never PASS)
    /// applies to individual PATHS too, not just whole artifacts --
    /// `compare_config_contract` only ever compares `emitted_paths`,
    /// never guesses at what an opaque path might have meant.
    opaque_paths: Vec<String>,
}

/// The real, pinned consumer's own accepted-path surface -- D's actual
/// result, extracted from real vendored consumer source (never modeled
/// generically; every real consumer format needs its own real,
/// disclosed, bounded extractor, listed below per anchor).
#[derive(Debug, Clone, PartialEq)]
struct ConsumerConfigContract {
    consumer: String,
    accepted_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConfigContractVerdict {
    Pass,
    Finding { unaccepted_path: String },
}

/// Pure. The ENTIRE comparison semantics for this whole family --
/// deliberately boring (stop condition 5: every selected candidate goes
/// through this ONE function; stop condition 6: no `if producer ==
/// "..."` anywhere in it, ever). `emitted_path ∈ accepted_paths`, full
/// stop. Every real complication (format quirks, binding shape,
/// consumer research, which paths were even reachable) happened
/// upstream, in `acquire_*`/`parse_producer_paths`/the per-consumer
/// `*_consumer_contract` functions -- not here.
fn compare_config_contract(
    evidence: &GeneratedConfigArtifactEvidence,
    contract: &ConsumerConfigContract,
) -> ConfigContractVerdict {
    for path in &evidence.emitted_paths {
        if !contract.accepted_paths.iter().any(|p| p == path) {
            return ConfigContractVerdict::Finding { unaccepted_path: path.clone() };
        }
    }
    ConfigContractVerdict::Pass
}

/// Flattens a real Nix-evaluated JSON structured value into dotted leaf
/// paths -- the extraction half of `ArtifactContent::StructuredValue`.
/// A JSON object recurses, extending the dotted path; any scalar
/// (string/number/bool/null) becomes a real leaf path. A JSON ARRAY is
/// never indexed into -- its own path is recorded as opaque instead,
/// never guessed at (unpackerr's own real `radarr` setting -- a list of
/// per-instance blocks -- is the real corpus case this exists for, not
/// a hypothetical). v1's own explicit scope limit, per the user's own
/// instruction not to solve arrays/dynamic map keys in the same round
/// as everything else.
fn flatten_structured_value(
    value: &JsonValue,
    prefix: &str,
    emitted: &mut Vec<String>,
    opaque: &mut Vec<String>,
) {
    match value {
        // C-E1.2b, found while implementing `akkoma`: a real, general
        // correctness fix -- `pkgs.formats.elixirConf`'s own SHARED
        // library (`pkgs/pkgs-lib/formats/elixir-conf/default.nix`)
        // represents any `mkRaw`/`mkAtom`/`mkTuple`/`mkCharlist`/`mkMap`
        // value as a real, general `{ _elixirType = "..."; value = ...;
        // }` wrapper object -- NOT genuine nested config structure, a
        // Nix-level encoding artifact of the generator itself. Any
        // future elixirConf-based candidate can produce one, not just
        // akkoma. Recursing into it would produce nonsense paths like
        // `...secret_key_base._secret`; the whole wrapper is opaque at
        // this leaf instead -- real content exists, this v1 doesn't try
        // to further decompose it, same precedent as a JSON array.
        // `_secret` is currently only observed in akkoma's own module
        // (its own `mkSecret`-shaped local convention, not part of the
        // shared elixirConf library) but is checked the same
        // structural way -- a marker KEY SHAPE, never an app-identity
        // check.
        JsonValue::Object(map) if map.contains_key("_elixirType") || map.contains_key("_secret") => {
            opaque.push(prefix.to_string());
        }
        JsonValue::Object(map) => {
            for (k, v) in map {
                let path = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                flatten_structured_value(v, &path, emitted, opaque);
            }
        }
        JsonValue::Array(_) => {
            opaque.push(prefix.to_string());
        }
        // C-E1.2b, found while implementing `i2pd`: a real, general
        // correctness fix, not app-specific -- a JSON `null` leaf means
        // "this option was never configured" for every candidate built
        // on a freeform Nix attrset (i2pd's own real module confirms
        // this directly: `removeNulls = lib.filterAttrsRecursive (_: v:
        // !isNull v);` strips every null-valued key before rendering the
        // real artifact, so a null in `cfg.settings` is never actually
        // emitted). Neither emitted nor opaque -- it simply isn't part
        // of the real artifact at all, the same way an absent key isn't.
        JsonValue::Null => {}
        _ => {
            if !prefix.is_empty() {
                emitted.push(prefix.to_string());
            }
        }
    }
}

/// Bounded, disclosed extraction for a hand-rolled `section:\n  key:
/// value` rendered-text artifact -- unbound's own real shape (see
/// `fixtures/cdc/generated-config-artifact/unbound/`). A section header
/// is a line ending in `:` at column 0 (no leading whitespace); an
/// indented `  key: value` line under it becomes `section.key`. A key
/// seen MORE than once under the same section (unbound's own real
/// `access-control`/`control-interface`, which can legitimately repeat)
/// is moved to `opaque` -- the same "never guess at a real list"
/// discipline `flatten_structured_value` uses for a JSON array, applied
/// to rendered text's own equivalent shape.
fn extract_unbound_style_paths(rendered: &str) -> (Vec<String>, Vec<String>) {
    let mut emitted: Vec<String> = Vec::new();
    let mut opaque: Vec<String> = Vec::new();
    let mut seen_once: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut section = String::new();
    for line in rendered.lines() {
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(char::is_whitespace) {
            if let Some(name) = line.trim_end().strip_suffix(':') {
                section = name.to_string();
            }
            continue;
        }
        let trimmed = line.trim();
        let Some((key, _val)) = trimmed.split_once(':') else {
            continue;
        };
        let path = format!("{section}.{}", key.trim());
        if opaque.contains(&path) {
            continue;
        }
        if !seen_once.insert(path.clone()) {
            // second occurrence -- a real repeat, move to opaque and
            // drop any earlier single-occurrence entry for it.
            emitted.retain(|p| p != &path);
            opaque.push(path);
            continue;
        }
        emitted.push(path);
    }
    (emitted, opaque)
}

/// C-E1.2b: privoxy's own real rendered shape -- flat, with NO sections
/// at all (confirmed against the real consumer source, `loadcfg.c`'s
/// own single flat `switch(hash_string(cmd))` dispatch -- there is no
/// section CONCEPT here to discard, unlike `unbound`'s). Each non-empty
/// line is `<key> <value...>` (space-separated, no `:` delimiter --
/// confirmed real privoxy syntax, e.g. `listen-address 127.0.0.1:8118`,
/// where the colon is part of the VALUE, not a delimiter). A key
/// repeated (e.g. `actionsfile`, a real Nix-list-valued option rendered
/// one line per element) moves to `opaque_paths`, matching
/// `extract_unbound_style_paths`'s own repeated-key precedent.
fn extract_privoxy_style_paths(rendered: &str) -> (Vec<String>, Vec<String>) {
    let mut emitted: Vec<String> = Vec::new();
    let mut opaque: Vec<String> = Vec::new();
    let mut seen_once: std::collections::HashSet<String> = std::collections::HashSet::new();
    for line in rendered.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.split_whitespace().next().unwrap_or(trimmed).to_string();
        if opaque.contains(&key) {
            continue;
        }
        if !seen_once.insert(key.clone()) {
            emitted.retain(|p| p != &key);
            opaque.push(key);
            continue;
        }
        emitted.push(key);
    }
    (emitted, opaque)
}

/// Bounded, disclosed extraction of `toml:"..."` struct-field tags from
/// a real vendored Go source file -- unpackerr's own real
/// `fixtures/cdc/generated-config-artifact/unpackerr/apps.go`. Same
/// "bounded literal scan over one observed real source shape"
/// discipline K4c's own `extract_go_flagset_literal_names` already
/// established for a different Go convention -- not a general Go
/// struct-tag parser.
fn extract_go_toml_tags(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    const NEEDLE: &str = "toml:\"";
    let mut search_from = 0;
    while let Some(rel) = source[search_from..].find(NEEDLE) {
        let pos = search_from + rel;
        let after = &source[pos + NEEDLE.len()..];
        if let Some(end) = after.find('"') {
            let tag = &after[..end];
            // Go's own `,omitempty`-style tag suffix is a serialization
            // modifier, not part of the real accepted key name.
            let name = tag.split(',').next().unwrap_or(tag);
            if name != "-" && !name.is_empty() {
                names.push(name.to_string());
            }
        }
        search_from = pos + NEEDLE.len();
    }
    names
}

/// Bounded, disclosed extraction of `NAME{COLON}` lexer keyword entries
/// from a real vendored excerpt of unbound's own
/// `util/configlexer.lex` -- see
/// `fixtures/cdc/generated-config-artifact/unbound/configlexer-excerpt.lex`'s
/// own doc comment for exactly which real lines/directives this covers
/// (NOT unbound's complete real accepted-config surface, which is
/// hundreds of directives -- a real, disclosed, bounded excerpt,
/// covering exactly what this project's own real test artifact emits).
fn extract_unbound_lexer_keywords(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let (keyword, _rest) = line.trim().split_once("{COLON}")?;
            (!keyword.is_empty()).then(|| keyword.to_string())
        })
        .collect()
}

/// Bounded, disclosed extraction of `get_config(path, 'NAME', ...)`
/// literal key names from a real vendored Python source file --
/// nebula-lighthouse-service's own real
/// `fixtures/cdc/generated-config-artifact/nebula-lighthouse-service/file_config.py`.
fn extract_python_get_config_keys(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    const NEEDLE: &str = "get_config(path, '";
    let mut search_from = 0;
    while let Some(rel) = source[search_from..].find(NEEDLE) {
        let pos = search_from + rel;
        let after = &source[pos + NEEDLE.len()..];
        if let Some(end) = after.find('\'') {
            names.push(after[..end].to_string());
        }
        search_from = pos + NEEDLE.len();
    }
    names
}

/// Bounded, disclosed extraction of `config :mobilizon, :instance,`
/// block field names from a real vendored Elixir schema file --
/// `fixtures/cdc/generated-config-artifact/mobilizon/config.exs`.
/// Scoped deliberately narrow: only the `:instance` block (what this
/// project's own real anchor test actually touches), not mobilizon's
/// entire real config surface (dozens of other real top-level blocks,
/// out of this bounded v1's own scope -- see the block's own doc
/// comment on `acquire_mobilizon_evidence` for why).
fn extract_elixir_instance_block_keys(source: &str) -> Vec<String> {
    let Some(start) = source.find("config :mobilizon, :instance,") else {
        return Vec::new();
    };
    let after = &source[start..];
    let Some(end) = after.find("\n\n") else {
        return Vec::new();
    };
    let block = &after[..end];
    block
        .lines()
        .skip(1) // the `config :mobilizon, :instance,` line itself
        .filter_map(|line| {
            let trimmed = line.trim().trim_end_matches(',');
            trimmed.split_once(':').map(|(key, _)| key.trim().to_string())
        })
        .collect()
}

/// Reads a real vendored C-E1.2a consumer-source fixture (never a live
/// fetch -- same discipline as every vendored consumer fixture in this
/// project). `Inconclusive`, not a panic: a missing fixture means this
/// consumer's contract genuinely can't be extracted right now, not that
/// the tool should crash.
fn read_ce12_fixture(relative_path: &str) -> Result<String, CdcError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    std::fs::read_to_string(&path)
        .map_err(|e| CdcError::Inconclusive(format!("reading vendored fixture {}: {e}", path.display())))
}

/// Real: unpackerr's own producer/binding evidence -- A (`pkgs.formats.
/// toml.generate` over the real `cfg.settings`) + B (the same
/// artifact's own store path appears literally, behind a real
/// `--config=` flag, in the real evaluated `ExecStart`). `settings.
/// radarr` (a real list of per-instance blocks) is genuinely opaque to
/// this bounded v1 -- the real corpus case `flatten_structured_value`'s
/// own doc comment names, not a hypothetical.
fn acquire_unpackerr_evidence(rev: &str) -> Result<GeneratedConfigArtifactEvidence, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            services.unpackerr = {{
              enable = true;
              settings.debug = true;
              settings.radarr = [ {{ api_key = "test-key-123"; url = "http://localhost:7878"; }} ];
            }};
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        in {{
          settings = eval.config.services.unpackerr.settings;
          execStart = eval.config.systemd.services.unpackerr.serviceConfig.ExecStart;
        }}"#
    );
    let value = eval_nix_json(&expr)?;
    let settings = value
        .get("settings")
        .cloned()
        .ok_or_else(|| CdcError::ToolError("nix eval result missing settings".to_string()))?;
    let exec_start = value
        .get("execStart")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing execStart".to_string()))?;
    if !exec_start.contains("--config=") {
        return Err(CdcError::Inconclusive(
            "unpackerr's real ExecStart no longer contains a --config= flag -- binding evidence lost"
                .to_string(),
        ));
    }
    let mut emitted_paths = Vec::new();
    let mut opaque_paths = Vec::new();
    flatten_structured_value(&settings, "", &mut emitted_paths, &mut opaque_paths);
    Ok(GeneratedConfigArtifactEvidence {
        producer: "unpackerr".to_string(),
        format: ConfigFormat::Toml,
        content: ArtifactContent::StructuredValue(settings),
        binding: ArtifactBindingEvidence::DirectArgv {
            flag: "--config".to_string(),
            argv: exec_start.to_string(),
        },
        emitted_paths,
        opaque_paths,
    })
}

fn unpackerr_consumer_contract() -> Result<ConsumerConfigContract, CdcError> {
    let source = read_ce12_fixture("fixtures/cdc/generated-config-artifact/unpackerr/apps.go")?;
    Ok(ConsumerConfigContract {
        consumer: "unpackerr".to_string(),
        accepted_paths: extract_go_toml_tags(&source),
    })
}

/// Real: unbound's own producer/binding evidence. A -- a hand-rolled
/// `toConf` serializer, NOT a `pkgs.formats.*` call, so there is no
/// single structured Nix value the file's own content maps to 1:1;
/// `builtins.readFile` on the real generated derivation (which Nix
/// builds on demand, `--impure` already required for `fetchTarball`)
/// gets the real rendered TEXT instead. B -- a MEANINGFULLY DIFFERENT
/// real shape from unpackerr's: `ExecStart` never puts a store path on
/// the command line at all, only the fixed `/etc/unbound/unbound.conf`
/// -- the real binding is the `environment.etc` activation-time
/// symlink from that fixed path to the SAME real artifact `A` reads.
fn acquire_unbound_evidence(rev: &str) -> Result<GeneratedConfigArtifactEvidence, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            services.unbound = {{
              enable = true;
              settings.server = {{ interface = [ "127.0.0.1" ]; port = 5353; }};
            }};
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        in {{
          rendered = builtins.readFile eval.config.environment.etc."unbound/unbound.conf".source;
          execStart = eval.config.systemd.services.unbound.serviceConfig.ExecStart;
        }}"#
    );
    let value = eval_nix_json(&expr)?;
    let rendered = value
        .get("rendered")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing rendered".to_string()))?;
    let exec_start = value
        .get("execStart")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing execStart".to_string()))?;
    const ETC_PATH: &str = "/etc/unbound/unbound.conf";
    if !exec_start.contains(ETC_PATH) {
        return Err(CdcError::Inconclusive(format!(
            "unbound's real ExecStart no longer references {ETC_PATH} -- binding evidence lost"
        )));
    }
    let (section_emitted, section_opaque) = extract_unbound_style_paths(rendered);
    // D's own real extraction granularity (`extract_unbound_lexer_keywords`)
    // is a bounded scan over BARE directive keywords -- unbound's real
    // yacc grammar defines each directive's accepted clause(s) in a
    // separate production this v1 deliberately doesn't parse (out of
    // the bounded-scope limits this whole round was told to keep to).
    // Comparing consistently at D's own real granularity means dropping
    // the section prefix `extract_unbound_style_paths` otherwise carries
    // -- a disclosed limitation (a same-named directive under two
    // different real clauses would collide here), not a silent one.
    let strip_section = |paths: Vec<String>| -> Vec<String> {
        paths.into_iter().map(|p| p.rsplit('.').next().unwrap_or_default().to_string()).collect()
    };
    let emitted_paths = strip_section(section_emitted);
    let opaque_paths = strip_section(section_opaque);
    Ok(GeneratedConfigArtifactEvidence {
        producer: "unbound".to_string(),
        format: ConfigFormat::HandRolled,
        content: ArtifactContent::RenderedText(rendered.to_string()),
        binding: ArtifactBindingEvidence::EnvironmentEtcSymlink { etc_path: ETC_PATH.to_string() },
        emitted_paths,
        opaque_paths,
    })
}

fn unbound_consumer_contract() -> Result<ConsumerConfigContract, CdcError> {
    let source =
        read_ce12_fixture("fixtures/cdc/generated-config-artifact/unbound/configlexer-excerpt.lex")?;
    Ok(ConsumerConfigContract {
        consumer: "unbound".to_string(),
        accepted_paths: extract_unbound_lexer_keywords(&source),
    })
}

/// Real: mobilizon's own producer/binding evidence. A -- `pkgs.formats.
/// elixirConf.generate` over the real `cfg.settings`. B -- a THIRD real
/// shape: `ExecStart` runs a `makeWrapper`-generated launcher script,
/// which sets `MOBILIZON_CONFIG_PATH` to the SAME real artifact before
/// exec'ing the real binary -- proved by reading the wrapper script's
/// own real content (`builtins.readFile`, reached via
/// `builtins.substring` rather than a regex `match`, since Nix's own
/// string-context tracking -- needed for `readFile` to know which
/// derivation to build -- survives substring/concatenation but is
/// documented to NOT survive a `builtins.match` capture).
///
/// D-extraction scope, deliberately bounded: only the `:instance` block
/// (see `extract_elixir_instance_block_keys`'s own doc comment) --
/// every OTHER real top-level Elixir config block mobilizon exposes
/// (`Mobilizon.Web.Endpoint`, `Mobilizon.Storage.Repo`, ...) is real but
/// out of this v1's own scope, so `emitted_paths`/`opaque_paths` here
/// are scoped to the SAME `:instance` sub-tree, not mobilizon's whole
/// real settings surface -- comparing a narrower A against a narrower D
/// consistently, never a mismatched partial comparison.
fn acquire_mobilizon_evidence(rev: &str) -> Result<GeneratedConfigArtifactEvidence, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            services.mobilizon = {{
              enable = true;
              settings = {{
                ":mobilizon" = {{
                  ":instance" = {{ name = "Test Mobilizon"; hostname = "test.example.com"; }};
                }};
              }};
            }};
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        execStart = eval.config.systemd.services.mobilizon.serviceConfig.ExecStart;
        binPath = builtins.substring 0 (builtins.stringLength execStart - 6) execStart;
        in {{
          settings = eval.config.services.mobilizon.settings;
          wrapperContent = builtins.readFile binPath;
        }}"#
    );
    let value = eval_nix_json(&expr)?;
    let settings = value
        .get("settings")
        .cloned()
        .ok_or_else(|| CdcError::ToolError("nix eval result missing settings".to_string()))?;
    let wrapper = value
        .get("wrapperContent")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing wrapperContent".to_string()))?;
    const VAR_NAME: &str = "MOBILIZON_CONFIG_PATH";
    if !wrapper.contains(&format!("export {VAR_NAME}=")) {
        return Err(CdcError::Inconclusive(format!(
            "mobilizon's real wrapper script no longer sets {VAR_NAME} -- binding evidence lost"
        )));
    }
    let instance = settings
        .get(":mobilizon")
        .and_then(|v| v.get(":instance"))
        .cloned()
        .ok_or_else(|| {
            CdcError::ToolError("mobilizon settings missing :mobilizon.:instance".to_string())
        })?;
    let mut emitted_paths = Vec::new();
    let mut opaque_paths = Vec::new();
    flatten_structured_value(&instance, "", &mut emitted_paths, &mut opaque_paths);
    if let Some(JsonValue::Object(inner)) = settings.get(":mobilizon") {
        for key in inner.keys() {
            if key != ":instance" {
                opaque_paths.push(key.clone());
            }
        }
    }
    Ok(GeneratedConfigArtifactEvidence {
        producer: "mobilizon".to_string(),
        format: ConfigFormat::ElixirConf,
        content: ArtifactContent::StructuredValue(instance),
        binding: ArtifactBindingEvidence::WrapperScriptEnvVar { var_name: VAR_NAME.to_string() },
        emitted_paths,
        opaque_paths,
    })
}

fn mobilizon_consumer_contract() -> Result<ConsumerConfigContract, CdcError> {
    let source = read_ce12_fixture("fixtures/cdc/generated-config-artifact/mobilizon/config.exs")?;
    Ok(ConsumerConfigContract {
        consumer: "mobilizon".to_string(),
        accepted_paths: extract_elixir_instance_block_keys(&source),
    })
}

/// Real: nebula-lighthouse-service's own producer/binding evidence. A --
/// `pkgs.formats.yaml.generate` over the real `cfg.settings` (already
/// flat, dotted-string keys like `"webserver.port"` -- the module's own
/// real convention, matching the consumer's own `get_config(path,
/// 'webserver.port', ...)` calls exactly, confirmed by
/// `flatten_structured_value` needing no further splitting). B -- the
/// FOURTH, adversarial real shape: `ExecStart` takes ZERO arguments
/// referencing the artifact at all. Binding is proved only by the real,
/// vendored consumer source (`webservice.py:40`) hardcoding the
/// identical default path `environment.etc` targets -- nothing on the
/// Nix side names it, so this function only confirms the artifact
/// itself exists and `ExecStart` genuinely takes no arguments; the path
/// MATCH itself is a real, disclosed, cited fact from vendored source
/// (`NEBULA_HARDCODED_DEFAULT_PATH`), not re-derived live every run.
const NEBULA_HARDCODED_DEFAULT_PATH: &str = "/etc/nebula-lighthouse-service/config.yaml";

fn acquire_nebula_lighthouse_service_evidence(
    rev: &str,
) -> Result<GeneratedConfigArtifactEvidence, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            services.nebula-lighthouse-service.enable = true;
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        in {{
          settings = eval.config.services.nebula-lighthouse-service.settings;
          execStart = eval.config.systemd.services.nebula-lighthouse-service.serviceConfig.ExecStart;
          etcSource = eval.config.environment.etc."nebula-lighthouse-service/config.yaml".source;
        }}"#
    );
    let value = eval_nix_json(&expr)?;
    let settings = value
        .get("settings")
        .cloned()
        .ok_or_else(|| CdcError::ToolError("nix eval result missing settings".to_string()))?;
    let exec_start = value
        .get("execStart")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing execStart".to_string()))?;
    // `environment.etc."nebula-lighthouse-service/config.yaml"` itself
    // resolving at all (no attribute-missing error above) is the real
    // artifact-exists proof; nothing further needed from its value here.
    let _etc_source = value
        .get("etcSource")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing etcSource".to_string()))?;
    if exec_start.split_whitespace().count() != 1 {
        return Err(CdcError::Inconclusive(
            "nebula-lighthouse-service's real ExecStart now has arguments -- the implicit-binding \
             shape this anchor demonstrates no longer applies"
                .to_string(),
        ));
    }
    let mut emitted_paths = Vec::new();
    let mut opaque_paths = Vec::new();
    flatten_structured_value(&settings, "", &mut emitted_paths, &mut opaque_paths);
    Ok(GeneratedConfigArtifactEvidence {
        producer: "nebula-lighthouse-service".to_string(),
        format: ConfigFormat::Yaml,
        content: ArtifactContent::StructuredValue(settings),
        binding: ArtifactBindingEvidence::ImplicitDefaultPath {
            path: NEBULA_HARDCODED_DEFAULT_PATH.to_string(),
        },
        emitted_paths,
        opaque_paths,
    })
}

fn nebula_lighthouse_service_consumer_contract() -> Result<ConsumerConfigContract, CdcError> {
    let source = read_ce12_fixture(
        "fixtures/cdc/generated-config-artifact/nebula-lighthouse-service/file_config.py",
    )?;
    Ok(ConsumerConfigContract {
        consumer: "nebula-lighthouse-service".to_string(),
        accepted_paths: extract_python_get_config_keys(&source),
    })
}

// ---------------------------------------------------------------------
// C-E1.2b: the small, tightly-scoped generic-additions follow-up the
// transfer census (`fixtures/c-e1.2b-transfer-census/census.md`)
// authorized -- 2 new `ArtifactBindingEvidence` variants, 1 new
// `RenderedText` extractor shape, 7 new per-consumer D-extractors, for
// exactly the 7 candidates the census verdicted `PASS`/`FINDING`
// (privoxy, misskey, kavita, transmission, i2pd, spacecookie, akkoma).
// `vault` is DELIBERATELY NOT implemented here -- the census's own
// `INCONCLUSIVE` verdict for it is a real, disclosed evidence-chain
// gap (D is not boundedly extractable for the fields its real default
// config actually emits), not something this small PR is authorized to
// "just build anyway." Zero changes to `compare_config_contract()`,
// zero `if app == "X"` anywhere in comparison/contract semantics --
// every real complication below is resolved inside its own acquire/
// extractor function, the same "weirdness ends before compare()"
// discipline every earlier candidate in this vertical already follows.
// ---------------------------------------------------------------------

/// C-E1.2b: privoxy's own real bounded D-extractor -- a scan over
/// `loadcfg.c`'s own `#define hash_X <NUMBER>U /* "name" */` table
/// (the real directive name is literally present in each line's own
/// trailing C comment). Tolerates one real upstream typo found while
/// vendoring (`hash_buffer_limit`'s own comment is missing its closing
/// quote, `/* "buffer-limit */` -- the scan accepts either a closing
/// `"` or the comment's own closing `*/` as the name's end, so this one
/// real malformed line still yields the correct name).
fn extract_privoxy_hash_table_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    const NEEDLE: &str = "#define hash_";
    let mut search_from = 0;
    while let Some(rel) = source[search_from..].find(NEEDLE) {
        let pos = search_from + rel;
        let Some(line_end) = source[pos..].find('\n') else {
            break;
        };
        let line = &source[pos..pos + line_end];
        search_from = pos + line_end;
        let Some(quote_pos) = line.find('"') else {
            continue;
        };
        let after_quote = &line[quote_pos + 1..];
        let end = after_quote.find('"').or_else(|| after_quote.find("*/")).unwrap_or(after_quote.len());
        let name = after_quote[..end].trim();
        if !name.is_empty() {
            names.push(name.to_string());
        }
    }
    names
}

fn acquire_privoxy_evidence(rev: &str) -> Result<GeneratedConfigArtifactEvidence, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            services.privoxy = {{
              enable = true;
              settings = {{ listen-address = "127.0.0.1:8118"; enable-edit-actions = true; }};
            }};
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        execStart = eval.config.systemd.services.privoxy.serviceConfig.ExecStart;
        lastSpaceIdx = s:
          let len = builtins.stringLength s;
              go = i: if i < 0 then -1
                      else if builtins.substring i 1 s == " " then i
                      else go (i - 1);
          in go (len - 1);
        idx = lastSpaceIdx execStart;
        binPath = builtins.substring (idx + 1) (builtins.stringLength execStart - idx - 1) execStart;
        in {{
          execStart = execStart;
          rendered = builtins.readFile binPath;
        }}"#
    );
    let value = eval_nix_json(&expr)?;
    let exec_start = value
        .get("execStart")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing execStart".to_string()))?;
    let rendered = value
        .get("rendered")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing rendered".to_string()))?;
    let (emitted_paths, opaque_paths) = extract_privoxy_style_paths(rendered);
    Ok(GeneratedConfigArtifactEvidence {
        producer: "privoxy".to_string(),
        format: ConfigFormat::HandRolled,
        content: ArtifactContent::RenderedText(rendered.to_string()),
        binding: ArtifactBindingEvidence::DirectPositionalArg { argv: exec_start.to_string() },
        emitted_paths,
        opaque_paths,
    })
}

fn privoxy_consumer_contract() -> Result<ConsumerConfigContract, CdcError> {
    let source = read_ce12_fixture(
        "fixtures/cdc/generated-config-artifact/privoxy/loadcfg-hashtable-excerpt.c",
    )?;
    Ok(ConsumerConfigContract {
        consumer: "privoxy".to_string(),
        accepted_paths: extract_privoxy_hash_table_names(&source),
    })
}

/// Bounded scan of ONE named TypeScript object-type literal (`type
/// NAME = { ... };` or `type NAME = Something & { ... };`), returning
/// each field's own name paired with its own raw type text.
fn ts_type_literal_body<'a>(source: &'a str, type_name: &str) -> Option<&'a str> {
    let needle = format!("type {type_name} = ");
    let rel = source.find(&needle)?;
    let after = &source[rel + needle.len()..];
    let brace_rel = after.find('{')?;
    let body_start = brace_rel + 1;
    let bytes = after.as_bytes();
    let mut depth = 1i32;
    let mut j = body_start;
    while j < after.len() && depth > 0 {
        match bytes[j] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        j += 1;
    }
    Some(&after[body_start..j.saturating_sub(1)])
}

/// Recursively scans a TypeScript object-type literal BODY into
/// `(dotted_path, raw_type)` pairs, descending into an INLINE nested
/// object type (misskey's own real `db: { host: string; ... };`
/// shape) automatically, but treating a field typed as a bare NAMED
/// type reference (e.g. `redis: RedisOptionsSource;`) as a leaf here
/// -- the caller resolves a named-type reference separately (one
/// bounded scan per real observed named type, not a general TS type
/// checker).
fn scan_ts_object_body(body: &str, prefix: &str, out: &mut Vec<(String, String)>) {
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < body.len() {
        while i < body.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        if i >= body.len() {
            break;
        }
        if body[i..].starts_with("//") {
            match body[i..].find('\n') {
                Some(nl) => i += nl + 1,
                None => break,
            }
            continue;
        }
        if bytes[i] == b',' || bytes[i] == b';' {
            i += 1;
            continue;
        }
        let name_start = i;
        while i < body.len() && bytes[i] != b':' {
            i += 1;
        }
        if i >= body.len() {
            break;
        }
        let raw_name = body[name_start..i].trim();
        let name = raw_name.trim_end_matches('?').trim();
        i += 1;
        while i < body.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        if name.is_empty() || !name.chars().next().is_some_and(|c| c.is_alphabetic()) {
            ts_skip_to_next_separator(body, &mut i);
            continue;
        }
        let path = if prefix.is_empty() { name.to_string() } else { format!("{prefix}.{name}") };
        if i < body.len() && bytes[i] == b'{' {
            let inner_start = i + 1;
            let mut depth = 1i32;
            let mut j = inner_start;
            while j < body.len() && depth > 0 {
                match bytes[j] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            let inner_body = &body[inner_start..j.saturating_sub(1)];
            scan_ts_object_body(inner_body, &path, out);
            i = j;
            // real shape: an inline nested object type can be an ARRAY
            // of that shape (misskey's own real `dbSlaves?: { ... }[];`)
            // -- skip the trailing `[]` (and any whitespace before it)
            // so the next field isn't accidentally consumed as part of
            // this one's own terminator scan.
            while i < body.len() && (bytes[i] as char).is_whitespace() {
                i += 1;
            }
            if body[i..].starts_with("[]") {
                i += 2;
            }
            ts_skip_to_next_separator(body, &mut i);
        } else {
            let value_start = i;
            ts_skip_to_next_separator(body, &mut i);
            let raw_type = body[value_start..i].trim().trim_end_matches(';').trim_end_matches(',').trim();
            out.push((path, raw_type.to_string()));
        }
    }
}

fn ts_skip_to_next_separator(body: &str, i: &mut usize) {
    let bytes = body.as_bytes();
    let mut depth = 0i32;
    while *i < body.len() {
        match bytes[*i] {
            b'{' | b'(' | b'<' | b'[' => depth += 1,
            b'}' | b')' | b'>' | b']' if depth > 0 => depth -= 1,
            b';' | b',' if depth == 0 => return,
            _ => {}
        }
        *i += 1;
    }
}

/// C-E1.2b: misskey's own real bounded D-extractor -- `Source`'s own
/// top-level fields, with every field typed exactly `RedisOptionsSource`
/// (`redis`/`redisForPubsub`/`redisForJobQueue`/`redisForTimelines`/
/// `redisForReactions`) expanded into `<field>.<redisField>` using that
/// SECOND named type's own real fields, matching the real two-hop shape
/// C-E1.1 itself already found.
fn extract_misskey_source_paths(source: &str) -> Vec<String> {
    let mut redis_fields = Vec::new();
    if let Some(body) = ts_type_literal_body(source, "RedisOptionsSource") {
        scan_ts_object_body(body, "", &mut redis_fields);
    }
    let mut raw_fields = Vec::new();
    if let Some(body) = ts_type_literal_body(source, "Source") {
        scan_ts_object_body(body, "", &mut raw_fields);
    }
    let mut paths = Vec::new();
    for (path, ty) in raw_fields {
        if ty.trim() == "RedisOptionsSource" {
            for (sub_path, _) in &redis_fields {
                paths.push(format!("{path}.{sub_path}"));
            }
        } else {
            paths.push(path);
        }
    }
    paths
}

fn acquire_misskey_evidence(rev: &str) -> Result<GeneratedConfigArtifactEvidence, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            services.misskey = {{
              enable = true;
              settings = {{ url = "https://misskey.example.org/"; }};
            }};
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        in {{
          environment = eval.config.systemd.services.misskey.environment;
          settings = eval.config.services.misskey.settings;
        }}"#
    );
    let value = eval_nix_json(&expr)?;
    let settings = value
        .get("settings")
        .cloned()
        .ok_or_else(|| CdcError::ToolError("nix eval result missing settings".to_string()))?;
    const VAR_NAME: &str = "MISSKEY_CONFIG_YML";
    let install_path = value
        .get("environment")
        .and_then(|e| e.get(VAR_NAME))
        .and_then(JsonValue::as_str)
        .ok_or_else(|| {
            CdcError::Inconclusive(format!(
                "misskey's real systemd environment no longer sets {VAR_NAME} -- binding evidence lost"
            ))
        })?;
    let mut emitted_paths = Vec::new();
    let mut opaque_paths = Vec::new();
    flatten_structured_value(&settings, "", &mut emitted_paths, &mut opaque_paths);
    Ok(GeneratedConfigArtifactEvidence {
        producer: "misskey".to_string(),
        format: ConfigFormat::Yaml,
        content: ArtifactContent::StructuredValue(settings),
        binding: ArtifactBindingEvidence::ExecStartPreInstalledEnvVar {
            var_name: VAR_NAME.to_string(),
            install_path: install_path.to_string(),
        },
        emitted_paths,
        opaque_paths,
    })
}

fn misskey_consumer_contract() -> Result<ConsumerConfigContract, CdcError> {
    let source = read_ce12_fixture("fixtures/cdc/generated-config-artifact/misskey/config.ts")?;
    Ok(ConsumerConfigContract {
        consumer: "misskey".to_string(),
        accepted_paths: extract_misskey_source_paths(&source),
    })
}

/// Bounded scan of ONE named C# class's own public property
/// declarations (`public <Type> <Name> { get; set; }` / `{ get; init;
/// }` -- kavita's real `AppSettings` mixes both accessor forms).
/// Returns each property's own name paired with its own declared type
/// text.
fn extract_csharp_class_properties(source: &str, class_name: &str) -> Vec<(String, String)> {
    let mut props = Vec::new();
    let needle = format!("class {class_name}");
    let Some(class_rel) = source.find(&needle) else {
        return props;
    };
    let after = &source[class_rel..];
    let Some(brace_rel) = after.find('{') else {
        return props;
    };
    let body_start = brace_rel + 1;
    let bytes = after.as_bytes();
    let mut depth = 1i32;
    let mut j = body_start;
    while j < after.len() && depth > 0 {
        match bytes[j] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        j += 1;
    }
    let body = &after[body_start..j.saturating_sub(1)];
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("public ") {
            continue;
        }
        let Some(brace_pos) = trimmed.find('{') else {
            continue;
        };
        let decl = trimmed[..brace_pos].trim();
        let mut tokens = decl.split_whitespace();
        let Some(_public_kw) = tokens.next() else {
            continue;
        };
        let Some(ty) = tokens.next() else {
            continue;
        };
        let Some(name) = tokens.next() else {
            continue;
        };
        props.push((name.to_string(), ty.to_string()));
    }
    props
}

/// C-E1.2b: kavita's own real bounded D-extractor -- `AppSettings`'s
/// own top-level properties, with the one field typed exactly
/// `OpenIdConnectSettings` expanded into `<field>.<subfield>` using
/// that second, real nested class's own properties.
fn extract_kavita_appsettings_paths(source: &str) -> Vec<String> {
    let top = extract_csharp_class_properties(source, "AppSettings");
    let nested = extract_csharp_class_properties(source, "OpenIdConnectSettings");
    let mut paths = Vec::new();
    for (name, ty) in top {
        if ty == "OpenIdConnectSettings" {
            for (sub_name, _) in &nested {
                paths.push(format!("{name}.{sub_name}"));
            }
        } else {
            paths.push(name);
        }
    }
    paths
}

fn acquire_kavita_evidence(rev: &str) -> Result<GeneratedConfigArtifactEvidence, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            services.kavita.enable = true;
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        in {{
          workingDirectory = eval.config.systemd.services.kavita.serviceConfig.WorkingDirectory;
          settings = eval.config.services.kavita.settings;
        }}"#
    );
    let value = eval_nix_json(&expr)?;
    let settings = value
        .get("settings")
        .cloned()
        .ok_or_else(|| CdcError::ToolError("nix eval result missing settings".to_string()))?;
    let working_directory = value
        .get("workingDirectory")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing workingDirectory".to_string()))?;
    // Real, disclosed acquire-time computation (not a new type/variant):
    // kavita's own real `preStart` installs the artifact to a fixed
    // relative path under `WorkingDirectory` (`Configuration.cs`'s own
    // hardcoded `config/<filename>` convention) -- the resolved path
    // combines BOTH real evidence sources, unlike `nebula-lighthouse-
    // service`'s own single hardcoded literal.
    let install_path = format!("{working_directory}/config/appsettings.json");
    let mut emitted_paths = Vec::new();
    let mut opaque_paths = Vec::new();
    flatten_structured_value(&settings, "", &mut emitted_paths, &mut opaque_paths);
    Ok(GeneratedConfigArtifactEvidence {
        producer: "kavita".to_string(),
        format: ConfigFormat::Yaml,
        content: ArtifactContent::StructuredValue(settings),
        binding: ArtifactBindingEvidence::ImplicitDefaultPath { path: install_path },
        emitted_paths,
        opaque_paths,
    })
}

fn kavita_consumer_contract() -> Result<ConsumerConfigContract, CdcError> {
    let source =
        read_ce12_fixture("fixtures/cdc/generated-config-artifact/kavita/configuration-excerpt.cs")?;
    Ok(ConsumerConfigContract {
        consumer: "kavita".to_string(),
        accepted_paths: extract_kavita_appsettings_paths(&source),
    })
}

/// C-E1.2b: transmission's own real bounded D-extractor -- every real
/// KEBAB-CASE string literal in the vendored `quark.cc` excerpt (see
/// that fixture's own citation for why kebab-spelling, not a per-entry
/// usage-site comment, is the real, reliable filter here).
fn extract_transmission_kebab_quarks(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix('"') else {
            continue;
        };
        let Some(end) = rest.find('"') else {
            continue;
        };
        let name = &rest[..end];
        // real, revised filter (see the vendored fixture's own header
        // comment for the real bug this closes): "contains a dash"
        // alone silently excludes real single-word keys like `umask`,
        // which can't be kebab-vs-snake-cased at all. Broadened to
        // "entirely lowercase/digit/dash" -- covers both real shapes
        // while still excluding every real snake_case (has `_`) and
        // camelCase (has an uppercase letter) alias sibling.
        if !name.is_empty()
            && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            names.push(name.to_string());
        }
    }
    names
}

fn acquire_transmission_evidence(rev: &str) -> Result<GeneratedConfigArtifactEvidence, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        pkgs = import nixpkgsSrc {{ system = "x86_64-linux"; }};
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            services.transmission = {{
              enable = true;
              package = pkgs.transmission_4;
            }};
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        in {{
          execStart = eval.config.systemd.services.transmission.serviceConfig.ExecStart;
          settings = eval.config.services.transmission.settings;
        }}"#
    );
    let value = eval_nix_json(&expr)?;
    let settings = value
        .get("settings")
        .cloned()
        .ok_or_else(|| CdcError::ToolError("nix eval result missing settings".to_string()))?;
    let exec_start = value
        .get("execStart")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing execStart".to_string()))?;
    if !exec_start.contains("-g ") {
        return Err(CdcError::Inconclusive(
            "transmission's real ExecStart no longer contains a -g flag -- binding evidence lost"
                .to_string(),
        ));
    }
    let mut emitted_paths = Vec::new();
    let mut opaque_paths = Vec::new();
    flatten_structured_value(&settings, "", &mut emitted_paths, &mut opaque_paths);
    Ok(GeneratedConfigArtifactEvidence {
        producer: "transmission".to_string(),
        format: ConfigFormat::Json,
        content: ArtifactContent::StructuredValue(settings),
        binding: ArtifactBindingEvidence::DirectArgv {
            flag: "-g".to_string(),
            argv: exec_start.to_string(),
        },
        emitted_paths,
        opaque_paths,
    })
}

fn transmission_consumer_contract() -> Result<ConsumerConfigContract, CdcError> {
    let source = read_ce12_fixture(
        "fixtures/cdc/generated-config-artifact/transmission/quark-kebab-excerpt.cc",
    )?;
    Ok(ConsumerConfigContract {
        consumer: "transmission".to_string(),
        accepted_paths: extract_transmission_kebab_quarks(&source),
    })
}

/// C-E1.2b: i2pd's own real bounded D-extractor -- every real
/// `("key[.subkey...]", value<T>()...)` `boost::program_options`
/// literal registration in `Config.cpp`. Already fully dotted/section-
/// qualified in the real consumer source itself (this candidate's own
/// real positive control for the census's adversarial normalization
/// check -- see the module's own header comment), so no further path
/// reconstruction is needed on this side.
fn extract_i2pd_program_options_keys(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    const NEEDLE: &str = "(\"";
    let mut search_from = 0;
    while let Some(rel) = source[search_from..].find(NEEDLE) {
        let pos = search_from + rel;
        let after = &source[pos + NEEDLE.len()..];
        let Some(end) = after.find('"') else {
            break;
        };
        let name = &after[..end];
        search_from = pos + NEEDLE.len() + end;
        if !name.is_empty()
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        {
            names.push(name.to_string());
        }
    }
    names
}

fn acquire_i2pd_evidence(rev: &str) -> Result<GeneratedConfigArtifactEvidence, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            services.i2pd = {{
              enable = true;
              settings = {{ ipv4 = true; ipv6 = false; }};
            }};
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        in {{
          execStart = eval.config.systemd.services.i2pd.serviceConfig.ExecStart;
          settings = eval.config.services.i2pd.settings;
        }}"#
    );
    let value = eval_nix_json(&expr)?;
    let settings = value
        .get("settings")
        .cloned()
        .ok_or_else(|| CdcError::ToolError("nix eval result missing settings".to_string()))?;
    let exec_start = value
        .get("execStart")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing execStart".to_string()))?;
    if !exec_start.contains("--conf=") {
        return Err(CdcError::Inconclusive(
            "i2pd's real ExecStart no longer contains a --conf= flag -- binding evidence lost"
                .to_string(),
        ));
    }
    let mut emitted_paths = Vec::new();
    let mut opaque_paths = Vec::new();
    flatten_structured_value(&settings, "", &mut emitted_paths, &mut opaque_paths);
    Ok(GeneratedConfigArtifactEvidence {
        producer: "i2pd".to_string(),
        format: ConfigFormat::Ini,
        content: ArtifactContent::StructuredValue(settings),
        binding: ArtifactBindingEvidence::DirectArgv {
            flag: "--conf".to_string(),
            argv: exec_start.to_string(),
        },
        emitted_paths,
        opaque_paths,
    })
}

fn i2pd_consumer_contract() -> Result<ConsumerConfigContract, CdcError> {
    let source = read_ce12_fixture("fixtures/cdc/generated-config-artifact/i2pd/Config.cpp")?;
    Ok(ConsumerConfigContract {
        consumer: "i2pd".to_string(),
        accepted_paths: extract_i2pd_program_options_keys(&source),
    })
}

fn extract_spacecookie_fromjson_block<'a>(source: &'a str, type_name: &str) -> &'a str {
    let needle = format!("instance FromJSON {type_name} where");
    let Some(rel) = source.find(&needle) else {
        return "";
    };
    let after = &source[rel + needle.len()..];
    let end = after.find("instance FromJSON").unwrap_or(after.len());
    &after[..end]
}

fn extract_spacecookie_block_keys(block: &str) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for needle in [".: \"", ".:? \"", ".:?  \""] {
        let mut search_from = 0;
        while let Some(rel) = block[search_from..].find(needle) {
            let pos = search_from + rel;
            let after = &block[pos + needle.len()..];
            let Some(end) = after.find('"') else {
                break;
            };
            let name = &after[..end];
            search_from = pos + needle.len() + end;
            if !name.is_empty() && !paths.iter().any(|p| p == name) {
                paths.push(name.to_string());
            }
        }
    }
    const MAYBE_PATH_NEEDLE: &str = "maybePath [ \"";
    let mut search_from = 0;
    while let Some(rel) = block[search_from..].find(MAYBE_PATH_NEEDLE) {
        let pos = search_from + rel;
        let after = &block[pos + MAYBE_PATH_NEEDLE.len()..];
        let Some(bracket_end) = after.find(']') else {
            break;
        };
        let segment_list = &after[..bracket_end];
        search_from = pos + MAYBE_PATH_NEEDLE.len() + bracket_end;
        let segments: Vec<String> = segment_list
            .split(',')
            .filter_map(|s| {
                let s = s.trim().trim_matches('"');
                (!s.is_empty()).then(|| s.to_string())
            })
            .collect();
        if !segments.is_empty() {
            let joined = segments.join(".");
            if !paths.iter().any(|p| p == &joined) {
                paths.push(joined);
            }
        }
    }
    paths
}

/// C-E1.2b: spacecookie's own real bounded D-extractor -- scans each
/// real `instance FromJSON <Name> where` block separately (`Config`'s
/// own top-level accessors, and `LogConfig`'s own, prefixed with
/// `log.` since `Config`'s own `log` field maps to it), recognizing
/// the exact two real Aeson call shapes both blocks use: a bare `.:
/// "X"`/`.:? "X"` accessor, and a `maybePath [ "X", "Y" ]` call.
fn extract_spacecookie_config_paths(source: &str) -> Vec<String> {
    let config_block = extract_spacecookie_fromjson_block(source, "Config");
    let log_block = extract_spacecookie_fromjson_block(source, "LogConfig");
    let mut paths: Vec<String> = extract_spacecookie_block_keys(config_block)
        .into_iter()
        .filter(|p| p != "log")
        .collect();
    for key in extract_spacecookie_block_keys(log_block) {
        paths.push(format!("log.{key}"));
    }
    paths
}

fn acquire_spacecookie_evidence(rev: &str) -> Result<GeneratedConfigArtifactEvidence, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            services.spacecookie = {{
              enable = true;
              settings = {{ hostname = "gopher.example.org"; root = "/var/lib/spacecookie"; }};
            }};
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        execStart = eval.config.systemd.services.spacecookie.serviceConfig.ExecStart;
        lastSpaceIdx = s:
          let len = builtins.stringLength s;
              go = i: if i < 0 then -1
                      else if builtins.substring i 1 s == " " then i
                      else go (i - 1);
          in go (len - 1);
        idx = lastSpaceIdx execStart;
        jsonPath = builtins.substring (idx + 1) (builtins.stringLength execStart - idx - 1) execStart;
        in {{
          execStart = execStart;
          settingsJson = builtins.fromJSON (builtins.readFile jsonPath);
        }}"#
    );
    let value = eval_nix_json(&expr)?;
    let exec_start = value
        .get("execStart")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing execStart".to_string()))?;
    let settings = value
        .get("settingsJson")
        .cloned()
        .ok_or_else(|| CdcError::ToolError("nix eval result missing settingsJson".to_string()))?;
    let mut emitted_paths = Vec::new();
    let mut opaque_paths = Vec::new();
    flatten_structured_value(&settings, "", &mut emitted_paths, &mut opaque_paths);
    Ok(GeneratedConfigArtifactEvidence {
        producer: "spacecookie".to_string(),
        format: ConfigFormat::Json,
        content: ArtifactContent::StructuredValue(settings),
        binding: ArtifactBindingEvidence::DirectPositionalArg { argv: exec_start.to_string() },
        emitted_paths,
        opaque_paths,
    })
}

fn spacecookie_consumer_contract() -> Result<ConsumerConfigContract, CdcError> {
    let source = read_ce12_fixture("fixtures/cdc/generated-config-artifact/spacecookie/Config.hs")?;
    Ok(ConsumerConfigContract {
        consumer: "spacecookie".to_string(),
        accepted_paths: extract_spacecookie_config_paths(&source),
    })
}

/// C-E1.2b: akkoma's own real bounded D-extractor -- see the module's
/// own citation on the vendored `description-excerpt.exs` for the
/// real record shape. Emits FULLY QUALIFIED `<group>.<key>.<field>`
/// paths, per the census's own real FINDING (a naive bare-field
/// extractor would merge at least 18 real, semantically distinct
/// config values under the single bare name `:enabled` alone). v1
/// scope, deliberately bounded: only a record's own DIRECT children
/// are extracted -- a child that itself declares a further NESTED
/// `children:` (a real shape, e.g. `:welcome.direct_message`/
/// `:welcome.email`, each with their OWN `:enabled` sub-field) becomes
/// its own single leaf path (`:pleroma.:welcome.direct_message`)
/// rather than being expanded further -- expanding it correctly would
/// need full recursive nesting support this round doesn't build, and a
/// naive one-more-level scan would re-introduce the exact collision
/// bug this fix exists to close.
fn extract_akkoma_description_paths(source: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = source[search_from..].find("group: :") {
        let record_start = search_from + rel;
        let after = &source[record_start + "group: :".len()..];
        let Some(comma_rel) = after.find(',') else {
            break;
        };
        let group = after[..comma_rel].trim().to_string();
        let body_search_start = record_start + "group: :".len();
        let record_body_end = source[body_search_start..]
            .find("group: :")
            .map(|rel| body_search_start + rel)
            .unwrap_or(source.len());
        let record_body = &source[record_start..record_body_end];
        let Some(key_rel) = record_body.find("key: ") else {
            search_from = record_body_end;
            continue;
        };
        let after_key = &record_body[key_rel + "key: ".len()..];
        let Some(key_comma_rel) = after_key.find(',') else {
            search_from = record_body_end;
            continue;
        };
        let top_key = after_key[..key_comma_rel].trim().to_string();
        if let Some(children_rel) = record_body.find("children: [") {
            let children_start = children_rel + "children: [".len();
            let bytes = record_body.as_bytes();
            let mut i = children_start;
            while i < record_body.len() {
                let Some(entry_rel) = record_body[i..].find("%{") else {
                    break;
                };
                let entry_start = i + entry_rel;
                let mut depth = 1i32;
                let mut j = entry_start + 2;
                while j < record_body.len() && depth > 0 {
                    match bytes[j] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        _ => {}
                    }
                    j += 1;
                }
                let entry_body = &record_body[entry_start..j];
                if let Some(fk_rel) = entry_body.find("key: :") {
                    let after_fk = &entry_body[fk_rel + "key: :".len()..];
                    let end = after_fk
                        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .unwrap_or(after_fk.len());
                    let field = &after_fk[..end];
                    if !field.is_empty() {
                        paths.push(format!(":{group}.{top_key}.{field}"));
                    }
                }
                i = j;
            }
        }
        search_from = record_body_end;
    }
    paths
}

fn acquire_akkoma_evidence(rev: &str) -> Result<GeneratedConfigArtifactEvidence, CdcError> {
    let expr = format!(
        r#"let nixpkgsSrc = builtins.fetchTarball "https://github.com/PhysShell/nixpkgs/archive/{rev}.tar.gz";
        eval = import (nixpkgsSrc + "/nixos") {{
          system = "x86_64-linux";
          configuration = {{
            networking.hostName = "akkoma";
            networking.domain = "example.org";
            services.akkoma = {{
              enable = true;
              config = {{
                ":pleroma" = {{
                  ":instance" = {{
                    name = "Test Akkoma";
                    description = "Test Akkoma server";
                    email = "akkoma@example.org";
                    notify_email = "akkoma@example.org";
                    registrations_open = true;
                  }};
                  ":media_proxy" = {{ enabled = false; }};
                  "Pleroma.Upload" = {{ base_url = "https://media.example.org/media/"; }};
                }};
              }};
              nginx.enable = false;
            }};
            system.stateVersion = "24.05";
            fileSystems."/" = {{ device = "/dev/sda1"; fsType = "ext4"; }};
            boot.loader.grub.device = "/dev/sda";
          }};
        }};
        in {{
          execStart = eval.config.systemd.services.akkoma.serviceConfig.ExecStart;
          configAttrs = eval.config.services.akkoma.config;
        }}"#
    );
    let value = eval_nix_json(&expr)?;
    let config_attrs = value
        .get("configAttrs")
        .cloned()
        .ok_or_else(|| CdcError::ToolError("nix eval result missing configAttrs".to_string()))?;
    let exec_start = value
        .get("execStart")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| CdcError::ToolError("nix eval result missing execStart".to_string()))?;
    const VAR_NAME: &str = "AKKOMA_CONFIG_PATH";
    // B's own real mechanism (see C-E1.1's own citation, re-confirmed
    // here): `ExecStart` itself carries no path at all -- the wrapper
    // sets `AKKOMA_CONFIG_PATH` from a SEPARATE `akkoma-config.service`
    // unit that copies the SAME real config derivation into place
    // before `akkoma.service` starts. Structurally proving the
    // cross-unit `bindsTo` link here would need a second real `nix
    // eval` this v1 doesn't add; the real ExecStart shape is confirmed
    // instead (the real wrapper binary path, matching the module's own
    // documented mechanism), consistent with this round's own bounded
    // scope.
    if !exec_start.contains("akkoma-env") {
        return Err(CdcError::Inconclusive(
            "akkoma's real ExecStart no longer references the real akkoma-env wrapper -- binding evidence lost"
                .to_string(),
        ));
    }
    // Real, disclosed D-extraction-scope limit found while writing this
    // acquire function's own real end-to-end test: a default-configured
    // akkoma module's real `cfg.config` ALWAYS also carries `:joken`/
    // `:logger`/`:tzdata`/`:web_push_encryption` groups (library-
    // internal Elixir dependency config, emitted unconditionally by the
    // module regardless of this project's own `config` setting) -- and
    // `description.exs` genuinely does NOT document `:joken`/`:tzdata`
    // at all (confirmed directly: zero `key: :joken`/`key: :tzdata`
    // entries anywhere in the real file), since that schema's own real
    // job is documenting the ADMIN-UI-settable `:pleroma` surface, not
    // every dependency's own internal config. Scoped the same way
    // `mobilizon`'s own evidence already needed for its bounded
    // `:instance` block: extract ONLY the real `":pleroma"` sub-object,
    // every other real top-level group is disclosed as `opaque_paths`
    // rather than silently compared against a schema that was never
    // going to describe it.
    //
    // A second, narrower real gap of the SAME shape was found by the
    // real end-to-end test even after this scoping: WITHIN `:pleroma`
    // itself, a default-configured module also injects real fields
    // `description.exs` never documents either -- `:instance.
    // upload_dir` (a computed state-directory path) and the whole
    // `Pleroma.Repo` group (real Postgres connection settings the
    // module derives from `services.postgresql`, not a user-settable
    // admin-UI field). Confirmed directly, not assumed: zero `key:
    // :upload_dir`/`key: Pleroma.Repo` occurrences anywhere in the real
    // file. Not chased further with more vendored records -- the real,
    // honest result this produces (`real_akkoma_end_to_end_is_a_real_
    // finding_not_a_forced_pass`, in `mod ce12_tests` below) is a
    // genuine executable FINDING, not a bug to paper over: it is the
    // real, disclosed boundary of what `description.exs` actually
    // documents versus what `Config.Reader` will accept, exactly the
    // distinction C-E1.1's own original akkoma research already named.
    let mut emitted_paths = Vec::new();
    let mut opaque_paths = Vec::new();
    if let JsonValue::Object(top) = &config_attrs {
        for (key, value) in top {
            if key == ":pleroma" {
                flatten_structured_value(value, ":pleroma", &mut emitted_paths, &mut opaque_paths);
            } else {
                opaque_paths.push(key.clone());
            }
        }
    }
    Ok(GeneratedConfigArtifactEvidence {
        producer: "akkoma".to_string(),
        format: ConfigFormat::ElixirConf,
        content: ArtifactContent::StructuredValue(config_attrs),
        binding: ArtifactBindingEvidence::WrapperScriptEnvVar { var_name: VAR_NAME.to_string() },
        emitted_paths,
        opaque_paths,
    })
}

fn akkoma_consumer_contract() -> Result<ConsumerConfigContract, CdcError> {
    let source =
        read_ce12_fixture("fixtures/cdc/generated-config-artifact/akkoma/description-excerpt.exs")?;
    Ok(ConsumerConfigContract {
        consumer: "akkoma".to_string(),
        accepted_paths: extract_akkoma_description_paths(&source),
    })
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

    // --- K2f: Laravel/Illuminate consumer adapter, offline
    // (extract_config_key_for_env_var, illuminate_connector_accepts_key,
    // build_consumer_contract_evidence are all pure -- only
    // fetch_config_database_php/illuminate_fixture_source/
    // acquire_illuminate_consumer_contract touch nix/network or the
    // filesystem, covered separately below) ---

    #[test]
    fn config_mapping_env_var_absent_is_none_not_an_error() {
        let php = "return ['host' => env('DB_HOST', 'localhost')];";
        assert_eq!(extract_config_key_for_env_var(php, "DB_SOCKET").unwrap(), None);
    }

    #[test]
    fn config_mapping_resolves_the_real_shaped_entry() {
        let php = "'unix_socket' => env('DB_SOCKET', ''),";
        assert_eq!(
            extract_config_key_for_env_var(php, "DB_SOCKET").unwrap(),
            Some("unix_socket".to_string())
        );
    }

    #[test]
    fn config_mapping_tolerates_a_simple_cast_with_or_without_a_space() {
        let php_with_space = "'port' => (int) env('DB_PORT', 3306),";
        let php_without_space = "'port'=>(int)env('DB_PORT', 3306),";
        assert_eq!(
            extract_config_key_for_env_var(php_with_space, "DB_PORT").unwrap(),
            Some("port".to_string())
        );
        assert_eq!(
            extract_config_key_for_env_var(php_without_space, "DB_PORT").unwrap(),
            Some("port".to_string())
        );
    }

    /// The real corpus case this exists to NOT flag as ambiguous:
    /// snipe-it's `mysql` and `mariadb` blocks both map `DB_SOCKET` ->
    /// `unix_socket` identically.
    #[test]
    fn config_mapping_same_key_repeated_is_not_ambiguous() {
        let php = "'unix_socket' => env('DB_SOCKET', ''), 'unix_socket' => env('DB_SOCKET', ''),";
        assert_eq!(
            extract_config_key_for_env_var(php, "DB_SOCKET").unwrap(),
            Some("unix_socket".to_string())
        );
    }

    #[test]
    fn config_mapping_two_distinct_keys_is_inconclusive() {
        let php = "'unix_socket' => env('DB_SOCKET', ''), 'db_socket' => env('DB_SOCKET', ''),";
        assert!(matches!(
            extract_config_key_for_env_var(php, "DB_SOCKET"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn config_mapping_unrecognized_shape_is_inconclusive_not_skipped() {
        let php = "some_function(env('DB_SOCKET', ''));"; // not preceded by "'<key>' =>"
        assert!(matches!(
            extract_config_key_for_env_var(php, "DB_SOCKET"),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn illuminate_accepts_array_keyed_form() {
        let source = "return isset($config['unix_socket']) && ! empty($config['unix_socket']);";
        assert!(illuminate_connector_accepts_key(source, "unix_socket"));
        assert!(!illuminate_connector_accepts_key(source, "charset"));
    }

    #[test]
    fn illuminate_accepts_unconditional_array_access_not_just_isset() {
        // host/database are read unconditionally in the real vendored
        // source -- never isset()-gated.
        let source = r#""mysql:host={$config['host']};dbname={$config['database']}""#;
        assert!(illuminate_connector_accepts_key(source, "host"));
        assert!(illuminate_connector_accepts_key(source, "database"));
    }

    #[test]
    fn illuminate_accepts_bare_var_form_with_boundary_check() {
        let source = "$host = isset($host) ? \"host={$host};\" : '';";
        assert!(illuminate_connector_accepts_key(source, "host"));
        // must not false-match inside a longer identifier
        assert!(!illuminate_connector_accepts_key(source, "hos"));
    }

    #[test]
    fn illuminate_rejects_a_longer_identifier_false_match() {
        let source = "$hostname = 'example.com';";
        assert!(!illuminate_connector_accepts_key(source, "host"));
    }

    #[test]
    fn consumer_contract_evidence_absent_from_config_is_inconclusive() {
        let php = "return ['host' => env('DB_HOST', 'localhost')];";
        assert!(matches!(
            build_consumer_contract_evidence(
                "DB_SOCKET",
                "config/database.php",
                php,
                "",
                IlluminateDriver::MySql
            ),
            Err(CdcError::Inconclusive(_))
        ));
    }

    #[test]
    fn consumer_contract_evidence_success_carries_the_full_route_and_passes() {
        let php = "'unix_socket' => env('DB_SOCKET', ''),";
        let connector = "isset($config['unix_socket'])";
        let evidence = build_consumer_contract_evidence(
            "DB_SOCKET",
            "config/database.php",
            php,
            connector,
            IlluminateDriver::MySql,
        )
        .unwrap();
        match &evidence.route {
            ConsumerRoute::Mediated { hops } => {
                assert_eq!(hops.len(), 1);
                assert_eq!(hops[0].layer, "config/database.php");
                assert_eq!(hops[0].from_key, "DB_SOCKET");
                assert_eq!(hops[0].to_key, "unix_socket");
            }
            other => panic!("expected Mediated, got {other:?}"),
        }
        assert_eq!(evidence.accepted_keys, vec!["unix_socket".to_string()]);
        assert_eq!(verdict_for_consumer_contract(&evidence, "DB_SOCKET"), CdcVerdict::Pass);
    }

    /// Mutation-style proof the detector is actually alive: renaming the
    /// real accepted key breaks the same evidence into a real Finding,
    /// not a false Pass -- a positive control, per this project's own
    /// discipline (every detector needs one proving it isn't vacuous).
    #[test]
    fn consumer_contract_evidence_mismatched_key_is_a_real_finding() {
        let php = "'unixSocket' => env('DB_SOCKET', ''),"; // app-layer rename
        let connector = "isset($config['unix_socket'])"; // real Illuminate contract, unchanged
        let evidence = build_consumer_contract_evidence(
            "DB_SOCKET",
            "config/database.php",
            php,
            connector,
            IlluminateDriver::MySql,
        )
        .unwrap();
        assert!(evidence.accepted_keys.is_empty());
        assert!(matches!(
            verdict_for_consumer_contract(&evidence, "DB_SOCKET"),
            CdcVerdict::Finding { .. }
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

    // --- K2f: Laravel/Illuminate consumer adapter, real (opt-in only,
    // same as every other real test in this module). ONE shared
    // acquire_illuminate_consumer_contract call for all three apps below
    // -- the only thing that varies per call is the (rev, package_attr,
    // nix_key, driver) arguments, never a branch on app identity. Zero
    // reference to doctrine/dbal or composer.lock anywhere in this
    // section, satisfying requirement (d) by construction, not by
    // omission. ---

    /// The exact K1-defect-class key, on the one app in this trio that
    /// actually renders it: snipe-it's real `DB_SOCKET` maps to
    /// `unix_socket` in `config/database.php` and IS accepted by
    /// Illuminate's real `MySqlConnector` -- Pass, via a fully mediated
    /// route, never Doctrine's contract.
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn snipeit_db_socket_is_accepted_by_illuminate_mysql_connector() {
        let evidence = acquire_illuminate_consumer_contract(
            AFTER_REV,
            "snipe-it",
            "DB_SOCKET",
            IlluminateDriver::MySql,
        )
        .unwrap();
        match &evidence.route {
            ConsumerRoute::Mediated { hops } => {
                assert_eq!(hops.len(), 1);
                assert_eq!(hops[0].layer, "config/database.php");
                assert_eq!(hops[0].from_key, "DB_SOCKET");
                assert_eq!(hops[0].to_key, "unix_socket");
            }
            other => panic!("expected Mediated, got {other:?}"),
        }
        assert_eq!(evidence.consumer_library, "Illuminate\\Database\\Connectors\\MySqlConnector");
        assert_eq!(verdict_for_consumer_contract(&evidence, "DB_SOCKET"), CdcVerdict::Pass);
    }

    /// agorakit renders no `DB_SOCKET` at all (confirmed real in K2d) --
    /// `DB_HOST` is the key that actually proves the pipeline generalizes
    /// beyond the one socket-shaped field.
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn agorakit_db_host_is_accepted_by_illuminate_mysql_connector() {
        let evidence = acquire_illuminate_consumer_contract(
            AFTER_REV,
            "agorakit",
            "DB_HOST",
            IlluminateDriver::MySql,
        )
        .unwrap();
        assert_eq!(evidence.route.resolved_key("DB_HOST"), "host");
        assert_eq!(verdict_for_consumer_contract(&evidence, "DB_HOST"), CdcVerdict::Pass);
    }

    /// movim, via the SAME shared adapter, branching only on
    /// `IlluminateDriver::Postgres` -- proves the "one adapter, no
    /// app-name branch" requirement across a genuinely different
    /// connector source, not just a second MySQL app.
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn movim_db_host_is_accepted_by_illuminate_postgres_connector() {
        let evidence = acquire_illuminate_consumer_contract(
            AFTER_REV,
            "movim",
            "DB_HOST",
            IlluminateDriver::Postgres,
        )
        .unwrap();
        assert_eq!(evidence.route.resolved_key("DB_HOST"), "host");
        assert_eq!(evidence.consumer_library, "Illuminate\\Database\\Connectors\\PostgresConnector");
        assert_eq!(verdict_for_consumer_contract(&evidence, "DB_HOST"), CdcVerdict::Pass);
    }

    /// A real, valuable negative finding, not a gap: movim's own
    /// `config/database.php` never renders a `DB_SOCKET` key at all, and
    /// Postgres's real Illuminate connector has no unix-socket concept
    /// whatsoever (confirmed by reading `PostgresConnector.php` directly
    /// -- no `hasSocket()`-equivalent exists there). The K1 defect class
    /// this whole project is built around structurally cannot occur for
    /// movim's Postgres deployment.
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn movim_has_no_unix_socket_concept_via_postgres() {
        assert!(matches!(
            acquire_illuminate_consumer_contract(
                AFTER_REV,
                "movim",
                "DB_SOCKET",
                IlluminateDriver::Postgres,
            ),
            Err(CdcError::Inconclusive(_))
        ));
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

    // --- K2g: Phase D vendoring for strichliste/part-db, real (opt-in
    // only, same as every other real test in this module). Both reuse
    // verdict_for_evidence/the K2a provenance pipeline completely
    // unchanged -- no new comparison semantics, only two new real
    // acquisition paths and two new vendored consumer fixtures. ---

    /// `strichliste` pins doctrine/dbal 3.10.5, confirmed byte-identical
    /// to the already-vendored 3.10.6 fixture (see
    /// `fixtures/integrity-lock.toml`'s own K2g comment) -- reuses
    /// `accepted_doctrine_keys()` directly, no separate helper needed.
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn strichliste_golden_is_pass() {
        let evidence =
            acquire_strichliste_evidence(AFTER_REV, "/__OBA_CONTRACT_socket_k2g__/mysql.sock", None)
                .unwrap();
        assert!(matches!(evidence, ProducerEvidence::SentinelFlow { .. }));
        assert_eq!(verdict_for_evidence(&evidence), CdcVerdict::Pass);

        // K2a's own provenance pipeline, unchanged -- proves the
        // auto-resolved identity for THIS app matches the vendored
        // fixture's own recorded provenance (3.10.5, not 3.10.6 --
        // resolved independently, not assumed shared).
        let lock = fetch_composer_lock(AFTER_REV, "strichliste").unwrap();
        let identity = resolve_consumer_identity(&lock, "doctrine/dbal").unwrap();
        assert_eq!(identity.version, "3.10.5");
        assert_eq!(identity.source_reference, "95d84866bf3c04b2ddca1df7c049714660959aef");
    }

    fn accepted_part_db_keys() -> Vec<String> {
        let doctrine_src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/cdc/doctrine-dbal-4.4.3-pgsql/PDO-PgSQL-Driver.php"),
        )
        .unwrap();
        extract_accepted_keys(&doctrine_src).unwrap()
    }

    /// `part-db`'s real Postgres deployment has no `unix_socket` DSN
    /// parameter at all -- the sentinel goes into `host=` (Postgres
    /// overloads it for both a TCP hostname and a unix-socket directory
    /// path), so this asserts `emitted_key == "host"`, not
    /// `"unix_socket"`, and compares against the PGSQL accepted-keys
    /// list (which genuinely has no `unix_socket` entry either) -- a
    /// real Pass on a real, different dialect, not a copy of the MySQL
    /// case with different words.
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball + a package source fetch)"]
    fn part_db_golden_is_pass() {
        let evidence =
            acquire_part_db_evidence(AFTER_REV, "/__OBA_CONTRACT_socket_k2g__/pg.sock", None)
                .unwrap();
        match &evidence {
            ProducerEvidence::SentinelFlow { emitted_key, .. } => {
                assert_eq!(emitted_key, "host");
            }
            other => panic!("expected SentinelFlow, got {other:?}"),
        }
        let accepted = accepted_part_db_keys();
        assert!(!accepted.iter().any(|k| k == "unix_socket"));
        assert_eq!(compare_contract(evidence.emitted_key().unwrap(), &accepted), CdcVerdict::Pass);

        let lock = fetch_composer_lock(AFTER_REV, "part-db").unwrap();
        let identity = resolve_consumer_identity(&lock, "doctrine/dbal").unwrap();
        assert_eq!(identity.version, "4.4.3");
        assert_eq!(identity.source_reference, "61e730f1658814821a85f2402c945f3883407dec");
    }
}

#[cfg(test)]
mod k3a_tests {
    use super::*;

    fn contract(library: &str, dialect: &str, version: &str, keys: &[&str]) -> ConsumerContract {
        ConsumerContract {
            library: library.to_string(),
            dialect: dialect.to_string(),
            version: version.to_string(),
            accepted_keys: keys.iter().map(|k| k.to_string()).collect(),
        }
    }

    fn doctrine_mysql(version: &str, keys: &[&str]) -> ConsumerContract {
        contract("doctrine/dbal", "mysql", version, keys)
    }

    #[test]
    fn diff_of_a_contract_with_itself_is_empty() {
        let c = doctrine_mysql("3.10.6", &["host", "port", "dbname", "unix_socket", "charset"]);
        let diff = diff_contracts(&c, &c).unwrap();
        assert!(diff.removed.is_empty());
        assert!(diff.added.is_empty());
        let mut expected = vec!["charset", "dbname", "host", "port", "unix_socket"];
        expected.sort();
        assert_eq!(diff.retained, expected);
    }

    #[test]
    fn diff_detects_a_real_removed_and_added_key() {
        let base = doctrine_mysql("4.2.0", &["host", "port", "socket_path"]);
        let head = doctrine_mysql("4.3.0", &["host", "port", "socket-path"]);
        let diff = diff_contracts(&base, &head).unwrap();
        assert_eq!(diff.removed, vec!["socket_path".to_string()]);
        assert_eq!(diff.added, vec!["socket-path".to_string()]);
        assert_eq!(diff.retained, vec!["host".to_string(), "port".to_string()]);
    }

    #[test]
    fn diff_mismatched_library_is_inconclusive() {
        let base = doctrine_mysql("3.10.6", &["host"]);
        let head = contract("illuminate/database", "mysql", "12.0.0", &["host"]);
        assert!(matches!(diff_contracts(&base, &head), Err(CdcError::Inconclusive(_))));
    }

    #[test]
    fn diff_mismatched_dialect_is_inconclusive_even_for_the_same_library() {
        // the exact part-db lesson: Doctrine MySQL vs Doctrine Postgres
        // are not comparable as bare string sets.
        let base = doctrine_mysql("3.10.6", &["host", "port", "unix_socket"]);
        let head = contract("doctrine/dbal", "pgsql", "4.4.3", &["host", "port"]);
        assert!(matches!(diff_contracts(&base, &head), Err(CdcError::Inconclusive(_))));
    }

    #[test]
    fn diff_duplicate_key_in_base_is_inconclusive_not_deduped() {
        let base = doctrine_mysql("3.10.6", &["host", "host"]);
        let head = doctrine_mysql("3.10.7", &["host"]);
        assert!(matches!(diff_contracts(&base, &head), Err(CdcError::Inconclusive(_))));
    }

    #[test]
    fn diff_duplicate_key_in_head_is_inconclusive_not_deduped() {
        let base = doctrine_mysql("3.10.6", &["host"]);
        let head = doctrine_mysql("3.10.7", &["host", "host"]);
        assert!(matches!(diff_contracts(&base, &head), Err(CdcError::Inconclusive(_))));
    }

    #[test]
    fn diff_result_does_not_depend_on_input_key_ordering() {
        let base_a = doctrine_mysql("3.10.6", &["host", "port", "unix_socket"]);
        let base_b = doctrine_mysql("3.10.6", &["unix_socket", "host", "port"]);
        let head = doctrine_mysql("3.10.7", &["port", "host"]);
        assert_eq!(diff_contracts(&base_a, &head).unwrap(), diff_contracts(&base_b, &head).unwrap());
    }

    // --- Property-based tests (proptest), same evidence tier AGENTS.md
    // asks for and the same crate H2's own pure-IR tests already use --
    // not a new precedent.

    proptest::proptest! {
        #[test]
        fn prop_diff_of_a_contract_with_itself_is_always_empty(keys in proptest::prelude::prop::collection::btree_set("[a-c]{1,4}", 0..6)) {
            let keys: Vec<String> = keys.into_iter().collect();
            let c = doctrine_mysql("1.0.0", &keys.iter().map(String::as_str).collect::<Vec<_>>());
            let diff = diff_contracts(&c, &c).unwrap();
            proptest::prop_assert!(diff.removed.is_empty());
            proptest::prop_assert!(diff.added.is_empty());
        }

        #[test]
        fn prop_removed_and_added_are_symmetric_under_swap(
            base_keys in proptest::prelude::prop::collection::btree_set("[a-c]{1,4}", 0..6),
            head_keys in proptest::prelude::prop::collection::btree_set("[a-c]{1,4}", 0..6),
        ) {
            let base_keys: Vec<String> = base_keys.into_iter().collect();
            let head_keys: Vec<String> = head_keys.into_iter().collect();
            let base = doctrine_mysql("1.0.0", &base_keys.iter().map(String::as_str).collect::<Vec<_>>());
            let head = doctrine_mysql("2.0.0", &head_keys.iter().map(String::as_str).collect::<Vec<_>>());
            let forward = diff_contracts(&base, &head).unwrap();
            let backward = diff_contracts(&head, &base).unwrap();
            proptest::prop_assert_eq!(forward.removed, backward.added);
            proptest::prop_assert_eq!(forward.added, backward.removed);
            proptest::prop_assert_eq!(forward.retained, backward.retained);
        }
    }
}

#[cfg(test)]
mod k4a_tests {
    use super::*;

    fn contract(program: &str, version: &str, flags: &[&str]) -> CliContract {
        CliContract {
            program: program.to_string(),
            version: version.to_string(),
            flags: flags.iter().map(|f| f.to_string()).collect(),
        }
    }

    #[test]
    fn diff_of_a_contract_with_itself_is_empty() {
        let c = contract("foo", "1.4", &["--host", "--port", "--socket"]);
        let diff = diff_cli_contracts(&c, &c).unwrap();
        assert!(diff.removed.is_empty());
        assert!(diff.added.is_empty());
        let mut expected = vec!["--host", "--port", "--socket"];
        expected.sort();
        assert_eq!(diff.retained, expected);
    }

    /// The exact motivating scenario named going into K4: a real
    /// removed/renamed flag, with a producer/consumer-relevant name.
    #[test]
    fn diff_detects_a_real_removed_and_added_flag() {
        let base = contract("foo", "1.4", &["--host", "--socket"]);
        let head = contract("foo", "1.5", &["--host", "--unix-socket"]);
        let diff = diff_cli_contracts(&base, &head).unwrap();
        assert_eq!(diff.removed, vec!["--socket".to_string()]);
        assert_eq!(diff.added, vec!["--unix-socket".to_string()]);
        assert_eq!(diff.retained, vec!["--host".to_string()]);
    }

    #[test]
    fn diff_mismatched_program_is_inconclusive() {
        let base = contract("foo", "1.4", &["--socket"]);
        let head = contract("bar", "1.0", &["--socket"]);
        assert!(matches!(diff_cli_contracts(&base, &head), Err(CdcError::Inconclusive(_))));
    }

    #[test]
    fn diff_duplicate_flag_in_base_is_inconclusive_not_deduped() {
        let base = contract("foo", "1.4", &["--socket", "--socket"]);
        let head = contract("foo", "1.5", &["--socket"]);
        assert!(matches!(diff_cli_contracts(&base, &head), Err(CdcError::Inconclusive(_))));
    }

    #[test]
    fn diff_duplicate_flag_in_head_is_inconclusive_not_deduped() {
        let base = contract("foo", "1.4", &["--socket"]);
        let head = contract("foo", "1.5", &["--socket", "--socket"]);
        assert!(matches!(diff_cli_contracts(&base, &head), Err(CdcError::Inconclusive(_))));
    }

    #[test]
    fn diff_result_does_not_depend_on_input_flag_ordering() {
        let base_a = contract("foo", "1.4", &["--host", "--port", "--socket"]);
        let base_b = contract("foo", "1.4", &["--socket", "--host", "--port"]);
        let head = contract("foo", "1.5", &["--port", "--host"]);
        assert_eq!(
            diff_cli_contracts(&base_a, &head).unwrap(),
            diff_cli_contracts(&base_b, &head).unwrap()
        );
    }

    // --- Property-based tests (proptest), same crate/tier K3a's own
    // tests already use -- not a new precedent.

    proptest::proptest! {
        #[test]
        fn prop_diff_of_a_contract_with_itself_is_always_empty(flags in proptest::prelude::prop::collection::btree_set("--[a-c]{1,4}", 0..6)) {
            let flags: Vec<String> = flags.into_iter().collect();
            let c = contract("foo", "1.0", &flags.iter().map(String::as_str).collect::<Vec<_>>());
            let diff = diff_cli_contracts(&c, &c).unwrap();
            proptest::prop_assert!(diff.removed.is_empty());
            proptest::prop_assert!(diff.added.is_empty());
        }

        #[test]
        fn prop_removed_and_added_are_symmetric_under_swap(
            base_flags in proptest::prelude::prop::collection::btree_set("--[a-c]{1,4}", 0..6),
            head_flags in proptest::prelude::prop::collection::btree_set("--[a-c]{1,4}", 0..6),
        ) {
            let base_flags: Vec<String> = base_flags.into_iter().collect();
            let head_flags: Vec<String> = head_flags.into_iter().collect();
            let base = contract("foo", "1.0", &base_flags.iter().map(String::as_str).collect::<Vec<_>>());
            let head = contract("foo", "2.0", &head_flags.iter().map(String::as_str).collect::<Vec<_>>());
            let forward = diff_cli_contracts(&base, &head).unwrap();
            let backward = diff_cli_contracts(&head, &base).unwrap();
            proptest::prop_assert_eq!(forward.removed, backward.added);
            proptest::prop_assert_eq!(forward.added, backward.removed);
            proptest::prop_assert_eq!(forward.retained, backward.retained);
        }
    }
}

#[cfg(test)]
mod k4c_tests {
    use super::*;

    // --- extract_go_flagset_literal_names: offline, pure ---

    #[test]
    fn extractor_captures_a_direct_literal_flag() {
        let src = r#"f.StringVar(&cfg.Foo, "my.flag", "", "help text")"#;
        assert_eq!(extract_go_flagset_literal_names(src), vec!["my.flag".to_string()]);
    }

    #[test]
    fn extractor_captures_multiple_flags_in_order_found() {
        let src = r#"
            f.StringVar(&cfg.Foo, "foo.flag", "", "help")
            f.BoolVar(&cfg.Bar, "bar.flag", true, "help")
        "#;
        assert_eq!(
            extract_go_flagset_literal_names(src),
            vec!["foo.flag".to_string(), "bar.flag".to_string()]
        );
    }

    /// The real, confirmed extraction limitation: a flag registered via
    /// a named Go constant is NOT captured. Deliberate -- not a general
    /// Go parser, a bounded scan over this one observed shape.
    #[test]
    fn extractor_skips_a_constant_referenced_flag_name() {
        let src = r#"f.DurationVar(&cfg.Foo, someConstantFlagName, 0, "help")"#;
        assert!(extract_go_flagset_literal_names(src).is_empty());
    }

    #[test]
    fn extractor_skips_a_concatenation_expression() {
        let src = r#"f.DurationVar(&cfg.Foo, someConst+"-suffix", 0, "help")"#;
        assert!(extract_go_flagset_literal_names(src).is_empty());
    }

    #[test]
    fn extractor_ignores_unrelated_ampersand_cfg_usage() {
        let src = r#"someOtherFunc(&cfg.Foo)"#;
        assert!(extract_go_flagset_literal_names(src).is_empty());
    }

    // --- argv_contains_flag: offline, pure -- the exact correctness
    // trap this section exists to avoid: prefer-availability-zone is a
    // literal PREFIX of prefer-availability-zones. ---

    #[test]
    fn argv_contains_flag_matches_single_dash() {
        assert!(argv_contains_flag("mimir -querier.prefer-availability-zone=eu", "querier.prefer-availability-zone"));
    }

    #[test]
    fn argv_contains_flag_matches_double_dash() {
        assert!(argv_contains_flag("mimir --querier.prefer-availability-zone=eu", "querier.prefer-availability-zone"));
    }

    #[test]
    fn argv_contains_flag_does_not_false_match_a_shorter_flag_inside_a_longer_one() {
        // only the PLURAL flag is present; searching for the SINGULAR
        // (a strict prefix of the plural) must NOT match.
        let argv = "mimir --querier.prefer-availability-zones=eu-west,eu-east";
        assert!(!argv_contains_flag(argv, "querier.prefer-availability-zone"));
        assert!(argv_contains_flag(argv, "querier.prefer-availability-zones"));
    }

    #[test]
    fn argv_contains_flag_absent_entirely() {
        assert!(!argv_contains_flag(
            "mimir --config.file=/etc/mimir.yaml",
            "querier.prefer-availability-zone"
        ));
    }

    // --- classify_cli_drift_relevance: offline, pure -- the A/B/C
    // distinction the design review required. ---

    #[test]
    fn classify_a_producer_still_emits_removed() {
        let argv = "mimir --querier.prefer-availability-zone=eu";
        assert_eq!(
            classify_cli_drift_relevance("querier.prefer-availability-zone", "querier.prefer-availability-zones", argv),
            CliDriftRelevance::ProducerStillEmitsRemoved
        );
    }

    #[test]
    fn classify_b_producer_adopted_new() {
        let argv = "mimir --querier.prefer-availability-zones=eu-west,eu-east";
        assert_eq!(
            classify_cli_drift_relevance("querier.prefer-availability-zone", "querier.prefer-availability-zones", argv),
            CliDriftRelevance::ProducerAdoptedNew
        );
    }

    #[test]
    fn classify_c_producer_irrelevant() {
        let argv = "mimir --config.file=/etc/mimir.yaml";
        assert_eq!(
            classify_cli_drift_relevance("querier.prefer-availability-zone", "querier.prefer-availability-zones", argv),
            CliDriftRelevance::ProducerIrrelevant
        );
    }

    // --- Real: the vendored contracts + diff_cli_contracts (K4a,
    // unchanged) against the two real vendored files -- offline (no
    // nix/network needed, these files are already vendored), but
    // exercising the real extraction end-to-end, locking in the exact
    // real result this round hand-verified. ---

    #[test]
    fn real_vendored_mimir_contracts_diff_matches_the_hand_verified_result() {
        let base = mimir_querier_go_contract(MIMIR_2_14_0_FIXTURE, "2.14.0").unwrap();
        let head = mimir_querier_go_contract(MIMIR_3_2_1_FIXTURE, "3.2.1").unwrap();
        let diff = diff_cli_contracts(&base, &head).unwrap();

        assert!(diff.removed.contains(&MIMIR_REMOVED_FLAG.to_string()));
        assert!(diff.added.contains(&MIMIR_ADDED_FLAG.to_string()));

        // The real, hand-verified extraction blind spot (K4c's own doc
        // comment on extract_go_flagset_literal_names): these two flags
        // were literals in 2.14.0 and became named-constant references
        // in 3.2.1 (constants confirmed, by reading the source, to
        // resolve to the IDENTICAL strings) -- a real limitation of
        // this bounded extractor, not a second genuine rename. Asserted
        // here explicitly so a future accidental fix of the extractor
        // (or a real further upstream change) is caught by this test,
        // not silently absorbed.
        assert!(diff.removed.contains(&"querier.streaming-chunks-per-ingester-buffer-size".to_string()));
        assert!(diff.removed.contains(&"querier.streaming-chunks-per-store-gateway-buffer-size".to_string()));

        // Real, confirmed retained flags -- present as literals in BOTH
        // versions.
        assert!(diff.retained.contains(&"querier.query-engine".to_string()));
        assert!(diff.retained.contains(&"querier.enable-query-engine-fallback".to_string()));
    }

    // --- Real end-to-end: needs a real `nix` binary and network access
    // (fetchTarball), opt-in only, same as every other real test in
    // this module. ---

    /// The actual K4c finding: mimir's real NixOS module, evaluated
    /// with its own default configuration, never emits EITHER the
    /// removed or the added flag -- outcome C. Real, upstream CLI
    /// contract drift (K4b) exists, but this Nix producer never reached
    /// it either way (the module renders its config as a YAML file,
    /// `--config.file=...` is the module's only default CLI flag;
    /// `services.mimir.extraFlags` exists as a real escape hatch but
    /// defaults to empty and nothing in this corpus sets it to either
    /// name).
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn mimir_producer_correlation_is_outcome_c_producer_irrelevant() {
        let relevance = evaluate_mimir_cli_drift(AFTER_REV).unwrap();
        assert_eq!(relevance, CliDriftRelevance::ProducerIrrelevant);
    }
}

#[cfg(test)]
mod k5a_tests {
    use super::*;

    fn contract(consumer: &str, version: &str, vars: &[&str]) -> EnvContract {
        EnvContract {
            consumer: consumer.to_string(),
            version: version.to_string(),
            variables: vars.iter().map(|v| v.to_string()).collect(),
        }
    }

    #[test]
    fn diff_of_a_contract_with_itself_is_empty() {
        let c = contract("foo", "1.4", &["DB_HOST", "DB_PORT", "DB_SOCKET"]);
        let diff = diff_env_contracts(&c, &c).unwrap();
        assert!(diff.removed.is_empty());
        assert!(diff.added.is_empty());
        let mut expected = vec!["DB_HOST", "DB_PORT", "DB_SOCKET"];
        expected.sort();
        assert_eq!(diff.retained, expected);
    }

    /// The exact motivating scenario named going into K5: a real
    /// removed/renamed env var, with a producer/consumer-relevant name.
    #[test]
    fn diff_detects_a_real_removed_and_added_variable() {
        let base = contract("foo", "1.4", &["DATABASE_URL", "DB_HOST"]);
        let head = contract("foo", "1.5", &["DB_URL", "DB_HOST"]);
        let diff = diff_env_contracts(&base, &head).unwrap();
        assert_eq!(diff.removed, vec!["DATABASE_URL".to_string()]);
        assert_eq!(diff.added, vec!["DB_URL".to_string()]);
        assert_eq!(diff.retained, vec!["DB_HOST".to_string()]);
    }

    #[test]
    fn diff_mismatched_consumer_is_inconclusive() {
        let base = contract("foo", "1.4", &["DB_HOST"]);
        let head = contract("bar", "1.0", &["DB_HOST"]);
        assert!(matches!(diff_env_contracts(&base, &head), Err(CdcError::Inconclusive(_))));
    }

    #[test]
    fn diff_duplicate_variable_in_base_is_inconclusive_not_deduped() {
        let base = contract("foo", "1.4", &["DB_HOST", "DB_HOST"]);
        let head = contract("foo", "1.5", &["DB_HOST"]);
        assert!(matches!(diff_env_contracts(&base, &head), Err(CdcError::Inconclusive(_))));
    }

    #[test]
    fn diff_duplicate_variable_in_head_is_inconclusive_not_deduped() {
        let base = contract("foo", "1.4", &["DB_HOST"]);
        let head = contract("foo", "1.5", &["DB_HOST", "DB_HOST"]);
        assert!(matches!(diff_env_contracts(&base, &head), Err(CdcError::Inconclusive(_))));
    }

    #[test]
    fn diff_result_does_not_depend_on_input_variable_ordering() {
        let base_a = contract("foo", "1.4", &["DB_HOST", "DB_PORT", "DB_SOCKET"]);
        let base_b = contract("foo", "1.4", &["DB_SOCKET", "DB_HOST", "DB_PORT"]);
        let head = contract("foo", "1.5", &["DB_PORT", "DB_HOST"]);
        assert_eq!(
            diff_env_contracts(&base_a, &head).unwrap(),
            diff_env_contracts(&base_b, &head).unwrap()
        );
    }

    // --- Property-based tests (proptest), same crate/tier K3a's and
    // K4a's own tests already use -- not a new precedent.

    proptest::proptest! {
        #[test]
        fn prop_diff_of_a_contract_with_itself_is_always_empty(vars in proptest::prelude::prop::collection::btree_set("[A-C]{1,4}", 0..6)) {
            let vars: Vec<String> = vars.into_iter().collect();
            let c = contract("foo", "1.0", &vars.iter().map(String::as_str).collect::<Vec<_>>());
            let diff = diff_env_contracts(&c, &c).unwrap();
            proptest::prop_assert!(diff.removed.is_empty());
            proptest::prop_assert!(diff.added.is_empty());
        }

        #[test]
        fn prop_removed_and_added_are_symmetric_under_swap(
            base_vars in proptest::prelude::prop::collection::btree_set("[A-C]{1,4}", 0..6),
            head_vars in proptest::prelude::prop::collection::btree_set("[A-C]{1,4}", 0..6),
        ) {
            let base_vars: Vec<String> = base_vars.into_iter().collect();
            let head_vars: Vec<String> = head_vars.into_iter().collect();
            let base = contract("foo", "1.0", &base_vars.iter().map(String::as_str).collect::<Vec<_>>());
            let head = contract("foo", "2.0", &head_vars.iter().map(String::as_str).collect::<Vec<_>>());
            let forward = diff_env_contracts(&base, &head).unwrap();
            let backward = diff_env_contracts(&head, &base).unwrap();
            proptest::prop_assert_eq!(forward.removed, backward.added);
            proptest::prop_assert_eq!(forward.added, backward.removed);
            proptest::prop_assert_eq!(forward.retained, backward.retained);
        }
    }
}

#[cfg(test)]
mod k5c_tests {
    use super::*;

    // --- classify_env_drift_relevance: offline, pure ---

    #[test]
    fn classify_a_producer_still_emits_removed() {
        let emitted: std::collections::BTreeSet<String> =
            ["PATH".to_string(), "GF_AUTH_PASSWORDLESS_ENABLED".to_string()].into_iter().collect();
        assert_eq!(
            classify_env_drift_relevance(&GRAFANA_REMOVED_ENV_VARS, &GRAFANA_ADDED_ENV_VARS, &emitted),
            EnvDriftRelevance::ProducerStillEmitsRemoved
        );
    }

    #[test]
    fn classify_b_producer_uses_added() {
        // a synthetic removed/added pair -- grafana's own real added set
        // is empty, so this branch is exercised with hypothetical names,
        // not grafana's constants (see `classify_empty_added_set_...`
        // below for the real, always-empty-added case).
        let removed = ["OLD_VAR"];
        let added = ["NEW_VAR"];
        let emitted: std::collections::BTreeSet<String> = ["NEW_VAR".to_string()].into_iter().collect();
        assert_eq!(
            classify_env_drift_relevance(&removed, &added, &emitted),
            EnvDriftRelevance::ProducerUsesAdded
        );
    }

    #[test]
    fn classify_c_producer_irrelevant() {
        let emitted: std::collections::BTreeSet<String> = ["PATH".to_string()].into_iter().collect();
        assert_eq!(
            classify_env_drift_relevance(&GRAFANA_REMOVED_ENV_VARS, &GRAFANA_ADDED_ENV_VARS, &emitted),
            EnvDriftRelevance::ProducerIrrelevant
        );
    }

    /// Grafana's own real case: a pure removal, no replacement -- the
    /// `ProducerUsesAdded` branch is real code, but genuinely
    /// unreachable for THIS correlation (`GRAFANA_ADDED_ENV_VARS` is
    /// empty). Confirmed explicitly rather than left implicit.
    #[test]
    fn classify_empty_added_set_never_fires_producer_uses_added() {
        let emitted: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        assert_eq!(
            classify_env_drift_relevance(&GRAFANA_REMOVED_ENV_VARS, &GRAFANA_ADDED_ENV_VARS, &emitted),
            EnvDriftRelevance::ProducerIrrelevant
        );
    }

    #[test]
    fn classify_removed_takes_priority_over_added() {
        let removed = ["OLD_VAR"];
        let added = ["NEW_VAR"];
        let emitted: std::collections::BTreeSet<String> =
            ["OLD_VAR".to_string(), "NEW_VAR".to_string()].into_iter().collect();
        assert_eq!(
            classify_env_drift_relevance(&removed, &added, &emitted),
            EnvDriftRelevance::ProducerStillEmitsRemoved
        );
    }

    // --- Real end-to-end: needs a real `nix` binary and network access
    // (fetchTarball), opt-in only, same as every other real test in
    // this module. ---

    /// The actual K5c finding: grafana's real NixOS module, evaluated
    /// with its own default configuration, configures Grafana entirely
    /// via a generated `config.ini` (`-config <path>`, part of
    /// `serviceConfig.ExecStart`) -- it never sets `environment` beyond
    /// systemd's own default `PATH`, and never sets `EnvironmentFile`
    /// either. Outcome C: real, source-verified upstream env-var removal
    /// (K5b) exists, but this producer's interface to Grafana was never
    /// `GF_*` env vars at all -- a categorically stronger C than K4c's
    /// mimir result (which at least emitted SOME CLI flags, just not
    /// either contested one).
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn grafana_producer_correlation_is_outcome_c_producer_irrelevant() {
        let relevance = evaluate_grafana_env_drift(AFTER_REV).unwrap();
        assert_eq!(relevance, EnvDriftRelevance::ProducerIrrelevant);
    }

    /// Symmetry check: the BASE producer (`12.3.3`) already used the
    /// same INI-only interface. `12.3.3`'s module requires an explicit
    /// `settings.security.secret_key` to reach `serviceConfig.ExecStart`
    /// at all (a real, unrelated module difference this project doesn't
    /// otherwise model), so [`eval_grafana_producer_environment`] -- which
    /// never touches `ExecStart` -- is exactly the right amount of real
    /// evidence to check here.
    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn grafana_base_producer_also_never_used_the_env_interface() {
        let producer_emitted = eval_grafana_producer_environment(GRAFANA_BASE_REV).unwrap();
        assert_eq!(producer_emitted, std::collections::BTreeSet::from(["PATH".to_string()]));
    }
}

#[cfg(test)]
mod ce12_tests {
    use super::*;

    // --- compare_config_contract: offline, pure. Deliberately boring,
    // per C-E1.2a's own stop conditions 5/6 -- one shared function,
    // never branching on `producer`/`consumer` identity. ---

    fn evidence(emitted: &[&str]) -> GeneratedConfigArtifactEvidence {
        GeneratedConfigArtifactEvidence {
            producer: "synth".to_string(),
            format: ConfigFormat::Toml,
            content: ArtifactContent::RenderedText(String::new()),
            binding: ArtifactBindingEvidence::DirectArgv {
                flag: "--config".to_string(),
                argv: "synth --config /x".to_string(),
            },
            emitted_paths: emitted.iter().map(|s| s.to_string()).collect(),
            opaque_paths: Vec::new(),
        }
    }

    fn contract(accepted: &[&str]) -> ConsumerConfigContract {
        ConsumerConfigContract {
            consumer: "synth".to_string(),
            accepted_paths: accepted.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn compare_all_emitted_paths_accepted_is_pass() {
        let e = evidence(&["debug", "server.port"]);
        let c = contract(&["debug", "server.port", "server.host"]);
        assert_eq!(compare_config_contract(&e, &c), ConfigContractVerdict::Pass);
    }

    #[test]
    fn compare_one_unaccepted_emitted_path_is_a_finding() {
        // stop condition 8: mutating a REAL emitted path away from what
        // the consumer actually accepts must flip PASS -> FINDING.
        let e = evidence(&["debug", "server.oldname"]);
        let c = contract(&["debug", "server.port"]);
        assert_eq!(
            compare_config_contract(&e, &c),
            ConfigContractVerdict::Finding { unaccepted_path: "server.oldname".to_string() }
        );
    }

    #[test]
    fn compare_mutating_an_unrelated_accepted_path_leaves_the_result_unchanged() {
        // stop condition 9: a real ACCEPTED path unrelated to anything
        // actually emitted must never change the verdict either way.
        let e = evidence(&["debug"]);
        let c_before = contract(&["debug", "server.port"]);
        let c_after_unrelated_mutation = contract(&["debug", "server.port", "totally.unrelated"]);
        assert_eq!(
            compare_config_contract(&e, &c_before),
            compare_config_contract(&e, &c_after_unrelated_mutation)
        );
    }

    #[test]
    fn compare_empty_emitted_paths_is_always_pass() {
        let e = evidence(&[]);
        let c = contract(&["anything"]);
        assert_eq!(compare_config_contract(&e, &c), ConfigContractVerdict::Pass);
    }

    // --- flatten_structured_value: offline, pure ---

    #[test]
    fn flatten_nested_object_produces_dotted_leaf_paths() {
        let value: JsonValue = serde_json::json!({
            "debug": true,
            "server": { "host": "127.0.0.1", "port": 8080 }
        });
        let mut emitted = Vec::new();
        let mut opaque = Vec::new();
        flatten_structured_value(&value, "", &mut emitted, &mut opaque);
        emitted.sort();
        assert_eq!(emitted, vec!["debug", "server.host", "server.port"]);
        assert!(opaque.is_empty());
    }

    #[test]
    fn flatten_a_list_value_is_opaque_not_indexed_into() {
        // the real unpackerr corpus shape: `radarr = [{...}];`
        let value: JsonValue = serde_json::json!({
            "debug": true,
            "radarr": [ { "api_key": "x", "url": "http://y" } ]
        });
        let mut emitted = Vec::new();
        let mut opaque = Vec::new();
        flatten_structured_value(&value, "", &mut emitted, &mut opaque);
        assert_eq!(emitted, vec!["debug".to_string()]);
        assert_eq!(opaque, vec!["radarr".to_string()]);
    }

    // --- extract_unbound_style_paths: offline, pure, against a
    // synthetic sample matching the REAL rendered shape exactly
    // (fixtures/cdc/generated-config-artifact/unbound/rendered.conf
    // carries the real, nix-built content this was modeled on). ---

    #[test]
    fn extract_unbound_style_single_occurrence_keys_are_emitted() {
        let rendered = "server:\n  port: 5353\n  chroot: \"\"\nremote-control:\n  control-enable: no\n";
        let (emitted, opaque) = extract_unbound_style_paths(rendered);
        assert!(emitted.contains(&"server.port".to_string()));
        assert!(emitted.contains(&"server.chroot".to_string()));
        assert!(emitted.contains(&"remote-control.control-enable".to_string()));
        assert!(opaque.is_empty());
    }

    #[test]
    fn extract_unbound_style_repeated_key_becomes_opaque_not_first_or_last() {
        // the real unbound shape: access-control can appear more than
        // once under `server:`.
        let rendered =
            "server:\n  access-control: 127.0.0.0/8 allow\n  access-control: ::1/128 allow\n";
        let (emitted, opaque) = extract_unbound_style_paths(rendered);
        assert!(!emitted.contains(&"server.access-control".to_string()));
        assert!(opaque.contains(&"server.access-control".to_string()));
    }

    // --- per-consumer extractors: offline, against the REAL vendored
    // fixtures (fetched at each package's own exact pinned version, see
    // fixtures/cdc/generated-config-artifact/<name>/) ---

    fn read_vendored(path: &str) -> String {
        let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        std::fs::read_to_string(&full)
            .unwrap_or_else(|e| panic!("reading vendored fixture {}: {e}", full.display()))
    }

    #[test]
    fn real_vendored_unpackerr_go_struct_yields_the_real_toml_tags() {
        let source = read_vendored("fixtures/cdc/generated-config-artifact/unpackerr/apps.go");
        let tags = extract_go_toml_tags(&source);
        for real in ["debug", "interval", "log_file", "radarr", "sonarr", "webserver"] {
            assert!(tags.iter().any(|t| t == real), "missing real tag {real:?}; got {tags:?}");
        }
    }

    #[test]
    fn real_vendored_unbound_lexer_excerpt_yields_the_real_keywords() {
        let source = read_vendored(
            "fixtures/cdc/generated-config-artifact/unbound/configlexer-excerpt.lex",
        );
        let keywords = extract_unbound_lexer_keywords(&source);
        for real in ["port", "chroot", "access-control", "control-enable", "interface"] {
            assert!(keywords.iter().any(|k| k == real), "missing real keyword {real:?}; got {keywords:?}");
        }
    }

    #[test]
    fn real_vendored_nebula_file_config_yields_the_real_get_config_keys() {
        let source = read_vendored(
            "fixtures/cdc/generated-config-artifact/nebula-lighthouse-service/file_config.py",
        );
        let keys = extract_python_get_config_keys(&source);
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted, vec!["max-port", "min-port", "webserver.ip", "webserver.port"]);
    }

    #[test]
    fn real_vendored_mobilizon_config_exs_yields_the_real_instance_keys() {
        let source = read_vendored("fixtures/cdc/generated-config-artifact/mobilizon/config.exs");
        let keys = extract_elixir_instance_block_keys(&source);
        for real in ["name", "hostname", "registrations_open", "federating"] {
            assert!(keys.iter().any(|k| k == real), "missing real key {real:?}; got {keys:?}");
        }
    }

    // --- C-E1.2b: flatten_structured_value's own 2 new real
    // correctness fixes, offline, pure ---

    #[test]
    fn flatten_a_null_leaf_is_neither_emitted_nor_opaque() {
        // the real i2pd corpus shape: a freeform attrset key left at
        // its own real `null` default is stripped by the module's own
        // `removeNulls` before rendering -- never actually emitted.
        let value: JsonValue = serde_json::json!({ "ipv4": true, "bandwidth": null });
        let mut emitted = Vec::new();
        let mut opaque = Vec::new();
        flatten_structured_value(&value, "", &mut emitted, &mut opaque);
        assert_eq!(emitted, vec!["ipv4".to_string()]);
        assert!(opaque.is_empty());
    }

    #[test]
    fn flatten_an_elixir_type_wrapper_object_is_opaque_not_recursed_into() {
        // the real akkoma corpus shape: `pkgs.formats.elixirConf`'s own
        // shared library wraps `mkAtom`/`mkTuple`/`mkRaw`/... values as
        // `{ _elixirType = "..."; value = ...; }` -- a Nix-level
        // encoding artifact, never genuine nested config structure.
        let value: JsonValue = serde_json::json!({
            "level": { "_elixirType": "atom", "value": ":info" }
        });
        let mut emitted = Vec::new();
        let mut opaque = Vec::new();
        flatten_structured_value(&value, "", &mut emitted, &mut opaque);
        assert!(emitted.is_empty());
        assert_eq!(opaque, vec!["level".to_string()]);
    }

    #[test]
    fn flatten_a_secret_wrapper_object_is_opaque_not_recursed_into() {
        // akkoma's own real per-module `_secret` placeholder convention
        // (not part of the shared elixirConf library, but checked the
        // same structural way -- a marker key shape, not an app-
        // identity check).
        let value: JsonValue = serde_json::json!({
            "secret_key_base": { "_secret": "/var/lib/secrets/akkoma/key-base" }
        });
        let mut emitted = Vec::new();
        let mut opaque = Vec::new();
        flatten_structured_value(&value, "", &mut emitted, &mut opaque);
        assert!(emitted.is_empty());
        assert_eq!(opaque, vec!["secret_key_base".to_string()]);
    }

    // --- C-E1.2b: the 7 new per-consumer D-extractors, offline,
    // against the real vendored fixtures ---

    #[test]
    fn real_vendored_privoxy_hash_table_yields_the_real_directive_names() {
        let source = read_vendored(
            "fixtures/cdc/generated-config-artifact/privoxy/loadcfg-hashtable-excerpt.c",
        );
        let names = extract_privoxy_hash_table_names(&source);
        for real in ["actionsfile", "listen-address", "enable-edit-actions", "buffer-limit"] {
            assert!(names.iter().any(|n| n == real), "missing real name {real:?}; got {names:?}");
        }
    }

    #[test]
    fn real_vendored_misskey_config_ts_yields_the_real_source_and_redis_paths() {
        let source = read_vendored("fixtures/cdc/generated-config-artifact/misskey/config.ts");
        let paths = extract_misskey_source_paths(&source);
        for real in ["url", "port", "db.host", "db.port", "redis.host", "redis.port", "redisForPubsub.host"] {
            assert!(paths.iter().any(|p| p == real), "missing real path {real:?}; got {paths:?}");
        }
    }

    #[test]
    fn real_vendored_kavita_configuration_yields_the_real_appsettings_and_oidc_paths() {
        let source =
            read_vendored("fixtures/cdc/generated-config-artifact/kavita/configuration-excerpt.cs");
        let paths = extract_kavita_appsettings_paths(&source);
        for real in ["TokenKey", "Port", "IpAddresses", "OpenIdConnectSettings.Authority", "OpenIdConnectSettings.ClientId"] {
            assert!(paths.iter().any(|p| p == real), "missing real path {real:?}; got {paths:?}");
        }
    }

    #[test]
    fn real_vendored_transmission_quark_excerpt_yields_the_real_kebab_keys() {
        let source = read_vendored(
            "fixtures/cdc/generated-config-artifact/transmission/quark-kebab-excerpt.cc",
        );
        let names = extract_transmission_kebab_quarks(&source);
        for real in ["peer-port", "rpc-bind-address", "watch-dir", "download-dir"] {
            assert!(names.iter().any(|n| n == real), "missing real name {real:?}; got {names:?}");
        }
    }

    #[test]
    fn real_vendored_i2pd_config_cpp_yields_the_real_dotted_keys() {
        let source = read_vendored("fixtures/cdc/generated-config-artifact/i2pd/Config.cpp");
        let names = extract_i2pd_program_options_keys(&source);
        for real in ["http.enabled", "httpproxy.enabled", "bob.enabled", "ipv4", "ipv6"] {
            assert!(names.iter().any(|n| n == real), "missing real name {real:?}; got {names:?}");
        }
    }

    #[test]
    fn real_vendored_spacecookie_config_hs_yields_the_real_dotted_paths() {
        let source = read_vendored("fixtures/cdc/generated-config-artifact/spacecookie/Config.hs");
        let paths = extract_spacecookie_config_paths(&source);
        for real in ["hostname", "listen.addr", "listen.port", "port", "user", "root", "log.enable", "log.hide-ips"] {
            assert!(paths.iter().any(|p| p == real), "missing real path {real:?}; got {paths:?}");
        }
    }

    #[test]
    fn real_vendored_akkoma_description_excerpt_yields_the_real_fully_qualified_paths() {
        let source = read_vendored(
            "fixtures/cdc/generated-config-artifact/akkoma/description-excerpt.exs",
        );
        let paths = extract_akkoma_description_paths(&source);
        for real in [
            ":pleroma.:instance.name",
            ":pleroma.:instance.email",
            ":pleroma.Pleroma.Upload.base_url",
            ":pleroma.:media_proxy.enabled",
        ] {
            assert!(paths.iter().any(|p| p == real), "missing real path {real:?}; got {paths:?}");
        }
    }

    #[test]
    fn real_vendored_akkoma_welcome_record_does_not_collapse_its_two_real_enabled_fields() {
        // the exact adversarial shape this whole fix exists for: a
        // naive flat scan would merge `:welcome.direct_message.enabled`
        // and `:welcome.email.enabled` into one bare `:welcome.enabled`.
        // This v1's own bounded design instead stops one level short --
        // `:welcome.direct_message`/`:welcome.email` are their own
        // single leaf paths, never expanded into a colliding `enabled`.
        let source = read_vendored(
            "fixtures/cdc/generated-config-artifact/akkoma/description-excerpt.exs",
        );
        let paths = extract_akkoma_description_paths(&source);
        assert!(paths.iter().any(|p| p == ":pleroma.:welcome.direct_message"));
        assert!(paths.iter().any(|p| p == ":pleroma.:welcome.email"));
        assert!(!paths.iter().any(|p| p == ":pleroma.:welcome.enabled"));
    }

    // --- C-E1.2b's own real adversarial-normalization invariant,
    // turned into a permanent, checked test (not just a research
    // finding) -- the SAME bare-key-collision shape resolved in the
    // OPPOSITE direction for three real, different consumers,
    // confirming the correct answer is per-consumer semantic work,
    // never a safe default either way. ---

    #[test]
    fn normalization_is_per_consumer_never_a_default_unbound_vs_i2pd_vs_akkoma() {
        // unbound: bare-key stripping is SOUND (its D-extractor,
        // `extract_unbound_lexer_keywords`, only ever returns bare
        // names -- confirmed via the real grammar recheck that its two
        // real clauses share zero tokens, see census.md).
        let unbound_accepted = extract_unbound_lexer_keywords(&read_vendored(
            "fixtures/cdc/generated-config-artifact/unbound/configlexer-excerpt.lex",
        ));
        assert!(unbound_accepted.iter().any(|k| k == "port"));
        assert!(!unbound_accepted.iter().any(|k| k.contains('.')));

        // i2pd: bare-key stripping would be UNSOUND -- its own real
        // consumer source already registers fully-dotted keys, and the
        // bare name "enabled" genuinely collides across 7 real
        // sections. The correct D-extraction keeps the dots.
        let i2pd_accepted = extract_i2pd_program_options_keys(&read_vendored(
            "fixtures/cdc/generated-config-artifact/i2pd/Config.cpp",
        ));
        assert!(i2pd_accepted.iter().any(|k| k == "http.enabled"));
        assert!(i2pd_accepted.iter().any(|k| k == "bob.enabled"));
        assert!(!i2pd_accepted.iter().any(|k| k == "enabled"));

        // akkoma: bare-key stripping would ALSO be unsound (18-way
        // collision) -- confirming this is real per-consumer semantic
        // work, not something INI-vs-line-format alone predicts
        // (i2pd's own real format is INI-flavored, akkoma's is
        // Elixir's `Config` DSL -- two structurally different real
        // formats reaching the identical "don't strip" conclusion).
        let akkoma_accepted = extract_akkoma_description_paths(&read_vendored(
            "fixtures/cdc/generated-config-artifact/akkoma/description-excerpt.exs",
        ));
        assert!(akkoma_accepted.iter().any(|p| p == ":pleroma.:media_proxy.enabled"));
        assert!(!akkoma_accepted.iter().any(|p| p == "enabled"));
    }

    // --- C-E1.2a real end-to-end anchor proofs: needs a real `nix`
    // binary + network access (`fetchTarball`), same discipline as the
    // pre-existing `#[ignore]` tests above. Each anchor proves the FULL
    // real pipeline (acquire -> compare_config_contract) through the
    // exact SAME shared `compare_config_contract` (stop conditions 5/6:
    // one function, zero `if app == ...` branching anywhere in it), for
    // a genuinely different real language/format/binding shape -- see
    // this module's own header doc comment for which. Each anchor also
    // gets its own stop-condition-8 (mutate a REAL emitted path -> must
    // flip PASS -> FINDING) and stop-condition-9 (mutate an UNRELATED
    // accepted path -> comparison must not change) proof against REAL
    // acquired data, not just the synthetic `evidence()`/`contract()`
    // helpers already covering the same two conditions abstractly above. ---

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_unpackerr_end_to_end_is_a_clean_pass() {
        let evidence = acquire_unpackerr_evidence(CE12_REV).unwrap();
        let contract = unpackerr_consumer_contract().unwrap();
        assert_eq!(compare_config_contract(&evidence, &contract), ConfigContractVerdict::Pass);
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_unpackerr_mutating_a_real_emitted_path_flips_to_finding() {
        let mut evidence = acquire_unpackerr_evidence(CE12_REV).unwrap();
        let contract = unpackerr_consumer_contract().unwrap();
        assert!(evidence.emitted_paths.contains(&"debug".to_string()));
        for p in evidence.emitted_paths.iter_mut() {
            if p == "debug" {
                *p = "debug-renamed-to-something-nobody-accepts".to_string();
            }
        }
        assert_eq!(
            compare_config_contract(&evidence, &contract),
            ConfigContractVerdict::Finding {
                unaccepted_path: "debug-renamed-to-something-nobody-accepts".to_string()
            }
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_unpackerr_mutating_an_unrelated_accepted_path_leaves_the_result_unchanged() {
        let evidence = acquire_unpackerr_evidence(CE12_REV).unwrap();
        let contract_before = unpackerr_consumer_contract().unwrap();
        let mut contract_after = contract_before.clone();
        contract_after.accepted_paths.push("totally-unrelated-key".to_string());
        assert_eq!(
            compare_config_contract(&evidence, &contract_before),
            compare_config_contract(&evidence, &contract_after)
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_unbound_end_to_end_is_a_clean_pass() {
        let evidence = acquire_unbound_evidence(CE12_REV).unwrap();
        let contract = unbound_consumer_contract().unwrap();
        assert_eq!(compare_config_contract(&evidence, &contract), ConfigContractVerdict::Pass);
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_unbound_mutating_a_real_emitted_path_flips_to_finding() {
        let mut evidence = acquire_unbound_evidence(CE12_REV).unwrap();
        let contract = unbound_consumer_contract().unwrap();
        assert!(!evidence.emitted_paths.is_empty());
        let real_path = evidence.emitted_paths[0].clone();
        evidence.emitted_paths[0] = format!("{real_path}-renamed-to-something-nobody-accepts");
        assert_eq!(
            compare_config_contract(&evidence, &contract),
            ConfigContractVerdict::Finding {
                unaccepted_path: format!("{real_path}-renamed-to-something-nobody-accepts")
            }
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_unbound_mutating_an_unrelated_accepted_path_leaves_the_result_unchanged() {
        let evidence = acquire_unbound_evidence(CE12_REV).unwrap();
        let contract_before = unbound_consumer_contract().unwrap();
        let mut contract_after = contract_before.clone();
        contract_after.accepted_paths.push("totally-unrelated-key".to_string());
        assert_eq!(
            compare_config_contract(&evidence, &contract_before),
            compare_config_contract(&evidence, &contract_after)
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_mobilizon_end_to_end_is_a_clean_pass() {
        let evidence = acquire_mobilizon_evidence(CE12_REV).unwrap();
        let contract = mobilizon_consumer_contract().unwrap();
        assert_eq!(compare_config_contract(&evidence, &contract), ConfigContractVerdict::Pass);
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_mobilizon_mutating_a_real_emitted_path_flips_to_finding() {
        let mut evidence = acquire_mobilizon_evidence(CE12_REV).unwrap();
        let contract = mobilizon_consumer_contract().unwrap();
        assert!(evidence.emitted_paths.contains(&"hostname".to_string()));
        for p in evidence.emitted_paths.iter_mut() {
            if p == "hostname" {
                *p = "hostname-renamed-to-something-nobody-accepts".to_string();
            }
        }
        assert_eq!(
            compare_config_contract(&evidence, &contract),
            ConfigContractVerdict::Finding {
                unaccepted_path: "hostname-renamed-to-something-nobody-accepts".to_string()
            }
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_mobilizon_mutating_an_unrelated_accepted_path_leaves_the_result_unchanged() {
        let evidence = acquire_mobilizon_evidence(CE12_REV).unwrap();
        let contract_before = mobilizon_consumer_contract().unwrap();
        let mut contract_after = contract_before.clone();
        contract_after.accepted_paths.push("totally-unrelated-key".to_string());
        assert_eq!(
            compare_config_contract(&evidence, &contract_before),
            compare_config_contract(&evidence, &contract_after)
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_nebula_lighthouse_service_end_to_end_is_a_clean_pass() {
        let evidence = acquire_nebula_lighthouse_service_evidence(CE12_REV).unwrap();
        let contract = nebula_lighthouse_service_consumer_contract().unwrap();
        assert_eq!(compare_config_contract(&evidence, &contract), ConfigContractVerdict::Pass);
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_nebula_lighthouse_service_mutating_a_real_emitted_path_flips_to_finding() {
        let mut evidence = acquire_nebula_lighthouse_service_evidence(CE12_REV).unwrap();
        let contract = nebula_lighthouse_service_consumer_contract().unwrap();
        assert!(evidence.emitted_paths.contains(&"min-port".to_string()));
        for p in evidence.emitted_paths.iter_mut() {
            if p == "min-port" {
                *p = "min-port-renamed-to-something-nobody-accepts".to_string();
            }
        }
        assert_eq!(
            compare_config_contract(&evidence, &contract),
            ConfigContractVerdict::Finding {
                unaccepted_path: "min-port-renamed-to-something-nobody-accepts".to_string()
            }
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_nebula_lighthouse_service_mutating_an_unrelated_accepted_path_leaves_the_result_unchanged(
    ) {
        let evidence = acquire_nebula_lighthouse_service_evidence(CE12_REV).unwrap();
        let contract_before = nebula_lighthouse_service_consumer_contract().unwrap();
        let mut contract_after = contract_before.clone();
        contract_after.accepted_paths.push("totally-unrelated-key".to_string());
        assert_eq!(
            compare_config_contract(&evidence, &contract_before),
            compare_config_contract(&evidence, &contract_after)
        );
    }

    // --- C-E1.2b real end-to-end anchor proofs: the 7 candidates the
    // transfer census verdicted PASS/FINDING, same real discipline as
    // the 4 C-E1.2a anchors above (clean pass + stop-condition-8
    // mutation + stop-condition-9 mutation, all against real acquired
    // data). `akkoma`'s own "clean pass" is real per this v1's own
    // bounded fully-qualified extraction -- consistent with the
    // census's own note that a full qualified extractor might not
    // reproduce the FINDING verdict, which is fine: 2b was a fit
    // census, not a verdict pre-commitment. `vault` is deliberately
    // absent -- see this file's own C-E1.2b module header comment. ---

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_privoxy_end_to_end_is_a_clean_pass() {
        let evidence = acquire_privoxy_evidence(CE12_REV).unwrap();
        let contract = privoxy_consumer_contract().unwrap();
        assert_eq!(compare_config_contract(&evidence, &contract), ConfigContractVerdict::Pass);
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_privoxy_mutating_a_real_emitted_path_flips_to_finding() {
        let mut evidence = acquire_privoxy_evidence(CE12_REV).unwrap();
        let contract = privoxy_consumer_contract().unwrap();
        assert!(evidence.emitted_paths.contains(&"listen-address".to_string()));
        for p in evidence.emitted_paths.iter_mut() {
            if p == "listen-address" {
                *p = "listen-address-renamed-to-something-nobody-accepts".to_string();
            }
        }
        assert_eq!(
            compare_config_contract(&evidence, &contract),
            ConfigContractVerdict::Finding {
                unaccepted_path: "listen-address-renamed-to-something-nobody-accepts".to_string()
            }
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_privoxy_mutating_an_unrelated_accepted_path_leaves_the_result_unchanged() {
        let evidence = acquire_privoxy_evidence(CE12_REV).unwrap();
        let contract_before = privoxy_consumer_contract().unwrap();
        let mut contract_after = contract_before.clone();
        contract_after.accepted_paths.push("totally-unrelated-key".to_string());
        assert_eq!(
            compare_config_contract(&evidence, &contract_before),
            compare_config_contract(&evidence, &contract_after)
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_misskey_end_to_end_is_a_clean_pass() {
        let evidence = acquire_misskey_evidence(CE12_REV).unwrap();
        let contract = misskey_consumer_contract().unwrap();
        assert_eq!(compare_config_contract(&evidence, &contract), ConfigContractVerdict::Pass);
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_misskey_mutating_a_real_emitted_path_flips_to_finding() {
        let mut evidence = acquire_misskey_evidence(CE12_REV).unwrap();
        let contract = misskey_consumer_contract().unwrap();
        assert!(evidence.emitted_paths.contains(&"url".to_string()));
        for p in evidence.emitted_paths.iter_mut() {
            if p == "url" {
                *p = "url-renamed-to-something-nobody-accepts".to_string();
            }
        }
        assert_eq!(
            compare_config_contract(&evidence, &contract),
            ConfigContractVerdict::Finding {
                unaccepted_path: "url-renamed-to-something-nobody-accepts".to_string()
            }
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_misskey_mutating_an_unrelated_accepted_path_leaves_the_result_unchanged() {
        let evidence = acquire_misskey_evidence(CE12_REV).unwrap();
        let contract_before = misskey_consumer_contract().unwrap();
        let mut contract_after = contract_before.clone();
        contract_after.accepted_paths.push("totally-unrelated-key".to_string());
        assert_eq!(
            compare_config_contract(&evidence, &contract_before),
            compare_config_contract(&evidence, &contract_after)
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_kavita_end_to_end_is_a_clean_pass() {
        let evidence = acquire_kavita_evidence(CE12_REV).unwrap();
        let contract = kavita_consumer_contract().unwrap();
        assert_eq!(compare_config_contract(&evidence, &contract), ConfigContractVerdict::Pass);
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_kavita_mutating_a_real_emitted_path_flips_to_finding() {
        let mut evidence = acquire_kavita_evidence(CE12_REV).unwrap();
        let contract = kavita_consumer_contract().unwrap();
        assert!(evidence.emitted_paths.contains(&"Port".to_string()));
        for p in evidence.emitted_paths.iter_mut() {
            if p == "Port" {
                *p = "Port-renamed-to-something-nobody-accepts".to_string();
            }
        }
        assert_eq!(
            compare_config_contract(&evidence, &contract),
            ConfigContractVerdict::Finding {
                unaccepted_path: "Port-renamed-to-something-nobody-accepts".to_string()
            }
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_kavita_mutating_an_unrelated_accepted_path_leaves_the_result_unchanged() {
        let evidence = acquire_kavita_evidence(CE12_REV).unwrap();
        let contract_before = kavita_consumer_contract().unwrap();
        let mut contract_after = contract_before.clone();
        contract_after.accepted_paths.push("totally-unrelated-key".to_string());
        assert_eq!(
            compare_config_contract(&evidence, &contract_before),
            compare_config_contract(&evidence, &contract_after)
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_transmission_end_to_end_is_a_clean_pass() {
        let evidence = acquire_transmission_evidence(CE12_REV).unwrap();
        let contract = transmission_consumer_contract().unwrap();
        assert_eq!(compare_config_contract(&evidence, &contract), ConfigContractVerdict::Pass);
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_transmission_mutating_a_real_emitted_path_flips_to_finding() {
        let mut evidence = acquire_transmission_evidence(CE12_REV).unwrap();
        let contract = transmission_consumer_contract().unwrap();
        assert!(evidence.emitted_paths.contains(&"peer-port".to_string()));
        for p in evidence.emitted_paths.iter_mut() {
            if p == "peer-port" {
                *p = "peer-port-renamed-to-something-nobody-accepts".to_string();
            }
        }
        assert_eq!(
            compare_config_contract(&evidence, &contract),
            ConfigContractVerdict::Finding {
                unaccepted_path: "peer-port-renamed-to-something-nobody-accepts".to_string()
            }
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_transmission_mutating_an_unrelated_accepted_path_leaves_the_result_unchanged() {
        let evidence = acquire_transmission_evidence(CE12_REV).unwrap();
        let contract_before = transmission_consumer_contract().unwrap();
        let mut contract_after = contract_before.clone();
        contract_after.accepted_paths.push("totally-unrelated-key".to_string());
        assert_eq!(
            compare_config_contract(&evidence, &contract_before),
            compare_config_contract(&evidence, &contract_after)
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_i2pd_end_to_end_is_a_clean_pass() {
        let evidence = acquire_i2pd_evidence(CE12_REV).unwrap();
        let contract = i2pd_consumer_contract().unwrap();
        assert_eq!(compare_config_contract(&evidence, &contract), ConfigContractVerdict::Pass);
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_i2pd_mutating_a_real_emitted_path_flips_to_finding() {
        let mut evidence = acquire_i2pd_evidence(CE12_REV).unwrap();
        let contract = i2pd_consumer_contract().unwrap();
        assert!(evidence.emitted_paths.contains(&"http.enabled".to_string()));
        for p in evidence.emitted_paths.iter_mut() {
            if p == "http.enabled" {
                *p = "http.enabled-renamed-to-something-nobody-accepts".to_string();
            }
        }
        assert_eq!(
            compare_config_contract(&evidence, &contract),
            ConfigContractVerdict::Finding {
                unaccepted_path: "http.enabled-renamed-to-something-nobody-accepts".to_string()
            }
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_i2pd_mutating_an_unrelated_accepted_path_leaves_the_result_unchanged() {
        let evidence = acquire_i2pd_evidence(CE12_REV).unwrap();
        let contract_before = i2pd_consumer_contract().unwrap();
        let mut contract_after = contract_before.clone();
        contract_after.accepted_paths.push("totally-unrelated-key".to_string());
        assert_eq!(
            compare_config_contract(&evidence, &contract_before),
            compare_config_contract(&evidence, &contract_after)
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_spacecookie_end_to_end_is_a_clean_pass() {
        let evidence = acquire_spacecookie_evidence(CE12_REV).unwrap();
        let contract = spacecookie_consumer_contract().unwrap();
        assert_eq!(compare_config_contract(&evidence, &contract), ConfigContractVerdict::Pass);
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_spacecookie_mutating_a_real_emitted_path_flips_to_finding() {
        let mut evidence = acquire_spacecookie_evidence(CE12_REV).unwrap();
        let contract = spacecookie_consumer_contract().unwrap();
        assert!(evidence.emitted_paths.contains(&"hostname".to_string()));
        for p in evidence.emitted_paths.iter_mut() {
            if p == "hostname" {
                *p = "hostname-renamed-to-something-nobody-accepts".to_string();
            }
        }
        assert_eq!(
            compare_config_contract(&evidence, &contract),
            ConfigContractVerdict::Finding {
                unaccepted_path: "hostname-renamed-to-something-nobody-accepts".to_string()
            }
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_spacecookie_mutating_an_unrelated_accepted_path_leaves_the_result_unchanged() {
        let evidence = acquire_spacecookie_evidence(CE12_REV).unwrap();
        let contract_before = spacecookie_consumer_contract().unwrap();
        let mut contract_after = contract_before.clone();
        contract_after.accepted_paths.push("totally-unrelated-key".to_string());
        assert_eq!(
            compare_config_contract(&evidence, &contract_before),
            compare_config_contract(&evidence, &contract_after)
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_akkoma_end_to_end_is_a_real_finding_not_a_forced_pass() {
        // Genuine, executable-verified result, NOT the outcome this
        // test was originally written expecting: a default-configured
        // akkoma module's own real `:instance` block ALWAYS also
        // carries a module-INJECTED `upload_dir` field (a computed
        // state-directory path, confirmed present regardless of this
        // acquire function's own config) that `description.exs` itself
        // genuinely never documents anywhere (confirmed directly --
        // zero `key: :upload_dir` occurrences in the real file; `:joken`/
        // `:tzdata` have the identical zero-coverage shape for their
        // own entire groups, see `acquire_akkoma_evidence`'s own doc
        // comment). The census's own protocol explicitly allowed this
        // outcome ("if akkoma stops being a finding after a real
        // qualified extractor, that's fine too -- 2b was a fit census,
        // not a verdict pre-commitment") -- here executable evidence
        // decided the opposite direction just as legitimately: this
        // candidate's real default config does NOT cleanly pass against
        // `description.exs`'s own real, honestly-bounded schema.
        let evidence = acquire_akkoma_evidence(CE12_REV).unwrap();
        let contract = akkoma_consumer_contract().unwrap();
        assert_eq!(
            compare_config_contract(&evidence, &contract),
            ConfigContractVerdict::Finding {
                unaccepted_path: ":pleroma.:instance.upload_dir".to_string()
            }
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_akkoma_mutating_a_real_emitted_path_flips_to_a_different_finding() {
        // stop condition 8, isolated to exactly ONE real, known-
        // accepted path (`Pleroma.Upload.base_url`) rather than the
        // full real acquired evidence -- a default-configured akkoma
        // module injects several more real fields beyond what
        // `description.exs` documents (`upload_dir` above is one;
        // `Pleroma.Repo.*` connection settings are another), and
        // `compare_config_contract` reports only the FIRST unaccepted
        // path it finds. Isolating to a single, definitely-accepted
        // real path (confirmed by the clean baseline `Pass` assertion
        // below) is what makes mutating it provably the CAUSE of its
        // own new Finding, without this test depending on JSON
        // iteration order across several unrelated real gaps.
        let mut evidence = acquire_akkoma_evidence(CE12_REV).unwrap();
        let real_path = ":pleroma.Pleroma.Upload.base_url".to_string();
        assert!(evidence.emitted_paths.contains(&real_path));
        evidence.emitted_paths.retain(|p| p == &real_path);
        let contract = akkoma_consumer_contract().unwrap();
        assert_eq!(compare_config_contract(&evidence, &contract), ConfigContractVerdict::Pass);
        for p in evidence.emitted_paths.iter_mut() {
            *p = format!("{real_path}-renamed-to-something-nobody-accepts");
        }
        assert_eq!(
            compare_config_contract(&evidence, &contract),
            ConfigContractVerdict::Finding {
                unaccepted_path: format!("{real_path}-renamed-to-something-nobody-accepts")
            }
        );
    }

    #[test]
    #[ignore = "needs a real `nix` binary and network access (fetchTarball)"]
    fn real_akkoma_mutating_an_unrelated_accepted_path_leaves_the_result_unchanged() {
        let evidence = acquire_akkoma_evidence(CE12_REV).unwrap();
        let contract_before = akkoma_consumer_contract().unwrap();
        let mut contract_after = contract_before.clone();
        contract_after.accepted_paths.push("totally-unrelated-key".to_string());
        assert_eq!(
            compare_config_contract(&evidence, &contract_before),
            compare_config_contract(&evidence, &contract_after)
        );
    }

    // --- stop condition 10: old K1-K5/OBA results must not change.
    // Enforced by the pre-existing full suites in `mod tests`/
    // `mod k4c_tests`/`mod k5c_tests` above, all still present and
    // unmodified by this module -- no dedicated test needed here beyond
    // running the full suite, done as part of closing this round. ---
}
