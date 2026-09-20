//! oba — Option Branch Activation evidence for NixOS modules.
//!
//! Layer 1 only, per the explicit non-goals agreed on:
//!   - This answers "did a test ever drive this branch's predicate to the
//!     *opposite* boolean outcome from what the option's own default
//!     produces", NOT "does the branch's value reach a consumer with the
//!     right name" (that is CDC, a separate tool) and NOT "does the value
//!     actually change observed runtime behavior" (that is ROB — the
//!     pdo_mysql.default_socket confound from the session this tool grew
//!     out of is exactly why OBA PASS must never be read as "works
//!     end-to-end").
//!   - No VM tests are run. No upstream consumer is consulted. No Nix
//!     evaluation happens — `ValueClass` is a syntax-level classifier, not
//!     an interpreter. The tool doesn't know what Doctrine is.
//!   - cfg-alias resolution (`foo = cfg.x; if foo != null then ...`) is out
//!     of scope for this MVP and reported as PredicateNotFound rather than
//!     silently skipped, so a target with an aliased predicate (davis in
//!     this corpus) doesn't masquerade as a clean PASS.
//!
//! Every watched option's verdict comes from a hard 6-step gate chain —
//! see `run_target` — where any gate failing short-circuits to an
//! inconclusive verdict rather than falling through to a guess:
//! declaration found → predicate found → default's ValueClass resolves to
//! a predicate outcome → known opposite-outcome test evidence (wins
//! outright) → no unseen test-config region could hide the option
//! (`imports`/alias/function call — see the TestSpecRoot/ModuleRoot/
//! ConfigTree walker below) → no matching assignment's own value is
//! itself unresolvable. Exit code is 4-state: 0 clean / 1 FINDING / 2
//! INCONCLUSIVE (takes precedence over FINDING) / 3 TOOL_ERROR (the
//! analysis never ran at all — bad manifest, missing files, a bad CLI
//! flag — distinct from INCONCLUSIVE, where it ran but couldn't prove
//! something).

use rnix::SyntaxKind::*;
use rnix::{SyntaxNode, SyntaxToken};
use rowan::ast::AstNode;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;

/// K1: a separate, experimental Contract Drift Checker (CDC) spike --
/// answers a genuinely different question from OBA ("does the external
/// output contract match a real consumer", not "did a test exercise this
/// option"). Deliberately NOT wired into this binary's CLI/exit-code
/// contract or the `check`/`diff` report envelope -- see the module's own
/// doc comment. Exercised only via `cargo test` (offline unit tests) and
/// `cargo test --ignored -- cdc::` (the real historical/mutation proof,
/// needs `nix` + network).
#[allow(dead_code)]
mod cdc;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Deprecated legacy invocation, kept working for the real `v0.1.0`
    /// public release that shipped before `check`/`--root` existed:
    /// equivalent to `check --root . --targets <TARGETS>`, sharing that
    /// exact code path (not a separately-maintained approximation of it).
    /// Path to a TOML target manifest (see targets/*.toml). Required
    /// unless --census or a subcommand is given.
    #[arg(long, conflicts_with = "census")]
    targets: Option<PathBuf>,
    /// Emit machine-readable JSON instead of the human report.
    #[arg(long)]
    json: bool,
    /// Syntax-visibility census, not option-branch analysis: recursively
    /// walks every `.nix` file under this directory as a *test-file* scan
    /// (the same TestSpecRoot/ModuleRoot/ConfigTree walker `run_target`
    /// uses, run standalone) and reports which root/nodes/module-root
    /// forms it recognized vs. gave up on opaquely. A cheap rnix-only
    /// pass -- no Nix evaluation, no VM, no option/predicate matching.
    /// Exists to answer one narrow question before Layer 1 is considered
    /// settled: does "no assignment found" mean "genuinely not there" or
    /// "the walker couldn't see this file's shape at all" more often than
    /// expected, across a real corpus rather than a handful of fixtures.
    /// Mutually exclusive with --targets (H1.3a: previously only enforced
    /// by `run()` silently preferring --census and ignoring --targets when
    /// both were given -- clap now rejects the combination outright). Not
    /// folded into the `check` subcommand in this PR -- deliberately not
    /// touching --census's shape here at all.
    #[arg(long, conflicts_with = "targets")]
    census: Option<PathBuf>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Run option-branch analysis over one root and target manifest.
    Check(CheckArgs),
    /// Compare option-branch analysis between two roots under one
    /// manifest.
    Diff(DiffArgs),
    /// P3a: run OBA (option-branch analysis) and/or CDC
    /// (`GeneratedConfigArtifact` contract checks) over one root and one
    /// manifest, unified under one finding schema. Read-only, exactly
    /// like `check` -- no new analysis logic, this wires the two
    /// already-frozen engines (OBA's own `analyze()`, CDC's own
    /// `cdc::run_cdc_candidate`) into one command.
    Audit(AuditArgs),
    /// P3b: compare `audit`'s own unified result between two roots --
    /// the same real D2 "two directories, no Git inside core" philosophy
    /// `oba diff` already established, extended to cover CDC's own
    /// results too.
    AuditDiff(AuditDiffArgs),
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum AuditFormat {
    Text,
    Json,
}

#[derive(clap::Args)]
struct CheckArgs {
    /// Filesystem root every target's `module`/`test` path is resolved
    /// against. A real security boundary, not just a `PathBuf::join`: an
    /// absolute `module`/`test`, a `../` escape, or a symlink that
    /// resolves outside `root` after canonicalization all become a
    /// TOOL_ERROR (exit 3) rather than a read from wherever they happen to
    /// point. This matters the moment `--targets` can name a manifest that
    /// lives in the very repository being analyzed (an untrusted PR could
    /// otherwise point a target at, say, `/etc/shadow`).
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Path to a TOML target manifest, resolved relative to the current
    /// directory (NOT to `--root`) -- deliberately the odd one out, so a
    /// manifest checked out once can later be pointed at two different
    /// roots (PR D: a base root and a head root) without "relative to
    /// which root?" ever being an ambiguous question for the manifest
    /// path itself. Every `module`/`test` *inside* the manifest, by
    /// contrast, is always root-relative -- see `root` above.
    #[arg(long)]
    targets: PathBuf,
    /// Emit machine-readable JSON (the versioned `schema_version: 1`
    /// envelope) instead of the human report.
    #[arg(long)]
    json: bool,
}

#[derive(clap::Args)]
struct DiffArgs {
    /// Filesystem root analyzed as "base" -- same security boundary as
    /// `check --root` (see its own doc comment): an escaping
    /// `module`/`test` is a TOOL_ERROR, not a read from wherever it
    /// points. This is plumbing reuse, not a second implementation --
    /// both roots go through the exact same `analyze()`.
    #[arg(long)]
    base_root: PathBuf,
    /// Filesystem root analyzed as "head". Same boundary as `base_root`.
    #[arg(long)]
    head_root: PathBuf,
    /// Path to a TOML target manifest, resolved relative to the current
    /// directory. Deliberately ONE manifest applied to both roots, not
    /// `--base-targets`/`--head-targets` -- comparing two different
    /// *specifications* of what to watch is a different, messier
    /// question (did the target change, or did what we're even looking
    /// at change) that this tool isn't trying to answer. `module`/`test`
    /// inside the manifest are root-relative, same as `check`.
    #[arg(long)]
    targets: PathBuf,
    /// Emit machine-readable JSON (`mode: "diff"` under the same
    /// `schema_version: 1` envelope family) instead of the human report.
    #[arg(long)]
    json: bool,
}

#[derive(clap::Args)]
struct AuditArgs {
    /// Filesystem root BOTH engines resolve against: OBA's own
    /// `module`/`test` paths (same real security boundary as `check
    /// --root`, see its own doc comment) AND CDC's own nixpkgs tree
    /// (`cdc::NixpkgsSource::LocalPath`, so `oba audit --root
    /// ./my-nixpkgs-checkout` can check a REAL local checkout, not just
    /// this project's own internally-pinned commits).
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Path to a TOML target manifest, resolved relative to the current
    /// directory (same convention as `check`/`diff`). May contain
    /// `[[target]]` entries (OBA), `[[cdc_target]]` entries (CDC), or
    /// both -- `validate_manifest` requires at least one of either, never
    /// both empty.
    #[arg(long)]
    targets: PathBuf,
    /// `text` (default, human-readable) or `json` (the versioned
    /// `schema_version: 1`, `mode: "audit"` envelope).
    #[arg(long, value_enum, default_value_t = AuditFormat::Text)]
    format: AuditFormat,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct TargetFile {
    #[serde(default)]
    target: Vec<Target>,
    /// P3a: `oba audit`'s own real CDC surface -- a target here is
    /// nothing but a NAME selecting one of `cdc::CDC_CANDIDATE_NAMES`'s
    /// own already-frozen-v1 candidates (validated in
    /// `validate_manifest`), never a place to describe a NEW analysis
    /// (that would be reopening `GeneratedConfigArtifact` v1's own
    /// freeze, C-E1.2c, not wiring the existing engine into a CLI).
    /// Absent/empty in every pre-existing manifest -- `check`/`diff`
    /// never read this field, only `audit` does.
    #[serde(default)]
    cdc_target: Vec<CdcTargetSpec>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct CdcTargetSpec {
    name: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct Target {
    name: String,
    module: PathBuf,
    test: PathBuf,
    /// Identifier the module uses for the per-instance config, e.g. "cfg".
    cfg_ident: String,
    /// Absolute dotted path prefix used in TEST files, "*" = one wildcard
    /// segment (an attrsOf-submodule instance name). e.g.
    /// ["services","kimai","sites","*"]
    option_prefix: Vec<String>,
    /// Option paths (relative to cfg_ident, dot-joined, e.g. "database.socket")
    /// this target is specifically vouching for. Every entry gets an explicit
    /// verdict -- PredicateNotFound if the scanner found no direct branch for
    /// it, so an aliased or missed predicate can never just be silently
    /// absent from the report and mistaken for "nothing to see here".
    /// Required and validated non-empty by `validate_manifest` -- a target
    /// with nothing to watch would scan everything and assert nothing,
    /// exiting 0 with zero findings and zero inconclusive: the exact same
    /// "detector died, green light stayed on" failure mode this field
    /// exists to prevent in the first place, just one level up the stack.
    watch: Vec<String>,
}

fn validate_manifest(m: &TargetFile) -> Result<(), String> {
    // P3a: a manifest with real `cdc_target` entries and zero `[[target]]`
    // ones is legitimate for `oba audit` (a CDC-only audit) -- the
    // original "detector died, silently green" concern this check
    // exists for is about a manifest with NOTHING to watch at all, not
    // specifically about OBA's own `[[target]]` array.
    if m.target.is_empty() && m.cdc_target.is_empty() {
        return Err("manifest has no [[target]] or [[cdc_target]] entries".to_string());
    }
    {
        let mut seen_cdc_names = std::collections::HashSet::new();
        for t in &m.cdc_target {
            if t.name.trim().is_empty() {
                return Err("a [[cdc_target]] has an empty name".to_string());
            }
            if !seen_cdc_names.insert(t.name.as_str()) {
                return Err(format!("duplicate [[cdc_target]] name: {}", t.name));
            }
            if !cdc::CDC_CANDIDATE_NAMES.contains(&t.name.as_str()) {
                return Err(format!(
                    "[[cdc_target]] {:?} is not a known GeneratedConfigArtifact candidate -- known candidates: {:?} (adding a NEW candidate means reopening the frozen v1 engine in src/cdc.rs, not editing a manifest)",
                    t.name,
                    cdc::CDC_CANDIDATE_NAMES
                ));
            }
        }
    }
    let mut seen_names = std::collections::HashSet::new();
    for t in &m.target {
        if t.name.trim().is_empty() {
            return Err("a target has an empty name".to_string());
        }
        if !seen_names.insert(t.name.as_str()) {
            return Err(format!("duplicate target name: {}", t.name));
        }
        if !is_valid_ident(&t.cfg_ident) {
            return Err(format!(
                "target {}: cfg_ident {:?} is not a valid identifier (a garbage cfg_ident would never match anything and silently surface as PredicateNotFound instead of the manifest error it actually is)",
                t.name, t.cfg_ident
            ));
        }
        if t.option_prefix.is_empty() {
            return Err(format!(
                "target {}: option_prefix must not be empty",
                t.name
            ));
        }
        if t.option_prefix.iter().any(|s| s.trim().is_empty()) {
            return Err(format!(
                "target {}: option_prefix contains an empty segment",
                t.name
            ));
        }
        if t.watch.is_empty() {
            return Err(format!(
                "target {}: watch must not be empty (a target that watches nothing proves nothing)",
                t.name
            ));
        }
        let mut seen_watch = std::collections::HashSet::new();
        for w in &t.watch {
            if w.trim().is_empty() {
                return Err(format!("target {}: watch contains an empty entry", t.name));
            }
            if w.split('.').any(|seg| seg.trim().is_empty()) {
                return Err(format!(
                    "target {}: watch entry {w:?} has an empty path segment (e.g. a stray \"..\")",
                    t.name
                ));
            }
            if !seen_watch.insert(w.as_str()) {
                return Err(format!("target {}: duplicate watch entry {w}", t.name));
            }
        }
    }
    Ok(())
}

fn is_valid_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ---------------------------------------------------------------------
// Source position
// ---------------------------------------------------------------------

#[derive(Serialize, Debug, Clone, PartialEq)]
struct Span {
    file: String,
    line: u32,
    col: u32,
}

fn span_of(file: &str, src: &str, node: &SyntaxNode) -> Span {
    let start: usize = node.text_range().start().into();
    let mut line = 1u32;
    let mut col = 1u32;
    for ch in src[..start].chars() {
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    Span {
        file: file.to_string(),
        line,
        col,
    }
}

// ---------------------------------------------------------------------
// Generic helpers over the raw rowan tree (typed rnix::ast API skipped
// deliberately -- raw SyntaxKind matching is enough for the fixed, narrow
// grammar this MVP targets, and keeps the walker independent of rnix's
// typed-API churn across versions).
// ---------------------------------------------------------------------

fn ident_text(node: &SyntaxNode) -> Option<String> {
    if node.kind() != NODE_IDENT {
        return None;
    }
    node.children_with_tokens()
        .find_map(|e| e.into_token())
        .map(|t: SyntaxToken| t.text().to_string())
}

/// Plain (non-interpolated) string literal content, e.g. `"localhost"` -> `localhost`.
fn string_text(node: &SyntaxNode) -> Option<String> {
    if node.kind() != NODE_STRING {
        return None;
    }
    let mut out = String::new();
    for child in node.children_with_tokens() {
        if let Some(tok) = child.as_token() {
            if tok.kind() == TOKEN_STRING_CONTENT {
                out.push_str(tok.text());
            }
        } else {
            // interpolation ($ { ... }) -- bail, MVP only handles literal strings
            return None;
        }
    }
    Some(out)
}

/// A single path segment from an NODE_ATTRPATH: either an ident or a plain string.
fn attrpath_segments(attrpath: &SyntaxNode) -> Option<Vec<String>> {
    let mut segs = Vec::new();
    for child in attrpath.children() {
        match child.kind() {
            NODE_IDENT => segs.push(ident_text(&child)?),
            NODE_STRING => segs.push(string_text(&child)?),
            _ => return None,
        }
    }
    Some(segs)
}

/// If `node` is `<ident>.<a>.<b>...`, return (`ident` text, [a,b,...]).
fn as_select(node: &SyntaxNode) -> Option<(String, Vec<String>)> {
    if node.kind() != NODE_SELECT {
        return None;
    }
    let mut children = node.children();
    let root = children.next()?;
    let root_name = ident_text(&root)?;
    let attrpath = children.next()?;
    if attrpath.kind() != NODE_ATTRPATH {
        return None;
    }
    let segs = attrpath_segments(&attrpath)?;
    Some((root_name, segs))
}

/// Flatten a possibly-curried NODE_APPLY chain: `f a b c` -> (f, [a,b,c]).
fn flatten_apply(node: &SyntaxNode) -> (SyntaxNode, Vec<SyntaxNode>) {
    if node.kind() != NODE_APPLY {
        return (node.clone(), vec![]);
    }
    let mut children = node.children();
    let head = children.next().expect("apply has a function child");
    let arg = children.next().expect("apply has an argument child");
    let arg = unwrap_paren(arg);
    if head.kind() == NODE_APPLY {
        let (root, mut args) = flatten_apply(&head);
        args.push(arg);
        (root, args)
    } else {
        (head, vec![arg])
    }
}

fn unwrap_paren(node: SyntaxNode) -> SyntaxNode {
    if node.kind() == NODE_PAREN {
        if let Some(inner) = node.children().next() {
            return unwrap_paren(inner);
        }
    }
    node
}

/// Resolve a call-head expression (Ident or `lib.`-qualified Select) to its
/// bare function name, e.g. `mkIf` or `lib.mkIf` -> "mkIf".
fn call_head_name(node: &SyntaxNode) -> Option<String> {
    if let Some(name) = ident_text(node) {
        return Some(name);
    }
    if let Some((_, segs)) = as_select(node) {
        return segs.last().cloned();
    }
    None
}

// ---------------------------------------------------------------------
// Value classification: a small, syntax-level classifier, NOT a textual
// comparison. H1's `is_default_class()` compared raw source text and was
// accidentally correct only because kimai's null default happens to be
// the literal token "null"; H1.1's review caught the general case: a
// `!= null` predicate with a non-literal default/test value (an `if`
// expression, another option's alias, `lib.mkDefault null`, ...) has no
// text that equals "null" even when its runtime value definitely is null.
// This classifies by AST node kind instead, and is honestly Unknown for
// anything it can't decide from syntax alone -- never guessed either way.
// ---------------------------------------------------------------------

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
enum ValueClass {
    /// The literal identifier `null`.
    Null,
    /// The literal identifier `true` or `false`.
    Bool(bool),
    /// A syntactic form that can never evaluate to `null` regardless of
    /// what it contains: a string, number, list, attrset, or path
    /// literal. Doesn't need to know the *value* to know it isn't null.
    DefinitelyNonNull,
    /// Anything else: function application, `if`/`let`/`with`, a
    /// `cfg.foo`-style select, a bare identifier referring to some other
    /// binding, `lib.mkDefault x`, etc. Statically undecidable from
    /// syntax alone without evaluating Nix, which this tool doesn't do.
    Unknown,
}

fn classify_value(node: &SyntaxNode) -> ValueClass {
    match node.kind() {
        NODE_IDENT => match ident_text(node).as_deref() {
            Some("null") => ValueClass::Null,
            Some("true") => ValueClass::Bool(true),
            Some("false") => ValueClass::Bool(false),
            _ => ValueClass::Unknown, // a reference to some other binding
        },
        NODE_STRING | NODE_LITERAL | NODE_ATTR_SET | NODE_LIST | NODE_PATH => {
            ValueClass::DefinitelyNonNull
        }
        _ => ValueClass::Unknown,
    }
}

// ---------------------------------------------------------------------
// Predicate discovery (module.nix)
// ---------------------------------------------------------------------

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
enum PredicateKind {
    /// `cfg.foo != null`
    NullNeq,
    /// `cfg.foo == null`
    NullEq,
    /// `if cfg.foo then ...` / `lib.mkIf cfg.foo ...` / `lib.optional* cfg.foo ...`
    Truthy(String), // carries the helper name ("if", "mkIf", "optionalString", ...)
    /// `if !cfg.foo then ...`
    NegTruthy,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
struct Predicate {
    /// path relative to cfg_ident, e.g. ["database","socket"]
    path: Vec<String>,
    kind: PredicateKind,
    span: Span,
    source: String,
}

const HELPER_NAMES: &[&str] = &[
    "mkIf",
    "optional",
    "optionals",
    "optionalString",
    "optionalAttrs",
];

fn scan_predicates(file: &str, src: &str, root: &SyntaxNode, cfg_ident: &str) -> Vec<Predicate> {
    let mut out = Vec::new();
    for node in root.descendants() {
        match node.kind() {
            NODE_IF_ELSE => {
                let Some(cond) = node.children().next() else {
                    continue;
                };
                // cond != null / cond == null
                if cond.kind() == NODE_BIN_OP {
                    let mut ch = cond.children();
                    let (Some(lhs), Some(rhs)) = (ch.next(), ch.next()) else {
                        continue;
                    };
                    let op_tok = cond
                        .children_with_tokens()
                        .filter_map(|e| e.into_token())
                        .find(|t| t.kind() == TOKEN_NOT_EQUAL || t.kind() == TOKEN_EQUAL);
                    let Some(op_tok) = op_tok else { continue };
                    let is_null = |n: &SyntaxNode| ident_text(n).as_deref() == Some("null");
                    let select_side = if is_null(&rhs) {
                        Some(&lhs)
                    } else if is_null(&lhs) {
                        Some(&rhs)
                    } else {
                        None
                    };
                    if let Some(sel) = select_side {
                        if let Some((root_name, path)) = as_select(sel) {
                            if root_name == cfg_ident {
                                let kind = if op_tok.kind() == TOKEN_NOT_EQUAL {
                                    PredicateKind::NullNeq
                                } else {
                                    PredicateKind::NullEq
                                };
                                out.push(Predicate {
                                    path,
                                    kind,
                                    span: span_of(file, src, &node),
                                    source: first_line(&node),
                                });
                            }
                        }
                    }
                    continue;
                }
                // if cfg.foo then ...
                if let Some((root_name, path)) = as_select(&cond) {
                    if root_name == cfg_ident {
                        out.push(Predicate {
                            path,
                            kind: PredicateKind::Truthy("if".into()),
                            span: span_of(file, src, &node),
                            source: first_line(&node),
                        });
                    }
                    continue;
                }
                // if !cfg.foo then ...
                if cond.kind() == NODE_UNARY_OP {
                    let is_not = cond
                        .children_with_tokens()
                        .filter_map(|e| e.into_token())
                        .any(|t| t.kind() == TOKEN_INVERT);
                    if is_not {
                        if let Some(inner) = cond.children().next() {
                            if let Some((root_name, path)) = as_select(&inner) {
                                if root_name == cfg_ident {
                                    out.push(Predicate {
                                        path,
                                        kind: PredicateKind::NegTruthy,
                                        span: span_of(file, src, &node),
                                        source: first_line(&node),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            NODE_APPLY => {
                // only handle the outermost Apply of a chain to avoid
                // double-reporting inner Apply nodes of the same call
                if node
                    .parent()
                    .map(|p| p.kind() == NODE_APPLY)
                    .unwrap_or(false)
                {
                    continue;
                }
                let (head, args) = flatten_apply(&node);
                let Some(name) = call_head_name(&head) else {
                    continue;
                };
                if !HELPER_NAMES.contains(&name.as_str()) {
                    continue;
                }
                let Some(first_arg) = args.first() else {
                    continue;
                };
                if let Some((root_name, path)) = as_select(first_arg) {
                    if root_name == cfg_ident {
                        out.push(Predicate {
                            path,
                            kind: PredicateKind::Truthy(name),
                            span: span_of(file, src, &node),
                            source: first_line(&node),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn first_line(node: &SyntaxNode) -> String {
    node.text()
        .to_string()
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

// ---------------------------------------------------------------------
// Option declaration discovery (module.nix): mkOption { default = ...; }
// nested under any `options = { ... };` attrpath-value, anywhere in the
// file (covers both a module's own top-level `options` block and a
// separately-defined submodule's `options` block, e.g. kimai's siteOpts).
// ---------------------------------------------------------------------

#[derive(Serialize, Debug, Clone)]
struct OptionDecl {
    /// path relative to the enclosing `options = { ... }` block
    path: Vec<String>,
    default_source: Option<String>,
    /// `None` iff there's no `default = ...;` field at all; `Some(Unknown)`
    /// for a present-but-non-literal default (an `if`, an alias select, a
    /// helper call like `lib.mkDefault x`, ...) -- these are meaningfully
    /// different results and both must survive to gate 3 in `run_target`.
    default_class: Option<ValueClass>,
    /// H2: the exact `KnownValue` of the default, when classifiable --
    /// see `TestAssignment::known_value` for why `default_class` alone
    /// isn't enough for the counterfactual gate.
    default_known_value: Option<KnownValue>,
    span: Span,
}

/// H2 gate-1 fix: nixpkgs modules use two conventions for declaring
/// options -- nested (`options = { foo = ...; };`, e.g. kimai's per-site
/// `siteOpts` submodule, where "options" is already relative to that
/// submodule's own scope) and flat-dotted (`options.services.davis = {
/// ...};`, e.g. real davis.nix, which roots its whole option tree at the
/// exact same absolute path its own `cfg = config.services.davis;`
/// binding uses). The flat form is resolved generically against the
/// *target's own* `option_prefix` from the manifest -- never hardcoded to
/// any specific module or path. If the segments after the leading
/// "options" exactly match `option_prefix` (only possible when
/// `option_prefix` is fully concrete: a wildcard `"*"` is a runtime
/// attrsOf-submodule instance name, which can never appear literally in a
/// module's own static declaration path), that root's contents are
/// already cfg-relative once the shared prefix is stripped, and get
/// walked exactly like the nested form. A wildcarded `option_prefix`
/// (kimai's shape) simply never matches here and falls through to the
/// nested form instead, unchanged -- this fix doesn't alter kimai's path
/// at all.
///
/// Safety gate, scope-aware: for each candidate flat root, `resolve_cfg_root`
/// (defined below, alongside the rest of the alias resolver) resolves
/// `cfg_ident` *as seen from that declaration's own tree position* down to
/// the `config.<path>` it actually denotes, and only walks the root if that
/// resolved path matches `option_prefix` exactly. This used to be a flat,
/// scope-blind search for the first `NODE_ATTRPATH_VALUE` named `cfg_ident`
/// anywhere in the whole module -- reviewed and replaced: a flat search
/// could find an unrelated helper function's own shadowed `cfg` before the
/// real top-level one (wrongly rejecting a legitimate flat root), or the
/// reverse (wrongly accepting one), and either way it was a second,
/// independent, scope-blind way of answering "what does `cfg_ident` mean
/// here" sitting right next to the scope-aware resolver H2 had just built
/// for everything else -- two systems that would inevitably disagree. One
/// resolver, one answer, used everywhere `cfg_ident`'s meaning matters.
fn scan_options(
    file: &str,
    src: &str,
    root: &SyntaxNode,
    cfg_ident: &str,
    option_prefix: &[String],
) -> Vec<OptionDecl> {
    let mut out = Vec::new();
    let prefix_is_concrete = !option_prefix.iter().any(|s| s == "*");
    for node in root.descendants() {
        if node.kind() != NODE_ATTRPATH_VALUE {
            continue;
        }
        let mut children = node.children();
        let Some(attrpath) = children.next() else {
            continue;
        };
        let Some(segs) = attrpath_segments(&attrpath) else {
            continue;
        };
        let Some(value) = children.next() else {
            continue;
        };

        // S2-F1: the SAME real gap `walk_options_block`'s own fix
        // closes, one level up -- this entry point's own early
        // "value must be a literal attrset" check would otherwise skip
        // a top-level `options.<path> = {...} // <shared>;` (or the
        // nested `options = {...} // <shared>;` form) before ever
        // reaching the `segs` match below at all. `walk_merge_operands`
        // is a real no-op for anything that's neither a literal attrset
        // nor a `//`-merge, so dropping the old early `continue` here is
        // safe -- it never widens what gets walked, only what gets
        // UNWRAPPED first.
        if segs == ["options"] {
            // E1/GAP-4 fix: an `options = {...}` block reached ONLY by
            // walking down from another option's own `mkOption {...}`
            // call (e.g. xandikos's real `nginx = mkOption { type =
            // types.submodule { options = { enable = ...; }; }; };`) is
            // NOT a second top-level root -- it's that option's own
            // nested submodule, and `walk_options_block`'s own recursion
            // (below, inside the `is_mk_option_call` branch) now finds
            // and walks it with the correct, extended path. Processing
            // it AGAIN here, from a fresh empty path, is exactly the bug
            // this fix closes: a same-named leaf anywhere inside that
            // submodule (`enable`, `host`, `port`, ...) would silently
            // collide with an unrelated option of the same bare name
            // elsewhere in the file, and `run_target`'s gate-1 lookup
            // (`options.iter().find(|o| o.path == watched_path)`) has no
            // way to tell the two apart -- a real, demonstrated
            // false-positive-CAPABLE bug (see
            // `fixtures/synthetic/h2-nested-submodule-collision/`),
            // found by the E1 holdout audit on real, unfamiliar code
            // (`xandikos`), not a hypothetical. Kimai's own `siteOpts`
            // submodule is unaffected: it's a plain `let`-bound function
            // value, referenced only by NAME (`types.submodule
            // siteOpts`) elsewhere, never itself inside an `mkOption
            // {...}` call -- `is_nested_inside_mk_option_call` correctly
            // returns `false` for it, so it keeps being walked from a
            // fresh empty path exactly as before this fix.
            if is_nested_inside_mk_option_call(&node) {
                continue;
            }
            walk_merge_operands(&value, |attrset| {
                walk_options_block(file, src, attrset, &mut Vec::new(), &mut out, option_prefix);
            });
            continue;
        }

        if prefix_is_concrete
            && !option_prefix.is_empty()
            && segs.len() == option_prefix.len() + 1
            && segs[0] == "options"
            && segs[1..] == option_prefix[..]
            && resolve_cfg_root(&node, cfg_ident)
                .map(|resolved| resolved == option_prefix)
                .unwrap_or(false)
        {
            walk_merge_operands(&value, |attrset| {
                walk_options_block(file, src, attrset, &mut Vec::new(), &mut out, option_prefix);
            });
        }
    }
    out
}

fn walk_options_block(
    file: &str,
    src: &str,
    attrset: &SyntaxNode,
    path: &mut Vec<String>,
    out: &mut Vec<OptionDecl>,
    option_prefix: &[String],
) {
    let prefix_is_concrete = !option_prefix.iter().any(|s| s == "*");
    for entry in attrset.children() {
        if entry.kind() != NODE_ATTRPATH_VALUE {
            continue;
        }
        let mut children = entry.children();
        let Some(attrpath) = children.next() else {
            continue;
        };
        let Some(segs) = attrpath_segments(&attrpath) else {
            continue;
        };
        let Some(value) = children.next() else {
            continue;
        };

        path.extend(segs.clone());

        // E1/P2/GAP-2 fix. A module using the nested `options = {
        // services.<name> = { ... }; };` idiom (one bare top-level
        // `options` attrpath whose value nests `services.<name>` inside
        // it, e.g. real xandikos/svnserve/cadvisor -- as opposed to the
        // flat-dotted `options.services.<name> = { ... };` form the
        // SECOND branch of `scan_options` already strips generically)
        // has its option_prefix embedded partway INSIDE the walk, not at
        // its own entry point -- so a real, plain `mkOption`-declared
        // leaf like `enable` was being recorded at the over-qualified
        // `["services","<name>","enable"]`, never the `option_prefix`-
        // relative `["enable"]` `run_target`'s own gate-1 lookup expects
        // (`watch` is documented as "relative to cfg_ident", the same
        // contract the flat-dotted branch already honors). The moment
        // the ACCUMULATED path exactly equals `option_prefix` -- however
        // many separate attrpath segments it took to get there, whether
        // one combined `services.svnserve = {...}` or nested `services =
        // { svnserve = {...}; };` -- everything under this point is
        // relative to THAT instance, exactly the same "reset to a fresh
        // path" treatment the flat-dotted branch already gives its own
        // root. Kimai's own `siteOpts` is unaffected: nothing inside its
        // `options = {...}` block ever accumulates to
        // `option_prefix` (`["services","kimai","sites","*"]` is
        // wildcarded, `prefix_is_concrete`-guarded the same way the
        // flat-dotted branch already is, and no real attrpath can ever
        // spell a literal `"*"` segment), so this check simply never
        // fires for it.
        let at_prefix_root =
            prefix_is_concrete && !option_prefix.is_empty() && path.as_slice() == option_prefix;

        if let Some(helper) = classify_option_helper_call(&value) {
            let (default_source, default_class, default_known_value) =
                option_helper_default(&helper, &value);
            out.push(OptionDecl {
                path: path.clone(),
                default_source,
                default_class,
                default_known_value,
                span: span_of(file, src, &entry),
            });
            // E1/GAP-4 fix: this option's own declaration is recorded
            // above regardless (a submodule-typed option, e.g. `nginx`
            // itself, is still a real, directly declared option) --
            // additionally, if its type wraps a nested `options = {...}`
            // submodule block (inline `types.submodule {...}`, possibly
            // itself wrapped in `types.attrsOf`/`types.nullOr`/...), walk
            // THAT block too, with the path already extended by this
            // entry's own name (`path` above, not a fresh `Vec::new()`)
            // -- so `nginx.enable` is recorded at `["nginx","enable"]`,
            // never at the bare, collision-prone `["enable"]`.
            if let Some(nested) = find_nested_options_block(&value) {
                walk_options_block(file, src, &nested, path, out, option_prefix);
            }
        } else {
            walk_merge_operands(&value, |attrset| {
                if at_prefix_root {
                    walk_options_block(file, src, attrset, &mut Vec::new(), out, option_prefix);
                } else {
                    walk_options_block(file, src, attrset, path, out, option_prefix);
                }
            });
        }

        for _ in 0..segs.len() {
            path.pop();
        }
    }
}

/// S2-F1 (a real nixpkgs PR shadow audit): a `//`-merge (`A // B`) as a
/// nested options block's own VALUE is a real, common nixpkgs idiom for
/// combining a literal set of declarations with a shared/external one
/// -- e.g. `networking.firewall`'s real `{ enable = lib.mkOption
/// {...}; ... } // commonOptions;`, found by a real shadow-audit false
/// `OptionNotFound` on `enable` (PR #558149: `enable` genuinely IS
/// declared, directly, with no indirection at all -- the whole `//`
/// expression was previously treated as unrecognized, silently losing
/// every declaration on EITHER side). `walk_options_block`'s own
/// attrset-literal check alone (`value.kind() == NODE_ATTR_SET`) never
/// matched this shape at all.
///
/// Fixed here by unwrapping ANY operand that is, itself, either a
/// literal attrset or another `//`-merge (recursively) -- deliberately
/// bounded to exactly these two node shapes, nothing else. An operand
/// that's anything other than a literal attrset or a further `//`
/// (a bare identifier reference, a function call, ...) is left opaque,
/// exactly as before this fix: a real, disclosed limit, not a general
/// Nix evaluator creeping in through the back door. This is the SAME
/// general mechanism behind part of the real `exporters.nix` gap this
/// same shadow-audit round found (`mkExporterOpts {...} // extraOpts`)
/// -- but only PART of it: `extraOpts` there is a bare identifier
/// reference (deferred through a function parameter to a separate call
/// site elsewhere in the file), not a literal attrset at its own use
/// site, so it stays correctly opaque here too. The rest of that gap
/// is a real multi-source-resolution problem, out of scope for this
/// narrow, bounded fix -- see the S2-F3 census before any generic
/// multi-file support is attempted.
fn walk_merge_operands(value: &SyntaxNode, mut visit: impl FnMut(&SyntaxNode)) {
    fn go(value: &SyntaxNode, visit: &mut dyn FnMut(&SyntaxNode)) {
        if value.kind() == NODE_ATTR_SET {
            visit(value);
            return;
        }
        if let Some(bin) = rnix::ast::BinOp::cast(value.clone()) {
            if bin.operator() == Some(rnix::ast::BinOpKind::Update) {
                if let Some(lhs) = bin.lhs() {
                    go(&lhs.syntax().clone(), visit);
                }
                if let Some(rhs) = bin.rhs() {
                    go(&rhs.syntax().clone(), visit);
                }
            }
        }
    }
    go(value, &mut visit);
}

fn is_mk_option_call(node: &SyntaxNode) -> bool {
    is_call_named(node, "mkOption")
}

/// Resolves a call-head expression the same way `call_head_name` does,
/// then checks it against `name` -- e.g. `is_call_named(node,
/// "mkEnableOption")` matches both `mkEnableOption "x"` and
/// `lib.mkEnableOption "x"`. Shared by `is_mk_option_call` and P1's own
/// `classify_option_helper_call` below, instead of each hand-rolling its
/// own `flatten_apply` + `call_head_name` check.
fn is_call_named(node: &SyntaxNode, name: &str) -> bool {
    if node.kind() != NODE_APPLY {
        return false;
    }
    let (head, _args) = flatten_apply(node);
    call_head_name(&head).as_deref() == Some(name)
}

fn mk_option_field(apply_node: &SyntaxNode, field: &str) -> Option<SyntaxNode> {
    let (_head, args) = flatten_apply(apply_node);
    find_attrset_field(args.first()?, field)
}

/// Shared by `mk_option_field` (an `mkOption {...}` call's own first
/// argument) and P1's own `classify_option_helper_call` (an
/// `mkEnableOption ... // {...}` merge's RHS attrset) -- looking a named
/// field up in an attrset is the same operation either way, not two
/// independently-maintained copies of the same walk.
fn find_attrset_field(attrset: &SyntaxNode, field: &str) -> Option<SyntaxNode> {
    if attrset.kind() != NODE_ATTR_SET {
        return None;
    }
    for entry in attrset.children() {
        if entry.kind() != NODE_ATTRPATH_VALUE {
            continue;
        }
        let mut children = entry.children();
        let attrpath = children.next()?;
        let segs = attrpath_segments(&attrpath)?;
        if segs == vec![field.to_string()] {
            return children.next();
        }
    }
    None
}

/// E1/P1 fix. A recognized nixpkgs option-declaring HELPER call, with
/// its own KNOWN, stable semantic contract -- a small, explicit,
/// honestly maintained table of "known helper -> known synthesized
/// declaration shape", not a literal-string special case buried in the
/// walker's control flow (the exact "regular-expression farm" this
/// design was reviewed to avoid: a future `lib.mkEnableOption`
/// re-alias/wrapper needs a new entry HERE, in one place, not a new
/// scattered string match). `mkOption` needs nothing synthesized -- its
/// own literal `default =` field is read directly, unchanged from
/// before this fix. `mkEnableOption <description>` is nixpkgs's own
/// stable, documented helper (`lib/options.nix`): always `mkOption {
/// type = types.bool; default = false; ...};` -- modeled here as that
/// exact known default, UNLESS the call is immediately `//`-merged with
/// its own attrset (e.g. libinput's real `lib.mkEnableOption "libinput"
/// // { default = config.services.xserver.enable; };`), in which case
/// the override's own `default` field wins -- the same precedence real
/// Nix's `//` operator itself has (the override's OTHER keys, and the
/// synthesized `default = false` if the override doesn't mention
/// `default` at all, both survive untouched).
enum OptionHelperCall {
    Explicit,
    EnableOption { override_attrs: Option<SyntaxNode> },
}

fn classify_option_helper_call(node: &SyntaxNode) -> Option<OptionHelperCall> {
    if let Some(bin) = rnix::ast::BinOp::cast(node.clone()) {
        if bin.operator() == Some(rnix::ast::BinOpKind::Update) {
            let lhs = bin.lhs()?.syntax().clone();
            let rhs = bin.rhs()?.syntax().clone();
            if is_call_named(&lhs, "mkEnableOption") && rhs.kind() == NODE_ATTR_SET {
                return Some(OptionHelperCall::EnableOption {
                    override_attrs: Some(rhs),
                });
            }
        }
        return None;
    }
    if is_call_named(node, "mkOption") {
        return Some(OptionHelperCall::Explicit);
    }
    if is_call_named(node, "mkEnableOption") {
        return Some(OptionHelperCall::EnableOption {
            override_attrs: None,
        });
    }
    None
}

/// The `(default_source, default_class, default_known_value)` triple
/// for one recognized `OptionHelperCall`, the same three pieces of
/// evidence `walk_options_block` has always recorded per `OptionDecl` --
/// factored out so the `mkOption`/`mkEnableOption` cases can't drift
/// out of sync with each other.
fn option_helper_default(
    helper: &OptionHelperCall,
    value: &SyntaxNode,
) -> (Option<String>, Option<ValueClass>, Option<KnownValue>) {
    let default_node = match helper {
        OptionHelperCall::Explicit => mk_option_field(value, "default"),
        OptionHelperCall::EnableOption { override_attrs } => override_attrs
            .as_ref()
            .and_then(|attrs| find_attrset_field(attrs, "default")),
    };
    match default_node {
        Some(node) => (
            Some(node.text().to_string()),
            Some(classify_value(&node)),
            classify_known_value(&node),
        ),
        None => match helper {
            // mkEnableOption's own known, stable default -- synthesized,
            // never read from source text, since there IS no source
            // text for it at a bare `mkEnableOption "..."` call site.
            OptionHelperCall::EnableOption { .. } => (
                Some("false".to_string()),
                Some(ValueClass::Bool(false)),
                Some(KnownValue::Exact(Scalar::Bool(false))),
            ),
            OptionHelperCall::Explicit => (None, None, None),
        },
    }
}

/// E1/GAP-4 fix. True if `node` sits inside (a strict descendant of)
/// some `mkOption {...}` call's own subtree -- i.e. reachable only by
/// walking DOWN from that call, never a genuinely independent top-level
/// block. `.ancestors()` includes `node` itself first (same convention
/// `resolve_ident_binding` already uses), hence `.skip(1)`. Distinguishes
/// xandikos's real `nginx = mkOption { type = types.submodule { options
/// = {...}; }; };` (nested inside `nginx`'s own call -- `true`) from
/// kimai's real `siteOpts = {...}: { options = {...}; };` (a plain
/// `let`-bound function value, referenced only by name elsewhere,
/// `types.submodule siteOpts` -- never itself inside an `mkOption` call
/// -- `false`).
fn is_nested_inside_mk_option_call(node: &SyntaxNode) -> bool {
    node.ancestors()
        .skip(1)
        .any(|a| a.kind() == NODE_APPLY && is_mk_option_call(&a))
}

/// E1/GAP-4 fix. Finds a `types.submodule { options = {...}; }`-shaped
/// nested options block declared INLINE inside one option's own
/// `mkOption {...}` call -- searched generically anywhere within the
/// call's own subtree (not hardcoded to the `type` field specifically),
/// since a submodule can appear through `types.attrsOf`/`types.nullOr`/
/// ... wrapping it too. The FIRST such block found (by `descendants()`'s
/// own pre-order traversal) is returned; a call containing more than one
/// `options = {...}` block at the same nesting level is a shape this
/// project's real corpus has never shown, not silently guessed at.
fn find_nested_options_block(mk_option_call: &SyntaxNode) -> Option<SyntaxNode> {
    mk_option_call.descendants().find_map(|d| {
        if d.kind() != NODE_ATTRPATH_VALUE {
            return None;
        }
        let mut children = d.children();
        let attrpath = children.next()?;
        let segs = attrpath_segments(&attrpath)?;
        if segs != ["options"] {
            return None;
        }
        let value = children.next()?;
        (value.kind() == NODE_ATTR_SET).then_some(value)
    })
}

// ---------------------------------------------------------------------
// H2: reuse survey (per AGENTS.md, recorded before writing any of the code
// below).
//
// Checked, this session, against the actual dependency already vendored
// in Cargo.lock: rnix 0.11's typed `ast` module (`rnix::ast::{BinOp,
// UnaryOp, BinOpKind, UnaryOpKind}`) -- confirmed by reading
// `~/.cargo/registry/.../rnix-0.11.0/src/ast/{nodes.rs,operators.rs}`
// directly, not assumed from the changelog. `BinOp::operator() ->
// Option<BinOpKind>` and `UnaryOp::operator() -> Option<UnaryOpKind>`
// already classify `&&`/`||`/`==`/`!=`/`!` into a clean enum from a
// single call, replacing what would otherwise be more hand-rolled
// `TOKEN_AND_AND`/`TOKEN_OR_OR`/... matching (the existing H1 code already
// hand-matches `TOKEN_NOT_EQUAL`/`TOKEN_EQUAL`/`TOKEN_INVERT` for the
// narrower unary case, in `scan_predicates` below and `NegTruthy`
// handling -- left as-is there since H1 is frozen; used going forward in
// `lower_pred`/`lower_value_expr`). This is a zero-cost reuse: no new
// dependency, no API surface beyond what `rowan::ast::AstNode` (already
// imported) already requires.
//
// Checked, not vendored, kept as reference/design input only (per the
// user's own prior research, cross-checked against what's structurally
// plausible for each project, not independently re-verified line-by-line
// in this session): `oxalica/nil`'s name-resolution
// (`crates/ide/src/def/nameres.rs`) implements full lexical scope
// resolution over `let`/`rec`/`with`/function params, but lives inside
// nil's own IDE crate graph (a Salsa incremental-computation database, and
// its own syntax-tree model layered over rnix) -- pulling in the
// algorithm would mean pulling in most of nil's architecture along with
// it for one function's worth of logic. `nixd`/`libnixf`'s semantic
// variable lookup is a C++ stack built on the real Nix evaluator
// libraries -- not a Rust dependency at all, and far heavier than a small
// rnix-based checker needs. Tvix has a real Nix evaluator with its own
// scope tracking, live and correct by construction, but "vendor an
// evaluator" is a different, heavier tool than this one; kept in mind as
// a possible future *differential oracle* (run both, compare) rather than
// something to embed in the analysis core.
//
// What's actually reused: rnix's typed AST directly, as above. What's
// reused as a design reference, not code: nil's general shape of "resolve
// an identifier by walking enclosing scopes outward, stop at the first
// binding, treat shadowing and cycles explicitly" -- standard lexical
// scoping, applied narrowly (see the alias-resolution work landing in the
// next commit) to exactly `let`/`rec` bindings and function parameters,
// not a general Nix evaluator.
//
// The concrete project-specific gap that remains, and that no reused
// artifact fills: a *non-evaluating*, fail-closed resolver that maps a
// `let`-bound identifier used inside a `cfg`-rooted predicate/value
// expression back to either something this pure IR can represent (a
// select rooted at `cfg_ident`, a literal, or a whole aliased boolean
// expression) or an explicit `None` -- never a guess, never partial Nix
// evaluation, no `with`, no dynamic attribute names, no imports.
// ---------------------------------------------------------------------

// ---------------------------------------------------------------------
// H2: a pure Predicate IR (Pred/ValueExpr/Scalar), independent of the
// H1-era `PredicateKind` enum above (kept as-is; H1 is frozen). Unlike
// `PredicateKind`, this represents a predicate as an actual boolean
// expression tree over option references, so a compound condition like
// `db.createLocally && db.driver == "mysql"` can be represented and
// evaluated as itself, rather than forcing every predicate down to
// "truthy/falsy check of exactly one option's value" the way H1's model
// does. Every lowering function below returns `Result<_, ResolveFailure>`,
// not a guess: a typed failure means "this syntactic shape isn't
// representable in this pure IR, and here's specifically why", propagated
// by `?` rather than silently discarded or collapsed into an
// undifferentiated `None` -- the same fail-closed idiom
// `ValueClass`/`predicate_outcome` already use throughout H1, sharpened
// once this lowering started feeding the real verdict chain (see
// `ResolveFailure`'s own doc comment for why a bare `Option` stopped being
// enough).
// ---------------------------------------------------------------------

/// A dotted option path, relative to `cfg_ident` -- e.g. `cfg.database.
/// driver` lowers to `["database", "driver"]`. Same representation
/// `Predicate.path`/`OptionDecl.path` already use; aliased here for
/// readability in the IR types below.
type OptionPath = Vec<String>;

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
enum Scalar {
    Null,
    Bool(bool),
    Str(String),
}

/// An environment value as actually known at counterfactual-evaluation
/// time (see the counterfactual gate below), distinct from `Scalar`
/// (which only appears as a *literal in source*). `Exact` covers a value
/// this tool pinned down precisely (`null`/`true`/`false`/a specific
/// string literal, from either an explicit test assignment or a
/// classifiable declared default). `DefinitelyNonNull` covers a value
/// classified as `ValueClass::DefinitelyNonNull` in H1's sense (a
/// string/number/list/attrset/path literal) whose *exact* contents this
/// tool didn't bother extracting (a list or attrset literal has no
/// `Scalar` representation at all) -- reviewed and deliberately NOT
/// collapsed into a fabricated placeholder `Scalar`: doing that would let
/// the evaluator draw equality conclusions (`"placeholder" == "mysql"`)
/// it has no actual basis for. `DefinitelyNonNull` only ever proves a
/// null-comparison's outcome (see `eval_known_eq`), never an equality
/// against a *specific* other value.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
enum KnownValue {
    Exact(Scalar),
    DefinitelyNonNull,
}

/// Classifies an AST node's *known* runtime value the same way H1's
/// `classify_value`/`ValueClass` does, but preserving the exact literal
/// content (`Scalar`) when the node is one of `null`/`true`/`false`/a
/// plain string, instead of collapsing it into a class. `None` where
/// `ValueClass` would say `Unknown` -- genuinely not statically knowable.
fn classify_known_value(node: &SyntaxNode) -> Option<KnownValue> {
    match node.kind() {
        NODE_IDENT => match ident_text(node).as_deref() {
            Some("null") => Some(KnownValue::Exact(Scalar::Null)),
            Some("true") => Some(KnownValue::Exact(Scalar::Bool(true))),
            Some("false") => Some(KnownValue::Exact(Scalar::Bool(false))),
            _ => None,
        },
        NODE_STRING => string_text(node).map(|s| KnownValue::Exact(Scalar::Str(s))),
        NODE_LITERAL | NODE_ATTR_SET | NODE_LIST | NODE_PATH => Some(KnownValue::DefinitelyNonNull),
        _ => None,
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
enum ValueExpr {
    /// A `cfg_ident`-rooted select, e.g. `cfg.database.driver`.
    Ref(OptionPath),
    Literal(Scalar),
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
enum Pred {
    Eq(ValueExpr, ValueExpr),
    Not(Box<Pred>),
    And(Box<Pred>, Box<Pred>),
    Or(Box<Pred>, Box<Pred>),
}

/// Why `lower_pred`/`lower_value_expr` couldn't produce an IR value.
/// H2 commit 1 used a bare `Option<_>`, which was fine while the IR was
/// only unit-tested in isolation -- but once this lowering feeds the real
/// verdict chain, a bare `None` starts meaning too many different things
/// at once ("no predicate here at all" vs. "there's a real alias this
/// resolver just can't trace yet" vs. "cyclic aliasing" are very
/// different facts a report reader needs to tell apart, the same way
/// `Opacity`'s `reason` field exists so H1's walker never collapses
/// "gave up" into one undifferentiated bucket). Reviewed and fixed before
/// this landed in `run_target`, not after.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
enum ResolveFailure {
    /// An identifier with no reachable binding in any enclosing scope.
    Unbound(String),
    /// Resolving this identifier required passing through the same name
    /// twice -- a cyclic alias, never followed indefinitely. Carries the
    /// chain of names that led back to itself.
    Cycle(Vec<String>),
    /// A scope construct this resolver deliberately doesn't model:
    /// `with` (dynamic, can't be resolved statically), a function
    /// parameter (bound to a runtime argument, not a lexical alias to
    /// any expression), `inherit` (would need to trace an outer scope or
    /// an arbitrary expression further), or a `rec` attrset's own
    /// internal mutual visibility (a materially different scoping rule
    /// from `let`, and not how real modules alias predicates in
    /// practice).
    UnsupportedScope(&'static str),
    /// A syntactic shape this pure IR (`Pred`/`ValueExpr`) can't
    /// represent at all, even after alias resolution successfully found
    /// a binding -- an operator this IR doesn't model, a helper call, a
    /// non-boolean-shaped resolved expression, and so on.
    UnsupportedExpression,
    /// A condition that's literally the constant `true` or `false` (`if
    /// true then ...`, `mkIf true {...}`) -- not a real option-branch
    /// predicate, but also not a risk: unlike every other
    /// `UnsupportedExpression`, this failure carries zero uncertainty
    /// about what the condition evaluates to. Kept as its own variant
    /// specifically so callers deciding "does an unresolved site block a
    /// negative conclusion" (see `scan_resolved_predicates`'s
    /// `unresolved_sites`) can tell it apart from a genuine unknown --
    /// reviewed after a synthetic adversarial fixture's own `mkIf true
    /// {...}` wrapper was initially (wrongly) treated as exactly that
    /// kind of risk.
    TrivialConstant,
}

/// The chain of alias names currently being resolved, used purely for
/// cycle detection (`a = b; b = a;` must fail as `Cycle`, never recurse
/// until the stack overflows).
type AliasChain = Vec<String>;

/// Resolves a bare identifier used at `ident_node`'s position to the
/// `SyntaxNode` of whatever expression it's lexically bound to, by
/// walking outward through enclosing scopes and stopping at the first one
/// that defines `name` (nearest-binding-wins -- correct lexical
/// shadowing: `let x = cfg.a; in let x = cfg.b; in x` resolves `x` to
/// `cfg.b`, not `cfg.a`). Reference resolution only -- this never
/// rewrites or re-parses any text, it just walks the existing tree and
/// hands back a pointer into it.
fn resolve_ident_binding(
    ident_node: &SyntaxNode,
    name: &str,
) -> Result<SyntaxNode, ResolveFailure> {
    for ancestor in ident_node.ancestors().skip(1) {
        match ancestor.kind() {
            NODE_LET_IN => {
                let mut children: Vec<SyntaxNode> = ancestor.children().collect();
                if children.pop().is_none() {
                    continue; // no body -- malformed, nothing to bind here
                }
                for entry in &children {
                    match entry.kind() {
                        NODE_ATTRPATH_VALUE => {
                            let mut c = entry.children();
                            let (Some(attrpath), Some(value)) = (c.next(), c.next()) else {
                                continue;
                            };
                            if let Some(segs) = attrpath_segments(&attrpath) {
                                if segs.len() == 1 && segs[0] == name {
                                    return Ok(value);
                                }
                            }
                        }
                        NODE_INHERIT
                            if entry.children().any(|c| {
                                c.kind() == NODE_IDENT && ident_text(&c).as_deref() == Some(name)
                            }) =>
                        {
                            return Err(ResolveFailure::UnsupportedScope("inherit"));
                        }
                        _ => {}
                    }
                }
            }
            NODE_LAMBDA => {
                // A simple `x: body` parameter is `NODE_IDENT_PARAM`
                // (which itself wraps an inner `NODE_IDENT` child) --
                // NOT a bare `NODE_IDENT` directly. Caught live by
                // `lambda_parameter_shadows_outer_alias_and_is_unsupported`:
                // matching on `NODE_IDENT` here let `x` inside a lambda
                // body silently resolve past the lambda's own parameter
                // to an outer `x = cfg.a;` alias -- exactly the kind of
                // wrong-shadowing bug this whole function exists to
                // prevent, on the simplest possible lambda form.
                let shadows =
                    ancestor
                        .children()
                        .next()
                        .is_some_and(|pattern| match pattern.kind() {
                            NODE_IDENT_PARAM => {
                                pattern
                                    .children()
                                    .next()
                                    .and_then(|id| ident_text(&id))
                                    .as_deref()
                                    == Some(name)
                            }
                            NODE_PATTERN => pattern.children().any(|child| {
                                matches!(
                                    child.kind(),
                                    NODE_PAT_ENTRY | NODE_IDENT_PARAM | NODE_PAT_BIND
                                ) && child
                                    .children()
                                    .next()
                                    .and_then(|id| ident_text(&id))
                                    .as_deref()
                                    == Some(name)
                            }),
                            _ => false,
                        });
                if shadows {
                    return Err(ResolveFailure::UnsupportedScope("function parameter"));
                }
            }
            NODE_WITH => {
                return Err(ResolveFailure::UnsupportedScope("with"));
            }
            NODE_ATTR_SET => {
                let is_rec = ancestor
                    .children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .any(|t| t.kind() == TOKEN_REC);
                let binds_name = ancestor.children().any(|entry| {
                    entry.kind() == NODE_ATTRPATH_VALUE
                        && entry
                            .children()
                            .next()
                            .and_then(|ap| attrpath_segments(&ap))
                            .is_some_and(|segs| segs.len() == 1 && segs[0] == name)
                });
                if is_rec && binds_name {
                    return Err(ResolveFailure::UnsupportedScope("rec attrset"));
                }
            }
            _ => {}
        }
    }
    Err(ResolveFailure::Unbound(name.to_string()))
}

/// Shared cycle-checking + scope-resolution wrapper: if `name` is already
/// in `chain`, fails as `Cycle` instead of resolving further; otherwise
/// resolves `name`'s binding and recurses into `lower_bound` with `name`
/// pushed onto `chain` (popped again before returning), so a cycle
/// spanning multiple hops (`a -> b -> a`) is caught regardless of which
/// hop closes the loop.
fn resolve_alias_recursively<T>(
    ident_node: &SyntaxNode,
    name: &str,
    chain: &mut AliasChain,
    lower_bound: impl FnOnce(&SyntaxNode, &mut AliasChain) -> Result<T, ResolveFailure>,
) -> Result<T, ResolveFailure> {
    if chain.iter().any(|n| n == name) {
        let mut c = chain.clone();
        c.push(name.to_string());
        return Err(ResolveFailure::Cycle(c));
    }
    let bound_node = resolve_ident_binding(ident_node, name)?;
    chain.push(name.to_string());
    let result = lower_bound(&bound_node, chain);
    chain.pop();
    result
}

/// Resolves `cfg_ident` *as seen from `use_site`'s own tree position* down
/// to the `OptionPath` it denotes relative to `config` -- e.g. `cfg =
/// config.services.davis;` resolves to `["services","davis"]`. The single
/// source of truth for "does this occurrence of `cfg_ident` really mean
/// this target's `option_prefix`", used by the gate-1 flat-root safety
/// check (see `scan_options`) so it can never disagree with the same
/// scope-aware resolution every alias lookup elsewhere in H2 already uses
/// -- deliberately NOT a separate, independent scope-blind search.
fn resolve_cfg_root(use_site: &SyntaxNode, cfg_ident: &str) -> Result<OptionPath, ResolveFailure> {
    resolve_alias_recursively(
        use_site,
        cfg_ident,
        &mut Vec::new(),
        resolve_config_rooted_path,
    )
}

/// Helper for `resolve_cfg_root`: given an expression already resolved to
/// (an alias of) `cfg_ident`'s binding, keeps chasing through further
/// aliases (`cfg = innerCfg; innerCfg = config.services.davis;`) until it
/// either lands on a literal `config`-rooted select or fails. Structurally
/// the same shape as `lower_value_expr_chained`, but deliberately a
/// separate function rather than reusing it directly: this one's target
/// root is always the literal identifier `"config"`, never `cfg_ident`
/// (calling `lower_value_expr_chained(_, cfg_ident, _)` here would ask
/// "does this resolve to `cfg_ident`", the wrong question when the whole
/// point is determining what `cfg_ident` itself means).
fn resolve_config_rooted_path(
    node: &SyntaxNode,
    chain: &mut AliasChain,
) -> Result<OptionPath, ResolveFailure> {
    if node.kind() == NODE_SELECT {
        let (root_name, path) = as_select(node).ok_or(ResolveFailure::UnsupportedExpression)?;
        if root_name == "config" {
            return Ok(path);
        }
        let root_node = node
            .children()
            .next()
            .ok_or(ResolveFailure::UnsupportedExpression)?;
        let base =
            resolve_alias_recursively(&root_node, &root_name, chain, resolve_config_rooted_path)?;
        let mut full = base;
        full.extend(path);
        return Ok(full);
    }
    if node.kind() == NODE_IDENT {
        if let Some(name) = ident_text(node) {
            return resolve_alias_recursively(node, &name, chain, resolve_config_rooted_path);
        }
    }
    Err(ResolveFailure::UnsupportedExpression)
}

/// Lowers a value-position expression into the pure IR: a `cfg_ident`-
/// rooted select becomes `Ref`; the literals `null`/`true`/`false`/a
/// plain string become `Literal`. A `cfg_ident`-rooted select reached
/// *through* an alias (`db.driver` where `db = cfg.database;`) resolves
/// the alias and splices the remaining path onto the resolved base --
/// select aliases and expression aliases are genuinely different shapes
/// (a select alias's resolved form must itself be a `Ref` for splicing to
/// make sense; resolving to a `Literal` or anything else is
/// `UnsupportedExpression`, not silently dropped).
fn lower_value_expr(node: &SyntaxNode, cfg_ident: &str) -> Result<ValueExpr, ResolveFailure> {
    lower_value_expr_chained(node, cfg_ident, &mut Vec::new())
}

fn lower_value_expr_chained(
    node: &SyntaxNode,
    cfg_ident: &str,
    chain: &mut AliasChain,
) -> Result<ValueExpr, ResolveFailure> {
    if node.kind() == NODE_SELECT {
        let (root_name, path) = as_select(node).ok_or(ResolveFailure::UnsupportedExpression)?;
        if root_name == cfg_ident {
            return Ok(ValueExpr::Ref(path));
        }
        let root_node = node
            .children()
            .next()
            .ok_or(ResolveFailure::UnsupportedExpression)?;
        let base = resolve_alias_recursively(&root_node, &root_name, chain, |bound, chain| {
            lower_value_expr_chained(bound, cfg_ident, chain)
        })?;
        return match base {
            ValueExpr::Ref(mut prefix) => {
                prefix.extend(path);
                Ok(ValueExpr::Ref(prefix))
            }
            ValueExpr::Literal(_) => Err(ResolveFailure::UnsupportedExpression),
        };
    }
    match node.kind() {
        NODE_IDENT => match ident_text(node).as_deref() {
            Some("null") => Ok(ValueExpr::Literal(Scalar::Null)),
            Some("true") => Ok(ValueExpr::Literal(Scalar::Bool(true))),
            Some("false") => Ok(ValueExpr::Literal(Scalar::Bool(false))),
            Some(name) => resolve_alias_recursively(node, name, chain, |bound, chain| {
                lower_value_expr_chained(bound, cfg_ident, chain)
            }),
            None => Err(ResolveFailure::UnsupportedExpression),
        },
        NODE_STRING => string_text(node)
            .map(|s| ValueExpr::Literal(Scalar::Str(s)))
            .ok_or(ResolveFailure::UnsupportedExpression),
        _ => Err(ResolveFailure::UnsupportedExpression),
    }
}

/// Lowers a boolean-valued expression into the pure `Pred` IR: `a != b`
/// (`Not(Eq(a,b))`), `a == b` (`Eq(a,b)`), a bare `cfg_ident`-rooted
/// reference used directly as a condition (`Eq(ref, true)`), `!p`
/// (`Not(p)`), `a && b` / `a || b` (`And`/`Or`). Uses rnix's typed
/// `ast::BinOp`/`ast::UnaryOp` operator classification (see the reuse
/// survey above) instead of hand-matching tokens.
///
/// A bare alias identifier used *directly as a condition* (`if
/// mysqlLocal then ...`) is resolved and lowered as a `Pred` directly,
/// not as a value reference: `mysqlLocal`'s binding is a whole compound
/// boolean expression (`db.createLocally && db.driver == "mysql"`), not
/// an option path, so the alias denotes the predicate itself, not
/// something to wrap in `Eq(_, true)`. That wrapping only applies to a
/// genuine `cfg_ident`-rooted option reference (possibly reached through
/// a *select* alias like `db`, handled by `lower_value_expr`) used as a
/// condition -- the two alias shapes are distinguished purely by what
/// they're bound to, discovered by attempting the resolution and looking
/// at the result, never guessed from the identifier's own spelling.
fn lower_pred(node: &SyntaxNode, cfg_ident: &str) -> Result<Pred, ResolveFailure> {
    lower_pred_chained(node, cfg_ident, &mut Vec::new())
}

fn lower_pred_chained(
    node: &SyntaxNode,
    cfg_ident: &str,
    chain: &mut AliasChain,
) -> Result<Pred, ResolveFailure> {
    let node = unwrap_paren(node.clone());
    if let Some(bin) = rnix::ast::BinOp::cast(node.clone()) {
        let op = bin
            .operator()
            .ok_or(ResolveFailure::UnsupportedExpression)?;
        let lhs_node = bin
            .lhs()
            .ok_or(ResolveFailure::UnsupportedExpression)?
            .syntax()
            .clone();
        let rhs_node = bin
            .rhs()
            .ok_or(ResolveFailure::UnsupportedExpression)?
            .syntax()
            .clone();
        return match op {
            rnix::ast::BinOpKind::And => Ok(Pred::And(
                Box::new(lower_pred_chained(&lhs_node, cfg_ident, chain)?),
                Box::new(lower_pred_chained(&rhs_node, cfg_ident, chain)?),
            )),
            rnix::ast::BinOpKind::Or => Ok(Pred::Or(
                Box::new(lower_pred_chained(&lhs_node, cfg_ident, chain)?),
                Box::new(lower_pred_chained(&rhs_node, cfg_ident, chain)?),
            )),
            rnix::ast::BinOpKind::Equal => Ok(Pred::Eq(
                lower_value_expr_chained(&lhs_node, cfg_ident, chain)?,
                lower_value_expr_chained(&rhs_node, cfg_ident, chain)?,
            )),
            rnix::ast::BinOpKind::NotEqual => Ok(Pred::Not(Box::new(Pred::Eq(
                lower_value_expr_chained(&lhs_node, cfg_ident, chain)?,
                lower_value_expr_chained(&rhs_node, cfg_ident, chain)?,
            )))),
            _ => Err(ResolveFailure::UnsupportedExpression),
        };
    }
    if let Some(un) = rnix::ast::UnaryOp::cast(node.clone()) {
        return if un.operator() == Some(rnix::ast::UnaryOpKind::Invert) {
            let inner = un
                .expr()
                .ok_or(ResolveFailure::UnsupportedExpression)?
                .syntax()
                .clone();
            Ok(Pred::Not(Box::new(lower_pred_chained(
                &inner, cfg_ident, chain,
            )?)))
        } else {
            Err(ResolveFailure::UnsupportedExpression)
        };
    }
    if node.kind() == NODE_SELECT {
        let v = lower_value_expr_chained(&node, cfg_ident, chain)?;
        return match v {
            ValueExpr::Ref(_) => Ok(Pred::Eq(v, ValueExpr::Literal(Scalar::Bool(true)))),
            ValueExpr::Literal(_) => Err(ResolveFailure::UnsupportedExpression),
        };
    }
    if node.kind() == NODE_IDENT {
        let name = ident_text(&node).ok_or(ResolveFailure::UnsupportedExpression)?;
        if name == "true" || name == "false" {
            // A stray `if true then ...`/`if false then ...` isn't a
            // real option-branch predicate -- left unresolved rather
            // than fabricating trivial evidence, but as `TrivialConstant`
            // specifically: its outcome is fully known (it's the literal
            // itself), so it carries none of the uncertainty a genuine
            // `UnsupportedExpression` does.
            return Err(ResolveFailure::TrivialConstant);
        }
        return resolve_alias_recursively(&node, &name, chain, |bound, chain| {
            lower_pred_chained(bound, cfg_ident, chain)
        });
    }
    Err(ResolveFailure::UnsupportedExpression)
}

/// An H2 predicate found and successfully lowered to the pure `Pred` IR
/// -- unlike H1's `Predicate` (which only ever represents a single
/// `cfg.<path>` unary check), `ir` may be a compound expression over
/// multiple options, and `refs` lists every one of them.
#[derive(Serialize, Debug, Clone, PartialEq)]
struct ResolvedPredicate {
    ir: Pred,
    refs: Vec<OptionPath>,
    span: Span,
    source: String,
}

/// A branch-condition site `scan_resolved_predicates` found but couldn't
/// lower to the pure IR -- reviewed and made visible on purpose, not
/// silently dropped: an unrelated helper function's `if isInt v then ...`
/// (unrelated to any option) is syntactically indistinguishable, without
/// deeper analysis, from a genuine option-branch predicate this resolver
/// simply doesn't understand yet. See `run_target`'s use of this for why
/// that distinction matters -- a target with unresolved sites can't
/// honestly claim `OBA001` ("no evidence anywhere") when part of its own
/// branch logic was never actually looked at.
#[derive(Serialize, Debug, Clone)]
struct UnresolvedPredicateSite {
    span: Span,
    source: String,
    failure: ResolveFailure,
    /// Every option path reachably referenced by the failed condition,
    /// found by `collect_reachable_refs` even though the condition as a
    /// whole couldn't lower -- may be empty (nothing cfg-rooted found
    /// anywhere reachable). This is what `run_target` matches against a
    /// specific watched option; see the doc comment on
    /// `collect_reachable_refs` for why a target-wide "any site exists"
    /// gate isn't used instead.
    refs: Vec<OptionPath>,
}

/// Finds every branch-condition site H1's `scan_predicates` also looks
/// at (`if <cond> then ...`, `lib.mkIf`/`mkIf`/`optional`/`optionals`/
/// `optionalString`/`optionalAttrs <cond> ...`) and lowers each
/// condition through `lower_pred`. Unlike `scan_predicates`, this isn't
/// restricted to conditions that are a single direct `cfg.<path>` select
/// -- `lower_pred`'s alias resolution means a condition like `if
/// mysqlLocal then ...` (a `let`-bound alias to a compound expression)
/// is found and represented here too. Conditions `lower_pred` can't
/// represent are silently skipped, same as H1's own scanner skips
/// non-`cfg`-rooted conditions -- absence here isn't itself a claim of
/// anything; `run_target` still falls back to H1's unary path first, and
/// a truly unfindable predicate still surfaces as `PredicateNotFound`.
///
/// For a *concrete* `option_prefix` (no `"*"` wildcard), each condition
/// site is additionally verified with `resolve_cfg_root(condition_site,
/// cfg_ident) == option_prefix` before being accepted -- the same
/// declaration-site safety check `scan_options` uses, applied here to
/// the predicate side of the correlation (reviewed: a `cfg`-rooted
/// select passing `lower_pred` only proves the *spelling* matches
/// `cfg_ident`, not that *this occurrence*, in its own lexical scope,
/// actually resolves to this target's `option_prefix` -- a predicate
/// found inside an unrelated shadowed scope must never be attributed to
/// this target). A wildcarded `option_prefix` (kimai's shape) skips this
/// extra check entirely and keeps trusting `cfg_ident` by spelling alone,
/// same as H1 always has -- requiring an absolute `cfg = config.a.b;`
/// proof would break the attrsOf-submodule idiom wildcarded targets rely
/// on, which was never in scope for this fix.
///
/// Returns unresolved sites alongside the resolved ones -- reviewed: an
/// earlier version silently `continue`d past whatever `lower_pred`
/// couldn't handle, the same "not found" reads as "genuinely absent"
/// silent-failure shape H1's own walker spent three review rounds
/// closing. A site scoped away by the `resolve_cfg_root` check above
/// (successfully lowered, but proven to belong to a different scope) is
/// NOT counted as unresolved -- that's a confident negative, not an
/// unknown. `ResolveFailure::TrivialConstant` (`if true then ...`) is
/// also excluded: its outcome is fully known, so unlike every other
/// failure it carries no actual risk of hiding evidence.
///
/// Every `UnresolvedPredicateSite` also records every option path
/// *reachably referenced* by the failed condition (`collect_reachable_refs`,
/// below), even though the condition as a whole couldn't be lowered --
/// this is what lets `run_target` gate a specific watched option's
/// verdict, not the whole target. Reviewed twice, both times by finding
/// a real gap empirically rather than guessing:
///
/// - First attempt: exclude a failed site unless it's inside the
///   module's `config = ...;` value. Wrong -- kimai's own `config = mkIf
///   (eachSite != { }) (mkMerge [ ... ]);` IS (part of) its `config`
///   value, so this wouldn't have excluded it at all.
/// - Second attempt: exclude a failed site unless its own text literally
///   contains the token `cfg_ident`. This correctly excluded kimai's
///   `eachSite != {}` (an existence check unrelated to any single
///   option, whose unresolvable half is a bare `{}` literal) and the
///   synthetic `if builtins.pathExists /etc/synth-baz ...` case (lives
///   inside an option's own `default = ...;`, never mentions `cfg`
///   either) -- but H2's own alias resolver made this unsound the moment
///   it shipped: a condition site can be a bare alias identifier
///   (`suspicious`) whose *binding*, not its own syntax, is what
///   actually references `cfg` (`suspicious = someUnsupportedHelper
///   cfg.database.driver;`) -- the same "alias hides the real reference"
///   shape the resolver exists to see THROUGH for successful lowerings,
///   now silently blind to it on the failure path. A target-wide gate
///   made this tolerable by accident (over-inclusion was the failure
///   mode, not under-inclusion), but per-option gating (this version)
///   cannot tolerate under-inclusion: a false negative here directly
///   produces a false `OBA001` on exactly the watched option this whole
///   project exists to protect.
///
/// Fixed by collecting refs, not by re-deciding relevance with a second
/// independent scan: `collect_reachable_refs` reuses the exact same
/// `lower_value_expr_chained`/`resolve_ident_binding`/`AliasChain`
/// primitives the successful-lowering path already uses, just with a
/// relaxed success criterion (any `Ref` found *anywhere* reachable from
/// the failed node, including through alias resolution, not "the whole
/// expression lowers"). `run_target` then matches a site's `refs`
/// against the specific watched option being evaluated -- kimai's
/// `eachSite != {}` now correctly resolves to `refs = [["sites"]]`
/// (genuinely cfg-rooted, but never matches `database.socket`) while
/// `suspicious`'s hidden `cfg.database.driver` resolves to `refs =
/// [["database","driver"]]` (correctly matches when THAT'S the watched
/// option). No target-wide fallback remains -- per-option matching
/// subsumes it exactly, without the over-inclusion the target-wide
/// version depended on.
fn collect_reachable_refs(
    node: &SyntaxNode,
    cfg_ident: &str,
    chain: &mut AliasChain,
    out: &mut Vec<OptionPath>,
) {
    match lower_value_expr_chained(node, cfg_ident, chain) {
        Ok(ValueExpr::Ref(path)) => {
            out.push(path);
            return;
        }
        Ok(ValueExpr::Literal(_)) => return,
        Err(_) => {}
    }
    if node.kind() == NODE_IDENT {
        if let Some(name) = ident_text(node) {
            if !chain.iter().any(|n| n == &name) {
                if let Ok(bound) = resolve_ident_binding(node, &name) {
                    chain.push(name);
                    collect_reachable_refs(&bound, cfg_ident, chain, out);
                    chain.pop();
                }
            }
        }
        return;
    }
    for child in node.children() {
        collect_reachable_refs(&child, cfg_ident, chain, out);
    }
}

fn scan_resolved_predicates(
    file: &str,
    src: &str,
    root: &SyntaxNode,
    cfg_ident: &str,
    option_prefix: &[String],
) -> (Vec<ResolvedPredicate>, Vec<UnresolvedPredicateSite>) {
    let mut out = Vec::new();
    let mut unresolved = Vec::new();
    let prefix_is_concrete = !option_prefix.iter().any(|s| s == "*");
    for node in root.descendants() {
        let cond = match node.kind() {
            NODE_IF_ELSE => node.children().next(),
            NODE_APPLY => {
                if node
                    .parent()
                    .map(|p| p.kind() == NODE_APPLY)
                    .unwrap_or(false)
                {
                    None
                } else {
                    let (head, args) = flatten_apply(&node);
                    call_head_name(&head)
                        .filter(|name| HELPER_NAMES.contains(&name.as_str()))
                        .and(args.into_iter().next())
                }
            }
            _ => None,
        };
        let Some(cond) = cond else { continue };
        let ir = match lower_pred(&cond, cfg_ident) {
            Ok(ir) => ir,
            Err(ResolveFailure::TrivialConstant) => continue,
            Err(failure) => {
                let mut refs = Vec::new();
                collect_reachable_refs(&cond, cfg_ident, &mut Vec::new(), &mut refs);
                unresolved.push(UnresolvedPredicateSite {
                    span: span_of(file, src, &node),
                    source: first_line(&node),
                    failure,
                    refs,
                });
                continue;
            }
        };
        if prefix_is_concrete && !option_prefix.is_empty() {
            match resolve_cfg_root(&cond, cfg_ident) {
                Ok(resolved) if resolved == option_prefix => {}
                _ => continue,
            }
        }
        let mut refs = Vec::new();
        refs_in_pred(&ir, &mut refs);
        out.push(ResolvedPredicate {
            ir,
            refs,
            span: span_of(file, src, &node),
            source: first_line(&node),
        });
    }
    (out, unresolved)
}

/// Evaluates a `ValueExpr` against a concrete environment: `env` supplies
/// the currently-known `Scalar` for every option path this evaluation
/// cares about. `None` means "this environment doesn't (yet) say", not
/// "false" -- callers must not conflate an absent lookup with a negative
/// result.
fn eval_value_expr(
    v: &ValueExpr,
    env: &std::collections::HashMap<OptionPath, KnownValue>,
) -> Option<KnownValue> {
    match v {
        ValueExpr::Literal(s) => Some(KnownValue::Exact(s.clone())),
        ValueExpr::Ref(path) => env.get(path).cloned(),
    }
}

/// Equality between two `KnownValue`s, the same fail-closed idiom as
/// everything else here: `None` means genuinely undecidable, not "assume
/// unequal". Two `Exact` values compare directly. A `DefinitelyNonNull`
/// against a known `null` is always `false` (a non-null value can never
/// equal `null`, regardless of which non-null value it actually is) --
/// but a `DefinitelyNonNull` against anything else (another
/// `DefinitelyNonNull`, or a specific non-null `Exact` value) is
/// undecidable: knowing only "not null" is not enough to know whether two
/// values are the same non-null value.
fn eval_known_eq(a: &KnownValue, b: &KnownValue) -> Option<bool> {
    match (a, b) {
        (KnownValue::Exact(x), KnownValue::Exact(y)) => Some(x == y),
        (KnownValue::DefinitelyNonNull, KnownValue::Exact(Scalar::Null))
        | (KnownValue::Exact(Scalar::Null), KnownValue::DefinitelyNonNull) => Some(false),
        _ => None,
    }
}

/// Evaluates a `Pred` against a concrete environment. `Eq`/`Not` propagate
/// `None` fail-closed the obvious way. `And`/`Or` use strong (Kleene)
/// three-valued logic instead of requiring *both* operands to resolve: a
/// known `false` operand pins `And`'s result to `false` regardless of
/// whether the other operand resolves, and symmetrically a known `true`
/// operand pins `Or` to `true`. This is sound, not just convenient: real
/// Nix's `a && b` either short-circuits on a `false` `a` without forcing
/// `b` at all, or forces `b` and gets exactly the value we already
/// statically know it to have -- both branches land on `false`, so which
/// one Nix actually takes at runtime doesn't change the answer. (An
/// earlier version of this function instead required `?` on both
/// operands unconditionally, reasoning that was the safer fail-closed
/// choice -- reviewed and corrected: refusing to conclude anything from a
/// known-false operand was needlessly pessimistic, not actually safer,
/// since the conclusion holds regardless of the unresolved operand's real
/// value.) Only when neither operand pins the result down (e.g.
/// `And(unknown, true)`, or a genuinely unresolved operand on both sides)
/// does the combinator itself become unresolved.
fn eval_pred(p: &Pred, env: &std::collections::HashMap<OptionPath, KnownValue>) -> Option<bool> {
    match p {
        Pred::Eq(a, b) => {
            let a = eval_value_expr(a, env)?;
            let b = eval_value_expr(b, env)?;
            eval_known_eq(&a, &b)
        }
        Pred::Not(inner) => eval_pred(inner, env).map(|b| !b),
        Pred::And(a, b) => match (eval_pred(a, env), eval_pred(b, env)) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (Some(true), Some(true)) => Some(true),
            _ => None,
        },
        Pred::Or(a, b) => match (eval_pred(a, env), eval_pred(b, env)) {
            (Some(true), _) | (_, Some(true)) => Some(true),
            (Some(false), Some(false)) => Some(false),
            _ => None,
        },
    }
}

/// Every `OptionPath` a `Pred` references (via `ValueExpr::Ref`), in
/// traversal order, duplicates included -- callers that need a set
/// dedupe themselves. Used by the counterfactual gate to know exactly
/// which options a predicate's evaluation depends on before building an
/// environment for it.
fn refs_in_pred(p: &Pred, out: &mut Vec<OptionPath>) {
    match p {
        Pred::Eq(a, b) => {
            for v in [a, b] {
                if let ValueExpr::Ref(path) = v {
                    out.push(path.clone());
                }
            }
        }
        Pred::Not(inner) => refs_in_pred(inner, out),
        Pred::And(a, b) | Pred::Or(a, b) => {
            refs_in_pred(a, out);
            refs_in_pred(b, out);
        }
    }
}

// ---------------------------------------------------------------------
// Test-file leaf assignment discovery (test.nix): walks AttrSets,
// transparently unwrapping Lambda bodies (containers.X = { ... }: { ... };),
// accumulating full dotted paths, recording every non-attrset leaf.
// ---------------------------------------------------------------------

#[derive(Serialize, Debug, Clone, PartialEq)]
struct TestAssignment {
    path: Vec<String>,
    value_source: String,
    value_class: ValueClass,
    /// H2: the exact `KnownValue` (preserving literal content, e.g. the
    /// actual string `"mysql"`, not just "definitely non-null") when
    /// classifiable -- `value_class` alone isn't enough for the
    /// counterfactual gate, which needs to evaluate equality against
    /// *specific* literals like `db.driver == "mysql"`, not just
    /// null-ness.
    known_value: Option<KnownValue>,
    /// nearest enclosing `containers.<x>` / `nodes.<x>` instance name, if any
    instance: Option<String>,
    span: Span,
}

/// A point in the test file's config where the walker had to stop looking
/// -- not because there was nothing there, but because what's there isn't
/// something it can see into: `imports = [ ./common.nix ];` (config lives
/// in a file this tool never reads), a bare identifier alias
/// (`nodes.machine = machineConfig;`), a function call (`mkMachineConfig
/// {...}`, `mkMerge [...]`), or similar. `path` is the scope this opacity
/// applies to: everything at or below it in the real option namespace may
/// exist in the parts of the test config this walker couldn't see, so a
/// watched option whose absolute path starts with `path` can't be safely
/// declared un-activated just because no matching assignment was *found*.
#[derive(Serialize, Debug, Clone)]
struct Opacity {
    path: Vec<String>,
    instance: Option<String>,
    reason: &'static str,
    span: Span,
}

/// Would `node`'s value possibly contain further nested option keys if we
/// could see into it? `false` for anything that's syntactically incapable
/// of that (a string/number/list/path literal, or the literals
/// `null`/`true`/`false`) -- those can't hide a `.bar` under them no
/// matter what they evaluate to. `true` for everything else this walker
/// doesn't already descend into directly (a bare identifier possibly
/// aliasing an attrset, a function application, a conditional, a `with`,
/// a `let`, ...): syntactically undecidable whether more structure is
/// hiding there, so treated as "might be", not "isn't".
fn could_contain_nested_options(node: &SyntaxNode) -> bool {
    match node.kind() {
        NODE_STRING | NODE_LITERAL | NODE_LIST | NODE_PATH => false,
        NODE_IDENT => !matches!(
            ident_text(node).as_deref(),
            Some("null") | Some("true") | Some("false")
        ),
        _ => true,
    }
}

// H1.3 review: the walker is a small explicit state machine, not a single
// function accreting `if`s. Three distinct contexts, each with genuinely
// different rules about what a key like `config` or a bare shorthand
// assignment *means*:
//
//   TestSpecRoot        -- top of the test file. Only `nodes`/`containers`
//                           (flat `nodes.foo = ...` or nested `nodes = {
//                           foo = ...; };`, both real nixosTest idioms) are
//                           option-relevant; everything else (`name`,
//                           `meta`, `testScript`) is harness metadata, not
//                           option data, and is intentionally not recorded.
//   ModuleRoot(instance) -- inside one resolved node/container's own
//                           NixOS-module value. `imports` and `config` are
//                           special here specifically because this is a
//                           module root (a real `containers.peer.config`
//                           *option* one level down, inside ConfigTree, is
//                           NOT the same key and must NOT be special-cased
//                           the same way -- that's the whole reason this
//                           needs to be a context, not a global string
//                           check). `config = { ... }` normalizes into the
//                           same option-path namespace as top-level
//                           shorthand definitions (`config.services.foo`
//                           and bare `services.foo` at module root are the
//                           same option), not a `config.*`-prefixed one.
//   ConfigTree(instance) -- ordinary recursive descent into option paths,
//                           entered either from ModuleRoot's shorthand
//                           entries or from an explicit `config = {...}`.
//
// The one invariant that must hold everywhere: every syntactic form either
// resolves to a known `TestAssignment`, or leaves an explicit `Opacity`.
// There is no third "silently continue" state for anything that could be
// option-relevant data -- `inherit`, a dynamic `${...}` key, an
// unrecognized test-file root expression, an un-analyzable node config,
// all become `Opacity`, not silence.

fn scan_test_assignments(
    file: &str,
    src: &str,
    root: &SyntaxNode,
) -> (Vec<TestAssignment>, Vec<Opacity>) {
    let mut assignments = Vec::new();
    let mut opacity = Vec::new();
    let body = resolve_test_root(unwrap_lambda_chain(root.clone()));
    if body.kind() == NODE_ATTR_SET {
        walk_test_spec_root(file, src, &body, &mut assignments, &mut opacity);
    } else {
        // Neither a literal attrset nor a recognized wrapper (see
        // resolve_test_root) -- e.g. some other helper function entirely.
        // Root-scope opacity: since we don't know the shape at all, any
        // watched option anywhere is potentially hidden in it.
        opacity.push(Opacity {
            path: Vec::new(),
            instance: None,
            reason: "test file's root expression is neither a literal attrset nor a recognized wrapper (e.g. import ./make-test-python.nix (...)) -- entirely unanalyzable",
            span: span_of(file, src, &body),
        });
    }
    (assignments, opacity)
}

/// Transparently unwraps `NODE_ROOT`, `NODE_LAMBDA` (`{ ... }: body`), and
/// `NODE_LET_IN` (`let ...bindings...; in body`) down to whatever's
/// underneath -- all three have the pattern "the meaningful continuation
/// is the last node child", regardless of how many `let` bindings or
/// lambda pattern entries precede it. `let ... in { ... }` is a common,
/// completely legitimate nixosTest idiom (locally naming a value before
/// using it); missing it isn't a syntax edge case, it's an entire
/// mainstream pattern the walker would otherwise just silently stop at --
/// caught when `c16-instance-alias`'s `let machineConfig = {...}; in {
/// nodes.machine = machineConfig; }` produced a NODE_LET_IN root that
/// `scan_test_assignments` never even started walking, before this
/// function grew a `NODE_LET_IN` case to go with `NODE_ROOT`/`NODE_LAMBDA`.
fn unwrap_lambda_chain(node: SyntaxNode) -> SyntaxNode {
    if node.kind() == NODE_ROOT {
        if let Some(child) = node.children().next() {
            return unwrap_lambda_chain(child);
        }
    }
    if node.kind() == NODE_LAMBDA || node.kind() == NODE_LET_IN {
        if let Some(body) = node.children().last() {
            return unwrap_lambda_chain(body);
        }
    }
    node
}

/// Recognizes the single most common nixosTest wrapper across nixpkgs
/// (559 files under `nixos/tests` use `nodes = {`, 95 use this wrapper
/// directly): `import ./make-test-python.nix (<arg>)`, where `<arg>` is
/// either the test attrset directly or a lambda producing it. Anything
/// past the first two arguments (e.g. a trailing `{ inherit pkgs; }`
/// application) is ignored -- the test spec itself is always the second
/// argument to `import`, regardless of what's chained after. Falls
/// through unchanged (to be caught as root opacity by the caller) for
/// anything else, rather than guessing.
fn resolve_test_root(node: SyntaxNode) -> SyntaxNode {
    if node.kind() == NODE_ATTR_SET {
        return node;
    }
    if node.kind() == NODE_APPLY {
        let (head, args) = flatten_apply(&node);
        if call_head_name(&head).as_deref() == Some("import") {
            if let Some(spec) = args.get(1) {
                return resolve_test_root(unwrap_lambda_chain(spec.clone()));
            }
        }
    }
    node
}

/// H1.3a review: the set of test-spec-root keys confirmed, by reading the
/// real schema (`nixos/lib/testing/*.nix` in nixpkgs: driver.nix, meta.nix,
/// testScript.nix, name.nix, run.nix), to be structurally incapable of
/// carrying NixOS module config -- a string, a bool, a fixed-shape metadata
/// attrset, a function over packages, or similar. Deliberately NOT on this
/// list, because each one genuinely does carry option-relevant module
/// config in real tests (verified in the same source): `machine` (the
/// single-node shorthand for `nodes.machine`), `interactive` (documented to
/// accept `interactive.nodes.<x> = {...}` overrides -- see
/// nixos/lib/testing/interactive.nix's own doc example), `defaults`,
/// `nodeDefaults`, `containerDefaults`, `extraBaseModules`,
/// `extraBaseNodeModules` (all four are explicitly "NixOS configuration
/// applied to all nodes/containers" per nodes.nix). Getting this allowlist
/// wrong in the unsafe direction is exactly the bug this pass exists to
/// close, so anything not independently confirmed safe stays OFF it.
const KNOWN_HARNESS_METADATA_KEYS: &[&str] = &[
    "name",
    "meta",
    "testScript",
    "testScriptString",
    "includeTestScriptReferences",
    "withoutTestScriptReferences",
    "enableOCR",
    "skipLint",
    "skipTypeCheck",
    "logLevel",
    "globalTimeout",
    "extraPythonPackages",
    "extraDriverArgs",
    "hostPkgs",
    "passthru",
    "requiredFeatures",
    "kvm",
    "devnet",
];

/// Shared with `run_census`'s `unclassified_root_entries` counter, so the
/// two can't drift apart.
const REASON_UNCLASSIFIED_ROOT_ENTRY: &str = "unrecognized test-spec-root entry -- not a known harness-metadata key and not nodes/containers; may itself define test scenario config (e.g. `name = runTest { nodes = ...; };`) this walker can't see into";

/// H1.3b review: the same problem as `REASON_UNCLASSIFIED_ROOT_ENTRY`, one
/// level down -- an entry under the NESTED `nodes = { ... };` form that
/// isn't a plain single-segment instance name (e.g. a multi-segment
/// attrpath like `hidden.services.synth.foo = null;` sitting next to a
/// normal `machine = ...;`). Shared with `run_census`'s
/// `unclassified_instance_entries` counter.
const REASON_UNCLASSIFIED_NESTED_INSTANCE_ENTRY: &str = "unrecognized entry under nested nodes/containers -- not a single-segment instance name; may itself carry option-relevant config this walker can't see into (e.g. a multi-segment attrpath)";

/// TestSpecRoot: finds `nodes`/`containers` bindings, both forms --
/// `nodes.foo = ...;` (flat) and `nodes = { foo = ...; bar = ...; };`
/// (nested, equally common in real nixosTests). Every OTHER top-level key
/// is checked against `KNOWN_HARNESS_METADATA_KEYS`: a confirmed-safe key
/// is genuinely not option data and is skipped; anything else gets its own
/// `Opacity` record. H1.3a review: the previous version treated "not
/// nodes/containers" as sufficient proof of "harmless metadata", which is
/// false -- e.g. `hiddenScenario = runTest { nodes.other = {...}: {
/// services.synth.foo = null; }; };` sitting next to a normal
/// `nodes.machine` was silently discarded as if it were `name`/`meta`,
/// even though it's a second, fully live test scenario this walker can't
/// see into (c25).
fn walk_test_spec_root(
    file: &str,
    src: &str,
    attrset: &SyntaxNode,
    out: &mut Vec<TestAssignment>,
    opacity: &mut Vec<Opacity>,
) {
    // Discovered by the H1.3 syntax-visibility census, not anticipated in
    // advance: a real, non-rare nixosTest convention has NO top-level
    // `nodes`/`containers` at all -- e.g. `{ justThePackage = runTest {
    // nodes.machine = ...; ...}; defaults = runTest { ... }; }` (multiple
    // independent scenarios, each a `runTest`/`makeTest` call keyed by
    // name) or a single `<name> = makeTest { nodes = ...; };` wrapper.
    // Before this flag, a file shaped like that produced neither an
    // assignment nor an opacity site -- a silent "nothing here" that's
    // actually "an entire convention this walker doesn't parse into" --
    // which is exactly the failure mode this whole review round exists to
    // eliminate. This case is now also covered per-entry (below, via
    // REASON_UNCLASSIFIED_ROOT_ENTRY), so this whole-file fallback now only
    // fires for the genuinely-empty-attrset edge case; kept as a backstop
    // rather than removed, since "zero entries at all" isn't reachable
    // through the per-entry loop below.
    let mut saw_any_entry = false;

    for entry in attrset.children() {
        if entry.kind() == NODE_INHERIT {
            saw_any_entry = true;
            opacity.push(Opacity {
                path: Vec::new(),
                instance: None,
                reason: "inherit binding at test spec root -- may pull in option-relevant values this walker can't trace",
                span: span_of(file, src, &entry),
            });
            continue;
        }
        if entry.kind() != NODE_ATTRPATH_VALUE {
            continue;
        }
        saw_any_entry = true;
        let mut children = entry.children();
        let Some(attrpath) = children.next() else {
            continue;
        };
        let Some(segs) = attrpath_segments(&attrpath) else {
            opacity.push(Opacity {
                path: Vec::new(),
                instance: None,
                reason: "dynamic (\"${...}\") attribute name at test spec root -- can't be statically resolved",
                span: span_of(file, src, &entry),
            });
            continue;
        };
        let Some(value) = children.next() else {
            continue;
        };

        if segs.len() == 2 && (segs[0] == "nodes" || segs[0] == "containers") {
            enter_instance(file, src, &segs[1], &value, &entry, out, opacity);
            continue;
        }

        if segs.len() == 1 && (segs[0] == "nodes" || segs[0] == "containers") {
            let body = unwrap_lambda_chain(value.clone());
            if body.kind() != NODE_ATTR_SET {
                opacity.push(Opacity {
                    path: Vec::new(),
                    instance: None,
                    reason: "nodes/containers value is not a literal attrset",
                    span: span_of(file, src, &entry),
                });
                continue;
            }
            for inst_entry in body.children() {
                // H1.3b review: this whole inner loop is a close relative
                // of the H1.3a root-entry bug, one level down -- an
                // unrecognized shape here was silently dropped instead of
                // opacity'd, and unlike the root case, a normal sibling
                // instance being present (e.g. `machine`) means neither
                // `found_any_instance`-style fallbacks nor
                // `REASON_UNCLASSIFIED_ROOT_ENTRY` ever see it: it's
                // already inside the recognized `nodes = { ... };` value.
                if inst_entry.kind() == NODE_INHERIT {
                    opacity.push(Opacity {
                        path: Vec::new(),
                        instance: None,
                        reason: "inherit binding under nested nodes/containers -- may pull in option-relevant values this walker can't trace",
                        span: span_of(file, src, &inst_entry),
                    });
                    continue;
                }
                if inst_entry.kind() != NODE_ATTRPATH_VALUE {
                    continue;
                }
                let mut ic = inst_entry.children();
                let Some(inst_attrpath) = ic.next() else {
                    continue;
                };
                let Some(inst_segs) = attrpath_segments(&inst_attrpath) else {
                    opacity.push(Opacity {
                        path: Vec::new(),
                        instance: None,
                        reason: "dynamic (\"${...}\") instance name under nodes/containers",
                        span: span_of(file, src, &inst_entry),
                    });
                    continue;
                };
                let Some(inst_value) = ic.next() else {
                    continue;
                };
                if inst_segs.len() == 1 {
                    enter_instance(
                        file,
                        src,
                        &inst_segs[0],
                        &inst_value,
                        &inst_entry,
                        out,
                        opacity,
                    );
                } else {
                    // A multi-segment attrpath under `nodes = { ... };`
                    // (e.g. `hidden.services.synth.foo = null;`) is not a
                    // single instance-name binding this walker understands
                    // -- silently falling through here (the H1.3b bug) let
                    // a fully live sibling scenario's assignment vanish
                    // with no trace, exactly like the root-level version
                    // of this bug H1.3a just fixed one level up (c25/c26).
                    opacity.push(Opacity {
                        path: Vec::new(),
                        instance: None,
                        reason: REASON_UNCLASSIFIED_NESTED_INSTANCE_ENTRY,
                        span: span_of(file, src, &inst_entry),
                    });
                }
            }
            continue;
        }

        // Neither a nodes/containers binding nor a confirmed-safe
        // metadata key: H1.3a review -- must not assume an unrecognized
        // shape is harmless just because it isn't nodes/containers. Could
        // be another live test scenario (`hiddenScenario = runTest {
        // nodes.other = ...; };`), a `machine` shorthand, an `interactive`
        // override, or something not anticipated at all.
        if !KNOWN_HARNESS_METADATA_KEYS.contains(&segs[0].as_str()) {
            opacity.push(Opacity {
                path: Vec::new(),
                instance: None,
                reason: REASON_UNCLASSIFIED_ROOT_ENTRY,
                span: span_of(file, src, &entry),
            });
        }
    }

    // Only reachable now when the attrset has zero real entries at all
    // (every unrecognized non-empty case already got its own per-entry
    // Opacity above) -- kept as a backstop for that edge, not the general
    // mechanism it used to be.
    if !saw_any_entry {
        opacity.push(Opacity {
            path: Vec::new(),
            instance: None,
            reason: "test spec root attrset has no entries at all",
            span: span_of(file, src, attrset),
        });
    }
}

/// Resolves one `nodes.<name>` / `containers.<name>` (or nested-form
/// equivalent) instance's value: if it's a literal attrset, that's a real
/// module root to walk; otherwise the whole instance is opaque (an alias,
/// a function call, ...) and nothing under it can be safely called
/// un-activated.
fn enter_instance(
    file: &str,
    src: &str,
    name: &str,
    value: &SyntaxNode,
    entry: &SyntaxNode,
    out: &mut Vec<TestAssignment>,
    opacity: &mut Vec<Opacity>,
) {
    let body = unwrap_lambda_chain(value.clone());
    if body.kind() == NODE_ATTR_SET {
        walk_module_root(file, src, &body, Some(name.to_string()), out, opacity);
    } else {
        opacity.push(Opacity {
            path: Vec::new(),
            instance: Some(name.to_string()),
            reason: "instance config is not a literal attrset (alias, function call, or similar)",
            span: span_of(file, src, entry),
        });
    }
}

/// ModuleRoot(instance): `imports` and `config` are special *here*,
/// specifically because this is the top of one node's own NixOS module --
/// the same two keys one level deeper (inside ConfigTree, i.e. already
/// inside a real option's value, such as `containers.peer.config` being a
/// genuine systemd-container option) are NOT module syntax and must NOT
/// be intercepted the same way. That asymmetry is exactly why this needs
/// to be a distinct context rather than a blanket "key is named config"
/// check anywhere in the tree.
fn walk_module_root(
    file: &str,
    src: &str,
    attrset: &SyntaxNode,
    instance: Option<String>,
    out: &mut Vec<TestAssignment>,
    opacity: &mut Vec<Opacity>,
) {
    for entry in attrset.children() {
        if entry.kind() == NODE_INHERIT {
            opacity.push(Opacity {
                path: Vec::new(),
                instance: instance.clone(),
                reason: "inherit binding at module root -- may pull in option-relevant values this walker can't trace",
                span: span_of(file, src, &entry),
            });
            continue;
        }
        if entry.kind() != NODE_ATTRPATH_VALUE {
            continue;
        }
        let mut children = entry.children();
        let Some(attrpath) = children.next() else {
            continue;
        };
        let Some(segs) = attrpath_segments(&attrpath) else {
            opacity.push(Opacity {
                path: Vec::new(),
                instance: instance.clone(),
                reason: "dynamic (\"${...}\") attribute name at module root -- can't be statically resolved to an option path",
                span: span_of(file, src, &entry),
            });
            continue;
        };
        let Some(value) = children.next() else {
            continue;
        };

        if segs == ["imports"] {
            opacity.push(Opacity {
                path: Vec::new(),
                instance: instance.clone(),
                reason: "imports present -- config for this scope may live in an unread file",
                span: span_of(file, src, &entry),
            });
            continue;
        }

        if segs == ["config"] {
            let body = unwrap_lambda_chain(value.clone());
            if body.kind() == NODE_ATTR_SET {
                walk_config_tree(
                    file,
                    src,
                    &body,
                    &mut Vec::new(),
                    instance.clone(),
                    out,
                    opacity,
                );
            } else if could_contain_nested_options(&body) {
                opacity.push(Opacity {
                    path: Vec::new(),
                    instance: instance.clone(),
                    reason:
                        "config = <non-literal expression> at module root -- may hide any option",
                    span: span_of(file, src, &entry),
                });
            }
            continue;
        }

        // Anything else at module root is shorthand equivalent to
        // `config.<path> = value;` -- the same option-path namespace as
        // an explicit `config = {...}` block, not a `config.*`-prefixed
        // one and not this instance's own top-level namespace either.
        walk_config_entry(
            file,
            src,
            &segs,
            &value,
            &entry,
            &mut Vec::new(),
            instance.clone(),
            out,
            opacity,
        );
    }
}

/// ConfigTree(instance): ordinary recursive descent into a real option
/// tree, reached either from ModuleRoot's shorthand definitions or from
/// an explicit `config = {...}`. Both routes end up here so option paths
/// are normalized identically either way.
fn walk_config_tree(
    file: &str,
    src: &str,
    attrset: &SyntaxNode,
    path: &mut Vec<String>,
    instance: Option<String>,
    out: &mut Vec<TestAssignment>,
    opacity: &mut Vec<Opacity>,
) {
    for entry in attrset.children() {
        if entry.kind() == NODE_INHERIT {
            opacity.push(Opacity {
                path: path.clone(),
                instance: instance.clone(),
                reason:
                    "inherit binding -- may pull in option-relevant values this walker can't trace",
                span: span_of(file, src, &entry),
            });
            continue;
        }
        if entry.kind() != NODE_ATTRPATH_VALUE {
            continue;
        }
        let mut children = entry.children();
        let Some(attrpath) = children.next() else {
            continue;
        };
        let Some(segs) = attrpath_segments(&attrpath) else {
            opacity.push(Opacity {
                path: path.clone(),
                instance: instance.clone(),
                reason: "dynamic (\"${...}\") attribute name -- can't be statically resolved to an option path",
                span: span_of(file, src, &entry),
            });
            continue;
        };
        let Some(value) = children.next() else {
            continue;
        };
        walk_config_entry(
            file,
            src,
            &segs,
            &value,
            &entry,
            path,
            instance.clone(),
            out,
            opacity,
        );
    }
}

/// Shared by ModuleRoot's shorthand branch and ConfigTree's recursive
/// descent: extends `path` by `segs`, recurses into a nested attrset, or
/// records a leaf `TestAssignment` (plus opacity if the leaf's value
/// could itself be hiding further structure).
fn walk_config_entry(
    file: &str,
    src: &str,
    segs: &[String],
    value: &SyntaxNode,
    entry: &SyntaxNode,
    path: &mut Vec<String>,
    instance: Option<String>,
    out: &mut Vec<TestAssignment>,
    opacity: &mut Vec<Opacity>,
) {
    path.extend(segs.iter().cloned());
    let body = unwrap_lambda_chain(value.clone());

    if body.kind() == NODE_ATTR_SET {
        walk_config_tree(file, src, &body, path, instance.clone(), out, opacity);
    } else {
        if could_contain_nested_options(&body) {
            opacity.push(Opacity {
                path: path.clone(),
                instance: instance.clone(),
                reason: "value is not a literal (alias, function call, or similar) -- may contain further nested options this walker can't see",
                span: span_of(file, src, entry),
            });
        }
        out.push(TestAssignment {
            path: path.clone(),
            value_source: body.text().to_string(),
            value_class: classify_value(&body),
            known_value: classify_known_value(&body),
            instance,
            span: span_of(file, src, entry),
        });
    }

    for _ in 0..segs.len() {
        path.pop();
    }
}

// ---------------------------------------------------------------------
// Matching + verdict
// ---------------------------------------------------------------------

/// Which predicate model actually produced a verdict: H1's original
/// unary `Predicate` (a single `cfg.<path>` check), or H2's compound
/// `ResolvedPredicate` (a boolean expression over one or more options,
/// reached through `lower_pred` and possibly alias resolution). `run_target`
/// always tries the H1 path first; `PredicateRef::Resolved` only ever
/// appears when H1's own unary scan found nothing for the watched option.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum PredicateRef {
    Unary(Predicate),
    Resolved(ResolvedPredicate),
}

impl PredicateRef {
    fn span(&self) -> &Span {
        match self {
            PredicateRef::Unary(p) => &p.span,
            PredicateRef::Resolved(p) => &p.span,
        }
    }

    fn source(&self) -> &str {
        match self {
            PredicateRef::Unary(p) => &p.source,
            PredicateRef::Resolved(p) => &p.source,
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(tag = "verdict")]
enum Verdict {
    /// The module has no `mkOption { ... }` declaration for this watched
    /// path at all. A prerequisite for everything downstream: PASS must
    /// never be reachable without the declaration scanner having actually
    /// found the option (previously it wasn't required -- a silently dead
    /// declaration scanner couldn't have blocked a PASS).
    OptionNotFound { option: String },
    /// The declaration was found, but no direct `cfg.<path>` branch
    /// predicate was found in the module (e.g. hidden behind a `let`-bound
    /// alias -- out of MVP scope, and intentionally never folded into a
    /// PASS, so an aliased predicate can't masquerade as a clean bill of
    /// health).
    PredicateNotFound { option: String },
    /// The predicate was found, but its declared default value's boolean
    /// outcome under this predicate can't be statically classified (e.g. a
    /// `Truthy` predicate whose default is some non-literal expression, not
    /// bare `true`/`false`). Without a default outcome there is nothing to
    /// prove a *transition* away from, so no verdict is possible.
    DefaultUnresolved {
        option: String,
        predicate: PredicateRef,
    },
    /// A classifiable default was found, and at least one structurally
    /// matching test assignment exists, but its value's outcome under this
    /// predicate can't be statically classified either (an expression, an
    /// alias, ...) -- and none of the *other* matching assignments (if
    /// any) is a known opposite-outcome value. The honest answer here is
    /// "can't tell", not "assume it doesn't count": an unresolvable test
    /// value might well be the one that actually flips the branch (see the
    /// H1.1 review's `builtins.elem "x" [ "x" "y" ]` example, which *is*
    /// `true` at runtime but is statically opaque). Silently treating
    /// `None` as "no evidence" was exactly this tool's own fail-closed
    /// principle broken on the test-value side after fixing it on the
    /// default side in H1.
    /// `default_outcome` is `Option` (not `bool`, unlike `Pass`) because
    /// H2's per-instance counterfactual model has no single instance-
    /// independent "the" default outcome the way H1's unary model always
    /// did -- other options referenced by a compound predicate can differ
    /// per instance, so "the default outcome" is only well-defined
    /// relative to a *specific* instance (as it is for `Pass`, reporting
    /// the witnessing instance's own). When no instance produced one,
    /// this is honestly `None`, not a guessed/global value.
    TestValueUnresolved {
        option: String,
        predicate: PredicateRef,
        default_outcome: Option<bool>,
        predicate_attempts: Vec<PredicateAttempt>,
    },
    /// A classifiable default was found, and no known opposite-outcome
    /// evidence exists -- but somewhere in the test config that could
    /// structurally contain this option (i.e. whose path is a prefix of
    /// the option's full absolute path), the walker hit something it
    /// can't see into: `imports`, an alias, a function call. The correct
    /// claim here is "not observed in the part of the config this tool
    /// can read", not "not activated" -- a silent absence caused by
    /// incomplete visibility is a different fact from a silent absence
    /// because the option genuinely was never touched, and OBA001 must
    /// never conflate them (H1.2 review: without this, a census over real
    /// modules would produce OBA001 findings that are actually just
    /// "config came in through `imports` and my walker never saw it").
    /// Known opposite-outcome evidence found *elsewhere* still wins over
    /// this -- see `c17`: an explicit, provable transition is a stronger
    /// claim than "some unrelated import exists", not a weaker one.
    TestConfigUnresolved {
        option: String,
        predicate: PredicateRef,
        default_outcome: Option<bool>,
        /// H2 only (empty for H1's unary path -- see `PredicateAttempt`'s
        /// own doc comment).
        predicate_attempts: Vec<PredicateAttempt>,
    },
    /// A predicate and a classifiable default were found, but no test
    /// assignment's value provably flips the predicate's outcome away from
    /// what the default produces.
    #[serde(rename = "OBA001")]
    Oba001 {
        option: String,
        predicate: PredicateRef,
        default_outcome: Option<bool>,
        predicate_attempts: Vec<PredicateAttempt>,
    },
    /// A predicate and a classifiable default were found, and at least one
    /// test assignment's value provably evaluates the predicate to the
    /// *opposite* outcome from the default -- i.e. the branch is proven to
    /// have been taken down a different path than it would with no
    /// configuration at all.
    #[serde(rename = "PASS")]
    Pass {
        option: String,
        predicate: PredicateRef,
        default_outcome: bool,
        evidence: Vec<TestAssignment>,
        predicate_attempts: Vec<PredicateAttempt>,
    },
}

/// H2: one option can be referenced by more than one branch predicate
/// (davis's `database.driver` is referenced by a simple `db.driver ==
/// "sqlite"` check AND by the compound `mysqlLocal` alias, among others)
/// -- "the predicate" stopped being a well-defined singular concept the
/// moment H2 could see more than one. Reviewed and fixed: `run_target`'s
/// H2 path now evaluates *every* predicate referencing a watched option
/// (not just the first found in document order, which was an accident of
/// AST traversal order, not a real semantic choice), and this records
/// what happened with each one so a PASS never silently hides that a
/// *different* predicate on the same option was inconclusive, and an
/// inconclusive verdict never silently hides that some OTHER predicate on
/// the same option already had real (if non-transitioning) evidence.
/// Always empty for verdicts produced by H1's original unary path -- that
/// model only ever considers exactly one predicate by construction, so
/// there is nothing this field would add.
#[derive(Serialize, Debug, Clone, PartialEq)]
struct PredicateAttempt {
    predicate: PredicateRef,
    /// `Some(true)`: at least one test instance witnessed a provable
    /// transition through this predicate (this is the predicate a `PASS`
    /// verdict's own top-level `predicate`/`evidence` fields describe, if
    /// it's the one that won; other witnessing predicates, if any, are
    /// listed here too, not silently dropped just because one was picked
    /// as primary). `Some(false)`: real evidence existed for this
    /// predicate (every attempted instance fully resolved) but no
    /// instance ever demonstrated a transition. `None`: every attempt at
    /// this predicate was unresolved (a required value or context was
    /// missing).
    witnessed: Option<bool>,
}

impl Verdict {
    /// "We couldn't establish a verdict" -- distinct from, and just as
    /// loud as, "we established a verdict and it's bad". A CI that only
    /// alarms on OBA001 and treats every inconclusive silently as green is
    /// exactly the failure mode `watch` was added to prevent.
    fn is_inconclusive(&self) -> bool {
        matches!(
            self,
            Verdict::OptionNotFound { .. }
                | Verdict::PredicateNotFound { .. }
                | Verdict::DefaultUnresolved { .. }
                | Verdict::TestValueUnresolved { .. }
                | Verdict::TestConfigUnresolved { .. }
        )
    }

    fn is_finding(&self) -> bool {
        matches!(self, Verdict::Oba001 { .. })
    }

    /// The watched option's dot-joined path -- every variant carries one,
    /// but Rust can't project a field shared across enum variants without
    /// a match. Part of a target identity's `watched_path` component
    /// (PR D's `compare()`).
    fn option(&self) -> &str {
        match self {
            Verdict::OptionNotFound { option }
            | Verdict::PredicateNotFound { option }
            | Verdict::DefaultUnresolved { option, .. }
            | Verdict::TestValueUnresolved { option, .. }
            | Verdict::TestConfigUnresolved { option, .. }
            | Verdict::Oba001 { option, .. }
            | Verdict::Pass { option, .. } => option,
        }
    }

    /// The bare discriminant, ignoring every payload field (predicate,
    /// evidence, spans, ...) -- what `compare()` considers "the verdict"
    /// for `ChangeKind::VerdictChanged` purposes. Two verdicts of the
    /// same kind but different evidence are NOT a verdict change in v1
    /// (that would be `EvidenceChanged`, reserved but not computed yet).
    fn kind(&self) -> VerdictKind {
        match self {
            Verdict::OptionNotFound { .. } => VerdictKind::OptionNotFound,
            Verdict::PredicateNotFound { .. } => VerdictKind::PredicateNotFound,
            Verdict::DefaultUnresolved { .. } => VerdictKind::DefaultUnresolved,
            Verdict::TestValueUnresolved { .. } => VerdictKind::TestValueUnresolved,
            Verdict::TestConfigUnresolved { .. } => VerdictKind::TestConfigUnresolved,
            Verdict::Oba001 { .. } => VerdictKind::Oba001,
            Verdict::Pass { .. } => VerdictKind::Pass,
        }
    }
}

/// `Verdict`'s bare discriminant -- see `Verdict::kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VerdictKind {
    OptionNotFound,
    PredicateNotFound,
    DefaultUnresolved,
    TestValueUnresolved,
    TestConfigUnresolved,
    Oba001,
    Pass,
}

impl VerdictKind {
    /// `oba diff`'s JSON `verdict_transitions` summary key component --
    /// e.g. `"pass"`/`"oba001"`, joined as `"pass->oba001"`. Deliberately
    /// the same fine-grained 7-way kind `compare()` itself already uses
    /// (`VerdictTransition`, from D1), not a coarser PASS/FINDING/
    /// INCONCLUSIVE bucketing -- D2 doesn't get to invent a new
    /// classification on top of what D1 already decided.
    fn as_str(&self) -> &'static str {
        match self {
            VerdictKind::OptionNotFound => "option_not_found",
            VerdictKind::PredicateNotFound => "predicate_not_found",
            VerdictKind::DefaultUnresolved => "default_unresolved",
            VerdictKind::TestValueUnresolved => "test_value_unresolved",
            VerdictKind::TestConfigUnresolved => "test_config_unresolved",
            VerdictKind::Oba001 => "oba001",
            VerdictKind::Pass => "pass",
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct TargetReport {
    name: String,
    /// Identity-relevant fields, copied verbatim from the manifest's own
    /// `Target` -- together with each `Verdict::option()` below, this is
    /// the `(module, test, cfg_ident, option_prefix, watched_path)` tuple
    /// PR D's `compare()` uses as a target's identity across two
    /// `AnalysisReport`s. Deliberately the manifest's own path/ident
    /// text, not a canonicalized/resolved filesystem path (same
    /// root-independence reasoning as `Span.file` elsewhere in this
    /// struct) -- identity must never depend on which `--root` happened
    /// to be used to produce a given report.
    module: PathBuf,
    test: PathBuf,
    cfg_ident: String,
    option_prefix: Vec<String>,
    /// Non-empty only when the module or test file failed to parse
    /// cleanly. Fail closed: a target with parse errors gets no per-watch
    /// verdicts at all (they'd be scanning a tree rnix patched together
    /// around damage, not the real one) -- the parse errors themselves
    /// count as inconclusive.
    parse_errors: Vec<String>,
    discovered_options: Vec<OptionDecl>,
    discovered_predicates: Vec<Predicate>,
    /// H2: every branch-condition site (`if`/`mkIf`/`optional`/...)
    /// successfully lowered through `lower_pred` -- unlike
    /// `discovered_predicates`, may include compound expressions over
    /// multiple options, reached through alias resolution. Kept
    /// unfiltered for the same transparency reason as the H1 fields
    /// above it.
    resolved_predicates: Vec<ResolvedPredicate>,
    /// H2: every branch-condition site found but NOT lowerable -- kept
    /// visible for the same reason as every other opacity-shaped field in
    /// this report, and directly load-bearing for `run_target`'s own
    /// verdicts: a target with unresolved sites can't honestly conclude
    /// `OBA001` for a watched option with no witness, since one of those
    /// sites could be a predicate this resolver simply doesn't understand
    /// yet, not proof that no such predicate exists.
    unresolved_predicate_sites: Vec<UnresolvedPredicateSite>,
    matched_test_assignments: Vec<TestAssignment>,
    /// Every place the test-file walker hit something it couldn't see
    /// into, regardless of whether it turned out to matter for any
    /// watched option (kept unfiltered here for the same reason
    /// `discovered_options`/`discovered_predicates` are: so a human or a
    /// positive-assertion test can see the scanner actually looked,
    /// rather than trusting a verdict that claims it did).
    test_config_opacity: Vec<Opacity>,
    verdicts: Vec<Verdict>,
}

fn path_matches_prefix(full: &[String], prefix: &[String], suffix: &[String]) -> bool {
    if full.len() != prefix.len() + suffix.len() {
        return false;
    }
    for (i, p) in prefix.iter().enumerate() {
        if p != "*" && full[i] != *p {
            return false;
        }
    }
    for (i, s) in suffix.iter().enumerate() {
        if full[prefix.len() + i] != *s {
            return false;
        }
    }
    true
}

/// Is `short` a prefix of the absolute target path `prefix ++ suffix`
/// (with `prefix`'s `*` wildcard segments matching anything)? Used to ask
/// "could this opacity site's scope contain the watched option" -- an
/// opacity recorded at `[]` (an entire instance being opaque) is trivially
/// a prefix of everything, which is exactly right: if nothing about an
/// instance's config was visible, no watched option under any path can be
/// safely called un-activated there.
fn path_is_prefix_of_target(short: &[String], prefix: &[String], suffix: &[String]) -> bool {
    let target_len = prefix.len() + suffix.len();
    // Strictly shorter, not `<=`: an opacity site recorded at exactly the
    // watched option's own full path isn't "more nested structure might be
    // hiding below" -- it *is* the leaf, and its value being unresolvable
    // is TestValueUnresolved's question, not this one. Getting this wrong
    // made c11 (a bare unresolvable leaf value, no nesting involved at
    // all) misreport as TestConfigUnresolved instead of TestValueUnresolved.
    if short.len() >= target_len {
        return false;
    }
    for (i, seg) in short.iter().enumerate() {
        let target_seg = if i < prefix.len() {
            &prefix[i]
        } else {
            &suffix[i - prefix.len()]
        };
        if target_seg != "*" && target_seg != seg {
            return false;
        }
    }
    true
}

/// The boolean outcome of evaluating `kind`'s predicate expression given a
/// value already classified as `class` -- `None` means genuinely unknown,
/// not "assume it doesn't count" and not "assume it does". Note this is a
/// pure function of (PredicateKind, ValueClass): `DefinitelyNonNull` is
/// enough to resolve a null-check regardless of what the value actually
/// contains (a string can never be null), but never enough to resolve a
/// truthy-check (a `DefinitelyNonNull` string is not "true").
fn predicate_outcome(kind: &PredicateKind, class: ValueClass) -> Option<bool> {
    match kind {
        PredicateKind::NullNeq => match class {
            ValueClass::Null => Some(false),
            ValueClass::Bool(_) | ValueClass::DefinitelyNonNull => Some(true),
            ValueClass::Unknown => None,
        },
        PredicateKind::NullEq => match class {
            ValueClass::Null => Some(true),
            ValueClass::Bool(_) | ValueClass::DefinitelyNonNull => Some(false),
            ValueClass::Unknown => None,
        },
        PredicateKind::Truthy(_) => match class {
            ValueClass::Bool(b) => Some(b),
            _ => None,
        },
        PredicateKind::NegTruthy => match class {
            ValueClass::Bool(b) => Some(!b),
            _ => None,
        },
    }
}

/// Every referenced option's `KnownValue` at its own declared default --
/// shared by `evaluate_predicate_witness`'s default-side environment and
/// `h2_counterfactual_verdict`'s diagnostic reporting, so the two can't
/// drift apart on what "the default" means for a given predicate.
fn declared_defaults_for(
    pred: &ResolvedPredicate,
    options: &[OptionDecl],
) -> std::collections::HashMap<OptionPath, KnownValue> {
    let mut declared_defaults = std::collections::HashMap::new();
    for r in &pred.refs {
        if let Some(decl) = options.iter().find(|o| &o.path == r) {
            if let Some(kv) = &decl.default_known_value {
                declared_defaults.insert(r.clone(), kv.clone());
            }
        }
    }
    declared_defaults
}

enum PredicateWitnessOutcome {
    /// At least one test instance's own value for the watched option
    /// provably flipped this predicate's outcome away from its default.
    Witness {
        default_outcome: bool,
        evidence: Vec<TestAssignment>,
    },
    /// Every instance this predicate could be evaluated for was fully
    /// resolved (or no instance assigned the watched option at all), but
    /// none ever demonstrated a transition -- H1's OBA001 shape, per
    /// predicate.
    EvidenceNoTransition,
    /// At least one attempted instance couldn't be evaluated at all (a
    /// required value -- the watched option's own test value, or another
    /// referenced option's context -- was unresolvable).
    Unresolved,
}

/// Evaluates one candidate predicate's counterfactual witness across
/// every real test instance that explicitly assigns the watched option
/// `x`: two hypothetical environments per instance -- everything *else*
/// the predicate references held at whatever that instance actually did
/// (or this option's own declared default, if the instance didn't touch
/// it), differing only in whether `x` itself is at its declared default
/// or this instance's test value. Deliberately per instance, never
/// merging assignments from different `nodes`/`containers` instances into
/// one synthetic environment -- that would prove something about two
/// independent machines' combined state, which no single real Nix
/// evaluation ever does.
fn evaluate_predicate_witness(
    pred: &ResolvedPredicate,
    watched_path: &[String],
    options: &[OptionDecl],
    assignments: &[TestAssignment],
    t: &Target,
) -> PredicateWitnessOutcome {
    let declared_defaults = declared_defaults_for(pred, options);

    // Every distinct instance that explicitly assigns `x` -- an instance
    // that never touches `x` at all has nothing to counterfactually
    // compare against its own default and is correctly never considered.
    //
    // Mutation-testing note: flipping this `&&` to `||` survives the
    // full test suite -- confirmed EQUIVALENT, not a real gap. It would
    // over-populate `instances_assigning_x` with irrelevant instances,
    // but the `x_assignment` lookup right below (which still correctly
    // filters on `path_matches_prefix`) silently drops every one of them
    // via its own `else { continue; }` before any other computation
    // happens, so no output is ever observably different. Left as `&&`
    // for clarity/correctness-by-construction, not "because a mutant
    // said so" -- but recorded here so a future reader re-running
    // mutation testing doesn't waste time chasing it as a live gap.
    let mut instances_assigning_x: Vec<Option<String>> = Vec::new();
    for a in assignments {
        if path_matches_prefix(&a.path, &t.option_prefix, watched_path)
            && !instances_assigning_x.contains(&a.instance)
        {
            instances_assigning_x.push(a.instance.clone());
        }
    }

    let mut has_unresolved = false;

    for instance in &instances_assigning_x {
        let Some(x_assignment) = assignments.iter().find(|a| {
            &a.instance == instance && path_matches_prefix(&a.path, &t.option_prefix, watched_path)
        }) else {
            continue; // unreachable: `instance` was derived from this same scan
        };
        let Some(x_test_value) = &x_assignment.known_value else {
            // The watched option's own test value isn't statically
            // classifiable -- can't build a test-side environment at all
            // for this instance. Fail-closed: contributes to Unresolved,
            // never silently skipped.
            has_unresolved = true;
            continue;
        };

        // Every OTHER referenced option, held at whatever THIS instance
        // actually assigned it, or this option's own declared default if
        // the instance didn't touch it. Deliberately the SAME for both
        // the default-env and test-env below -- only `x` varies between
        // the two hypothetical worlds.
        let mut env: std::collections::HashMap<OptionPath, KnownValue> =
            std::collections::HashMap::new();
        for r in &pred.refs {
            if r == watched_path {
                continue;
            }
            // Reviewed: an explicit assignment that exists but isn't
            // classifiable (`known_value: None`) must NEVER fall back to
            // the declared default -- "explicitly set to something this
            // walker can't read" is not the same fact as "never touched",
            // the exact fail-closed distinction H1's own `TestValueUnresolved`
            // already exists to protect elsewhere. The two `None` cases
            // (no assignment at all, vs. an assignment this tool can't
            // classify) must NOT be collapsed by a blanket `.or_else`.
            let assigned = assignments.iter().find(|a| {
                &a.instance == instance && path_matches_prefix(&a.path, &t.option_prefix, r)
            });
            let value = match assigned {
                Some(a) => a.known_value.clone(),
                None => declared_defaults.get(r).cloned(),
            };
            if let Some(v) = value {
                env.insert(r.clone(), v);
            }
            // Absent from `env` entirely if unresolvable either way --
            // `eval_pred`'s own Kleene semantics decide whether that
            // still permits a conclusion (e.g. another `false` operand
            // already pins an `And`) or leaves the outcome `None`.
        }

        let mut default_env = env.clone();
        if let Some(d) = declared_defaults.get(watched_path) {
            default_env.insert(watched_path.to_vec(), d.clone());
        }
        let mut test_env = env;
        test_env.insert(watched_path.to_vec(), x_test_value.clone());

        let default_outcome = eval_pred(&pred.ir, &default_env);
        let test_outcome = eval_pred(&pred.ir, &test_env);

        match (default_outcome, test_outcome) {
            (Some(d), Some(t2)) if d != t2 => {
                return PredicateWitnessOutcome::Witness {
                    default_outcome: d,
                    evidence: vec![x_assignment.clone()],
                };
            }
            (Some(_), Some(_)) => {
                // Real evidence, both sides fully resolved, but this
                // instance's own value didn't flip the predicate. Doesn't
                // end the search: another instance might still witness a
                // real transition through this same predicate.
            }
            _ => {
                has_unresolved = true;
            }
        }
    }

    if has_unresolved {
        PredicateWitnessOutcome::Unresolved
    } else {
        PredicateWitnessOutcome::EvidenceNoTransition
    }
}

/// The kind of verdict a watched option resolves to, once every
/// candidate predicate (H1's own and every H2 `ResolvedPredicate`) has
/// been tried -- carries no payload (no spans, evidence, or predicate
/// references) on purpose. `run_target` still builds the full `Verdict`
/// afterward, attaching whichever data belongs to the chosen kind; this
/// type exists solely so the PRIORITY DECISION itself is a small, pure,
/// exhaustively-checkable function, separated from the much larger
/// surrounding orchestration (scanning, evaluating candidates,
/// collecting evidence) that decides these facts in the first place.
///
/// Extracted specifically because hostile review has now twice found a
/// real bug in exactly this priority ordering, living inline inside
/// `run_target` both times (H2.2 Finding 2: H1's own predicate wrongly
/// short-circuited past H2 candidates; the post-`cef12d7` hostile-review
/// fixup: H1's own `DefaultUnresolved` wrongly short-circuited past H2
/// candidates too, one gate earlier) -- both were ordering/short-circuit
/// mistakes in code that mixed the DECISION with the LOOKUP that
/// produces its inputs. Pulling the decision out into `aggregate`, over
/// only 5 booleans (32 total input combinations, fully enumerable), is
/// what makes that class of bug amenable to exhaustive verification
/// (see KANI-0's K1 harnesses) instead of only spot-checked by whichever
/// golden fixtures happen to exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AggregateVerdict {
    Pass,
    DefaultUnresolved,
    TestConfigUnresolved,
    TestValueUnresolved,
    Oba001,
}

/// Every fact `aggregate` needs, computed by `run_target` from the real
/// predicate/opacity/unresolved-site data -- the decision itself never
/// touches that data directly, only these five booleans.
#[derive(Debug, Clone, Copy)]
struct AggregateFacts {
    /// A candidate (H1's own unary predicate, or any H2 `ResolvedPredicate`
    /// referencing the watched option) produced a real opposite-outcome
    /// transition -- `winner.is_some()` at the call site.
    has_witness: bool,
    /// H1 found its own direct predicate for the watched option, but its
    /// declared default's outcome under that predicate couldn't be
    /// statically classified.
    h1_default_unresolved: bool,
    /// Some part of the test config that could structurally contain the
    /// watched option lives in a region the walker couldn't see into.
    target_path_opaque: bool,
    /// At least one candidate's own evaluation (an H1 test-value
    /// classification, or an H2 counterfactual environment) was itself
    /// unresolvable.
    has_unresolved_attempt: bool,
    /// A branch-condition site the resolver found but couldn't lower
    /// reachably references the watched option (see
    /// `collect_reachable_refs`), even though it contributed no witness.
    relevant_unresolved_predicate: bool,
}

/// The pure priority decision: a real witness always wins (`Pass`);
/// otherwise `DefaultUnresolved` (H1's own predicate exists but its
/// default is unknown) stays at the same near-top priority it always
/// had, since a target this can apply to has nothing more specific to
/// fall back on; otherwise config-region opacity outranks a merely
/// unresolved value or predicate site, which in turn outranks a clean
/// `Oba001` -- `Oba001` is reachable only when NONE of the other four
/// facts hold, i.e. only when every known source of uncertainty this
/// tool tracks has been checked and found clear.
fn aggregate(f: AggregateFacts) -> AggregateVerdict {
    if f.has_witness {
        AggregateVerdict::Pass
    } else if f.h1_default_unresolved {
        AggregateVerdict::DefaultUnresolved
    } else if f.target_path_opaque {
        AggregateVerdict::TestConfigUnresolved
    } else if f.has_unresolved_attempt || f.relevant_unresolved_predicate {
        AggregateVerdict::TestValueUnresolved
    } else {
        AggregateVerdict::Oba001
    }
}

/// The result of analyzing one root against one target manifest -- the
/// pure analysis boundary, with no CLI/exit-code/JSON-envelope concerns
/// attached. `oba check` (single root) and `oba diff` (PR D: base root +
/// head root, each analyzed independently through this same function
/// before being compared) both bottom out here; nothing above this layer
/// should need to know how a `TargetReport` gets produced.
#[derive(Debug, Clone)]
struct AnalysisReport {
    targets: Vec<TargetReport>,
}

/// Analyze every target in `manifest` against `root`. `module`/`test` in
/// each `Target` are resolved relative to `root` -- never to the process's
/// cwd -- specifically so a manifest can later be applied unchanged to two
/// different roots (a base checkout and a head checkout) without the
/// question of "relative to which root?" ever coming up for the manifest
/// path itself (only `--targets` stays cwd-relative, at the CLI layer,
/// same as it always has been). Each `module`/`test` is resolved through
/// `resolve_within_root`, a real filesystem boundary, not a decorative
/// `PathBuf::join` -- see its own doc comment.
fn analyze(root: &Path, manifest: &TargetFile) -> anyhow::Result<AnalysisReport> {
    let mut targets = Vec::new();
    for t in &manifest.target {
        let module_path = resolve_within_root(root, &t.module)
            .map_err(|e| anyhow::anyhow!("target {}: module: {e}", t.name))?;
        let test_path = resolve_within_root(root, &t.test)
            .map_err(|e| anyhow::anyhow!("target {}: test: {e}", t.name))?;
        targets.push(run_target(t, &module_path, &test_path)?);
    }
    Ok(AnalysisReport { targets })
}

/// Resolve a target's `module`/`test` path against `root`, refusing to let
/// it leave `root`'s real (canonicalized, symlinks-resolved) filesystem
/// subtree. `--root` is meant to be a hard boundary, not a base path
/// that's merely joined and trusted: `module`/`test` come from a target
/// manifest, which can itself live inside the very repository under
/// analysis -- an untrusted PR that can edit `targets.toml` must not be
/// able to point a target at an absolute path, a `../` escape, or a
/// symlink resolving outside `root` and have this tool read it anyway.
/// All three become a TOOL_ERROR (exit 3), same as any other manifest
/// problem, not a silent read from wherever they happen to point.
fn resolve_within_root(root: &Path, rel: &Path) -> anyhow::Result<PathBuf> {
    if rel.is_absolute() {
        anyhow::bail!(
            "{} must be relative to --root, not absolute",
            rel.display()
        );
    }
    let root_canon = root
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("--root {}: {e}", root.display()))?;
    let canon = root_canon.join(rel).canonicalize().map_err(|e| {
        anyhow::anyhow!("resolving {} under --root {}: {e}", rel.display(), root.display())
    })?;
    if !canon.starts_with(&root_canon) {
        anyhow::bail!(
            "{} escapes --root {} (resolves to {})",
            rel.display(),
            root.display(),
            canon.display()
        );
    }
    Ok(canon)
}

/// S1-F1: like `resolve_within_root`, but distinguishes "the file
/// genuinely does not exist" (`Ok(None)`) from every other real
/// problem (`Err`, exactly the same cases `resolve_within_root` itself
/// already rejects: an absolute path, a root escape, a permission
/// error, ...). Exists ONLY for `audit-diff`'s own per-target
/// orchestration (`run_audit_diff`): a target whose module/test is
/// absent on one side of a real base/head comparison is real PR-diff
/// algebra (a brand-new or since-deleted module), not a tool
/// malfunction -- see `run_audit_diff`'s own `AddedSubject`/
/// `RemovedSubject` handling below. `check`/`diff`/`audit` keep calling
/// `resolve_within_root` directly, completely unchanged: asking to
/// analyze a single root whose named module is simply missing is, and
/// stays, a real error there -- there is no "other side" for it to be
/// diff algebra relative to.
fn resolve_within_root_if_exists(root: &Path, rel: &Path) -> anyhow::Result<Option<PathBuf>> {
    if rel.is_absolute() {
        anyhow::bail!(
            "{} must be relative to --root, not absolute",
            rel.display()
        );
    }
    let root_canon = root
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("--root {}: {e}", root.display()))?;
    let joined = root_canon.join(rel);
    let canon = match joined.canonicalize() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(anyhow::anyhow!(
                "resolving {} under --root {}: {e}",
                rel.display(),
                root.display()
            ));
        }
    };
    if !canon.starts_with(&root_canon) {
        anyhow::bail!(
            "{} escapes --root {} (resolves to {})",
            rel.display(),
            root.display(),
            canon.display()
        );
    }
    Ok(Some(canon))
}

// ---------------------------------------------------------------------
// PR D1: pure compare() -- no CLI, no Action, no Git/GitHub anywhere near
// this. Deliberately built and tested standalone against two in-memory
// `AnalysisReport`s before `oba diff`/Action wiring exists at all, per
// the design note's own sequencing (D design freeze -> pure compare() ->
// adversarial diff corpus -> oba diff CLI -> Action diff integration).
// ---------------------------------------------------------------------

/// A watched option's identity across two `AnalysisReport`s, independent
/// of revision. Deliberately syntactic and flat -- no rename/move
/// tracking, no content-similarity heuristic: a `module`/`test` path
/// edit changes this tuple, so it surfaces as `Removed` + `Added`, never
/// a detected "same target, different path" (see the PR D design note).
/// Nothing presentation-shaped (`Span`, source excerpts, evidence order)
/// belongs here -- identity must be stable even when only *how* a result
/// is reported changes, not what it's about.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
struct TargetIdentity {
    module: PathBuf,
    test: PathBuf,
    cfg_ident: String,
    option_prefix: Vec<String>,
    watched_path: String,
}

/// `Verdict`'s bare discriminant, ignoring payload -- see `Verdict::kind`.
/// What `TargetDiff::Changed`/`Added`/`Removed` actually carry for one
/// identity. Deliberately just the verdict for now (v1 only computes
/// `ChangeKind::VerdictChanged`) -- reserved as its own type so a later
/// `EvidenceChanged`/`PredicateChanged`/`VisibilityChanged` can grow this
/// struct without renegotiating `TargetDiff` itself.
#[derive(Debug, Clone, PartialEq, Serialize)]
struct TargetOutcome {
    verdict: Verdict,
}

/// What kind of change was detected between a base and head outcome for
/// the same identity. A marker list, not itself a judgment -- `compare()`
/// states facts, never regression/improvement opinions (see
/// `TargetDiff::verdict_transition`). Shared `Changed` postfix is
/// intentional (naming from the PR D design note, not accidental
/// copy-paste) -- clippy's glob-import-friendliness suggestion doesn't
/// apply, nothing here is ever glob-imported.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ChangeKind {
    VerdictChanged,
    EvidenceChanged,
    PredicateChanged,
    VisibilityChanged,
}

/// One identity's comparison result. `Unchanged` means "no change under
/// the `ChangeKind`s this version of `compare()` actually computes" --
/// v1 only computes `VerdictChanged`, so two outcomes with the same
/// verdict kind but entirely different evidence are `Unchanged` here, NOT
/// a claim that the two `TargetOutcome`s are byte-for-byte/structurally
/// identical. Widening what `Unchanged` notices means adding a
/// `ChangeKind` and populating it, not redefining this type.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TargetDiff {
    Unchanged,
    // Boxed: `TargetOutcome` wraps a full `Verdict`, whose own payload
    // (evidence/predicate_attempts/spans) makes it large enough that an
    // unboxed `Changed` would otherwise force every `TargetDiff` --
    // including the zero-payload `Unchanged` case, by far the most common
    // one in a clean run -- to reserve stack space for the biggest
    // variant (clippy::large_enum_variant, a real signal here, not noise).
    Added { head: Box<TargetOutcome> },
    Removed { base: Box<TargetOutcome> },
    Changed {
        base: Box<TargetOutcome>,
        head: Box<TargetOutcome>,
        changes: Vec<ChangeKind>,
    },
}

impl TargetDiff {
    /// The bare verdict-kind transition, when this diff represents one --
    /// `None` for `Unchanged`/`Added`/`Removed` (there's no "from" or no
    /// "to"), and for a hypothetical future `Changed` whose `changes`
    /// doesn't include `VerdictChanged` (not reachable by v1 alone, which
    /// only ever populates that one `ChangeKind`). A bare fact --
    /// `{from, to}`, not "regression"/"improvement": whether e.g.
    /// `PASS -> INCONCLUSIVE` should block a PR is CI policy, decided
    /// somewhere above this, same boundary `analyze()` already draws
    /// around exit codes.
    fn verdict_transition(&self) -> Option<VerdictTransition> {
        match self {
            TargetDiff::Changed { base, head, changes }
                if changes.contains(&ChangeKind::VerdictChanged) =>
            {
                Some(VerdictTransition {
                    from: base.verdict.kind(),
                    to: head.verdict.kind(),
                })
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VerdictTransition {
    from: VerdictKind,
    to: VerdictKind,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct ComparisonEntry {
    identity: TargetIdentity,
    diff: TargetDiff,
}

/// `compare()`'s full result -- `entries` is always sorted by `identity`
/// (see `TargetIdentity`'s derived `Ord`), regardless of the order
/// `AnalysisReport::targets`/each target's `watch` list happened to be
/// in. Determinism here is load-bearing, not cosmetic: a `oba diff` whose
/// output order depended on TOML array order would make CI annotations
/// and any snapshot/golden test of diff output flaky for reasons that
/// have nothing to do with the actual analysis.
#[derive(Debug, Clone, PartialEq)]
struct ComparisonReport {
    entries: Vec<ComparisonEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompareSide {
    Base,
    Head,
}

/// `compare()`'s only failure mode: the same identity appearing twice
/// within a single side. Deliberately an error, not "pair with the first
/// match" -- a manifest that (accidentally or via a hostile edit)
/// produces two verdicts for the exact same `(module, test, cfg_ident,
/// option_prefix, watched_path)` tuple makes pairing genuinely ambiguous,
/// and guessing would turn into exactly the "why does diff depend on
/// TOML order" investigation this type exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CompareError {
    side: CompareSide,
    // Boxed: TargetIdentity (2 PathBufs + a String + a Vec<String> + a
    // String) makes an unboxed CompareError ~128 bytes, which clippy
    // correctly flags (clippy::result_large_err) -- every `Ok` path
    // through `Result<_, CompareError>` would otherwise pay for the size
    // of the rare error case.
    identity: Box<TargetIdentity>,
}

impl std::fmt::Display for CompareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "duplicate target identity on the {:?} side: module={} test={} cfg_ident={} option_prefix={:?} watched_path={}",
            self.side,
            self.identity.module.display(),
            self.identity.test.display(),
            self.identity.cfg_ident,
            self.identity.option_prefix,
            self.identity.watched_path,
        )
    }
}

impl std::error::Error for CompareError {}

/// Every `(identity, &Verdict)` pair in one `AnalysisReport`, keyed by
/// identity -- `Err` the instant the same identity appears twice on this
/// side, before any base/head pairing is attempted.
fn index_by_identity(
    report: &AnalysisReport,
    side: CompareSide,
) -> Result<std::collections::BTreeMap<TargetIdentity, &Verdict>, CompareError> {
    let mut index = std::collections::BTreeMap::new();
    for target in &report.targets {
        for verdict in &target.verdicts {
            let identity = TargetIdentity {
                module: target.module.clone(),
                test: target.test.clone(),
                cfg_ident: target.cfg_ident.clone(),
                option_prefix: target.option_prefix.clone(),
                watched_path: verdict.option().to_string(),
            };
            if index.insert(identity.clone(), verdict).is_some() {
                return Err(CompareError { side, identity: Box::new(identity) });
            }
        }
    }
    Ok(index)
}

/// Compare two already-complete `AnalysisReport`s by identity. Pure: no
/// filesystem, no Git, no GitHub -- everything it needs is already inside
/// `base`/`head`. Deterministic: `entries` is always sorted by identity,
/// independent of either report's own target/watch order (`BTreeMap`
/// keyed by `TargetIdentity`, iterated by its `Ord`).
fn compare(base: &AnalysisReport, head: &AnalysisReport) -> Result<ComparisonReport, CompareError> {
    let base_index = index_by_identity(base, CompareSide::Base)?;
    let head_index = index_by_identity(head, CompareSide::Head)?;

    let mut identities: std::collections::BTreeSet<TargetIdentity> = std::collections::BTreeSet::new();
    identities.extend(base_index.keys().cloned());
    identities.extend(head_index.keys().cloned());

    let mut entries = Vec::with_capacity(identities.len());
    for identity in identities {
        let diff = match (base_index.get(&identity), head_index.get(&identity)) {
            (None, None) => unreachable!("identity came from one of the two indexes"),
            (None, Some(h)) => TargetDiff::Added {
                head: Box::new(TargetOutcome { verdict: (*h).clone() }),
            },
            (Some(b), None) => TargetDiff::Removed {
                base: Box::new(TargetOutcome { verdict: (*b).clone() }),
            },
            (Some(b), Some(h)) => {
                let mut changes = Vec::new();
                if b.kind() != h.kind() {
                    changes.push(ChangeKind::VerdictChanged);
                }
                if changes.is_empty() {
                    TargetDiff::Unchanged
                } else {
                    TargetDiff::Changed {
                        base: Box::new(TargetOutcome { verdict: (*b).clone() }),
                        head: Box::new(TargetOutcome { verdict: (*h).clone() }),
                        changes,
                    }
                }
            }
        };
        entries.push(ComparisonEntry { identity, diff });
    }

    Ok(ComparisonReport { entries })
}

fn run_target(
    t: &Target,
    module_path: &std::path::Path,
    test_path: &std::path::Path,
) -> anyhow::Result<TargetReport> {
    let module_src = fs::read_to_string(module_path).map_err(|e| {
        anyhow::anyhow!(
            "target {}: reading module {}: {e}",
            t.name,
            t.module.display()
        )
    })?;
    let test_src = fs::read_to_string(test_path).map_err(|e| {
        anyhow::anyhow!("target {}: reading test {}: {e}", t.name, t.test.display())
    })?;
    let module_file = t.module.display().to_string();
    let test_file = t.test.display().to_string();

    let module_parse = rnix::Root::parse(&module_src);
    let test_parse = rnix::Root::parse(&test_src);

    let mut parse_errors: Vec<String> = Vec::new();
    for e in module_parse.errors() {
        parse_errors.push(format!("{module_file}: {e}"));
    }
    for e in test_parse.errors() {
        parse_errors.push(format!("{test_file}: {e}"));
    }

    if !parse_errors.is_empty() {
        // Fail closed: don't scan a tree rnix stitched together around
        // damage and pretend the resulting (dis)coveries mean anything.
        return Ok(TargetReport {
            name: t.name.clone(),
            module: t.module.clone(),
            test: t.test.clone(),
            cfg_ident: t.cfg_ident.clone(),
            option_prefix: t.option_prefix.clone(),
            parse_errors,
            discovered_options: Vec::new(),
            discovered_predicates: Vec::new(),
            resolved_predicates: Vec::new(),
            unresolved_predicate_sites: Vec::new(),
            matched_test_assignments: Vec::new(),
            test_config_opacity: Vec::new(),
            verdicts: t
                .watch
                .iter()
                .map(|w| Verdict::OptionNotFound { option: w.clone() })
                .collect(),
        });
    }

    let module_root = module_parse.tree();
    let test_root = test_parse.tree();

    let options = scan_options(
        &module_file,
        &module_src,
        module_root.syntax(),
        &t.cfg_ident,
        &t.option_prefix,
    );
    let predicates = scan_predicates(
        &module_file,
        &module_src,
        module_root.syntax(),
        &t.cfg_ident,
    );
    let (resolved_predicates, unresolved_predicate_sites) = scan_resolved_predicates(
        &module_file,
        &module_src,
        module_root.syntax(),
        &t.cfg_ident,
        &t.option_prefix,
    );
    let (assignments, opacity) = scan_test_assignments(&test_file, &test_src, test_root.syntax());

    let mut matched_assignments = Vec::new();
    let mut verdicts = Vec::new();

    for watched in &t.watch {
        let watched_path: Vec<String> = watched.split('.').map(|s| s.to_string()).collect();

        // Gate 1: the declaration scanner must have actually found this
        // option. A dead/broken declaration scanner must never be able to
        // silently let a PASS through -- see the H1 review that caught
        // this: default_source was previously optional at the point PASS
        // became reachable.
        let Some(decl) = options.iter().find(|o| o.path == watched_path) else {
            verdicts.push(Verdict::OptionNotFound {
                option: watched.clone(),
            });
            continue;
        };

        // Gates 2-4, unified: H1's own unary predicate (if any, using its
        // already-established gate-3 semantics for THAT predicate
        // specifically) AND every H2 resolved predicate referencing this
        // option are evaluated as candidates in the SAME aggregation --
        // not "H1 first, H2 only as a fallback when H1 finds nothing".
        // Reviewed and fixed: the previous fallback structure meant a
        // watched option with BOTH a real but non-transitioning direct H1
        // predicate AND a real, transitioning H2 compound predicate would
        // report the H1 predicate's `OBA001` and never even look at the
        // H2 one -- a genuine false `OBA001`, not merely an incomplete
        // `PASS`.
        //
        // Reviewed a second time (hostile review of this very commit,
        // before it was even accepted as a checkpoint): H1's own gate 3
        // (`DefaultUnresolved`) used to stay an immediate `continue`
        // here, on the reasoning that it "preserves byte-identical
        // behavior for the existing `DefaultUnresolved` goldens without
        // unifying gate 3 into the aggregator too". That reasoning was
        // exactly the same shape of bug this whole corrective pass
        // exists to close: H1's `ValueClass` classifies a default's
        // outcome more coarsely than H2's `KnownValue` does for the
        // identical AST node (a plain string-literal default is
        // `ValueClass::DefinitelyNonNull` -- enough for a null-check
        // predicate, not enough for `Truthy`/`NegTruthy` -- but
        // `KnownValue::Exact(Scalar::Str(..))` under `classify_known_value`,
        // which a compound `Eq`-based H2 predicate on the very same
        // option CAN resolve). The old immediate `continue` meant H1
        // failing to classify its own predicate's default silently
        // skipped the H2 loop entirely for that option, even when an H2
        // candidate on the same option would have witnessed a real
        // transition -- a false `DefaultUnresolved` standing in for what
        // should have been `PASS`. `h1_default_unresolved` now only
        // decides the *fallback* verdict when nothing else (H2 included)
        // produces a witness or even an attempt; a witnessing H2
        // candidate always wins, matching the priority Pass already has
        // over everything else in this aggregation.
        let mut attempts: Vec<PredicateAttempt> = Vec::new();
        let mut winner: Option<(PredicateRef, bool, Vec<TestAssignment>)> = None;
        let mut has_unresolved = false;
        let mut h1_default_unresolved = false;

        if let Some(pred) = predicates.iter().find(|p| p.path == watched_path) {
            // Gate 3 for H1's own predicate specifically. A `None` here
            // no longer short-circuits the whole option -- it just means
            // H1's own predicate contributes no attempt at all (there is
            // no default outcome to compare a test value against), and
            // `h1_default_unresolved` is what the final fallback verdict
            // below checks once every H2 candidate has also been tried.
            match decl
                .default_class
                .and_then(|c| predicate_outcome(&pred.kind, c))
            {
                None => {
                    h1_default_unresolved = true;
                }
                Some(default_outcome) => {
                    // H1's own gate 4, unchanged, expressed as one
                    // candidate's attempt instead of an immediate verdict.
                    let matches: Vec<TestAssignment> = assignments
                        .iter()
                        .filter(|a| path_matches_prefix(&a.path, &t.option_prefix, &pred.path))
                        .cloned()
                        .collect();
                    matched_assignments.extend(matches.clone());

                    let mut opposite = Vec::new();
                    let mut h1_has_unresolved = false;
                    for a in &matches {
                        match predicate_outcome(&pred.kind, a.value_class) {
                            Some(o) if o == !default_outcome => opposite.push(a.clone()),
                            Some(_) => {} // known, same outcome as the default -- no evidence, not ambiguous either
                            None => h1_has_unresolved = true,
                        }
                    }

                    attempts.push(PredicateAttempt {
                        predicate: PredicateRef::Unary(pred.clone()),
                        witnessed: if !opposite.is_empty() {
                            Some(true)
                        } else if h1_has_unresolved {
                            None
                        } else {
                            Some(false)
                        },
                    });
                    if h1_has_unresolved {
                        has_unresolved = true;
                    }
                    if !opposite.is_empty() {
                        winner =
                            Some((PredicateRef::Unary(pred.clone()), default_outcome, opposite));
                    }
                }
            }
        }

        // Every H2 resolved predicate referencing this option -- always
        // evaluated now, regardless of whether H1 already found one.
        for pred in resolved_predicates
            .iter()
            .filter(|p| p.refs.iter().any(|r| r == &watched_path))
        {
            let outcome =
                evaluate_predicate_witness(pred, &watched_path, &options, &assignments, t);
            let witnessed = match &outcome {
                PredicateWitnessOutcome::Witness { .. } => Some(true),
                PredicateWitnessOutcome::EvidenceNoTransition => Some(false),
                PredicateWitnessOutcome::Unresolved => None,
            };
            attempts.push(PredicateAttempt {
                predicate: PredicateRef::Resolved(pred.clone()),
                witnessed,
            });
            match outcome {
                PredicateWitnessOutcome::Witness {
                    default_outcome,
                    evidence,
                } => {
                    if winner.is_none() {
                        winner = Some((
                            PredicateRef::Resolved(pred.clone()),
                            default_outcome,
                            evidence,
                        ));
                    }
                }
                PredicateWitnessOutcome::Unresolved => {
                    has_unresolved = true;
                }
                PredicateWitnessOutcome::EvidenceNoTransition => {}
            }
        }

        if attempts.is_empty() {
            // Neither H1 found its own predicate for this option, nor
            // did H2's scanner find any resolved predicate referencing
            // it -- genuinely nothing to report on. (In practice
            // `h1_default_unresolved` is essentially never true here: a
            // bare `cfg.foo` select H1 recognizes as a direct predicate
            // is independently picked up by `scan_resolved_predicates`
            // too, so H1 finding a predicate almost always means
            // `attempts` already has at least one H2-sourced entry --
            // see the `h1_default_unresolved` check below, which is what
            // actually handles that case now.)
            verdicts.push(Verdict::PredicateNotFound {
                option: watched.clone(),
            });
            continue;
        }

        // Does any part of the test config that could structurally contain
        // this option live in a region the walker couldn't see into
        // (imports, an alias, a function call)? Checked AFTER opposite
        // evidence, never before: an explicit, provable transition found
        // elsewhere in the file is a stronger claim than "some unrelated
        // opacity also exists" (c17).
        let target_path_opaque = opacity
            .iter()
            .any(|o| path_is_prefix_of_target(&o.path, &t.option_prefix, &watched_path));

        // Reviewed: a branch-condition site this resolver found but
        // couldn't lower at all (`UnresolvedPredicateSite`) means part of
        // this target's own branch logic was never actually looked at --
        // an unrelated helper's `if isInt v then ...` is syntactically
        // indistinguishable, without deeper analysis, from a genuine
        // option-branch predicate this tool simply doesn't understand
        // yet. A target with such a site can't honestly claim `OBA001`
        // ("no evidence anywhere") for a watched option that site's
        // `refs` (collected even from the failed lowering, see
        // `collect_reachable_refs`) actually reaches -- matched
        // per-option, not target-wide: an earlier version gated on "any
        // unresolved site exists anywhere in the target", which was
        // itself found unsound in both directions -- too coarse against
        // real corpus (kimai's unrelated `eachSite != {}` blocked every
        // watched option in the target) AND, separately, too narrow
        // against a purely syntactic relevance test that missed
        // alias-hidden references (`suspicious = someUnsupportedHelper
        // cfg.database.driver;`, condition site is just `suspicious`,
        // whose own text never mentions `cfg` even though its binding
        // does). Per-option `refs` matching is what actually resolves
        // both: kimai's `eachSite != {}` resolves to `refs =
        // [["sites"]]`, which never matches an unrelated watched option,
        // while `suspicious`'s hidden `cfg.database.driver` resolves to
        // `refs = [["database","driver"]]`, which correctly matches when
        // THAT option is being watched. Only weakens a would-be
        // `OBA001`; a real witness (checked above, already returned)
        // always wins regardless of how many unresolved sites exist
        // elsewhere in the same module.
        let relevant_predicate_site_unresolved = unresolved_predicate_sites
            .iter()
            .any(|s| s.refs.iter().any(|r| r == &watched_path));

        // Best-effort diagnostic `default_outcome` for the non-witness
        // verdicts below: H1's own (if it computed one) takes priority,
        // matching its exact previous reporting; otherwise every
        // referenced option of the first H2 candidate at its own declared
        // default. Reporting only -- which `Verdict` variant gets chosen
        // is driven entirely by `attempts`/`target_path_opaque`/
        // `relevant_predicate_site_unresolved` above, never by this value.
        let diagnostic_default_outcome = predicates
            .iter()
            .find(|p| p.path == watched_path)
            .and_then(|p| {
                decl.default_class
                    .and_then(|c| predicate_outcome(&p.kind, c))
            })
            .or_else(|| {
                resolved_predicates
                    .iter()
                    .find(|p| p.refs.iter().any(|r| r == &watched_path))
                    .and_then(|p| eval_pred(&p.ir, &declared_defaults_for(p, &options)))
            });
        let primary_predicate = attempts[0].predicate.clone();

        // The priority decision itself lives in `aggregate` -- a small,
        // pure function over these five facts, kept separate from the
        // lookups above precisely because both real bugs in this
        // priority ordering so far were short-circuit mistakes tangled
        // up with the code that computes its inputs. See `aggregate`'s
        // own doc comment.
        let facts = AggregateFacts {
            has_witness: winner.is_some(),
            h1_default_unresolved,
            target_path_opaque,
            has_unresolved_attempt: has_unresolved,
            relevant_unresolved_predicate: relevant_predicate_site_unresolved,
        };

        match aggregate(facts) {
            AggregateVerdict::Pass => {
                let (predicate, default_outcome, evidence) = winner
                    .expect("aggregate only returns Pass when facts.has_witness == winner.is_some() == true");
                verdicts.push(Verdict::Pass {
                    option: watched.clone(),
                    predicate,
                    default_outcome,
                    evidence,
                    predicate_attempts: attempts,
                });
            }
            AggregateVerdict::DefaultUnresolved => {
                let pred = predicates
                    .iter()
                    .find(|p| p.path == watched_path)
                    .expect("h1_default_unresolved is only set when this find() succeeded above");
                verdicts.push(Verdict::DefaultUnresolved {
                    option: watched.clone(),
                    predicate: PredicateRef::Unary(pred.clone()),
                });
            }
            AggregateVerdict::TestConfigUnresolved => {
                verdicts.push(Verdict::TestConfigUnresolved {
                    option: watched.clone(),
                    predicate: primary_predicate,
                    default_outcome: diagnostic_default_outcome,
                    predicate_attempts: attempts,
                });
            }
            AggregateVerdict::TestValueUnresolved => {
                verdicts.push(Verdict::TestValueUnresolved {
                    option: watched.clone(),
                    predicate: primary_predicate,
                    default_outcome: diagnostic_default_outcome,
                    predicate_attempts: attempts,
                });
            }
            AggregateVerdict::Oba001 => {
                verdicts.push(Verdict::Oba001 {
                    option: watched.clone(),
                    predicate: primary_predicate,
                    default_outcome: diagnostic_default_outcome,
                    predicate_attempts: attempts,
                });
            }
        }
    }

    Ok(TargetReport {
        name: t.name.clone(),
        module: t.module.clone(),
        test: t.test.clone(),
        cfg_ident: t.cfg_ident.clone(),
        option_prefix: t.option_prefix.clone(),
        parse_errors,
        discovered_options: options,
        discovered_predicates: predicates,
        resolved_predicates,
        unresolved_predicate_sites,
        matched_test_assignments: matched_assignments,
        test_config_opacity: opacity,
        verdicts,
    })
}

// ---------------------------------------------------------------------
// Syntax-visibility census (--census): runs only the test-file walker
// (scan_test_assignments) over every .nix file under a directory, with no
// module/option/predicate matching at all -- a standalone sanity pass
// answering one question before Layer 1 is called settled: across a real
// corpus, how often does "the walker found nothing" actually mean
// "genuinely nothing there" vs. "gave up and recorded opacity" vs. (the
// one outcome that would mean this tool still isn't safe to trust)
// "silently saw nothing at all, no assignment and no opacity, on a file
// that clearly has real option-setting content".
// ---------------------------------------------------------------------

#[derive(Serialize, Debug, Default)]
struct CensusReport {
    files_scanned: usize,
    /// H1.3a review: previously silently `continue`d past an unreadable
    /// file (permissions, non-UTF8, ...) with no record anywhere in the
    /// report -- a census could "successfully" cover a corpus it never
    /// fully read, and nothing about the printed report would say so.
    unreadable_files: usize,
    parse_errors: usize,
    /// Root resolved directly to a literal attrset (no unwrapping needed
    /// beyond the ordinary lambda/let-in chain).
    root_direct_attrset: usize,
    /// Root needed the `import ./make-test-python.nix (...)` unwrap.
    root_import_wrapper: usize,
    /// Root was neither -- an unrecognized wrapper, recorded as root
    /// opacity rather than guessed at.
    root_opaque: usize,
    total_assignments: usize,
    total_opacity_sites: usize,
    files_with_any_opacity: usize,
    /// Files where the walker found NEITHER an assignment NOR an opacity
    /// site -- the concerning bucket. Expected for files with no real
    /// node config at all (rare for an actual nixosTest); unexpected
    /// counts here on files that look real would mean a walker gap this
    /// census didn't anticipate.
    files_with_neither: usize,
    /// H1.3a review: the metric that actually matters, and a strictly
    /// stronger acceptance bar than `files_with_neither`. A file can have
    /// 200 cleanly recognized assignments AND one fully invisible sibling
    /// scenario (`hiddenScenario = runTest { ... };` next to a normal
    /// `nodes.machine`) -- `files_with_neither` would call that file
    /// clean, because *something* was found. This counts every
    /// test-spec-root entry that was neither `nodes`/`containers` nor a
    /// confirmed-safe metadata key (`REASON_UNCLASSIFIED_ROOT_ENTRY`),
    /// i.e. every point option-relevant syntax could have been silently
    /// discarded. The acceptance bar this pass exists to hit is this
    /// field being explainable/zero-per-file, not `files_with_neither`
    /// alone.
    unclassified_root_entries: usize,
    /// H1.3b review: the same "option-relevant syntax silently discarded"
    /// count as `unclassified_root_entries`, but for entries under the
    /// NESTED `nodes = { ... };` form specifically -- a file with a
    /// normal, recognized sibling instance (so it's invisible to both
    /// `files_with_neither` and `unclassified_root_entries`) can still
    /// hide a multi-segment attrpath or `inherit` at this level.
    unclassified_instance_entries: usize,
    opacity_reason_counts: std::collections::BTreeMap<String, usize>,
}

fn collect_nix_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_nix_files(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("nix") {
            out.push(path);
        }
    }
    Ok(())
}

fn run_census(dir: &std::path::Path, json: bool) -> anyhow::Result<i32> {
    let mut files = Vec::new();
    collect_nix_files(dir, &mut files)
        .map_err(|e| anyhow::anyhow!("walking census directory {}: {e}", dir.display()))?;
    files.sort();

    let mut report = CensusReport::default();

    for path in &files {
        report.files_scanned += 1;
        let src = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                // Unreadable (permissions, non-UTF8, ...) is NOT the same
                // fact as "read fine, parsed fine, genuinely found
                // nothing" -- must be visible in the report and must fail
                // the run closed (see the exit-code logic below), not
                // silently drop the file from the corpus this census
                // claims to have covered.
                report.unreadable_files += 1;
                continue;
            }
        };
        let parse = rnix::Root::parse(&src);
        if !parse.errors().is_empty() {
            report.parse_errors += 1;
            continue;
        }
        let root = parse.tree();
        let file = path.display().to_string();

        let unwrapped = unwrap_lambda_chain(root.syntax().clone());
        let resolved = resolve_test_root(unwrapped.clone());
        if resolved.kind() == NODE_ATTR_SET {
            if resolved == unwrapped {
                report.root_direct_attrset += 1;
            } else {
                report.root_import_wrapper += 1;
            }
        } else {
            report.root_opaque += 1;
        }

        let (assignments, opacity) = scan_test_assignments(&file, &src, root.syntax());
        report.total_assignments += assignments.len();
        report.total_opacity_sites += opacity.len();
        if !opacity.is_empty() {
            report.files_with_any_opacity += 1;
        }
        if assignments.is_empty() && opacity.is_empty() {
            report.files_with_neither += 1;
            if std::env::var("OBA_CENSUS_DEBUG").is_ok() {
                eprintln!("NEITHER: {file}");
            }
        }
        report.unclassified_root_entries += opacity
            .iter()
            .filter(|o| o.reason == REASON_UNCLASSIFIED_ROOT_ENTRY)
            .count();
        report.unclassified_instance_entries += opacity
            .iter()
            .filter(|o| o.reason == REASON_UNCLASSIFIED_NESTED_INSTANCE_ENTRY)
            .count();
        for o in &opacity {
            *report
                .opacity_reason_counts
                .entry(o.reason.to_string())
                .or_insert(0) += 1;
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("=== syntax-visibility census: {} ===", dir.display());
        println!("files scanned:             {}", report.files_scanned);
        println!("unreadable files:          {}", report.unreadable_files);
        println!("parse errors:              {}", report.parse_errors);
        println!("root: direct attrset:      {}", report.root_direct_attrset);
        println!("root: import-wrapper:      {}", report.root_import_wrapper);
        println!("root: opaque/unrecognized: {}", report.root_opaque);
        println!("total assignments found:   {}", report.total_assignments);
        println!("total opacity sites:       {}", report.total_opacity_sites);
        println!(
            "files with any opacity:    {}",
            report.files_with_any_opacity
        );
        println!(
            "files with NEITHER (weaker bucket): {}",
            report.files_with_neither
        );
        println!(
            "unclassified root entries (option-relevant syntax that could have been silently discarded): {}",
            report.unclassified_root_entries
        );
        println!(
            "unclassified nested-instance entries (same, one level down inside nodes = {{ ... }}): {}",
            report.unclassified_instance_entries
        );
        println!("opacity reasons:");
        for (reason, count) in &report.opacity_reason_counts {
            println!("  {count:5}  {reason}");
        }
    }

    // H1.3a review: this used to always return Ok(0), meaning "unreadable
    // files exist" and "parse errors exist" were only visible if someone
    // actually read the printed numbers. A census is evidence-gathering
    // for a freeze decision, so it gets the same fail-closed exit-code
    // discipline as `run()`: unreadable files mean the corpus wasn't fully
    // covered at all (TOOL_ERROR, 3); parse errors mean some files were
    // covered but not analyzable (INCONCLUSIVE-shaped, 2); anything else
    // is a clean pass (0). `1` (FINDING) doesn't apply -- a census
    // produces no verdicts to find.
    if report.unreadable_files > 0 {
        anyhow::bail!(
            "{} file(s) under {} could not be read -- census does not cover the full corpus",
            report.unreadable_files,
            dir.display()
        );
    }
    Ok(if report.parse_errors > 0 { 2 } else { 0 })
}

/// The versioned, external `--json` machine contract (PR B commit 3).
/// What's frozen as of `schema_version: 1`: the envelope shape itself
/// (`schema_version`/`tool`/`mode`/`summary`/`targets`), `tool.name`, and
/// every existing `TargetReport`/`Verdict` field. NOT frozen yet: the
/// payload `mode: "diff"` will carry (PR D) -- `mode` is reserved as a
/// sum-type tag specifically so adding it doesn't need `schema_version:
/// 2`, but no `transition`/`base`/`head` shape is speculated here before
/// PR D actually needs one. Deliberately excluded from this envelope:
/// absolute `--root`, wall-clock timestamps, hostname, PID -- anything
/// that would make the identical checkout produce a different payload
/// depending on where or when it happened to run (a GitHub Actions
/// runner's `/home/runner/work/...` vs a contributor's own `/home/...`
/// must diff as identical once source paths are normalized the same way,
/// which they already are: `TargetReport`'s `Span.file`/module/test
/// strings are always the manifest's own root-relative text, never a
/// canonicalized/joined filesystem path -- see `analyze`/`run_target`).
#[derive(Serialize, Debug)]
struct ReportEnvelope {
    schema_version: u32,
    tool: ToolInfo,
    /// Only `"check"` exists today; `"diff"` is PR D's, not this one's.
    mode: &'static str,
    summary: CheckSummary,
    targets: Vec<TargetReport>,
}

#[derive(Serialize, Debug)]
struct ToolInfo {
    name: &'static str,
    version: &'static str,
}

#[derive(Serialize, Debug)]
struct CheckSummary {
    pass: usize,
    finding: usize,
    inconclusive: usize,
}

/// Exit codes are a 4-state scheme, not 3: `0` clean analysis / `1` a
/// genuine finding / `2` analysis completed but some watched option
/// couldn't be resolved / `3` the tool itself didn't run at all (bad
/// manifest, missing files, I/O). `2` and `3` look similar from a shell
/// ("something's wrong") but mean structurally different things to a
/// machine consumer -- "I analyzed this and couldn't prove anything" is
/// not the same claim as "I never got to analyze it". Collapsing them
/// (the H1 state of this file: any `?`-propagated I/O/parse/manifest
/// error fell through to Rust's default error exit code, typically `1` --
/// FINDING, not even INCONCLUSIVE) is caught by
/// `h1_1_missing_file_is_tool_error_not_finding` in tests/golden.rs.
fn main() {
    // `Cli::parse()` (used before H1.2) calls `clap`'s own `Error::exit()`
    // internally on a bad CLI invocation, which prints and terminates the
    // process with *clap's* exit code (0 for --help/--version, 2 for a
    // genuine usage error) -- entirely bypassing this tool's own 4-state
    // exit-code contract. `oba --bogus-flag` exited 2, indistinguishable
    // from a real INCONCLUSIVE analysis result, even though no analysis
    // ever ran. `try_parse()` instead hands the error back so it can be
    // mapped onto TOOL_ERROR (3) like every other "never got to run" case.
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let _ = e.print();
            // clap still distinguishes "this was actually an error"
            // (use_stderr) from a deliberate early exit like --help or
            // --version, which isn't a tool failure and keeps exit 0.
            std::process::exit(if e.use_stderr() { 3 } else { 0 });
        }
    };
    match run(&cli) {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(e) => {
            eprintln!("TOOL_ERROR: {e:#}");
            std::process::exit(3);
        }
    }
}

fn run(cli: &Cli) -> anyhow::Result<i32> {
    if let Some(Command::Check(args)) = &cli.command {
        if cli.targets.is_some() || cli.census.is_some() {
            anyhow::bail!(
                "cannot combine the `check` subcommand with legacy top-level --targets/--census"
            );
        }
        return run_check(&args.root, &args.targets, args.json);
    }

    if let Some(Command::Diff(args)) = &cli.command {
        if cli.targets.is_some() || cli.census.is_some() {
            anyhow::bail!(
                "cannot combine the `diff` subcommand with legacy top-level --targets/--census"
            );
        }
        return run_diff(&args.base_root, &args.head_root, &args.targets, args.json);
    }

    if let Some(Command::Audit(args)) = &cli.command {
        if cli.targets.is_some() || cli.census.is_some() {
            anyhow::bail!(
                "cannot combine the `audit` subcommand with legacy top-level --targets/--census"
            );
        }
        return run_audit(&args.root, &args.targets, args.format);
    }

    if let Some(Command::AuditDiff(args)) = &cli.command {
        if cli.targets.is_some() || cli.census.is_some() {
            anyhow::bail!(
                "cannot combine the `audit-diff` subcommand with legacy top-level --targets/--census"
            );
        }
        return run_audit_diff(
            &args.base_root,
            &args.head_root,
            &args.targets,
            args.format,
            args.summary_path.as_deref(),
        );
    }

    if let Some(dir) = &cli.census {
        return run_census(dir, cli.json);
    }

    // Legacy invocation (kept working for the real `v0.1.0` public
    // release that predates `check`/`--root`): exactly `check --root .
    // --targets <TARGETS>`, through the identical function, not a
    // separately-maintained approximation of it.
    let targets_path = cli.targets.as_ref().ok_or_else(|| {
        anyhow::anyhow!("either the `check` subcommand, or legacy --targets/--census, is required")
    })?;
    run_check(Path::new("."), targets_path, cli.json)
}

/// `root` is a real filesystem boundary (see `resolve_within_root`), never
/// just a base path to join and trust; `targets_path` is resolved relative
/// to the current directory, not to `root` -- see `CheckArgs::targets`'s
/// own doc comment for why those two paths deliberately don't share a base.
fn run_check(root: &Path, targets_path: &Path, json: bool) -> anyhow::Result<i32> {
    let manifest_src = fs::read_to_string(targets_path)
        .map_err(|e| anyhow::anyhow!("reading targets manifest {}: {e}", targets_path.display()))?;
    let manifest: TargetFile = toml::from_str(&manifest_src)
        .map_err(|e| anyhow::anyhow!("parsing targets manifest {}: {e}", targets_path.display()))?;
    validate_manifest(&manifest).map_err(|e| anyhow::anyhow!("invalid targets manifest: {e}"))?;

    // analyze()'s own `?`s (missing module/test file, a path escaping
    // --root, etc.) bubble up here as a genuine tool error too: a target
    // naming a file that doesn't exist -- or reaches outside --root -- is
    // a manifest problem, not an OBA001 finding.
    let reports = analyze(root, &manifest)?.targets;

    let findings = reports
        .iter()
        .flat_map(|r| &r.verdicts)
        .filter(|v| v.is_finding())
        .count();
    let inconclusive = reports
        .iter()
        .flat_map(|r| &r.verdicts)
        .filter(|v| v.is_inconclusive())
        .count();
    let pass = reports
        .iter()
        .flat_map(|r| &r.verdicts)
        .filter(|v| matches!(v, Verdict::Pass { .. }))
        .count();
    let parse_failed_targets = reports
        .iter()
        .filter(|r| !r.parse_errors.is_empty())
        .count();

    let (exit_code, status) = if inconclusive > 0 || parse_failed_targets > 0 {
        (2, "INCONCLUSIVE")
    } else if findings > 0 {
        (1, "FINDING")
    } else {
        (0, "PASS")
    };

    if json {
        let envelope = ReportEnvelope {
            schema_version: 1,
            tool: ToolInfo {
                name: "oba",
                version: env!("CARGO_PKG_VERSION"),
            },
            mode: "check",
            summary: CheckSummary {
                pass,
                finding: findings,
                inconclusive,
            },
            targets: reports,
        };
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("=== summary: {status}  findings={findings}  inconclusive={inconclusive} ===");
        for r in &reports {
            print_human(r);
        }
    }

    Ok(exit_code)
}

/// `oba diff`'s own versioned JSON payload -- deliberately a SEPARATE
/// Rust type from `ReportEnvelope`/`CheckSummary`, not one shared struct
/// forced to cover both `check` and `diff` shapes (they share the
/// `schema_version`/`tool`/`mode` field *names* by convention, not a
/// common Rust type -- a consumer branches on `mode` before parsing
/// `targets`, same as any tagged JSON union). `targets` here is
/// `Vec<ComparisonEntry>` (identity + diff), not `Vec<TargetReport>` --
/// forcing those into one enum would only be worse Rust for a JSON
/// convenience that doesn't actually exist on the wire.
#[derive(Serialize, Debug)]
struct DiffEnvelope {
    schema_version: u32,
    tool: ToolInfo,
    mode: &'static str,
    summary: DiffSummary,
    targets: Vec<ComparisonEntry>,
}

#[derive(Serialize, Debug)]
struct DiffSummary {
    unchanged: usize,
    added: usize,
    removed: usize,
    changed: usize,
    /// Bare counts of `VerdictTransition{from,to}` pairs (e.g.
    /// `"pass->oba001": 2`), nothing more -- explicitly NOT a
    /// regression/improvement classification. That judgment is CI
    /// policy, built on top of this count, never baked in here (same
    /// boundary `compare()` itself already draws around
    /// `VerdictTransition`).
    verdict_transitions: std::collections::BTreeMap<String, usize>,
}

/// D2: pure orchestration around D1's `compare()` -- two `analyze()`
/// calls (each already a full, independent, fail-closed analysis
/// through the exact same code `check` uses) and one `compare()` call.
/// No new comparison semantics get introduced here; if that temptation
/// shows up, it means D1's model was incomplete, not that D2 gets to
/// patch around it.
///
/// Exit codes are DELIBERATELY not `check`'s 0/1/2/3 with the same
/// meanings -- `check` answers "what's HEAD's state", `diff` answers
/// "what changed", and `compare()` is intentionally transition-neutral
/// (no `Changed` is inherently bad). So: `0` = the comparison was
/// produced (regardless of what it found -- a real `Changed` entry is
/// not a failure); `2` = the comparison was produced, but at least one
/// side's own analysis of at least one target was inconclusive (parse
/// error or an inconclusive-shaped verdict) -- inspected directly on
/// `base`/`head`'s `AnalysisReport`s themselves (mirroring `check`'s own
/// `is_inconclusive()` test, applied to both sides), NOT reconstructed
/// from `ComparisonReport` -- `TargetDiff::Unchanged` deliberately
/// carries no payload (see its own doc comment), so an inconclusive
/// verdict that's identical on both sides wouldn't even be visible from
/// the diff alone; `3` = a tool/input error (bad manifest, a missing or
/// `--*-root`-escaping file on either side, or `CompareError` -- a
/// duplicate identity is a manifest problem, same class as any other
/// `check` TOOL_ERROR, not a special diff-only failure mode).
fn run_diff(
    base_root: &Path,
    head_root: &Path,
    targets_path: &Path,
    json: bool,
) -> anyhow::Result<i32> {
    let manifest_src = fs::read_to_string(targets_path)
        .map_err(|e| anyhow::anyhow!("reading targets manifest {}: {e}", targets_path.display()))?;
    let manifest: TargetFile = toml::from_str(&manifest_src)
        .map_err(|e| anyhow::anyhow!("parsing targets manifest {}: {e}", targets_path.display()))?;
    validate_manifest(&manifest).map_err(|e| anyhow::anyhow!("invalid targets manifest: {e}"))?;

    let base_analysis = analyze(base_root, &manifest)?;
    let head_analysis = analyze(head_root, &manifest)?;

    let comparison =
        compare(&base_analysis, &head_analysis).map_err(|e| anyhow::anyhow!("{e}"))?;

    let inconclusive_side = |report: &AnalysisReport| {
        report
            .targets
            .iter()
            .any(|t| !t.parse_errors.is_empty() || t.verdicts.iter().any(|v| v.is_inconclusive()))
    };
    let exit_code = if inconclusive_side(&base_analysis) || inconclusive_side(&head_analysis) {
        2
    } else {
        0
    };

    let mut unchanged = 0;
    let mut added = 0;
    let mut removed = 0;
    let mut changed = 0;
    let mut verdict_transitions: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for entry in &comparison.entries {
        match &entry.diff {
            TargetDiff::Unchanged => unchanged += 1,
            TargetDiff::Added { .. } => added += 1,
            TargetDiff::Removed { .. } => removed += 1,
            TargetDiff::Changed { .. } => changed += 1,
        }
        if let Some(t) = entry.diff.verdict_transition() {
            *verdict_transitions
                .entry(format!("{}->{}", t.from.as_str(), t.to.as_str()))
                .or_insert(0) += 1;
        }
    }

    if json {
        let envelope = DiffEnvelope {
            schema_version: 1,
            tool: ToolInfo {
                name: "oba",
                version: env!("CARGO_PKG_VERSION"),
            },
            mode: "diff",
            summary: DiffSummary { unchanged, added, removed, changed, verdict_transitions },
            targets: comparison.entries,
        };
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!(
            "=== diff summary: unchanged={unchanged}  added={added}  removed={removed}  changed={changed} ==="
        );
        print_diff_human(&comparison);
    }

    Ok(exit_code)
}

/// P3a: `oba audit`'s own unified schema. Wraps BOTH OBA's own
/// per-target `Verdict` results and CDC's own `GeneratedConfigArtifact`
/// comparison results under one JSON envelope, WITHOUT merging their
/// internal semantics into one Rust enum -- `Verdict` and
/// `cdc::ConfigContractVerdict` stay their own separate types; this is
/// a presentation-layer unification only. Every checked target
/// (OBA option or CDC candidate) produces exactly one `AuditResult`,
/// including a clean `Pass` -- `code`/`severity` are `None` for `Pass`
/// (a code names a PROBLEM class, "uncovered option branch"/"emitted
/// config path rejected", never "nothing's wrong").
#[derive(Serialize, Debug)]
struct AuditEnvelope {
    schema_version: u32,
    tool: ToolInfo,
    mode: &'static str,
    summary: AuditSummary,
    results: Vec<AuditResult>,
}

#[derive(Serialize, Debug, Default)]
struct AuditSummary {
    pass: usize,
    finding: usize,
    inconclusive: usize,
    tool_error: usize,
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ResultVerdict {
    Pass,
    Finding,
    Inconclusive,
    ToolError,
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Severity {
    Warning,
    Error,
    Inconclusive,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
struct AuditResult {
    /// Which real analyzer engine produced this result -- `"oba"` or
    /// `"cdc"`. Routes a JSON reader to the right evidence sub-shape;
    /// never changes comparison semantics (the two engines' own
    /// comparison functions are untouched by this whole command).
    engine: &'static str,
    target: String,
    verdict: ResultVerdict,
    /// `"OBA001"` / `"CDC001"` / `"CDC002"` -- `None` for `Pass`, a
    /// code names a problem class, never "nothing's wrong".
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    severity: Option<Severity>,
    message: String,
    /// A real, ordered, human-and-machine-readable chain from the Nix
    /// option/producer through to the mismatch (or the clean pass) --
    /// so a maintainer can understand a result without reading
    /// `cdc.rs`/`main.rs`'s own internal types.
    provenance: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cdc_evidence: Option<CdcResultEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    oba_evidence: Option<ObaResultEvidence>,
}

/// The exact shape the user's own P3a design sketch specified:
/// producer/binding/consumer/comparison as real, separately-inspectable
/// evidence, `proof_depth` visible rather than folded into a single
/// opaque `PASS` a reader could misread as "byte-exact proof this
/// process reads these exact bytes" -- C-E1.2c's own real, audited
/// finding (9 of 11 real candidates are architectural, not byte-exact)
/// stays visible on every single result, not just in a README.
#[derive(Serialize, Debug, Clone, PartialEq)]
struct CdcResultEvidence {
    producer: CdcProducerEvidence,
    binding: CdcBindingEvidence,
    consumer: CdcConsumerEvidence,
    comparison: CdcComparisonEvidence,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
struct CdcProducerEvidence {
    proved: bool,
    format: cdc::ConfigFormat,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
struct CdcBindingEvidence {
    kind: &'static str,
    proof_depth: cdc::ProofDepth,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
struct CdcConsumerEvidence {
    name: String,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
struct CdcComparisonEvidence {
    emitted_paths: Vec<String>,
    accepted_paths: Vec<String>,
    opaque_paths: Vec<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
struct ObaResultEvidence {
    option: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    predicate_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_outcome: Option<bool>,
    test_assignments: usize,
}

fn cdc_outcome_to_result(name: &str, outcome: cdc::CdcCandidateOutcome) -> AuditResult {
    let mut provenance = vec![
        format!("Nix option: services.{}", outcome.candidate),
        format!("rendered artifact: {:?} format", outcome.format),
        format!("process binding: {} ({:?})", outcome.binding_kind, outcome.proof_depth),
        format!("pinned upstream consumer: {}", outcome.consumer),
        format!("accepted contract: {} real accepted path(s)", outcome.accepted_paths.len()),
    ];
    let (verdict, code, severity, message) = match &outcome.verdict {
        cdc::ConfigContractVerdict::Pass => {
            provenance.push("comparison: every real emitted path is accepted".to_string());
            (ResultVerdict::Pass, None, None, "every real emitted config path is accepted by the pinned consumer".to_string())
        }
        cdc::ConfigContractVerdict::Finding { unaccepted_path } => {
            provenance.push(format!("mismatch: emitted path {unaccepted_path:?} is not in the accepted contract"));
            (
                ResultVerdict::Finding,
                Some("CDC001"),
                Some(Severity::Error),
                format!("emitted config path {unaccepted_path:?} is rejected by the pinned consumer's own real accepted contract"),
            )
        }
    };
    AuditResult {
        engine: "cdc",
        target: name.to_string(),
        verdict,
        code,
        severity,
        message,
        provenance,
        cdc_evidence: Some(CdcResultEvidence {
            producer: CdcProducerEvidence { proved: true, format: outcome.format },
            binding: CdcBindingEvidence { kind: outcome.binding_kind, proof_depth: outcome.proof_depth },
            consumer: CdcConsumerEvidence { name: outcome.consumer },
            comparison: CdcComparisonEvidence {
                emitted_paths: outcome.emitted_paths,
                accepted_paths: outcome.accepted_paths,
                opaque_paths: outcome.opaque_paths,
            },
        }),
        oba_evidence: None,
    }
}

fn cdc_error_to_result(name: &str, err: &cdc::CdcError) -> AuditResult {
    match err {
        cdc::CdcError::Inconclusive(reason) => AuditResult {
            engine: "cdc",
            target: name.to_string(),
            verdict: ResultVerdict::Inconclusive,
            code: Some("CDC002"),
            severity: Some(Severity::Inconclusive),
            message: format!("consumer/binding evidence incomplete: {reason}"),
            provenance: vec![
                format!("Nix option: services.{name}"),
                format!("evidence gap: {reason}"),
            ],
            cdc_evidence: None,
            oba_evidence: None,
        },
        cdc::CdcError::ToolError(reason) => AuditResult {
            engine: "cdc",
            target: name.to_string(),
            verdict: ResultVerdict::ToolError,
            code: None,
            severity: None,
            message: format!("analysis did not run: {reason}"),
            provenance: vec![format!("tool error: {reason}")],
            cdc_evidence: None,
            oba_evidence: None,
        },
    }
}

fn oba_verdict_to_result(option: &str, verdict: &Verdict) -> AuditResult {
    let target = option.to_string();
    match verdict {
        Verdict::OptionNotFound { option } => AuditResult {
            engine: "oba",
            target: target.clone(),
            verdict: ResultVerdict::Inconclusive,
            code: None,
            severity: Some(Severity::Inconclusive),
            message: "no mkOption declaration found for this watched option".to_string(),
            provenance: vec![format!("Nix option: {option}"), "declaration: not found".to_string()],
            cdc_evidence: None,
            oba_evidence: Some(ObaResultEvidence {
                option: option.clone(),
                predicate_source: None,
                default_outcome: None,
                test_assignments: 0,
            }),
        },
        Verdict::PredicateNotFound { option } => AuditResult {
            engine: "oba",
            target: target.clone(),
            verdict: ResultVerdict::Inconclusive,
            code: None,
            severity: Some(Severity::Inconclusive),
            message: "declaration found, but no direct branch predicate was found for this option".to_string(),
            provenance: vec![
                format!("Nix option: {option}"),
                "declaration: found".to_string(),
                "predicate: not found".to_string(),
            ],
            cdc_evidence: None,
            oba_evidence: Some(ObaResultEvidence {
                option: option.clone(),
                predicate_source: None,
                default_outcome: None,
                test_assignments: 0,
            }),
        },
        Verdict::DefaultUnresolved { option, predicate } => AuditResult {
            engine: "oba",
            target: target.clone(),
            verdict: ResultVerdict::Inconclusive,
            code: None,
            severity: Some(Severity::Inconclusive),
            message: "predicate found, but its default outcome could not be statically classified".to_string(),
            provenance: vec![
                format!("Nix option: {option}"),
                format!("predicate: {}", predicate.source()),
                "default outcome: unresolved".to_string(),
            ],
            cdc_evidence: None,
            oba_evidence: Some(ObaResultEvidence {
                option: option.clone(),
                predicate_source: Some(predicate.source().to_string()),
                default_outcome: None,
                test_assignments: 0,
            }),
        },
        Verdict::TestValueUnresolved { option, predicate, default_outcome, .. } => AuditResult {
            engine: "oba",
            target: target.clone(),
            verdict: ResultVerdict::Inconclusive,
            code: None,
            severity: Some(Severity::Inconclusive),
            message: "a classifiable default and predicate exist, but no test assignment's value could be statically classified".to_string(),
            provenance: vec![
                format!("Nix option: {option}"),
                format!("predicate: {}", predicate.source()),
                format!("default outcome: {default_outcome:?}"),
                "test evidence: unresolved".to_string(),
            ],
            cdc_evidence: None,
            oba_evidence: Some(ObaResultEvidence {
                option: option.clone(),
                predicate_source: Some(predicate.source().to_string()),
                default_outcome: *default_outcome,
                test_assignments: 0,
            }),
        },
        Verdict::TestConfigUnresolved { option, predicate, default_outcome, .. } => AuditResult {
            engine: "oba",
            target: target.clone(),
            verdict: ResultVerdict::Inconclusive,
            code: None,
            severity: Some(Severity::Inconclusive),
            message: "a classifiable default exists, but part of the test config that could contain this option was not visible to the scanner (imports/alias/function call)".to_string(),
            provenance: vec![
                format!("Nix option: {option}"),
                format!("predicate: {}", predicate.source()),
                format!("default outcome: {default_outcome:?}"),
                "test config: partially opaque".to_string(),
            ],
            cdc_evidence: None,
            oba_evidence: Some(ObaResultEvidence {
                option: option.clone(),
                predicate_source: Some(predicate.source().to_string()),
                default_outcome: *default_outcome,
                test_assignments: 0,
            }),
        },
        Verdict::Oba001 { option, predicate, default_outcome, .. } => AuditResult {
            engine: "oba",
            target: target.clone(),
            verdict: ResultVerdict::Finding,
            code: Some("OBA001"),
            severity: Some(Severity::Warning),
            message: "uncovered option branch: no test assignment provably flips this predicate away from its default".to_string(),
            provenance: vec![
                format!("Nix option: {option}"),
                format!("predicate: {}", predicate.source()),
                format!("default outcome: {default_outcome:?}"),
                "test evidence: none flips it".to_string(),
            ],
            cdc_evidence: None,
            oba_evidence: Some(ObaResultEvidence {
                option: option.clone(),
                predicate_source: Some(predicate.source().to_string()),
                default_outcome: *default_outcome,
                test_assignments: 0,
            }),
        },
        Verdict::Pass { option, predicate, default_outcome, evidence, .. } => AuditResult {
            engine: "oba",
            target: target.clone(),
            verdict: ResultVerdict::Pass,
            code: None,
            severity: None,
            message: "a real test assignment provably flips this predicate away from its default".to_string(),
            provenance: vec![
                format!("Nix option: {option}"),
                format!("predicate: {}", predicate.source()),
                format!("default outcome: {default_outcome}"),
                format!("test evidence: {} assignment(s) prove a transition", evidence.len()),
            ],
            cdc_evidence: None,
            oba_evidence: Some(ObaResultEvidence {
                option: option.clone(),
                predicate_source: Some(predicate.source().to_string()),
                default_outcome: Some(*default_outcome),
                test_assignments: evidence.len(),
            }),
        },
    }
}

/// `root` is the real, shared security/evaluation boundary for BOTH
/// engines -- see `AuditArgs::root`'s own doc comment. CDC's own
/// candidates need `root`'s real, absolute, canonicalized form (a
/// relative `--root .` would be meaningless once handed to `nix eval`
/// as an evaluation root from an arbitrary child process's own cwd).
fn run_audit(root: &Path, targets_path: &Path, format: AuditFormat) -> anyhow::Result<i32> {
    let manifest_src = fs::read_to_string(targets_path)
        .map_err(|e| anyhow::anyhow!("reading targets manifest {}: {e}", targets_path.display()))?;
    let manifest: TargetFile = toml::from_str(&manifest_src)
        .map_err(|e| anyhow::anyhow!("parsing targets manifest {}: {e}", targets_path.display()))?;
    validate_manifest(&manifest).map_err(|e| anyhow::anyhow!("invalid targets manifest: {e}"))?;

    let mut results = Vec::new();

    if !manifest.target.is_empty() {
        let oba_reports = analyze(root, &manifest)?.targets;
        for report in &oba_reports {
            for verdict in &report.verdicts {
                results.push(oba_verdict_to_result(verdict.option(), verdict));
            }
        }
    }

    if !manifest.cdc_target.is_empty() {
        let root_abs = root
            .canonicalize()
            .map_err(|e| anyhow::anyhow!("resolving --root {} for CDC: {e}", root.display()))?;
        let nixpkgs = cdc::NixpkgsSource::LocalPath(root_abs);
        for t in &manifest.cdc_target {
            let result = match cdc::run_cdc_candidate(&t.name, &nixpkgs) {
                Ok(outcome) => cdc_outcome_to_result(&t.name, outcome),
                Err(err) => cdc_error_to_result(&t.name, &err),
            };
            results.push(result);
        }
    }

    let mut summary = AuditSummary::default();
    for r in &results {
        match r.verdict {
            ResultVerdict::Pass => summary.pass += 1,
            ResultVerdict::Finding => summary.finding += 1,
            ResultVerdict::Inconclusive => summary.inconclusive += 1,
            ResultVerdict::ToolError => summary.tool_error += 1,
        }
    }

    // same 4-state escalation this whole project's exit-code contract
    // already uses (check/diff), worst-state-wins: a per-candidate CDC
    // tool error is real evidence the audit is incomplete, so it
    // outranks a mere finding the same way a manifest-level tool error
    // always has.
    let exit_code = if summary.tool_error > 0 {
        3
    } else if summary.inconclusive > 0 {
        2
    } else if summary.finding > 0 {
        1
    } else {
        0
    };

    match format {
        AuditFormat::Json => {
            let envelope = AuditEnvelope {
                schema_version: 1,
                tool: ToolInfo { name: "oba", version: env!("CARGO_PKG_VERSION") },
                mode: "audit",
                summary,
                results,
            };
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        AuditFormat::Text => {
            println!(
                "=== audit summary: pass={} finding={} inconclusive={} tool_error={} ===",
                summary.pass, summary.finding, summary.inconclusive, summary.tool_error
            );
            for r in &results {
                let tag = match r.verdict {
                    ResultVerdict::Pass => "PASS".to_string(),
                    ResultVerdict::Finding => format!("FINDING {}", r.code.unwrap_or("?")),
                    ResultVerdict::Inconclusive => {
                        format!("INCONCLUSIVE{}", r.code.map(|c| format!(" {c}")).unwrap_or_default())
                    }
                    ResultVerdict::ToolError => "TOOL_ERROR".to_string(),
                };
                println!("[{}] {} :: {} -- {}", r.engine, tag, r.target, r.message);
                for step in &r.provenance {
                    println!("    -> {step}");
                }
            }
        }
    }

    Ok(exit_code)
}

// =======================================================================
// P3b: `oba audit-diff` -- the unified counterpart to `oba diff`.
// `oba audit` (P3a) answers "what's in one state"; this answers "what
// changed between two states", the minimal next layer a real PR
// workflow actually needs (without it, a future GitHub Action would
// just print two huge reports side by side).
//
// Pre-registered invariants (checked by real tests below, not just
// asserted in prose):
//   1. `audit-diff A A` -> every entry `Unchanged`.
//   2. Target/finding order never affects the result (deterministic,
//      sorted output).
//   3. A duplicate identity on either side is a real TOOL_ERROR, never
//      first-wins -- OBA's own existing `compare()`/`CompareError`
//      already enforces this for OBA identities; CDC's own candidate
//      names are already guaranteed unique by `validate_manifest`
//      (defensively re-checked here anyway, not just trusted).
//   4. Presentation/provenance text changing alone never creates a
//      semantic (`VerdictChanged`) diff by itself -- only a REAL
//      `verdict` difference does; evidence-only differences (e.g. a
//      real `proof_depth` change, or a real dotted-path list changing)
//      become their own `EvidenceChanged`, never a brand-new finding.
//   5. PASS/FINDING/INCONCLUSIVE(/TOOL_ERROR for CDC) transitions are
//      counted explicitly, never silently absorbed into "Changed".
//   6. A CDC candidate's own real ANALYZABILITY changing (a real
//      `ToolError` on one side, a real verdict on the other) is its own
//      `AddedSubject`/`RemovedSubject` kind, never an ordinary
//      `VerdictChanged` -- "couldn't even try" -> "here's a real
//      verdict" is a qualitatively different, more significant event
//      than an ordinary verdict flip.
//   7. `--base-root`/`--head-root` keep the exact same real security
//      boundary `check --root`/`diff --*-root` already have.
//   8. No Git anywhere in this file -- two plain directories in, a pure
//      comparison out, exactly `oba diff`'s own D2 philosophy.
//   9. Exit code never fails just because a real `Finding` exists --
//      matching `oba diff`'s own existing 0/2/3 scheme (0 = comparison
//      produced; 2 = analysis inconclusive on either side, INCLUDING a
//      per-CDC-candidate real `ToolError` -- a real, disclosed
//      "couldn't determine this one" state within an otherwise-
//      successful run, not the same class as a whole-run failure; 3 =
//      a genuine whole-run tool/input error -- bad manifest, a missing
//      OBA module/test file, a duplicate identity). Exit `1` never
//      appears for this command at all, same as `oba diff`.
// =======================================================================

#[derive(clap::Args)]
struct AuditDiffArgs {
    /// Same real security boundary as `audit --root`/`diff --base-root`.
    #[arg(long)]
    base_root: PathBuf,
    /// Same real security boundary as `audit --root`/`diff --head-root`.
    #[arg(long)]
    head_root: PathBuf,
    /// ONE manifest applied to both roots -- same deliberate choice
    /// `oba diff` already made (comparing two different *specifications*
    /// of what to watch is a different, messier question this tool
    /// isn't trying to answer).
    #[arg(long)]
    targets: PathBuf,
    #[arg(long, value_enum, default_value_t = AuditFormat::Text)]
    format: AuditFormat,
    /// P3c: when set, ALSO write a bounded, ready-to-post GitHub Actions
    /// step-summary Markdown document to this path -- computed from the
    /// SAME run, zero extra `nix eval` cost. Exists so a consuming
    /// Action never has to reclassify or reformat the JSON report
    /// itself (see `render_github_summary`'s own doc comment); `--format`
    /// (json/text) is unaffected either way.
    #[arg(long = "summary-path")]
    summary_path: Option<PathBuf>,
}

/// CDC's own diff algebra, parallel to (never modifying) OBA's own
/// existing `ChangeKind`/`TargetDiff`/`compare()`. CDC candidates are
/// always present on both sides by construction (one shared manifest,
/// `validate_manifest` already guarantees each `[[cdc_target]]` name is
/// unique) -- there is no ordinary "Added/Removed" the way OBA's own
/// `compare()` has for an option genuinely absent from one side's own
/// watch list. What CAN happen, and needs its own real vocabulary
/// (invariant 6 above), is a candidate's own real analyzability
/// changing between base and head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CdcChangeKind {
    VerdictChanged,
    EvidenceChanged,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CdcDiff {
    Unchanged,
    AddedSubject { head: Box<AuditResult> },
    RemovedSubject { base: Box<AuditResult> },
    Changed { base: Box<AuditResult>, head: Box<AuditResult>, changes: Vec<CdcChangeKind> },
}

impl CdcDiff {
    fn verdict_transition(&self) -> Option<(ResultVerdict, ResultVerdict)> {
        match self {
            CdcDiff::Changed { base, head, changes } if changes.contains(&CdcChangeKind::VerdictChanged) => {
                Some((base.verdict, head.verdict))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct CdcComparisonEntry {
    candidate: String,
    diff: CdcDiff,
}

/// Mirrors `compare()`'s own `CompareError` shape and purpose (invariant
/// 3) -- defensive, not load-bearing: `validate_manifest` already
/// rejects a duplicate `[[cdc_target]]` name before any audit ever
/// runs, so this should be unreachable in practice, but "should be
/// unreachable" is exactly the kind of claim this project's own
/// discipline never takes on faith.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DuplicateCdcCandidateError {
    side: CompareSide,
    candidate: String,
}

impl std::fmt::Display for DuplicateCdcCandidateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "duplicate CDC candidate result on the {:?} side: {}", self.side, self.candidate)
    }
}

impl std::error::Error for DuplicateCdcCandidateError {}

fn index_cdc_results_by_candidate(
    results: &[AuditResult],
    side: CompareSide,
) -> Result<std::collections::BTreeMap<String, &AuditResult>, DuplicateCdcCandidateError> {
    let mut index = std::collections::BTreeMap::new();
    for r in results.iter().filter(|r| r.engine == "cdc") {
        if index.insert(r.target.clone(), r).is_some() {
            return Err(DuplicateCdcCandidateError { side, candidate: r.target.clone() });
        }
    }
    Ok(index)
}

/// Real difference in EVIDENCE (not just the top-level `verdict`) --
/// `proof_depth`, `binding.kind`, `consumer.name`, or any of the real
/// dotted-path lists differing. Deliberately does NOT look at
/// `provenance`/`message` text at all (invariant 4 -- presentation
/// text is explicitly excluded from ever counting as a change on its
/// own).
fn cdc_evidence_differs(base: &AuditResult, head: &AuditResult) -> bool {
    base.cdc_evidence != head.cdc_evidence || base.oba_evidence != head.oba_evidence
}

fn compare_cdc_result(base: &AuditResult, head: &AuditResult) -> CdcDiff {
    match (base.verdict, head.verdict) {
        (ResultVerdict::ToolError, ResultVerdict::ToolError) => CdcDiff::Unchanged,
        (ResultVerdict::ToolError, _) => CdcDiff::AddedSubject { head: Box::new(head.clone()) },
        (_, ResultVerdict::ToolError) => CdcDiff::RemovedSubject { base: Box::new(base.clone()) },
        (b, h) => {
            let mut changes = Vec::new();
            if b != h {
                changes.push(CdcChangeKind::VerdictChanged);
            } else if cdc_evidence_differs(base, head) {
                // only flagged when the verdict itself did NOT change --
                // if it did, the evidence differing is expected/implied,
                // not worth double-reporting (invariant 4/5).
                changes.push(CdcChangeKind::EvidenceChanged);
            }
            if changes.is_empty() {
                CdcDiff::Unchanged
            } else {
                CdcDiff::Changed { base: Box::new(base.clone()), head: Box::new(head.clone()), changes }
            }
        }
    }
}

/// Pure: given both sides' already-complete real `AuditResult` lists,
/// produces a deterministic, sorted-by-candidate-name CDC comparison.
/// No filesystem, no Git, no GitHub (invariant 8).
fn compare_cdc(
    base_results: &[AuditResult],
    head_results: &[AuditResult],
) -> anyhow::Result<Vec<CdcComparisonEntry>> {
    let base_index = index_cdc_results_by_candidate(base_results, CompareSide::Base)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let head_index = index_cdc_results_by_candidate(head_results, CompareSide::Head)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    names.extend(base_index.keys().cloned());
    names.extend(head_index.keys().cloned());
    let mut entries = Vec::with_capacity(names.len());
    for candidate in names {
        // both indexes are built from the SAME shared manifest's own
        // `[[cdc_target]]` list applied to both roots, so a name
        // present in the union is, by construction, present in BOTH
        // indexes -- there is no real "candidate only exists on one
        // side" case for CDC the way there is for OBA's own `[[target]]`
        // entries (see this whole section's own header comment).
        let (Some(base), Some(head)) = (base_index.get(&candidate), head_index.get(&candidate))
        else {
            anyhow::bail!(
                "internal error: CDC candidate {candidate:?} missing from one side despite one shared manifest"
            );
        };
        entries.push(CdcComparisonEntry { candidate, diff: compare_cdc_result(base, head) });
    }
    Ok(entries)
}

/// Classifies a bare `VerdictKind` into the SAME coarse Pass/Finding/
/// Inconclusive space `oba_verdict_to_result` already assigns every real
/// `Verdict` -- reused, not reinvented, so a `VerdictTransition{from,to}`
/// (kind-only, no full `Verdict` payload) can be bucketed the same way a
/// full result already is elsewhere in this file.
fn oba_kind_class(kind: VerdictKind) -> ResultVerdict {
    match kind {
        VerdictKind::Oba001 => ResultVerdict::Finding,
        VerdictKind::Pass => ResultVerdict::Pass,
        VerdictKind::OptionNotFound
        | VerdictKind::PredicateNotFound
        | VerdictKind::DefaultUnresolved
        | VerdictKind::TestValueUnresolved
        | VerdictKind::TestConfigUnresolved => ResultVerdict::Inconclusive,
    }
}

/// A bounded, already-classified transition worth a maintainer's
/// immediate attention -- P3c's own real requirement: a consuming
/// GitHub Action must never re-derive "is this a new finding" from raw
/// JSON itself (that would smuggle real analysis logic into YAML/jq).
/// Every field here is either a bare fact already decided elsewhere
/// (`bucket`/`engine`/`code`) or existing text this analysis already
/// produced verbatim (`message`/`detail`, the single most specific
/// existing provenance line) -- nothing here is new judgment.
#[derive(Serialize, Debug, Clone, PartialEq)]
struct NotableChange {
    bucket: &'static str,
    engine: &'static str,
    subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'static str>,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

/// Per invariant-adjacent P3c requirement: the summary stays bounded
/// even when a real corpus produces far more than a handful of new
/// findings/inconclusives -- `notable_total` (uncapped) lets a consumer
/// print "+N more in the full artifact" without recomputing anything.
const NOTABLE_LIMIT: usize = 10;

/// The one real judgment call in the bounded summary, made exactly ONCE
/// here in tested Rust -- never re-derived by a consuming Action's own
/// jq/shell logic. `from`/`to` are already-computed `ResultVerdict`s
/// (see `oba_kind_class`/`CdcDiff::verdict_transition`), so this is pure
/// classification, no analysis.
///
/// S1-F2/S1-F3 (a real nixpkgs PR shadow audit): the original P3c
/// taxonomy folded `Finding -> Pass` and `Finding -> Inconclusive` into
/// one `resolved_finding` bucket, and left `Inconclusive -> Pass`
/// unbucketed entirely. S1 found both choices cost real signal: a real
/// PR (#492803) showed `resolved_finding` reading as "the coverage gap
/// was fixed" when the true event was "the option was removed" (a
/// DIFFERENT transition shape entirely, `Finding -> RemovedSubject`,
/// handled separately in `run_audit_diff`'s own per-entry loop, never
/// here); two other real PRs (#559627, #561557) showed a genuine
/// `Inconclusive -> Pass` improvement that never reached a maintainer
/// because nothing surfaced it. Fixed here by SPLITTING, not by
/// smuggling in a "how happy should I be" score:
///
/// - `Finding -> Pass` only: `resolved_finding` -- the branch is now
///   provably covered, unambiguously good.
/// - `Finding -> Inconclusive`: `finding_became_inconclusive` -- a
///   DIFFERENT event (test/predicate visibility was lost, not gained;
///   see D1/D2's own "mechanism reports facts, never plays moral
///   philosopher" principle, applied here rather than judging whether
///   this is good or bad).
/// - `Pass -> Inconclusive`: `new_inconclusive`, unchanged.
/// - `Inconclusive -> Pass`: `resolved_inconclusive` -- now surfaced,
///   the real positive signal S1 found being silently dropped.
///
/// `Inconclusive -> Inconclusive` under a different underlying
/// `VerdictKind` (OBA's own sub-kinds only) still deliberately returns
/// `None` -- a real `Changed` that never crosses a Pass/Finding/
/// Inconclusive boundary at all.
fn classify_transition_bucket(from: ResultVerdict, to: ResultVerdict) -> Option<&'static str> {
    match (from, to) {
        (f, ResultVerdict::Finding) if f != ResultVerdict::Finding => Some("new_finding"),
        (ResultVerdict::Finding, ResultVerdict::Pass) => Some("resolved_finding"),
        (ResultVerdict::Finding, ResultVerdict::Inconclusive) => Some("finding_became_inconclusive"),
        (ResultVerdict::Pass, ResultVerdict::Inconclusive) => Some("new_inconclusive"),
        (ResultVerdict::Inconclusive, ResultVerdict::Pass) => Some("resolved_inconclusive"),
        _ => None,
    }
}

/// S1-F2: `TargetDiff::Removed`/`CdcDiff::RemovedSubject` whose REMOVED
/// (base) side was a real `Finding` is its own honest event --
/// `removed_subject_with_finding` -- never folded into
/// `resolved_finding` (that name specifically means "the same subject
/// is now provably Pass", not "the subject, and whatever it found,
/// both stopped existing"). Core states the fact; whether removing that
/// option/candidate was the right call for the PR to make is a human
/// judgment this function does not attempt.
fn removed_with_finding_bucket(removed_class: ResultVerdict) -> Option<&'static str> {
    (removed_class == ResultVerdict::Finding).then_some("removed_subject_with_finding")
}

/// S1-F2/S1-F3 symmetric counterpart for `Added`/`AddedSubject`: a
/// brand-new subject (module birth, or a CDC candidate that just
/// became analyzable) whose FIRST-EVER result is already a `Finding`
/// or `Inconclusive` is real, immediate signal -- folded into the SAME
/// `new_finding`/`new_inconclusive` buckets a same-subject transition
/// would use (there is no ambiguity to split here the way removal has:
/// "a new finding exists" is unambiguous regardless of whether the
/// subject itself is also new).
fn added_bucket(added_class: ResultVerdict) -> Option<&'static str> {
    match added_class {
        ResultVerdict::Finding => Some("new_finding"),
        ResultVerdict::Inconclusive => Some("new_inconclusive"),
        _ => None,
    }
}

fn push_notable(
    notable: &mut Vec<NotableChange>,
    notable_total: &mut usize,
    bucket: &'static str,
    engine: &'static str,
    subject: String,
    result: &AuditResult,
) {
    *notable_total += 1;
    if notable.len() < NOTABLE_LIMIT {
        notable.push(NotableChange {
            bucket,
            engine,
            subject,
            code: result.code,
            message: result.message.clone(),
            detail: result.provenance.last().cloned(),
        });
    }
}

/// Applies a `classify_transition_bucket`/`removed_with_finding_bucket`/
/// `added_bucket` result to `summary`'s own matching counter. A bucket
/// name that doesn't match any known field is a no-op, not a panic --
/// defensive against a future bucket being added to the classifiers
/// without this function being updated in lockstep (would rather silently
/// undercount than crash a real `audit-diff` run).
fn bump_summary_bucket(summary: &mut AuditDiffSummary, bucket: &str) {
    match bucket {
        "new_finding" => summary.new_findings += 1,
        "resolved_finding" => summary.resolved_findings += 1,
        "finding_became_inconclusive" => summary.finding_became_inconclusive += 1,
        "new_inconclusive" => summary.new_inconclusives += 1,
        "resolved_inconclusive" => summary.resolved_inconclusives += 1,
        "removed_subject_with_finding" => summary.removed_subjects_with_finding += 1,
        _ => {}
    }
}

/// Bumps the matching counter AND, for every bucket except
/// `resolved_finding`, pushes a bounded `notable` entry. `resolved_finding`
/// is the one deliberate exception (unchanged from P3c's own original
/// reasoning): "this branch is now provably covered" is unambiguous good
/// news that needs a count, not a maintainer's attention -- every other
/// bucket (including `resolved_inconclusive`, S1-F3's own real finding
/// that a genuine improvement was being silently dropped) is either a
/// real problem, a real loss of certainty, or positive-but-ambiguous-
/// enough-to-be-worth-a-glance news, so all of those surface.
fn record_bucket(
    summary: &mut AuditDiffSummary,
    notable_total: &mut usize,
    bucket: &'static str,
    engine: &'static str,
    subject: String,
    result: &AuditResult,
) {
    bump_summary_bucket(summary, bucket);
    if bucket != "resolved_finding" {
        push_notable(&mut summary.notable, notable_total, bucket, engine, subject, result);
    }
}

#[derive(Serialize, Debug)]
struct AuditDiffEnvelope {
    schema_version: u32,
    tool: ToolInfo,
    mode: &'static str,
    summary: AuditDiffSummary,
    oba: Vec<ComparisonEntry>,
    cdc: Vec<CdcComparisonEntry>,
}

#[derive(Serialize, Debug, Default)]
struct AuditDiffSummary {
    unchanged: usize,
    added: usize,
    removed: usize,
    changed: usize,
    /// CDC's own `AddedSubject`/`RemovedSubject` kept as their own
    /// counters, never merged into `added`/`removed` -- those two mean
    /// "watch list itself changed" for OBA; conflating a real
    /// analyzability change into the same bucket would make a future
    /// summary reader draw the wrong conclusion about WHY the count
    /// moved (invariant 6).
    cdc_added_subject: usize,
    cdc_removed_subject: usize,
    /// Bare counts of `"{from}->{to}"` pairs, kept SEPARATE per engine
    /// (OBA's own 7-way `VerdictKind` and CDC's own 4-way
    /// `ResultVerdict` are different vocabularies; merging them into one
    /// map would make e.g. `"pass->finding"` ambiguous about which
    /// engine it came from). Explicitly NOT a regression/improvement
    /// classification -- same boundary `oba diff` itself already draws.
    oba_verdict_transitions: std::collections::BTreeMap<String, usize>,
    cdc_verdict_transitions: std::collections::BTreeMap<String, usize>,
    /// Derived, bounded-summary-friendly counts, computed from the SAME
    /// per-entry transitions above -- never a second source of truth.
    /// See `classify_transition_bucket`'s own doc comment (S1-F2/S1-F3)
    /// for the exact, deliberately narrow meaning of each: `new_finding`
    /// (crossing INTO Finding, from either a same-subject transition or
    /// a brand-new subject's first result -- `added_bucket`);
    /// `resolved_finding` (Finding -> Pass ONLY, never Finding ->
    /// Inconclusive and never a removed subject); `finding_became_
    /// inconclusive` (Finding -> Inconclusive, a DIFFERENT event from
    /// `resolved_finding`, not merged into it); `new_inconclusive`
    /// (Pass -> Inconclusive, or a brand-new subject's first result);
    /// `resolved_inconclusive` (Inconclusive -> Pass, S1's own real
    /// finding that this used to be silently dropped);
    /// `removed_subjects_with_finding` (a `Removed`/`RemovedSubject`
    /// whose own removed side was a real Finding -- see
    /// `removed_with_finding_bucket`, never counted as a
    /// `resolved_finding`). `evidence_changed` is a `Changed` entry with
    /// NO verdict transition at all (CDC's own `EvidenceChanged`, e.g. a
    /// `proof_depth` move) -- distinct from `changed` above, which
    /// already counts every `Changed` entry regardless of kind.
    new_findings: usize,
    resolved_findings: usize,
    finding_became_inconclusive: usize,
    new_inconclusives: usize,
    resolved_inconclusives: usize,
    removed_subjects_with_finding: usize,
    evidence_changed: usize,
    /// Uncapped count backing `notable` below -- lets a consumer print
    /// "+N more in the full artifact" without recomputing anything.
    notable_total: usize,
    /// First `NOTABLE_LIMIT` entries across every bucket above (all six
    /// as of S1-F2/S1-F3), in deterministic (per-engine, then identity/
    /// candidate-sorted) order -- already fully classified and
    /// formatted; a consuming Action only ever needs to print these
    /// fields verbatim, never reclassify them.
    notable: Vec<NotableChange>,
}

fn cdc_verdict_str(v: ResultVerdict) -> &'static str {
    match v {
        ResultVerdict::Pass => "pass",
        ResultVerdict::Finding => "finding",
        ResultVerdict::Inconclusive => "inconclusive",
        ResultVerdict::ToolError => "tool_error",
    }
}

/// P3c: renders `--summary-path`'s own bounded GitHub Actions step
/// summary, in Markdown, ready to append to `$GITHUB_STEP_SUMMARY`
/// verbatim. Pure and fully offline-testable -- takes an
/// already-computed `AuditDiffSummary` (built once, by `run_audit_diff`
/// itself), never re-derives anything from raw JSON. This is the whole
/// point of P3c's own "dumb Action" requirement: every classification
/// decision (`new_finding` vs `new_inconclusive`, what counts as
/// notable) already happened in `classify_transition_bucket`/
/// `push_notable`; this function only ever formats already-decided
/// facts as Markdown, the same way a consuming Action's `run:` step is
/// only ever meant to `cat` this file into the real step summary, never
/// re-interpret the JSON report itself.
fn render_github_summary(summary: &AuditDiffSummary, exit_code: i32) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    out.push_str("## Nix contract audit\n\n");
    if exit_code == 2 {
        out.push_str(
            "_Analysis was inconclusive on at least one side -- see the full artifact for details._\n\n",
        );
    }
    out.push_str("| | |\n|---|---:|\n");
    let _ = writeln!(out, "| New findings | {} |", summary.new_findings);
    let _ = writeln!(out, "| Resolved findings | {} |", summary.resolved_findings);
    let _ = writeln!(out, "| Findings became inconclusive | {} |", summary.finding_became_inconclusive);
    let _ = writeln!(out, "| New inconclusives | {} |", summary.new_inconclusives);
    let _ = writeln!(out, "| Resolved inconclusives | {} |", summary.resolved_inconclusives);
    let _ = writeln!(out, "| Removed subjects with a finding | {} |", summary.removed_subjects_with_finding);
    let _ = writeln!(out, "| Evidence-only changes | {} |", summary.evidence_changed);
    let _ = writeln!(out, "| Unchanged | {} |", summary.unchanged);
    out.push('\n');

    for n in &summary.notable {
        let heading = match n.bucket {
            "new_finding" => "NEW FINDING",
            "new_inconclusive" => "NEW INCONCLUSIVE",
            "finding_became_inconclusive" => "FINDING BECAME INCONCLUSIVE",
            "resolved_inconclusive" => "RESOLVED INCONCLUSIVE",
            "removed_subject_with_finding" => "REMOVED SUBJECT (HAD A FINDING)",
            // Defensive: `record_bucket` never calls this with
            // "resolved_finding" (the one bucket that stays count-only,
            // see its own doc comment) or an unrecognized label, but this
            // function never assumes that silently -- either still
            // renders, just without a specially-cased heading.
            other => other,
        };
        let _ = writeln!(out, "### {heading}");
        let _ = writeln!(out, "**{}** `{}` ({})", n.code.unwrap_or("-"), n.subject, n.engine);
        out.push('\n');
        let _ = writeln!(out, "> {}", n.message);
        if let Some(detail) = &n.detail {
            let _ = writeln!(out, ">\n> {detail}");
        }
        out.push('\n');
    }

    if summary.notable_total > summary.notable.len() {
        let _ = writeln!(out, "_+{} more in the full artifact._", summary.notable_total - summary.notable.len());
    }

    out
}

fn run_audit_diff(
    base_root: &Path,
    head_root: &Path,
    targets_path: &Path,
    format: AuditFormat,
    summary_path: Option<&Path>,
) -> anyhow::Result<i32> {
    let manifest_src = fs::read_to_string(targets_path)
        .map_err(|e| anyhow::anyhow!("reading targets manifest {}: {e}", targets_path.display()))?;
    let manifest: TargetFile = toml::from_str(&manifest_src)
        .map_err(|e| anyhow::anyhow!("parsing targets manifest {}: {e}", targets_path.display()))?;
    validate_manifest(&manifest).map_err(|e| anyhow::anyhow!("invalid targets manifest: {e}"))?;

    // OBA half: reuse `analyze()`/`compare()` completely unchanged for
    // every target whose module+test genuinely exist on BOTH sides --
    // this is the EXACT machinery `oba diff` already ships, tested, and
    // relies on. S1-F1 (a real nixpkgs PR shadow audit): a target whose
    // module is absent on exactly ONE side is real PR-diff algebra (a
    // brand-new or since-deleted module -- S1 found this hard-failing
    // as a real TOOL_ERROR on 2/30 real PRs, all literally titled "init
    // module"), so it's partitioned out BEFORE calling `analyze()` on
    // the whole manifest and handled as a real `Added`/`Removed`
    // (mirroring exactly how CDC's own half already treats a candidate
    // that's unanalyzable on only one side). A target absent on BOTH
    // sides is a genuine manifest/input error -- nothing in this
    // comparison could ever say anything about it.
    let inconclusive_side = |report: &AnalysisReport| {
        report
            .targets
            .iter()
            .any(|t| !t.parse_errors.is_empty() || t.verdicts.iter().any(|v| v.is_inconclusive()))
    };
    let (oba_entries, base_oba_inconclusive, head_oba_inconclusive) = if manifest.target.is_empty() {
        (Vec::new(), false, false)
    } else {
        let mut both_present = Vec::new();
        let mut only_on_head = Vec::new();
        let mut only_on_base = Vec::new();
        for t in &manifest.target {
            let base_module = resolve_within_root_if_exists(base_root, &t.module)?;
            let base_test = resolve_within_root_if_exists(base_root, &t.test)?;
            let head_module = resolve_within_root_if_exists(head_root, &t.module)?;
            let head_test = resolve_within_root_if_exists(head_root, &t.test)?;
            let on_base = base_module.is_some() && base_test.is_some();
            let on_head = head_module.is_some() && head_test.is_some();
            match (on_base, on_head) {
                (true, true) => both_present.push(t.clone()),
                (false, true) => only_on_head.push(t.clone()),
                (true, false) => only_on_base.push(t.clone()),
                (false, false) => anyhow::bail!(
                    "target {}: module/test present under NEITHER --base-root nor --head-root -- a real manifest/input error, not a PR-introduced module addition or removal",
                    t.name
                ),
            }
        }

        let mut entries = Vec::new();
        let mut base_inconclusive = false;
        let mut head_inconclusive = false;

        if !both_present.is_empty() {
            let sub = TargetFile { target: both_present, cdc_target: Vec::new() };
            let base_analysis = analyze(base_root, &sub)?;
            let head_analysis = analyze(head_root, &sub)?;
            let comparison =
                compare(&base_analysis, &head_analysis).map_err(|e| anyhow::anyhow!("{e}"))?;
            base_inconclusive |= inconclusive_side(&base_analysis);
            head_inconclusive |= inconclusive_side(&head_analysis);
            entries.extend(comparison.entries);
        }
        if !only_on_head.is_empty() {
            let sub = TargetFile { target: only_on_head, cdc_target: Vec::new() };
            let head_analysis = analyze(head_root, &sub)?;
            head_inconclusive |= inconclusive_side(&head_analysis);
            let head_index = index_by_identity(&head_analysis, CompareSide::Head)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            for (identity, verdict) in head_index {
                entries.push(ComparisonEntry {
                    identity,
                    diff: TargetDiff::Added { head: Box::new(TargetOutcome { verdict: verdict.clone() }) },
                });
            }
        }
        if !only_on_base.is_empty() {
            let sub = TargetFile { target: only_on_base, cdc_target: Vec::new() };
            let base_analysis = analyze(base_root, &sub)?;
            base_inconclusive |= inconclusive_side(&base_analysis);
            let base_index = index_by_identity(&base_analysis, CompareSide::Base)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            for (identity, verdict) in base_index {
                entries.push(ComparisonEntry {
                    identity,
                    diff: TargetDiff::Removed { base: Box::new(TargetOutcome { verdict: verdict.clone() }) },
                });
            }
        }
        // invariant 2 (order-independence): the three passes above are
        // each individually sorted by identity, but their concatenation
        // is not -- one real final sort restores it, regardless of the
        // manifest's own target order or which partition a target fell
        // into.
        entries.sort_by(|a, b| a.identity.cmp(&b.identity));

        (entries, base_inconclusive, head_inconclusive)
    };

    // CDC half: real, new for P3b -- run_cdc_candidate for every
    // declared candidate against BOTH roots (each root's own real
    // NixpkgsSource::LocalPath), then compare_cdc().
    let mut cdc_entries = Vec::new();
    let mut cdc_inconclusive_or_tool_error = false;
    if !manifest.cdc_target.is_empty() {
        let base_abs = base_root
            .canonicalize()
            .map_err(|e| anyhow::anyhow!("resolving --base-root {} for CDC: {e}", base_root.display()))?;
        let head_abs = head_root
            .canonicalize()
            .map_err(|e| anyhow::anyhow!("resolving --head-root {} for CDC: {e}", head_root.display()))?;
        let base_nixpkgs = cdc::NixpkgsSource::LocalPath(base_abs);
        let head_nixpkgs = cdc::NixpkgsSource::LocalPath(head_abs);
        let mut base_results = Vec::new();
        let mut head_results = Vec::new();
        for t in &manifest.cdc_target {
            base_results.push(match cdc::run_cdc_candidate(&t.name, &base_nixpkgs) {
                Ok(outcome) => cdc_outcome_to_result(&t.name, outcome),
                Err(err) => cdc_error_to_result(&t.name, &err),
            });
            head_results.push(match cdc::run_cdc_candidate(&t.name, &head_nixpkgs) {
                Ok(outcome) => cdc_outcome_to_result(&t.name, outcome),
                Err(err) => cdc_error_to_result(&t.name, &err),
            });
        }
        cdc_inconclusive_or_tool_error = base_results
            .iter()
            .chain(head_results.iter())
            .any(|r| matches!(r.verdict, ResultVerdict::Inconclusive | ResultVerdict::ToolError));
        cdc_entries = compare_cdc(&base_results, &head_results)?;
    }

    let mut summary = AuditDiffSummary::default();
    let mut notable_total = 0usize;
    for entry in &oba_entries {
        match &entry.diff {
            TargetDiff::Unchanged => summary.unchanged += 1,
            TargetDiff::Added { head } => {
                summary.added += 1;
                if let Some(bucket) = added_bucket(oba_kind_class(head.verdict.kind())) {
                    let result = oba_verdict_to_result(&entry.identity.watched_path, &head.verdict);
                    record_bucket(
                        &mut summary,
                        &mut notable_total,
                        bucket,
                        "oba",
                        entry.identity.watched_path.clone(),
                        &result,
                    );
                }
            }
            TargetDiff::Removed { base } => {
                summary.removed += 1;
                if let Some(bucket) = removed_with_finding_bucket(oba_kind_class(base.verdict.kind())) {
                    let result = oba_verdict_to_result(&entry.identity.watched_path, &base.verdict);
                    record_bucket(
                        &mut summary,
                        &mut notable_total,
                        bucket,
                        "oba",
                        entry.identity.watched_path.clone(),
                        &result,
                    );
                }
            }
            TargetDiff::Changed { .. } => summary.changed += 1,
        }
        if let Some(t) = entry.diff.verdict_transition() {
            *summary
                .oba_verdict_transitions
                .entry(format!("{}->{}", t.from.as_str(), t.to.as_str()))
                .or_insert(0) += 1;

            let (from_class, to_class) = (oba_kind_class(t.from), oba_kind_class(t.to));
            let TargetDiff::Changed { head, .. } = &entry.diff else {
                unreachable!("verdict_transition() is only Some for TargetDiff::Changed")
            };
            if let Some(bucket) = classify_transition_bucket(from_class, to_class) {
                let result = oba_verdict_to_result(&entry.identity.watched_path, &head.verdict);
                record_bucket(
                    &mut summary,
                    &mut notable_total,
                    bucket,
                    "oba",
                    entry.identity.watched_path.clone(),
                    &result,
                );
            }
        }
    }
    for entry in &cdc_entries {
        match &entry.diff {
            CdcDiff::Unchanged => summary.unchanged += 1,
            CdcDiff::AddedSubject { head } => {
                summary.cdc_added_subject += 1;
                if let Some(bucket) = added_bucket(head.verdict) {
                    record_bucket(&mut summary, &mut notable_total, bucket, "cdc", entry.candidate.clone(), head);
                }
            }
            CdcDiff::RemovedSubject { base } => {
                summary.cdc_removed_subject += 1;
                if let Some(bucket) = removed_with_finding_bucket(base.verdict) {
                    record_bucket(&mut summary, &mut notable_total, bucket, "cdc", entry.candidate.clone(), base);
                }
            }
            CdcDiff::Changed { .. } => summary.changed += 1,
        }
        if let Some((from, to)) = entry.diff.verdict_transition() {
            *summary
                .cdc_verdict_transitions
                .entry(format!("{}->{}", cdc_verdict_str(from), cdc_verdict_str(to)))
                .or_insert(0) += 1;

            let CdcDiff::Changed { head, .. } = &entry.diff else {
                unreachable!("verdict_transition() is only Some for CdcDiff::Changed")
            };
            if let Some(bucket) = classify_transition_bucket(from, to) {
                record_bucket(&mut summary, &mut notable_total, bucket, "cdc", entry.candidate.clone(), head);
            }
        } else if matches!(entry.diff, CdcDiff::Changed { .. }) {
            summary.evidence_changed += 1;
        }
    }
    summary.notable_total = notable_total;

    let exit_code =
        if base_oba_inconclusive || head_oba_inconclusive || cdc_inconclusive_or_tool_error {
            2
        } else {
            0
        };

    if let Some(path) = summary_path {
        let markdown = render_github_summary(&summary, exit_code);
        fs::write(path, markdown)
            .map_err(|e| anyhow::anyhow!("writing --summary-path {}: {e}", path.display()))?;
    }

    match format {
        AuditFormat::Json => {
            let envelope = AuditDiffEnvelope {
                schema_version: 1,
                tool: ToolInfo { name: "oba", version: env!("CARGO_PKG_VERSION") },
                mode: "audit-diff",
                summary,
                oba: oba_entries,
                cdc: cdc_entries,
            };
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        AuditFormat::Text => {
            println!(
                "=== audit-diff summary: unchanged={} added={} removed={} changed={} cdc_added_subject={} cdc_removed_subject={} new_findings={} resolved_findings={} finding_became_inconclusive={} new_inconclusives={} resolved_inconclusives={} removed_subjects_with_finding={} evidence_changed={} ===",
                summary.unchanged,
                summary.added,
                summary.removed,
                summary.changed,
                summary.cdc_added_subject,
                summary.cdc_removed_subject,
                summary.new_findings,
                summary.resolved_findings,
                summary.finding_became_inconclusive,
                summary.new_inconclusives,
                summary.resolved_inconclusives,
                summary.removed_subjects_with_finding,
                summary.evidence_changed,
            );
            for entry in &oba_entries {
                print_oba_diff_entry_human(entry);
            }
            for entry in &cdc_entries {
                print_cdc_diff_entry_human(entry);
            }
        }
    }

    Ok(exit_code)
}

fn print_oba_diff_entry_human(entry: &ComparisonEntry) {
    let path = &entry.identity.watched_path;
    match &entry.diff {
        TargetDiff::Unchanged => {}
        TargetDiff::Added { head } => println!("[oba] ADDED    {path} -> {:?}", head.verdict.kind()),
        TargetDiff::Removed { base } => println!("[oba] REMOVED  {path} <- {:?}", base.verdict.kind()),
        TargetDiff::Changed { base, head, changes } => {
            println!(
                "[oba] CHANGED  {path}  {:?} -> {:?}  ({changes:?})",
                base.verdict.kind(),
                head.verdict.kind()
            );
        }
    }
}

fn print_cdc_diff_entry_human(entry: &CdcComparisonEntry) {
    let name = &entry.candidate;
    match &entry.diff {
        CdcDiff::Unchanged => {}
        CdcDiff::AddedSubject { head } => {
            println!("[cdc] ADDED_SUBJECT    {name} -> {:?}", head.verdict)
        }
        CdcDiff::RemovedSubject { base } => {
            println!("[cdc] REMOVED_SUBJECT  {name} <- {:?}", base.verdict)
        }
        CdcDiff::Changed { base, head, changes } => {
            println!("[cdc] CHANGED  {name}  {:?} -> {:?}  ({changes:?})", base.verdict, head.verdict);
        }
    }
}

fn print_diff_human(comparison: &ComparisonReport) {
    for entry in &comparison.entries {
        let path = &entry.identity.watched_path;
        match &entry.diff {
            TargetDiff::Unchanged => {}
            TargetDiff::Added { head } => {
                println!("  ADDED    {path}  ({})", head.verdict.kind().as_str())
            }
            TargetDiff::Removed { base } => {
                println!("  REMOVED  {path}  ({})", base.verdict.kind().as_str())
            }
            TargetDiff::Changed { base, head, .. } => println!(
                "  CHANGED  {path}  {} -> {}",
                base.verdict.kind().as_str(),
                head.verdict.kind().as_str()
            ),
        }
    }
}

fn print_human(r: &TargetReport) {
    println!("=== target: {} ===", r.name);
    if !r.parse_errors.is_empty() {
        for e in &r.parse_errors {
            println!("  PARSE_ERROR  {e}");
        }
        return;
    }
    println!(
        "discovered_options: {}",
        r.discovered_options
            .iter()
            .map(|o| o.path.join("."))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "discovered_predicates: {}",
        r.discovered_predicates
            .iter()
            .map(|p| format!(
                "{}({:?})@{}:{}",
                p.path.join("."),
                p.kind,
                p.span.line,
                p.span.col
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "matched_test_assignments: {}",
        r.matched_test_assignments
            .iter()
            .map(|a| format!(
                "{}={}[{}]@{}:{}",
                a.path.join("."),
                a.value_source.trim(),
                a.instance.as_deref().unwrap_or("?"),
                a.span.line,
                a.span.col
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "test_config_opacity: {}",
        r.test_config_opacity
            .iter()
            .map(|o| format!(
                "{}[{}]@{}:{} ({})",
                if o.path.is_empty() {
                    "<whole instance>".to_string()
                } else {
                    o.path.join(".")
                },
                o.instance.as_deref().unwrap_or("?"),
                o.span.line,
                o.span.col,
                o.reason
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    for v in &r.verdicts {
        match v {
            Verdict::OptionNotFound { option } => {
                println!("  OPTION_NOT_FOUND  {option}  (declaration scanner found no mkOption for this path)")
            }
            Verdict::PredicateNotFound { option } => println!(
                "  PREDICATE_NOT_FOUND  {option}  (no direct cfg.<path> branch found -- likely aliased, out of MVP scope)"
            ),
            Verdict::DefaultUnresolved { option, .. } => println!(
                "  DEFAULT_UNRESOLVED  {option}  (declared default's outcome under this predicate isn't a literal null/true/false)"
            ),
            Verdict::TestConfigUnresolved { option, .. } => println!(
                "  TEST_CONFIG_UNRESOLVED  {option}  (part of the test config that could contain this option wasn't visible to the walker -- imports/alias/function call)"
            ),
            Verdict::TestValueUnresolved {
                option,
                default_outcome,
                ..
            } => println!(
                "  TEST_VALUE_UNRESOLVED  {option}  default_outcome={default_outcome:?}  (a matching test assignment's value isn't statically classifiable, and no other match is a known opposite outcome)"
            ),
            Verdict::Oba001 {
                option,
                predicate,
                default_outcome,
                ..
            } => println!(
                "  OBA001  {option}  default_outcome={default_outcome:?}  [{}:{}]  `{}`",
                predicate.span().line,
                predicate.span().col,
                predicate.source()
            ),
            Verdict::Pass {
                option,
                default_outcome,
                evidence,
                ..
            } => println!(
                "  PASS    {option}  default_outcome={default_outcome}  evidence: {}",
                evidence
                    .iter()
                    .map(|e| format!(
                        "{}={} (instance {})",
                        e.path.join("."),
                        e.value_source.trim(),
                        e.instance.as_deref().unwrap_or("?")
                    ))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        }
    }
}

// ---------------------------------------------------------------------
// KANI-0: bounded formal proofs of semantic-core soundness.
//
// Explicit scope, agreed after the mutation-testing pass (`0afe187`)
// left a small, clean semantic core -- NOT an attempt to formally verify
// the analyzer as a whole. Two layers:
//
// K0: `eval_known_eq` never fabricates knowledge -- whenever it returns
// `Some(b)`, `b` is correct for EVERY concrete valuation consistent with
// its abstract input, not just plausible. This is the theorem the
// entire `PASS`/`OBA001` distinction rests on: a `Some` the evaluator
// didn't actually earn would silently corrupt every verdict built on
// top of it.
//
// K1: `aggregate`'s priority ordering is exhaustively correct -- already
// pinned by `aggregate_priority_is_exhaustively_correct_over_all_32_cases`
// via brute-force enumeration (cheap enough not to need Kani at all,
// 32 cases), restated here as `#[kani::proof]` harnesses per the
// explicit ask: two real regressions (H2.2 Finding 2, and the
// post-`cef12d7` hostile-review fixup) both lived in exactly this
// priority ordering, so it earns a proof artifact of its own, not just
// an enumeration.
//
// Deliberately NOT attempted here: `rnix`/`rowan`, `resolve_ident_binding`,
// `lower_pred_chained`, the test-file walker, or `run_target` as a
// whole -- a small self-contained function with an existing test base is
// exactly what Kani's own guidance recommends starting from; a deep call
// graph over untyped syntax trees is exactly what it warns against. That
// surface already has goldens, adversarial fixtures, proptest, mutation
// testing, and hostile review; formalizing the syntax layer now would be
// negative ROI, not a next step.
//
// **`eval_pred` soundness/monotonicity (originally planned K0.2/K0.3,
// plus a K0.4 sanity check) was ATTEMPTED and DROPPED, not merely
// deferred.** The property itself was well-formed and the harnesses
// compiled and were logically sound (recorded here for anyone tempted
// to retry with more resources) -- what killed it was CBMC's cost for
// symbolically modeling Rust `HashMap`'s SipHash hasher, present in any
// use of the real `HashMap<Vec<String>, KnownValue>` environment type
// eval_pred actually takes, essentially independent of harness size.
// Five escalating scope reductions on a local 1-core/1.9GB dev VPS (free
// recursive `Pred` generator over 3 paths/depth 2, then 2/1, then 1/1,
// then one FIXED tree shape per `Pred` constructor with a single `Ref`
// lookup) each still pushed the machine to the edge of OOM before
// finishing or finished only after several minutes. Moved to GitHub
// Actions on the theory that more RAM/CPU would fix it -- it didn't: on
// a real CI run, the single cheapest harness (`k0_4_double_not_is_identity`,
// same fixed-shape design) still hadn't finished after the remaining
// ~29 minutes of a 30-minute job timeout, and had to be force-killed as
// an orphan `cbmc` process. This confirmed the cost is fundamental to
// the (real HashMap, real String keys) approach, not a resource ceiling
// any one machine happened to hit -- exactly the risk the original K0
// design discussion flagged as the reason to consider a small
// finite-domain MIRROR type instead of the real environment type, which
// was offered again after the CI failure and explicitly declined in
// favor of stopping at K0.1 (`eval_known_eq`) + K1 (`aggregate`) as the
// final KANI-0 scope. `eval_pred`'s own Kleene-logic soundness stays
// covered by the existing proptest properties
// (`eval_pred_and_short_circuits_on_a_known_false_operand_either_side`
// and siblings) and mutation testing, not by a bounded proof.
//
// Verification-only code, invisible to every normal build (`cfg(kani)`
// gates it out of `cargo build`/`cargo test`/`cargo clippy` entirely --
// `cargo kani` is the only thing that ever compiles this module).
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    // -------------------------------------------------------------
    // Shared generators: a small, closed universe -- real `KnownValue`/
    // `Scalar` types (proving the actual production `eval_known_eq`, not
    // a mirror), but only ever holding values drawn from a tiny
    // alphabet: Null, Bool(false), Bool(true), and two DISTINCT non-null
    // strings "s0"/"s1" -- enough to tell "the same non-null scalar"
    // apart from "some other non-null scalar", the one distinction
    // `DefinitelyNonNull`'s own soundness actually needs.
    // -------------------------------------------------------------

    fn kani_any_scalar5() -> Scalar {
        let tag: u8 = kani::any();
        kani::assume(tag < 5);
        match tag {
            0 => Scalar::Null,
            1 => Scalar::Bool(false),
            2 => Scalar::Bool(true),
            3 => Scalar::Str("s0".to_string()),
            _ => Scalar::Str("s1".to_string()),
        }
    }

    /// γ (verification-only, never used by production code): is
    /// `concrete` one of the real values `abstract_kv` could denote?
    /// `Exact(s)` denotes exactly `s`; `DefinitelyNonNull` denotes any
    /// non-null scalar -- the same semantics `eval_known_eq`'s own doc
    /// comment describes.
    fn kani_concretizes(abstract_kv: &KnownValue, concrete: &Scalar) -> bool {
        match abstract_kv {
            KnownValue::Exact(s) => s == concrete,
            KnownValue::DefinitelyNonNull => !matches!(concrete, Scalar::Null),
        }
    }

    // -------------------------------------------------------------
    // K0.1 -- abstract equality never fabricates knowledge.
    // -------------------------------------------------------------

    #[kani::proof]
    #[kani::unwind(4)]
    fn k0_1_eval_known_eq_sound() {
        let a = if kani::any() {
            KnownValue::DefinitelyNonNull
        } else {
            KnownValue::Exact(kani_any_scalar5())
        };
        let b = if kani::any() {
            KnownValue::DefinitelyNonNull
        } else {
            KnownValue::Exact(kani_any_scalar5())
        };
        let ca = kani_any_scalar5();
        let cb = kani_any_scalar5();
        kani::assume(kani_concretizes(&a, &ca));
        kani::assume(kani_concretizes(&b, &cb));

        if let Some(x) = eval_known_eq(&a, &b) {
            assert_eq!(ca == cb, x, "eval_known_eq({a:?}, {b:?}) = Some({x}) but concretizations {ca:?}/{cb:?} disagree");
        }
    }

    // -------------------------------------------------------------
    // K1 -- `aggregate`'s priority ordering, exhaustively correct.
    // Already pinned by brute-force enumeration in the plain test
    // suite (32 cases is cheap); restated as Kani proofs because this
    // exact ordering has broken twice for real (H2.2 Finding 2, and the
    // post-`cef12d7` hostile-review fixup), so it earns its own formal
    // artifact.
    // -------------------------------------------------------------

    fn kani_any_facts() -> AggregateFacts {
        AggregateFacts {
            has_witness: kani::any(),
            h1_default_unresolved: kani::any(),
            target_path_opaque: kani::any(),
            has_unresolved_attempt: kani::any(),
            relevant_unresolved_predicate: kani::any(),
        }
    }

    #[kani::proof]
    fn k1_1_witness_always_wins() {
        let mut f = kani_any_facts();
        f.has_witness = true;
        assert_eq!(aggregate(f), AggregateVerdict::Pass);
    }

    #[kani::proof]
    fn k1_2_default_unresolved_priority() {
        let mut f = kani_any_facts();
        f.has_witness = false;
        f.h1_default_unresolved = true;
        assert_eq!(aggregate(f), AggregateVerdict::DefaultUnresolved);
    }

    #[kani::proof]
    fn k1_3_test_config_unresolved_priority() {
        let mut f = kani_any_facts();
        f.has_witness = false;
        f.h1_default_unresolved = false;
        f.target_path_opaque = true;
        assert_eq!(aggregate(f), AggregateVerdict::TestConfigUnresolved);
    }

    #[kani::proof]
    fn k1_4_test_value_unresolved_priority() {
        let mut f = kani_any_facts();
        f.has_witness = false;
        f.h1_default_unresolved = false;
        f.target_path_opaque = false;
        kani::assume(f.has_unresolved_attempt || f.relevant_unresolved_predicate);
        assert_eq!(aggregate(f), AggregateVerdict::TestValueUnresolved);
    }

    /// K1.5 -- the fail-closed guarantee stated as a postcondition on
    /// the verdict, not merely an absence of special-casing in the
    /// code: if `aggregate` ever returns `Oba001`, NONE of the other
    /// four uncertainty facts held. A system that prints `OBA001` never
    /// did so while ignoring a source of doubt it already knew about.
    #[kani::proof]
    fn k1_5_oba001_requires_complete_knowledge() {
        let f = kani_any_facts();
        if aggregate(f) == AggregateVerdict::Oba001 {
            assert!(!f.has_witness);
            assert!(!f.h1_default_unresolved);
            assert!(!f.target_path_opaque);
            assert!(!f.has_unresolved_attempt);
            assert!(!f.relevant_unresolved_predicate);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_eq(a: &[String], b: &[&str]) -> bool {
        a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x == y)
    }

    /// Exhaustive check over `aggregate`'s entire 32-case input space
    /// (5 independent booleans) -- cheap enough to just brute-force at
    /// the unit-test level, ahead of/independent from the Kani harnesses
    /// (KANI-0 K1) that will prove the same properties formally rather
    /// than by enumeration. Pins exactly the priority-ordering
    /// properties named in the K1 design, each stated as an implication
    /// so it stays true regardless of which OTHER facts happen to hold:
    ///
    /// - K1.1: `has_witness` alone implies `Pass`, unconditionally.
    /// - K1.2: no witness + `h1_default_unresolved` implies
    ///   `DefaultUnresolved`, regardless of the other three facts.
    /// - K1.3: no witness, no `h1_default_unresolved`, `target_path_opaque`
    ///   implies `TestConfigUnresolved`.
    /// - K1.4: none of the above three, but `has_unresolved_attempt` or
    ///   `relevant_unresolved_predicate`, implies `TestValueUnresolved`.
    /// - K1.5: `Oba001` implies NONE of the other four facts hold -- the
    ///   fail-closed guarantee stated as a postcondition on the verdict
    ///   itself, not just as an absence of special-casing in the code.
    #[test]
    fn aggregate_priority_is_exhaustively_correct_over_all_32_cases() {
        for has_witness in [false, true] {
            for h1_default_unresolved in [false, true] {
                for target_path_opaque in [false, true] {
                    for has_unresolved_attempt in [false, true] {
                        for relevant_unresolved_predicate in [false, true] {
                            let f = AggregateFacts {
                                has_witness,
                                h1_default_unresolved,
                                target_path_opaque,
                                has_unresolved_attempt,
                                relevant_unresolved_predicate,
                            };
                            let v = aggregate(f);

                            if has_witness {
                                assert_eq!(v, AggregateVerdict::Pass, "K1.1 violated by {f:?}");
                                continue;
                            }
                            if h1_default_unresolved {
                                assert_eq!(
                                    v,
                                    AggregateVerdict::DefaultUnresolved,
                                    "K1.2 violated by {f:?}"
                                );
                                continue;
                            }
                            if target_path_opaque {
                                assert_eq!(
                                    v,
                                    AggregateVerdict::TestConfigUnresolved,
                                    "K1.3 violated by {f:?}"
                                );
                                continue;
                            }
                            if has_unresolved_attempt || relevant_unresolved_predicate {
                                assert_eq!(
                                    v,
                                    AggregateVerdict::TestValueUnresolved,
                                    "K1.4 violated by {f:?}"
                                );
                                continue;
                            }
                            assert_eq!(v, AggregateVerdict::Oba001, "K1.5 violated by {f:?}");
                        }
                    }
                }
            }
        }
    }

    /// Scanner-level golden against real, unmodified nixpkgs source (not
    /// a synthetic fixture): PhysShell/nixpkgs@5530e24f2:nixos/tests/
    /// ifm.nix, vendored at fixtures/real/ifm-test.nix (locked in
    /// fixtures/integrity-lock.toml). Proves the walker actually handles
    /// mainstream nixosTest structure -- the nested `nodes = { server =
    /// ...; };` form, and a dynamic `${config.services.ifm.dataDir}.d`
    /// attrpath nested three levels deep -- on a file this project didn't
    /// write and didn't shape to be convenient. After 559 files under
    /// nixos/tests using `nodes = {` (H1.3 review), a synthetic-only
    /// corpus stopped being enough evidence on its own.
    #[test]
    fn scanner_reads_real_ifm_test_correctly() {
        let src = include_str!("../fixtures/real/ifm-test.nix");
        let root = rnix::Root::parse(src);
        assert!(root.errors().is_empty(), "ifm-test.nix must parse cleanly");
        let (assignments, opacity) =
            scan_test_assignments("ifm-test.nix", src, root.tree().syntax());

        let find = |path: &[&str]| assignments.iter().find(|a| path_eq(&a.path, path));

        let enable =
            find(&["services", "ifm", "enable"]).expect("services.ifm.enable must be found");
        assert_eq!(enable.instance.as_deref(), Some("server"));
        assert_eq!(enable.value_class, ValueClass::Bool(true));

        let port = find(&["services", "ifm", "port"]).expect("services.ifm.port must be found");
        assert_eq!(port.value_source.trim(), "9001");

        let data_dir =
            find(&["services", "ifm", "dataDir"]).expect("services.ifm.dataDir must be found");
        assert_eq!(data_dir.value_source.trim(), "\"/data\"");

        // The dynamic ${config.services.ifm.dataDir}.d key inside
        // systemd.tmpfiles.settings.ifm-data-dir must produce an explicit
        // Opacity, not silent absence -- exactly the construct c24
        // targets in isolation, confirmed here against the real file that
        // motivated it.
        assert!(
            opacity
                .iter()
                .any(|o| path_eq(&o.path, &["systemd", "tmpfiles", "settings", "ifm-data-dir"])),
            "the dynamic ${{config...}} key must be recorded as opacity, not silently skipped; got {opacity:?}"
        );
    }

    // --- H2 commit 1: pure Predicate IR (Pred/ValueExpr/Scalar) ---------
    //
    // Not yet wired into run_target/scan_predicates -- these are unit
    // tests of the standalone lowering functions and evaluator, proving
    // the pure core is correct in isolation before the next commit wires
    // alias resolution and the counterfactual gate-4 rewrite on top of it.

    fn parse_expr(src: &str) -> SyntaxNode {
        let root = rnix::Root::parse(src);
        assert!(root.errors().is_empty(), "test fixture must parse: {src}");
        root.tree()
            .syntax()
            .children()
            .next()
            .expect("root must have exactly one expression child")
    }

    #[test]
    fn lower_pred_handles_the_direct_h1_forms() {
        assert_eq!(
            lower_pred(&parse_expr("cfg.foo != null"), "cfg"),
            Ok(Pred::Not(Box::new(Pred::Eq(
                ValueExpr::Ref(vec!["foo".into()]),
                ValueExpr::Literal(Scalar::Null),
            ))))
        );
        assert_eq!(
            lower_pred(&parse_expr("cfg.foo == null"), "cfg"),
            Ok(Pred::Eq(
                ValueExpr::Ref(vec!["foo".into()]),
                ValueExpr::Literal(Scalar::Null),
            ))
        );
        // A bare cfg-rooted reference used directly as a condition (`if
        // cfg.foo then ...`) lowers to `cfg.foo == true`, matching H1's
        // `Truthy` semantics.
        assert_eq!(
            lower_pred(&parse_expr("cfg.foo"), "cfg"),
            Ok(Pred::Eq(
                ValueExpr::Ref(vec!["foo".into()]),
                ValueExpr::Literal(Scalar::Bool(true)),
            ))
        );
        assert_eq!(
            lower_pred(&parse_expr("!cfg.foo"), "cfg"),
            Ok(Pred::Not(Box::new(Pred::Eq(
                ValueExpr::Ref(vec!["foo".into()]),
                ValueExpr::Literal(Scalar::Bool(true)),
            ))))
        );
    }

    #[test]
    fn lower_pred_handles_compound_forms_new_in_h2() {
        // Compound forms with `db` already replaced by `cfg` directly,
        // isolating the `&&`/string-literal-equality lowering itself from
        // alias resolution (covered separately below).
        assert_eq!(
            lower_pred(
                &parse_expr(r#"cfg.createLocally && cfg.driver == "mysql""#),
                "cfg"
            ),
            Ok(Pred::And(
                Box::new(Pred::Eq(
                    ValueExpr::Ref(vec!["createLocally".into()]),
                    ValueExpr::Literal(Scalar::Bool(true)),
                )),
                Box::new(Pred::Eq(
                    ValueExpr::Ref(vec!["driver".into()]),
                    ValueExpr::Literal(Scalar::Str("mysql".into())),
                )),
            ))
        );
        assert_eq!(
            lower_pred(&parse_expr("cfg.a || cfg.b"), "cfg"),
            Ok(Pred::Or(
                Box::new(Pred::Eq(
                    ValueExpr::Ref(vec!["a".into()]),
                    ValueExpr::Literal(Scalar::Bool(true)),
                )),
                Box::new(Pred::Eq(
                    ValueExpr::Ref(vec!["b".into()]),
                    ValueExpr::Literal(Scalar::Bool(true)),
                )),
            ))
        );
    }

    #[test]
    fn lower_pred_is_unresolved_for_unsupported_shapes_not_silently_something_else() {
        // A helper call: not a shape this IR represents.
        assert_eq!(
            lower_pred(&parse_expr(r#"builtins.elem "x" [ "x" "y" ]"#), "cfg"),
            Err(ResolveFailure::UnsupportedExpression)
        );
        // A bare identifier with no enclosing binding at all (this test
        // parses a bare top-level expression, no `let`/lambda around it)
        // -- genuinely unbound, never silently treated as `false`/absent.
        assert_eq!(
            lower_pred(&parse_expr("mysqlLocal"), "cfg"),
            Err(ResolveFailure::Unbound("mysqlLocal".into()))
        );
        // An operator this IR doesn't model.
        assert_eq!(
            lower_pred(&parse_expr("cfg.a < cfg.b"), "cfg"),
            Err(ResolveFailure::UnsupportedExpression)
        );
    }

    #[test]
    fn lower_pred_reports_a_bare_true_or_false_condition_as_trivial_constant() {
        // Mutation-testing survivor: `if name == "true" || name ==
        // "false"` (the guard gating `TrivialConstant`) had its `||`
        // flipped to `&&`, which makes the guard permanently false (a
        // string can never equal both literals at once) and survived
        // against the whole golden suite -- because every existing
        // fixture's `mkIf true {...}` wrapper only affects
        // `unresolved_predicate_sites`' CONTENTS (an `UnsupportedExpression`/
        // `Unbound("true")` entry with empty `refs`, since a bare `true`
        // ident still lowers cleanly as a `ValueExpr::Literal` and
        // contributes no reachable ref), never any watched option's
        // actual verdict. Pinned directly at the unit level instead of
        // trying to observe it through a verdict.
        assert_eq!(
            lower_pred(&parse_expr("true"), "cfg"),
            Err(ResolveFailure::TrivialConstant)
        );
        assert_eq!(
            lower_pred(&parse_expr("false"), "cfg"),
            Err(ResolveFailure::TrivialConstant)
        );
    }

    // --- Alias resolution: adversarial cases, not just the one happy
    // Davis path. Per review: a resolver only proven against one real
    // module is a resolver that's only proven against one real module.

    /// Parses `src` as `let ...; in <body>` and returns `<body>`'s syntax
    /// node -- still positioned inside the `let`'s scope (its ancestors
    /// include the `NODE_LET_IN`), exactly like a real predicate
    /// expression `scan_predicates` would encounter nested inside a
    /// module's own `let cfg = ...; in { ... };`.
    fn let_body(src: &str) -> SyntaxNode {
        let expr = parse_expr(src);
        assert_eq!(
            expr.kind(),
            NODE_LET_IN,
            "expected a let..in expression: {src}"
        );
        expr.children().last().expect("let..in must have a body")
    }

    #[test]
    fn multi_hop_select_alias_resolves() {
        // a = cfg.database; b = a; -- resolving `b.driver` must chase
        // through BOTH hops (b -> a -> cfg.database) to land on
        // `Ref(["database","driver"])`, not stop at the first hop.
        assert_eq!(
            lower_pred(
                &let_body(r#"let a = cfg.database; b = a; in b.driver == "mysql""#),
                "cfg"
            ),
            Ok(Pred::Eq(
                ValueExpr::Ref(vec!["database".into(), "driver".into()]),
                ValueExpr::Literal(Scalar::Str("mysql".into())),
            ))
        );
    }

    #[test]
    fn cyclic_alias_is_unresolved_not_a_stack_overflow() {
        // a = b; b = a; -- must fail as Cycle, not recurse until the
        // stack dies. The exact chain contents matter less than the
        // *shape* of the failure (Cycle, not a panic or a wrong answer).
        let result = lower_pred(&let_body("let a = b; b = a; in a"), "cfg");
        match result {
            Err(ResolveFailure::Cycle(chain)) => {
                assert!(
                    chain.len() >= 2,
                    "cycle chain should record at least the two names involved; got {chain:?}"
                );
            }
            other => panic!("expected Cycle, got {other:?}"),
        }
    }

    #[test]
    fn nested_let_shadowing_nearest_binding_wins() {
        // let x = cfg.a; in let x = cfg.b; in x -- the inner `x` shadows
        // the outer one; resolving the body's bare `x` must land on
        // cfg.b, never cfg.a.
        let outer = parse_expr("let x = cfg.a; in let x = cfg.b; in x");
        assert_eq!(outer.kind(), NODE_LET_IN);
        let inner = outer.children().last().unwrap();
        assert_eq!(inner.kind(), NODE_LET_IN);
        let innermost_body = inner.children().last().unwrap();

        assert_eq!(
            lower_pred(&innermost_body, "cfg"),
            Ok(Pred::Eq(
                ValueExpr::Ref(vec!["b".into()]),
                ValueExpr::Literal(Scalar::Bool(true)),
            )),
            "nearest binding must win -- resolving to cfg.a here would be wrong shadowing"
        );
    }

    #[test]
    fn lambda_parameter_shadows_outer_alias_and_is_unsupported() {
        // let x = cfg.a; in x: x != null -- inside the lambda body, `x`
        // is the lambda's OWN parameter (a runtime argument, not a
        // lexical alias to any expression), shadowing the outer
        // `x = cfg.a`. Must be UnsupportedScope, never silently resolved
        // to the outer alias.
        let outer = parse_expr("let x = cfg.a; in x: x != null");
        assert_eq!(outer.kind(), NODE_LET_IN);
        let lambda = outer.children().last().unwrap();
        assert_eq!(lambda.kind(), NODE_LAMBDA);
        let lambda_body = lambda.children().last().unwrap();

        assert_eq!(
            lower_pred(&lambda_body, "cfg"),
            Err(ResolveFailure::UnsupportedScope("function parameter"))
        );
    }

    #[test]
    fn pattern_bind_at_name_also_shadows() {
        // let args = cfg.a; in ({ x, ... }@args: args) -- the `@args`
        // binding (NODE_PAT_BIND) is just as much a real lambda
        // parameter as a plain `NODE_IDENT_PARAM`, binding `args` to the
        // whole passed-in attrset. Caught by the same AST-shape probe
        // that found the NODE_IDENT_PARAM bug above: NODE_PAT_BIND was
        // initially missing from the PATTERN-branch shadow check
        // alongside it.
        let outer = parse_expr("let args = cfg.a; in ({ x, ... }@args: args)");
        assert_eq!(outer.kind(), NODE_LET_IN);
        let lambda_or_paren = outer.children().last().unwrap();
        let lambda = if lambda_or_paren.kind() == NODE_PAREN {
            lambda_or_paren.children().next().unwrap()
        } else {
            lambda_or_paren
        };
        assert_eq!(lambda.kind(), NODE_LAMBDA);
        let lambda_body = lambda.children().last().unwrap();

        assert_eq!(
            lower_pred(&lambda_body, "cfg"),
            Err(ResolveFailure::UnsupportedScope("function parameter"))
        );
    }

    #[test]
    fn function_produced_alias_is_unresolved_not_assumed_to_be_its_argument() {
        // db = someFunction cfg.database; -- `db` is NOT the same thing
        // as `cfg.database`; whatever someFunction does to it is opaque
        // to this resolver, so `db.driver` must stay unresolved rather
        // than being silently treated as `cfg.database.driver`.
        assert_eq!(
            lower_pred(
                &let_body(r#"let db = someFunction cfg.database; in db.driver == "mysql""#),
                "cfg"
            ),
            Err(ResolveFailure::UnsupportedExpression)
        );
    }

    #[test]
    fn scan_options_flat_root_requires_cfg_to_actually_match_option_prefix() {
        // Positive control: cfg really does bind to config.services.davis,
        // matching option_prefix -- the flat root must be walked.
        let good = r#"
            { config, lib, ... }:
            let
              cfg = config.services.davis;
            in
            {
              options.services.davis = {
                foo = lib.mkOption { default = null; };
              };
            }
        "#;
        let root = rnix::Root::parse(good);
        assert!(root.errors().is_empty());
        let prefix = vec!["services".to_string(), "davis".to_string()];
        let opts = scan_options("m.nix", good, root.tree().syntax(), "cfg", &prefix);
        assert!(
            opts.iter().any(|o| path_eq(&o.path, &["foo"])),
            "cfg matches option_prefix -- flat root must be discovered; got {opts:?}"
        );

        // The actual safety-gate case: manifest claims option_prefix =
        // ["services","davis"], but this module's own cfg binds to a
        // completely different scope. Correlating them anyway would be a
        // manifest-induced false correlation -- must NOT walk the flat
        // root just because option_prefix happens to line up textually
        // with the declaration path.
        let bad = r#"
            { config, lib, ... }:
            let
              cfg = config.services.wrongScope;
            in
            {
              options.services.davis = {
                foo = lib.mkOption { default = null; };
              };
            }
        "#;
        let root = rnix::Root::parse(bad);
        assert!(root.errors().is_empty());
        let opts = scan_options("m.nix", bad, root.tree().syntax(), "cfg", &prefix);
        assert!(
            opts.is_empty(),
            "cfg binds to a different scope than option_prefix claims -- must not correlate; \
             got {opts:?}"
        );
    }

    // --- S2-F1: a real nixpkgs PR shadow audit found a false
    // OptionNotFound on PR #558149's own real firewall.nix -- a nested
    // options block whose own VALUE is a `//`-merge
    // (`{ enable = lib.mkOption {...}; ... } // commonOptions;`), not a
    // literal attrset, which `walk_options_block`'s own attrset-literal
    // check silently never matched at all. Root-caused with a minimal
    // reproducer (case2/case3 under fixtures/s2-f1-firewall-merge/,
    // real vendored firewall.nix + a synthetic isolation) before fixing.

    #[test]
    fn walk_options_block_finds_a_declaration_on_the_literal_lhs_of_an_update_merge() {
        // The exact real shape from firewall.nix: a nested options block
        // whose value is `{ <declared options> } // <external attrset>;`.
        let src = r#"
            { config, lib, ... }:
            let
              cfg = config.networking.firewall;
              commonOptions = {
                allowedTCPPorts = lib.mkOption { default = [ ]; };
              };
            in
            {
              options = {
                networking.firewall = {
                  enable = lib.mkOption { default = true; };
                }
                // commonOptions;
              };
            }
        "#;
        let root = rnix::Root::parse(src);
        assert!(root.errors().is_empty());
        let prefix = vec!["networking".to_string(), "firewall".to_string()];
        let opts = scan_options("m.nix", src, root.tree().syntax(), "cfg", &prefix);
        assert!(
            opts.iter().any(|o| path_eq(&o.path, &["enable"])),
            "enable is declared directly on the LHS of a // merge, zero indirection -- \
             must be found; got {opts:?}"
        );
    }

    #[test]
    fn walk_options_block_finds_declarations_on_either_side_of_an_update_merge() {
        // Merge side must not matter when BOTH operands are literal
        // attrsets -- this is deliberately different from the
        // "stays opaque" test below: a `let`-bound IDENTIFIER reference
        // (even one that happens to resolve to an attrset) is NOT a
        // literal attrset at its own use site, and correctly stays
        // unresolved (see that test) -- this one instead tests two
        // attrset LITERALS directly, on each side.
        let src = r#"
            { config, lib, ... }:
            let
              cfg = config.services.x;
            in
            {
              options.services.x =
                {
                  fromLhs = lib.mkOption { default = null; };
                }
                // {
                  fromRhs = lib.mkOption { default = null; };
                };
            }
        "#;
        let root = rnix::Root::parse(src);
        assert!(root.errors().is_empty());
        let prefix = vec!["services".to_string(), "x".to_string()];
        let opts = scan_options("m.nix", src, root.tree().syntax(), "cfg", &prefix);
        assert!(
            opts.iter().any(|o| path_eq(&o.path, &["fromLhs"])),
            "the literal LHS of the merge must be found; got {opts:?}"
        );
        assert!(
            opts.iter().any(|o| path_eq(&o.path, &["fromRhs"])),
            "the literal RHS of the merge must be found; got {opts:?}"
        );
    }

    #[test]
    fn walk_options_block_stays_opaque_for_a_merge_with_a_non_literal_operand() {
        // Deliberately bounded, not a general evaluator: a merge operand
        // that's a bare identifier reference (e.g. exporters.nix's own
        // real `extraOpts`, a function parameter deferred to a separate
        // call site elsewhere in the file) is real, disclosed opacity
        // this fix does NOT resolve. The literal side must still be
        // found; nothing should panic or hallucinate a declaration for
        // the unresolvable side.
        let src = r#"
            { config, lib, extraOpts, ... }:
            let
              cfg = config.services.x;
            in
            {
              options.services.x = {
                known = lib.mkOption { default = null; };
              }
              // extraOpts;
            }
        "#;
        let root = rnix::Root::parse(src);
        assert!(root.errors().is_empty());
        let prefix = vec!["services".to_string(), "x".to_string()];
        let opts = scan_options("m.nix", src, root.tree().syntax(), "cfg", &prefix);
        assert!(
            opts.iter().any(|o| path_eq(&o.path, &["known"])),
            "the literal LHS must still be found even though the RHS is opaque; got {opts:?}"
        );
        assert_eq!(opts.len(), 1, "exactly one real declaration exists in this fixture; got {opts:?}");
    }

    // --- Scope-aware regression pair for the gate-1 safety gate, added
    // on review: the ORIGINAL safety-gate implementation did a flat,
    // scope-blind `root.descendants()` search for the first binding named
    // `cfg_ident` anywhere in the whole module -- correct for the simple
    // cases above, but wrong the moment an unrelated helper function has
    // its own, differently-scoped `cfg`. Both directions below.

    #[test]
    fn resolve_cfg_root_ignores_an_unrelated_earlier_shadow_in_document_order() {
        // helper's own `cfg` is a completely different, non-enclosing
        // scope relative to the real declaration -- and, critically,
        // appears EARLIER in the source than the real top-level `cfg`, so
        // a flat first-match search would find the wrong one first. A
        // scope-aware resolver, walking ancestors from the declaration's
        // own position, must never even visit helper's inner `cfg` at
        // all (it isn't an ancestor of the declaration).
        let src = r#"
            { config, lib, ... }:
            let
              helper = x:
                let
                  cfg = config.services.other;
                in
                cfg.foo;

              cfg = config.services.davis;
            in
            {
              options.services.davis = {
                foo = lib.mkOption { default = null; };
              };
            }
        "#;
        let root = rnix::Root::parse(src);
        assert!(root.errors().is_empty());
        let prefix = vec!["services".to_string(), "davis".to_string()];
        let opts = scan_options("m.nix", src, root.tree().syntax(), "cfg", &prefix);
        assert!(
            opts.iter().any(|o| path_eq(&o.path, &["foo"])),
            "the declaration's own scope must resolve cfg to config.services.davis, \
             ignoring an unrelated earlier helper's shadowed cfg; got {opts:?}"
        );
    }

    #[test]
    fn resolve_cfg_root_is_scope_aware_not_a_flat_grep() {
        // Direct proof the resolver itself (not just scan_options's
        // end-to-end behavior) distinguishes scopes: querying from the
        // real declaration's position resolves to config.services.davis;
        // querying from INSIDE the shadowed helper resolves to
        // config.services.other. A predicate found inside helper must
        // never be attributed to Davis's option_prefix -- this is the
        // machinery a future predicate-site check would rely on for that.
        let src = r#"
            { config, lib, ... }:
            let
              helper = x:
                let
                  cfg = config.services.other;
                in
                cfg.foo;

              cfg = config.services.davis;
            in
            {
              options.services.davis = {
                foo = lib.mkOption { default = null; };
              };
            }
        "#;
        let root = rnix::Root::parse(src);
        assert!(root.errors().is_empty());
        let tree = root.tree();

        let declaration = tree
            .syntax()
            .descendants()
            .find(|n| {
                n.kind() == NODE_ATTRPATH_VALUE
                    && n.children()
                        .next()
                        .and_then(|ap| attrpath_segments(&ap))
                        .as_deref()
                        == Some(
                            &[
                                "options".to_string(),
                                "services".to_string(),
                                "davis".to_string(),
                            ][..],
                        )
            })
            .expect("the flat declaration node must be found");
        assert_eq!(
            resolve_cfg_root(&declaration, "cfg"),
            Ok(vec!["services".to_string(), "davis".to_string()])
        );

        let inner_cfg_use = tree
            .syntax()
            .descendants()
            .find(|n| {
                n.kind() == NODE_SELECT && as_select(n).map(|(r, _)| r).as_deref() == Some("cfg")
            })
            .expect("the inner cfg.foo select inside helper must be found");
        assert_eq!(
            resolve_cfg_root(&inner_cfg_use, "cfg"),
            Ok(vec!["services".to_string(), "other".to_string()]),
            "a use site inside the inner shadowed scope must resolve to the INNER cfg, \
             never the outer one"
        );
    }

    // --- H1/H2 compatibility: the new IR+evaluator must reproduce H1's
    // existing predicate_outcome() table for every unary case that table
    // actually covers. `ValueClass::DefinitelyNonNull` is represented as
    // the real `KnownValue::DefinitelyNonNull` (not a fabricated
    // placeholder scalar -- see `KnownValue`'s own doc comment): any
    // concrete non-null value proves a null-comparison's outcome, which is
    // exactly what `DefinitelyNonNull` meant in H1 too, and
    // `eval_known_eq` encodes exactly that without needing to know which
    // non-null value it actually is. `ValueClass::Unknown` is represented
    // as the path being *absent* from the environment, matching
    // `eval_value_expr`'s own "unresolved, not a guess" semantics. One
    // case is deliberately NOT covered here: `Truthy`/`NegTruthy` combined
    // with `DefinitelyNonNull` returned `None` in H1's table, but H1 never
    // actually reached that state through any real predicate +
    // default/test-value combination in the golden suite -- a `Truthy`
    // predicate's operand must be a real Nix bool for evaluation to
    // succeed at all, so "known non-null, exact value withheld" was
    // already a degenerate corner of the old model, not a case this IR
    // needs to preserve bit-for-bit.

    fn env_of(
        pairs: &[(&[&str], KnownValue)],
    ) -> std::collections::HashMap<OptionPath, KnownValue> {
        pairs
            .iter()
            .map(|(path, s)| (path.iter().map(|s| s.to_string()).collect(), s.clone()))
            .collect()
    }

    #[test]
    fn h1_compat_null_predicates() {
        let neq = Pred::Not(Box::new(Pred::Eq(
            ValueExpr::Ref(vec!["foo".into()]),
            ValueExpr::Literal(Scalar::Null),
        )));
        let eq = Pred::Eq(
            ValueExpr::Ref(vec!["foo".into()]),
            ValueExpr::Literal(Scalar::Null),
        );

        // NullNeq (cfg.foo != null)
        assert_eq!(
            eval_pred(
                &neq,
                &env_of(&[(&["foo"], KnownValue::Exact(Scalar::Null))])
            ),
            predicate_outcome(&PredicateKind::NullNeq, ValueClass::Null),
        );
        assert_eq!(
            eval_pred(
                &neq,
                &env_of(&[(&["foo"], KnownValue::Exact(Scalar::Bool(true)))])
            ),
            predicate_outcome(&PredicateKind::NullNeq, ValueClass::Bool(true)),
        );
        assert_eq!(
            eval_pred(&neq, &env_of(&[(&["foo"], KnownValue::DefinitelyNonNull)])),
            predicate_outcome(&PredicateKind::NullNeq, ValueClass::DefinitelyNonNull),
        );
        assert_eq!(
            eval_pred(&neq, &env_of(&[])),
            predicate_outcome(&PredicateKind::NullNeq, ValueClass::Unknown),
        );

        // NullEq (cfg.foo == null) -- same environments, the other predicate.
        assert_eq!(
            eval_pred(&eq, &env_of(&[(&["foo"], KnownValue::Exact(Scalar::Null))])),
            predicate_outcome(&PredicateKind::NullEq, ValueClass::Null),
        );
        assert_eq!(
            eval_pred(
                &eq,
                &env_of(&[(&["foo"], KnownValue::Exact(Scalar::Bool(true)))])
            ),
            predicate_outcome(&PredicateKind::NullEq, ValueClass::Bool(true)),
        );
        assert_eq!(
            eval_pred(&eq, &env_of(&[(&["foo"], KnownValue::DefinitelyNonNull)])),
            predicate_outcome(&PredicateKind::NullEq, ValueClass::DefinitelyNonNull),
        );
        assert_eq!(
            eval_pred(&eq, &env_of(&[])),
            predicate_outcome(&PredicateKind::NullEq, ValueClass::Unknown),
        );
    }

    #[test]
    fn h1_compat_truthy_predicates() {
        let truthy = Pred::Eq(
            ValueExpr::Ref(vec!["foo".into()]),
            ValueExpr::Literal(Scalar::Bool(true)),
        );
        let neg_truthy = Pred::Not(Box::new(truthy.clone()));

        for b in [true, false] {
            assert_eq!(
                eval_pred(
                    &truthy,
                    &env_of(&[(&["foo"], KnownValue::Exact(Scalar::Bool(b)))])
                ),
                predicate_outcome(&PredicateKind::Truthy("if".into()), ValueClass::Bool(b)),
            );
            assert_eq!(
                eval_pred(
                    &neg_truthy,
                    &env_of(&[(&["foo"], KnownValue::Exact(Scalar::Bool(b)))])
                ),
                predicate_outcome(&PredicateKind::NegTruthy, ValueClass::Bool(b)),
            );
        }
        assert_eq!(
            eval_pred(&truthy, &env_of(&[])),
            predicate_outcome(&PredicateKind::Truthy("if".into()), ValueClass::Unknown),
        );
        assert_eq!(
            eval_pred(&neg_truthy, &env_of(&[])),
            predicate_outcome(&PredicateKind::NegTruthy, ValueClass::Unknown),
        );
    }

    // --- Kleene short-circuit refinement of eval_pred's And/Or, added on
    // review of commit 1: a known operand can pin the combined result even
    // when the *other* operand is unresolved. Symmetric -- it doesn't
    // matter which side carries the known absorbing value, since the
    // underlying claim ("a && b is false because b is false, regardless of
    // a's real value") holds either way. `p_true`/`p_false` are `Pred`s
    // with no `Ref` inside, so they evaluate the same in any environment.

    fn p_true() -> Pred {
        Pred::Eq(
            ValueExpr::Literal(Scalar::Bool(true)),
            ValueExpr::Literal(Scalar::Bool(true)),
        )
    }

    fn p_false() -> Pred {
        Pred::Not(Box::new(p_true()))
    }

    fn p_unknown() -> Pred {
        // A Ref to a path that's never in the environment used below.
        Pred::Eq(
            ValueExpr::Ref(vec!["never-bound".into()]),
            ValueExpr::Literal(Scalar::Bool(true)),
        )
    }

    #[test]
    fn eval_pred_and_short_circuits_on_a_known_false_operand_either_side() {
        let empty = std::collections::HashMap::new();
        assert_eq!(
            eval_pred(
                &Pred::And(Box::new(p_false()), Box::new(p_unknown())),
                &empty
            ),
            Some(false),
            "false && unknown must be false (real Nix never forces the right side)"
        );
        assert_eq!(
            eval_pred(
                &Pred::And(Box::new(p_unknown()), Box::new(p_false())),
                &empty
            ),
            Some(false),
            "unknown && false must also be false: whichever real value the left side \
             turns out to have, the result is false either way"
        );
    }

    #[test]
    fn eval_pred_or_short_circuits_on_a_known_true_operand_either_side() {
        let empty = std::collections::HashMap::new();
        assert_eq!(
            eval_pred(&Pred::Or(Box::new(p_true()), Box::new(p_unknown())), &empty),
            Some(true),
            "true || unknown must be true"
        );
        assert_eq!(
            eval_pred(&Pred::Or(Box::new(p_unknown()), Box::new(p_true())), &empty),
            Some(true),
            "unknown || true must also be true, symmetrically"
        );
    }

    #[test]
    fn eval_pred_and_or_stay_unresolved_when_no_operand_pins_the_result() {
        let empty = std::collections::HashMap::new();
        assert_eq!(
            eval_pred(
                &Pred::And(Box::new(p_true()), Box::new(p_unknown())),
                &empty
            ),
            None,
            "true && unknown genuinely depends on the unknown operand"
        );
        assert_eq!(
            eval_pred(
                &Pred::And(Box::new(p_unknown()), Box::new(p_true())),
                &empty
            ),
            None,
        );
        assert_eq!(
            eval_pred(
                &Pred::Or(Box::new(p_false()), Box::new(p_unknown())),
                &empty
            ),
            None,
            "false || unknown genuinely depends on the unknown operand"
        );
        assert_eq!(
            eval_pred(
                &Pred::Or(Box::new(p_unknown()), Box::new(p_false())),
                &empty
            ),
            None,
        );
    }

    // --- Property-based tests (proptest) over the pure IR -- the
    // "property-based tests" evidence tier AGENTS.md asks for, ahead of
    // mutation testing (task #8, once alias resolution exists too).

    fn arb_scalar() -> impl proptest::strategy::Strategy<Value = Scalar> {
        use proptest::prelude::*;
        prop_oneof![
            Just(Scalar::Null),
            any::<bool>().prop_map(Scalar::Bool),
            "[a-c]{1,3}".prop_map(Scalar::Str),
        ]
    }

    fn arb_value_expr() -> impl proptest::strategy::Strategy<Value = ValueExpr> {
        use proptest::prelude::*;
        prop_oneof![
            prop::collection::vec("[a-c]", 1..=2).prop_map(ValueExpr::Ref),
            arb_scalar().prop_map(ValueExpr::Literal),
        ]
    }

    fn arb_pred() -> impl proptest::strategy::Strategy<Value = Pred> {
        use proptest::prelude::*;
        let leaf = (arb_value_expr(), arb_value_expr()).prop_map(|(a, b)| Pred::Eq(a, b));
        leaf.prop_recursive(4, 32, 3, |inner| {
            prop_oneof![
                inner.clone().prop_map(|p| Pred::Not(Box::new(p))),
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| Pred::And(Box::new(a), Box::new(b))),
                (inner.clone(), inner).prop_map(|(a, b)| Pred::Or(Box::new(a), Box::new(b))),
            ]
        })
    }

    /// Always evaluates to `Some(true)` regardless of environment (no
    /// `Ref` inside) -- the neutral element `And(p, true_pred()) == p`
    /// needs.
    fn true_pred() -> Pred {
        Pred::Eq(
            ValueExpr::Literal(Scalar::Bool(true)),
            ValueExpr::Literal(Scalar::Bool(true)),
        )
    }

    fn false_pred() -> Pred {
        Pred::Not(Box::new(true_pred()))
    }

    proptest::proptest! {
        #[test]
        fn not_not_is_identity(p in arb_pred(), env in arb_value_expr()) {
            // `env` here just forces proptest to exercise a handful of
            // concrete single-path environments as a cheap source of
            // variety; `Not(Not(p)) == p` provably holds for *any*
            // environment (double negation of an Option<bool> via `.map`
            // is the identity), full, partial, or empty alike, so a full
            // arbitrary HashMap generator adds no extra coverage here.
            let env: std::collections::HashMap<OptionPath, KnownValue> = match env {
                ValueExpr::Ref(path) => {
                    [(path, KnownValue::Exact(Scalar::Bool(true)))].into_iter().collect()
                }
                ValueExpr::Literal(_) => std::collections::HashMap::new(),
            };
            proptest::prop_assert_eq!(
                eval_pred(&Pred::Not(Box::new(Pred::Not(Box::new(p.clone())))), &env),
                eval_pred(&p, &env)
            );
        }

        #[test]
        fn and_true_is_identity(p in arb_pred()) {
            let env = std::collections::HashMap::new();
            proptest::prop_assert_eq!(
                eval_pred(&Pred::And(Box::new(p.clone()), Box::new(true_pred())), &env),
                eval_pred(&p, &env)
            );
        }

        #[test]
        fn or_false_is_identity(p in arb_pred()) {
            let env = std::collections::HashMap::new();
            proptest::prop_assert_eq!(
                eval_pred(&Pred::Or(Box::new(p.clone()), Box::new(false_pred())), &env),
                eval_pred(&p, &env)
            );
        }

        // The property behind H1's own fail-closed discipline, generalized:
        // starting from an environment that fully resolves `p`, deleting
        // any single entry must never turn a known `Some(b)` into a
        // *different* known result -- it may only weaken to `None`. This is
        // the Pred/evaluator-level version of "replacing known input with
        // unknown may weaken a conclusion but must never create PASS or
        // OBA001" (the run_target-level guarantee lands with the
        // counterfactual gate-4 rewrite in the next commit, built on top of
        // this).
        #[test]
        fn removing_a_known_value_never_flips_a_known_result(p in arb_pred()) {
            let mut paths = Vec::new();
            refs_in_pred(&p, &mut paths);
            paths.sort();
            paths.dedup();

            let full_env: std::collections::HashMap<OptionPath, KnownValue> = paths
                .iter()
                .cloned()
                .map(|path| (path, KnownValue::Exact(Scalar::Bool(true))))
                .collect();
            let full_result = eval_pred(&p, &full_env);

            for path in &paths {
                let mut weakened = full_env.clone();
                weakened.remove(path);
                let weakened_result = eval_pred(&p, &weakened);
                proptest::prop_assert!(
                    weakened_result.is_none() || weakened_result == full_result,
                    "removing {:?} changed a known result from {:?} to {:?}",
                    path, full_result, weakened_result
                );
            }
        }
    }

    // -----------------------------------------------------------------
    // PR D1: pure compare() -- invariants + an adversarial corpus, all
    // standalone (no CLI, no filesystem, no Git/GitHub), per the design
    // note's own sequencing.
    // -----------------------------------------------------------------

    fn t_span() -> Span {
        Span { file: "f.nix".to_string(), line: 1, col: 1 }
    }

    fn t_predicate_ref() -> PredicateRef {
        PredicateRef::Unary(Predicate {
            path: vec!["x".to_string()],
            kind: PredicateKind::NullNeq,
            span: t_span(),
            source: "cfg.x != null".to_string(),
        })
    }

    const ALL_VERDICT_KINDS: [VerdictKind; 7] = [
        VerdictKind::OptionNotFound,
        VerdictKind::PredicateNotFound,
        VerdictKind::DefaultUnresolved,
        VerdictKind::TestValueUnresolved,
        VerdictKind::TestConfigUnresolved,
        VerdictKind::Oba001,
        VerdictKind::Pass,
    ];

    /// A representative `Verdict` of the given bare kind, for a given
    /// watched-option path -- payload content is arbitrary/minimal on
    /// purpose (tests that care about payload independence, e.g.
    /// `same_verdict_kind_different_evidence_or_span_is_unchanged`, vary
    /// it explicitly themselves).
    fn verdict_of_kind(kind: VerdictKind, option: &str) -> Verdict {
        match kind {
            VerdictKind::OptionNotFound => Verdict::OptionNotFound { option: option.to_string() },
            VerdictKind::PredicateNotFound => {
                Verdict::PredicateNotFound { option: option.to_string() }
            }
            VerdictKind::DefaultUnresolved => Verdict::DefaultUnresolved {
                option: option.to_string(),
                predicate: t_predicate_ref(),
            },
            VerdictKind::TestValueUnresolved => Verdict::TestValueUnresolved {
                option: option.to_string(),
                predicate: t_predicate_ref(),
                default_outcome: Some(false),
                predicate_attempts: Vec::new(),
            },
            VerdictKind::TestConfigUnresolved => Verdict::TestConfigUnresolved {
                option: option.to_string(),
                predicate: t_predicate_ref(),
                default_outcome: Some(false),
                predicate_attempts: Vec::new(),
            },
            VerdictKind::Oba001 => Verdict::Oba001 {
                option: option.to_string(),
                predicate: t_predicate_ref(),
                default_outcome: Some(false),
                predicate_attempts: Vec::new(),
            },
            VerdictKind::Pass => Verdict::Pass {
                option: option.to_string(),
                predicate: t_predicate_ref(),
                default_outcome: false,
                evidence: Vec::new(),
                predicate_attempts: Vec::new(),
            },
        }
    }

    fn mk_target_report(
        name: &str,
        module: &str,
        test: &str,
        cfg_ident: &str,
        option_prefix: &[&str],
        verdicts: Vec<Verdict>,
    ) -> TargetReport {
        TargetReport {
            name: name.to_string(),
            module: PathBuf::from(module),
            test: PathBuf::from(test),
            cfg_ident: cfg_ident.to_string(),
            option_prefix: option_prefix.iter().map(|s| s.to_string()).collect(),
            parse_errors: Vec::new(),
            discovered_options: Vec::new(),
            discovered_predicates: Vec::new(),
            resolved_predicates: Vec::new(),
            unresolved_predicate_sites: Vec::new(),
            matched_test_assignments: Vec::new(),
            test_config_opacity: Vec::new(),
            verdicts,
        }
    }

    fn mk_report(targets: Vec<TargetReport>) -> AnalysisReport {
        AnalysisReport { targets }
    }

    fn one_target_report(verdicts: Vec<Verdict>) -> AnalysisReport {
        mk_report(vec![mk_target_report(
            "t",
            "module.nix",
            "test.nix",
            "cfg",
            &["services", "x"],
            verdicts,
        )])
    }

    // --- adversarial corpus (example-based) ---

    #[test]
    fn compare_of_identical_report_is_all_unchanged() {
        let r = one_target_report(vec![
            verdict_of_kind(VerdictKind::Pass, "a"),
            verdict_of_kind(VerdictKind::Oba001, "b"),
            verdict_of_kind(VerdictKind::TestConfigUnresolved, "c"),
        ]);
        let diff = compare(&r, &r).unwrap();
        assert_eq!(diff.entries.len(), 3);
        assert!(diff.entries.iter().all(|e| e.diff == TargetDiff::Unchanged));
    }

    #[test]
    fn reordering_verdicts_does_not_change_the_comparison() {
        let base = one_target_report(vec![
            verdict_of_kind(VerdictKind::Pass, "a"),
            verdict_of_kind(VerdictKind::Oba001, "b"),
        ]);
        let base_reordered = one_target_report(vec![
            verdict_of_kind(VerdictKind::Oba001, "b"),
            verdict_of_kind(VerdictKind::Pass, "a"),
        ]);
        let head = one_target_report(vec![
            verdict_of_kind(VerdictKind::Pass, "a"),
            verdict_of_kind(VerdictKind::TestValueUnresolved, "b"),
        ]);
        assert_eq!(compare(&base, &head).unwrap(), compare(&base_reordered, &head).unwrap());
    }

    #[test]
    fn comparison_entries_are_sorted_by_identity() {
        // Deliberately inserted out of sorted order.
        let r = one_target_report(vec![
            verdict_of_kind(VerdictKind::Pass, "z"),
            verdict_of_kind(VerdictKind::Pass, "a"),
            verdict_of_kind(VerdictKind::Pass, "m"),
        ]);
        let diff = compare(&r, &r).unwrap();
        let identities: Vec<_> = diff.entries.iter().map(|e| &e.identity).collect();
        let mut sorted = identities.clone();
        sorted.sort();
        assert_eq!(identities, sorted);
    }

    #[test]
    fn added_target_is_reported_as_added_not_changed() {
        let base = one_target_report(vec![]);
        let head = one_target_report(vec![verdict_of_kind(VerdictKind::Pass, "a")]);
        let diff = compare(&base, &head).unwrap();
        assert_eq!(diff.entries.len(), 1);
        assert!(matches!(diff.entries[0].diff, TargetDiff::Added { .. }));
    }

    #[test]
    fn removed_target_is_reported_as_removed_not_changed() {
        let base = one_target_report(vec![verdict_of_kind(VerdictKind::Pass, "a")]);
        let head = one_target_report(vec![]);
        let diff = compare(&base, &head).unwrap();
        assert_eq!(diff.entries.len(), 1);
        assert!(matches!(diff.entries[0].diff, TargetDiff::Removed { .. }));
    }

    #[test]
    fn every_verdict_kind_transition_is_reported_correctly() {
        // Cheap enough to brute-force all 49 pairs at the unit-test level,
        // same discipline as aggregate's own 32-case exhaustive check.
        for from in ALL_VERDICT_KINDS {
            for to in ALL_VERDICT_KINDS {
                let base = one_target_report(vec![verdict_of_kind(from, "a")]);
                let head = one_target_report(vec![verdict_of_kind(to, "a")]);
                let diff = compare(&base, &head).unwrap();
                assert_eq!(diff.entries.len(), 1);
                if from == to {
                    assert_eq!(
                        diff.entries[0].diff,
                        TargetDiff::Unchanged,
                        "same kind {from:?} must be Unchanged"
                    );
                } else {
                    match &diff.entries[0].diff {
                        TargetDiff::Changed { changes, .. } => {
                            assert_eq!(changes, &vec![ChangeKind::VerdictChanged]);
                        }
                        other => panic!("{from:?} -> {to:?}: expected Changed, got {other:?}"),
                    }
                    let transition = diff.entries[0].diff.verdict_transition().unwrap();
                    assert_eq!(transition.from, from);
                    assert_eq!(transition.to, to);
                }
            }
        }
    }

    /// The contract that gives `Unchanged` its precise meaning: "no
    /// tracked `ChangeKind`", NOT "byte-for-byte/structurally identical
    /// `TargetOutcome`". Same verdict kind, deliberately different
    /// evidence -- must still be `Unchanged` in v1, which only computes
    /// `VerdictChanged`.
    #[test]
    fn same_verdict_kind_different_evidence_is_unchanged() {
        let base = one_target_report(vec![Verdict::Pass {
            option: "a".to_string(),
            predicate: t_predicate_ref(),
            default_outcome: false,
            evidence: vec![],
            predicate_attempts: vec![],
        }]);
        let head = one_target_report(vec![Verdict::Pass {
            option: "a".to_string(),
            predicate: t_predicate_ref(),
            default_outcome: false,
            evidence: vec![TestAssignment {
                path: vec!["a".to_string()],
                value_source: "true".to_string(),
                value_class: ValueClass::Bool(true),
                known_value: Some(KnownValue::Exact(Scalar::Bool(true))),
                instance: None,
                span: t_span(),
            }],
            predicate_attempts: vec![PredicateAttempt {
                predicate: t_predicate_ref(),
                witnessed: Some(true),
            }],
        }]);
        assert_eq!(compare(&base, &head).unwrap().entries[0].diff, TargetDiff::Unchanged);
    }

    /// Same guarantee, isolated to JUST a span difference (the most
    /// purely presentational field this codebase has) on an otherwise
    /// byte-identical verdict.
    #[test]
    fn same_verdict_kind_different_span_is_unchanged() {
        let mut predicate_a = t_predicate_ref();
        let mut predicate_b = t_predicate_ref();
        if let PredicateRef::Unary(p) = &mut predicate_a {
            p.span = Span { file: "a.nix".to_string(), line: 1, col: 1 };
        }
        if let PredicateRef::Unary(p) = &mut predicate_b {
            p.span = Span { file: "a.nix".to_string(), line: 99, col: 7 };
        }
        let base = one_target_report(vec![Verdict::Oba001 {
            option: "a".to_string(),
            predicate: predicate_a,
            default_outcome: Some(false),
            predicate_attempts: vec![],
        }]);
        let head = one_target_report(vec![Verdict::Oba001 {
            option: "a".to_string(),
            predicate: predicate_b,
            default_outcome: Some(false),
            predicate_attempts: vec![],
        }]);
        assert_eq!(compare(&base, &head).unwrap().entries[0].diff, TargetDiff::Unchanged);
    }

    #[test]
    fn moving_a_module_file_is_removed_plus_added_not_a_rename() {
        let base = mk_report(vec![mk_target_report(
            "t",
            "old.nix",
            "test.nix",
            "cfg",
            &["services", "x"],
            vec![verdict_of_kind(VerdictKind::Pass, "a")],
        )]);
        let head = mk_report(vec![mk_target_report(
            "t",
            "new.nix",
            "test.nix",
            "cfg",
            &["services", "x"],
            vec![verdict_of_kind(VerdictKind::Pass, "a")],
        )]);
        let diff = compare(&base, &head).unwrap();
        assert_eq!(diff.entries.len(), 2);
        // Order not asserted here on purpose -- entries sort by identity,
        // whose first field is `module`, so which of "new.nix"/"old.nix"
        // comes first is just lexicographic accident, not a claim this
        // test is about (sortedness itself has its own dedicated test).
        let mut kinds: Vec<_> = diff
            .entries
            .iter()
            .map(|e| match &e.diff {
                TargetDiff::Added { .. } => "added",
                TargetDiff::Removed { .. } => "removed",
                TargetDiff::Changed { .. } => "changed",
                TargetDiff::Unchanged => "unchanged",
            })
            .collect();
        kinds.sort();
        assert_eq!(kinds, vec!["added", "removed"]);
    }

    #[test]
    fn duplicate_identity_within_one_side_is_an_error() {
        // Two DIFFERENT target blocks that happen to share the exact same
        // (module, test, cfg_ident, option_prefix) and both watch "a" --
        // the identity tuple collides even though nothing about the
        // manifest looks obviously duplicated at a glance.
        let base = mk_report(vec![
            mk_target_report(
                "t1",
                "module.nix",
                "test.nix",
                "cfg",
                &["services", "x"],
                vec![verdict_of_kind(VerdictKind::Pass, "a")],
            ),
            mk_target_report(
                "t2",
                "module.nix",
                "test.nix",
                "cfg",
                &["services", "x"],
                vec![verdict_of_kind(VerdictKind::Oba001, "a")],
            ),
        ]);
        let head = one_target_report(vec![verdict_of_kind(VerdictKind::Pass, "a")]);
        let err = compare(&base, &head).unwrap_err();
        assert_eq!(err.side, CompareSide::Base);
        assert_eq!(err.identity.watched_path, "a");
    }

    #[test]
    fn multiple_simultaneous_changes_keep_a_stable_sorted_order() {
        let base = one_target_report(vec![
            verdict_of_kind(VerdictKind::Pass, "b_unchanged"),
            verdict_of_kind(VerdictKind::Pass, "c_removed"),
            verdict_of_kind(VerdictKind::Oba001, "a_changed"),
        ]);
        let head = one_target_report(vec![
            verdict_of_kind(VerdictKind::Pass, "b_unchanged"),
            verdict_of_kind(VerdictKind::Pass, "a_changed"),
            verdict_of_kind(VerdictKind::Pass, "d_added"),
        ]);
        let diff = compare(&base, &head).unwrap();
        let paths: Vec<_> = diff.entries.iter().map(|e| e.identity.watched_path.clone()).collect();
        // Sorted lexicographically by watched_path (this fixture's only
        // varying identity component), independent of which of
        // added/removed/changed/unchanged each one is.
        assert_eq!(
            paths,
            vec!["a_changed", "b_unchanged", "c_removed", "d_added"]
        );
        assert!(matches!(diff.entries[0].diff, TargetDiff::Changed { .. }));
        assert_eq!(diff.entries[1].diff, TargetDiff::Unchanged);
        assert!(matches!(diff.entries[2].diff, TargetDiff::Removed { .. }));
        assert!(matches!(diff.entries[3].diff, TargetDiff::Added { .. }));
    }

    // --- property-based invariants (proptest) ---

    fn arb_analysis_report() -> impl proptest::strategy::Strategy<Value = AnalysisReport> {
        use proptest::prelude::*;
        prop::collection::hash_set("[a-e]", 1..=5)
            .prop_flat_map(|names| {
                let names: Vec<String> = names.into_iter().collect();
                let n = names.len();
                (
                    Just(names),
                    prop::collection::vec(proptest::sample::select(&ALL_VERDICT_KINDS[..]), n),
                )
            })
            .prop_map(|(names, kinds)| {
                let verdicts = names
                    .iter()
                    .zip(kinds.iter())
                    .map(|(name, kind)| verdict_of_kind(*kind, name))
                    .collect();
                one_target_report(verdicts)
            })
    }

    proptest::proptest! {
        #[test]
        fn prop_compare_of_identical_report_is_all_unchanged(r in arb_analysis_report()) {
            let diff = compare(&r, &r).unwrap();
            proptest::prop_assert!(diff.entries.iter().all(|e| e.diff == TargetDiff::Unchanged));
        }

        #[test]
        fn prop_reordering_base_verdicts_does_not_change_the_comparison(
            base in arb_analysis_report(),
            head in arb_analysis_report(),
        ) {
            let mut base_reordered = base.clone();
            base_reordered.targets[0].verdicts.reverse();
            proptest::prop_assert_eq!(
                compare(&base, &head).unwrap(),
                compare(&base_reordered, &head).unwrap()
            );
        }

        #[test]
        fn prop_comparison_entries_are_always_sorted(
            base in arb_analysis_report(),
            head in arb_analysis_report(),
        ) {
            let diff = compare(&base, &head).unwrap();
            let identities: Vec<_> = diff.entries.iter().map(|e| e.identity.clone()).collect();
            let mut sorted = identities.clone();
            sorted.sort();
            proptest::prop_assert_eq!(identities, sorted);
        }

        #[test]
        fn prop_compare_is_mirrored_under_swap(
            base in arb_analysis_report(),
            head in arb_analysis_report(),
        ) {
            let forward = compare(&base, &head).unwrap();
            let backward = compare(&head, &base).unwrap();
            proptest::prop_assert_eq!(forward.entries.len(), backward.entries.len());
            for (f, b) in forward.entries.iter().zip(backward.entries.iter()) {
                proptest::prop_assert_eq!(&f.identity, &b.identity);
                match (&f.diff, &b.diff) {
                    (TargetDiff::Unchanged, TargetDiff::Unchanged) => {}
                    (TargetDiff::Added { head: h }, TargetDiff::Removed { base: b2 }) => {
                        proptest::prop_assert_eq!(h, b2);
                    }
                    (TargetDiff::Removed { base: b1 }, TargetDiff::Added { head: h2 }) => {
                        proptest::prop_assert_eq!(b1, h2);
                    }
                    (
                        TargetDiff::Changed { base: fb, head: fh, changes: fc },
                        TargetDiff::Changed { base: bb, head: bh, changes: bc },
                    ) => {
                        proptest::prop_assert_eq!(fb, bh);
                        proptest::prop_assert_eq!(fh, bb);
                        proptest::prop_assert_eq!(fc, bc);
                    }
                    (a, b) => proptest::prop_assert!(
                        false,
                        "mismatched diff shapes under swap: {:?} vs {:?}", a, b
                    ),
                }
            }
        }
    }

    // --- P3b: CDC's own diff algebra (compare_cdc_result/compare_cdc),
    // offline, pure -- covers every CdcDiff kind including the two
    // (AddedSubject/RemovedSubject) that don't have a ready-made real
    // historical trigger in this project's own real corpus (confirmed
    // by trying: akkoma/spacecookie are both already stable across the
    // real BEFORE_REV/current pin, so this specific edge needs a
    // constructed case to be tested at all -- exactly the situation
    // synthetic fixtures exist for). ---

    fn cdc_result(target: &str, verdict: ResultVerdict, emitted: &[&str]) -> AuditResult {
        AuditResult {
            engine: "cdc",
            target: target.to_string(),
            verdict,
            code: match verdict {
                ResultVerdict::Finding => Some("CDC001"),
                ResultVerdict::Inconclusive => Some("CDC002"),
                _ => None,
            },
            severity: None,
            message: "synthetic".to_string(),
            provenance: vec!["synthetic provenance".to_string()],
            cdc_evidence: Some(CdcResultEvidence {
                producer: CdcProducerEvidence { proved: true, format: cdc::ConfigFormat::Toml },
                binding: CdcBindingEvidence { kind: "exec-flag", proof_depth: cdc::ProofDepth::Structural },
                consumer: CdcConsumerEvidence { name: target.to_string() },
                comparison: CdcComparisonEvidence {
                    emitted_paths: emitted.iter().map(|s| s.to_string()).collect(),
                    accepted_paths: vec!["debug".to_string()],
                    opaque_paths: Vec::new(),
                },
            }),
            oba_evidence: None,
        }
    }

    fn cdc_tool_error_result(target: &str) -> AuditResult {
        AuditResult {
            engine: "cdc",
            target: target.to_string(),
            verdict: ResultVerdict::ToolError,
            code: None,
            severity: None,
            message: "synthetic tool error".to_string(),
            provenance: vec!["synthetic".to_string()],
            cdc_evidence: None,
            oba_evidence: None,
        }
    }

    #[test]
    fn cdc_diff_identical_results_is_unchanged() {
        let a = cdc_result("unpackerr", ResultVerdict::Pass, &["debug"]);
        let b = a.clone();
        assert_eq!(compare_cdc_result(&a, &b), CdcDiff::Unchanged);
    }

    #[test]
    fn cdc_diff_provenance_text_alone_never_creates_a_change() {
        // invariant 4: presentation/provenance text differing must
        // never, on its own, produce anything but Unchanged.
        let mut a = cdc_result("unpackerr", ResultVerdict::Pass, &["debug"]);
        let mut b = a.clone();
        a.provenance = vec!["one real phrasing".to_string()];
        b.provenance = vec!["a totally different real phrasing".to_string()];
        a.message = "phrasing A".to_string();
        b.message = "phrasing B".to_string();
        assert_eq!(compare_cdc_result(&a, &b), CdcDiff::Unchanged);
    }

    #[test]
    fn cdc_diff_verdict_changed_when_verdict_differs() {
        let base = cdc_result("akkoma", ResultVerdict::Pass, &["debug"]);
        let head = cdc_result("akkoma", ResultVerdict::Finding, &["debug", "upload_dir"]);
        let diff = compare_cdc_result(&base, &head);
        match &diff {
            CdcDiff::Changed { changes, .. } => {
                assert_eq!(changes, &vec![CdcChangeKind::VerdictChanged]);
            }
            other => panic!("expected Changed, got {other:?}"),
        }
        assert_eq!(diff.verdict_transition(), Some((ResultVerdict::Pass, ResultVerdict::Finding)));
    }

    #[test]
    fn cdc_diff_evidence_changed_when_verdict_same_but_evidence_differs() {
        // invariant 4/6: a real proof_depth (or any other evidence-only)
        // difference with the SAME verdict on both sides must be
        // EvidenceChanged, never treated as a brand-new finding.
        let base = cdc_result("unpackerr", ResultVerdict::Pass, &["debug"]);
        let mut head = base.clone();
        head.cdc_evidence.as_mut().unwrap().binding.proof_depth = cdc::ProofDepth::ByteExact;
        let diff = compare_cdc_result(&base, &head);
        match &diff {
            CdcDiff::Changed { changes, .. } => {
                assert_eq!(changes, &vec![CdcChangeKind::EvidenceChanged]);
            }
            other => panic!("expected Changed, got {other:?}"),
        }
        // EvidenceChanged alone must never be reported as a verdict
        // transition -- there isn't one.
        assert_eq!(diff.verdict_transition(), None);
    }

    #[test]
    fn cdc_diff_verdict_changed_alone_when_both_verdict_and_evidence_differ() {
        // when the verdict itself changes, the evidence differing too is
        // expected/implied -- must not ALSO report EvidenceChanged
        // redundantly (invariant 4/5's own "don't double-report" reading).
        let base = cdc_result("akkoma", ResultVerdict::Pass, &["debug"]);
        let mut head = cdc_result("akkoma", ResultVerdict::Finding, &["debug", "upload_dir"]);
        head.cdc_evidence.as_mut().unwrap().binding.proof_depth = cdc::ProofDepth::ByteExact;
        let diff = compare_cdc_result(&base, &head);
        match &diff {
            CdcDiff::Changed { changes, .. } => assert_eq!(changes, &vec![CdcChangeKind::VerdictChanged]),
            other => panic!("expected Changed, got {other:?}"),
        }
    }

    #[test]
    fn cdc_diff_added_subject_when_base_could_not_be_analyzed_and_head_is_real() {
        let base = cdc_tool_error_result("nats");
        let head = cdc_result("nats", ResultVerdict::Pass, &["config"]);
        match compare_cdc_result(&base, &head) {
            CdcDiff::AddedSubject { head: h } => assert_eq!(h.verdict, ResultVerdict::Pass),
            other => panic!("expected AddedSubject, got {other:?}"),
        }
    }

    #[test]
    fn cdc_diff_removed_subject_when_base_is_real_and_head_could_not_be_analyzed() {
        let base = cdc_result("nats", ResultVerdict::Pass, &["config"]);
        let head = cdc_tool_error_result("nats");
        match compare_cdc_result(&base, &head) {
            CdcDiff::RemovedSubject { base: b } => assert_eq!(b.verdict, ResultVerdict::Pass),
            other => panic!("expected RemovedSubject, got {other:?}"),
        }
    }

    #[test]
    fn cdc_diff_both_sides_tool_error_is_unchanged_not_added_or_removed() {
        let base = cdc_tool_error_result("nats");
        let head = cdc_tool_error_result("nats");
        assert_eq!(compare_cdc_result(&base, &head), CdcDiff::Unchanged);
    }

    #[test]
    fn compare_cdc_self_diff_is_all_unchanged() {
        // invariant 1, CDC half.
        let results = vec![
            cdc_result("unpackerr", ResultVerdict::Pass, &["debug"]),
            cdc_result("akkoma", ResultVerdict::Finding, &["debug", "upload_dir"]),
            cdc_tool_error_result("vault"),
        ];
        let entries = compare_cdc(&results, &results).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().all(|e| e.diff == CdcDiff::Unchanged));
    }

    #[test]
    fn compare_cdc_is_order_independent() {
        // invariant 2, CDC half.
        let base_a = vec![
            cdc_result("unpackerr", ResultVerdict::Pass, &["debug"]),
            cdc_result("akkoma", ResultVerdict::Finding, &["upload_dir"]),
        ];
        let base_b = vec![base_a[1].clone(), base_a[0].clone()];
        let head = vec![
            cdc_result("akkoma", ResultVerdict::Pass, &["debug"]),
            cdc_result("unpackerr", ResultVerdict::Pass, &["debug"]),
        ];
        let entries_a = compare_cdc(&base_a, &head).unwrap();
        let entries_b = compare_cdc(&base_b, &head).unwrap();
        assert_eq!(entries_a, entries_b);
    }

    #[test]
    fn compare_cdc_duplicate_candidate_on_one_side_is_an_error() {
        // invariant 3, CDC half -- defensive, since validate_manifest
        // already prevents this upstream; still checked directly here,
        // not just trusted.
        let base = vec![
            cdc_result("unpackerr", ResultVerdict::Pass, &["debug"]),
            cdc_result("unpackerr", ResultVerdict::Pass, &["debug"]),
        ];
        let head = vec![cdc_result("unpackerr", ResultVerdict::Pass, &["debug"])];
        let err = compare_cdc(&base, &head).unwrap_err();
        assert!(format!("{err}").contains("duplicate CDC candidate"));
    }

    // =======================================================================
    // P3c: bounded summary classification (`classify_transition_bucket`,
    // `oba_kind_class`) -- the one real judgment call in the bounded
    // summary, pinned down here so a consuming Action never has to
    // re-derive it from raw JSON.
    // =======================================================================

    #[test]
    fn oba_kind_class_matches_the_real_oba_verdict_to_result_mapping() {
        // Oba001 is the only real Finding-class kind, Pass the only
        // Pass-class kind, every other kind is Inconclusive -- the exact
        // same mapping `oba_verdict_to_result` already encodes per-kind.
        assert_eq!(oba_kind_class(VerdictKind::Oba001), ResultVerdict::Finding);
        assert_eq!(oba_kind_class(VerdictKind::Pass), ResultVerdict::Pass);
        for k in [
            VerdictKind::OptionNotFound,
            VerdictKind::PredicateNotFound,
            VerdictKind::DefaultUnresolved,
            VerdictKind::TestValueUnresolved,
            VerdictKind::TestConfigUnresolved,
        ] {
            assert_eq!(oba_kind_class(k), ResultVerdict::Inconclusive);
        }
    }

    #[test]
    fn classify_transition_bucket_crossing_into_finding_is_always_new_finding() {
        for from in [ResultVerdict::Pass, ResultVerdict::Inconclusive] {
            assert_eq!(
                classify_transition_bucket(from, ResultVerdict::Finding),
                Some("new_finding")
            );
        }
    }

    #[test]
    fn classify_transition_bucket_crossing_out_of_finding_splits_by_destination() {
        // S1-F2: Finding -> Pass and Finding -> Inconclusive are
        // DIFFERENT real events (the branch is now provably covered, vs.
        // visibility into it was lost) -- no longer folded into one
        // "resolved_finding" bucket.
        assert_eq!(
            classify_transition_bucket(ResultVerdict::Finding, ResultVerdict::Pass),
            Some("resolved_finding")
        );
        assert_eq!(
            classify_transition_bucket(ResultVerdict::Finding, ResultVerdict::Inconclusive),
            Some("finding_became_inconclusive")
        );
    }

    #[test]
    fn classify_transition_bucket_pass_to_inconclusive_is_new_inconclusive() {
        assert_eq!(
            classify_transition_bucket(ResultVerdict::Pass, ResultVerdict::Inconclusive),
            Some("new_inconclusive")
        );
    }

    #[test]
    fn classify_transition_bucket_inconclusive_to_pass_is_resolved_inconclusive() {
        // S1-F3: a real nixpkgs PR shadow audit found two genuine
        // Inconclusive -> Pass improvements that the original P3c design
        // left silently unbucketed. Now surfaced under its own name,
        // distinct from `resolved_finding`.
        assert_eq!(
            classify_transition_bucket(ResultVerdict::Inconclusive, ResultVerdict::Pass),
            Some("resolved_inconclusive")
        );
    }

    #[test]
    fn classify_transition_bucket_same_class_is_unbucketed() {
        // e.g. OBA's own OptionNotFound -> DefaultUnresolved: a real
        // VerdictChanged (different VerdictKind) that never crosses a
        // Pass/Finding/Inconclusive boundary at all.
        assert_eq!(classify_transition_bucket(ResultVerdict::Inconclusive, ResultVerdict::Inconclusive), None);
        assert_eq!(classify_transition_bucket(ResultVerdict::Pass, ResultVerdict::Pass), None);
        assert_eq!(classify_transition_bucket(ResultVerdict::Finding, ResultVerdict::Finding), None);
    }

    #[test]
    fn compare_cdc_new_finding_transition_is_classified_and_pushed_to_notable() {
        // Full pipeline, one level below run_audit_diff itself: a real
        // Pass -> Finding CdcDiff must both classify as "new_finding" and
        // carry the head result's own real code/message/provenance
        // verbatim into a NotableChange -- no new text invented.
        let base = cdc_result("akkoma", ResultVerdict::Pass, &["debug"]);
        let head = cdc_result("akkoma", ResultVerdict::Finding, &["debug", "upload_dir"]);
        let diff = compare_cdc_result(&base, &head);
        let CdcDiff::Changed { head: boxed_head, changes, .. } = &diff else {
            panic!("expected Changed, got {diff:?}");
        };
        assert!(changes.contains(&CdcChangeKind::VerdictChanged));
        let (from, to) = diff.verdict_transition().expect("a real verdict transition");
        assert_eq!(classify_transition_bucket(from, to), Some("new_finding"));

        let mut notable = Vec::new();
        let mut total = 0usize;
        push_notable(&mut notable, &mut total, "new_finding", "cdc", "akkoma".to_string(), boxed_head);
        assert_eq!(total, 1);
        assert_eq!(notable.len(), 1);
        assert_eq!(notable[0].bucket, "new_finding");
        assert_eq!(notable[0].engine, "cdc");
        assert_eq!(notable[0].code, Some("CDC001"));
        assert_eq!(notable[0].message, boxed_head.message);
        assert_eq!(notable[0].detail.as_deref(), Some("synthetic provenance"));
    }

    #[test]
    fn push_notable_stays_bounded_but_notable_total_keeps_counting() {
        // Invariant-adjacent for P3c: a real corpus with far more than
        // NOTABLE_LIMIT new findings must still produce a small, bounded
        // `notable` list, while `notable_total` stays the real, uncapped
        // count -- the difference is exactly what a consuming Action
        // prints as "+N more in the full artifact".
        let result = cdc_result("akkoma", ResultVerdict::Finding, &["upload_dir"]);
        let mut notable = Vec::new();
        let mut total = 0usize;
        for i in 0..(NOTABLE_LIMIT + 5) {
            push_notable(&mut notable, &mut total, "new_finding", "cdc", format!("candidate-{i}"), &result);
        }
        assert_eq!(total, NOTABLE_LIMIT + 5);
        assert_eq!(notable.len(), NOTABLE_LIMIT);
    }

    // =======================================================================
    // P3c: `render_github_summary` -- pure, offline, exactly the Markdown
    // a consuming Action `cat`s verbatim into `$GITHUB_STEP_SUMMARY`.
    // =======================================================================

    #[test]
    fn render_github_summary_bare_counts_appear_even_with_no_notable_entries() {
        let summary = AuditDiffSummary {
            unchanged: 47,
            new_findings: 2,
            resolved_findings: 1,
            new_inconclusives: 1,
            evidence_changed: 3,
            ..Default::default()
        };
        let md = render_github_summary(&summary, 0);
        assert!(md.contains("## Nix contract audit"));
        assert!(md.contains("| New findings | 2 |"));
        assert!(md.contains("| Resolved findings | 1 |"));
        assert!(md.contains("| New inconclusives | 1 |"));
        assert!(md.contains("| Evidence-only changes | 3 |"));
        assert!(md.contains("| Unchanged | 47 |"));
        // exit 0 -- no inconclusive caveat.
        assert!(!md.contains("inconclusive on at least one side"));
    }

    #[test]
    fn render_github_summary_exit_code_2_adds_the_inconclusive_caveat() {
        let md = render_github_summary(&AuditDiffSummary::default(), 2);
        assert!(md.contains("inconclusive on at least one side"));
    }

    #[test]
    fn render_github_summary_renders_a_notable_finding_with_real_text_verbatim() {
        let summary = AuditDiffSummary {
            new_findings: 1,
            notable_total: 1,
            notable: vec![NotableChange {
                bucket: "new_finding",
                engine: "cdc",
                subject: "akkoma".to_string(),
                code: Some("CDC001"),
                message: "emitted path is not in the accepted contract".to_string(),
                detail: Some("mismatch: :instance.upload_dir".to_string()),
            }],
            ..Default::default()
        };
        let md = render_github_summary(&summary, 0);
        assert!(md.contains("### NEW FINDING"));
        assert!(md.contains("**CDC001** `akkoma` (cdc)"));
        assert!(md.contains("> emitted path is not in the accepted contract"));
        assert!(md.contains("> mismatch: :instance.upload_dir"));
    }

    #[test]
    fn render_github_summary_new_inconclusive_gets_its_own_heading() {
        let summary = AuditDiffSummary {
            new_inconclusives: 1,
            notable_total: 1,
            notable: vec![NotableChange {
                bucket: "new_inconclusive",
                engine: "cdc",
                subject: "vault".to_string(),
                code: Some("CDC002"),
                message: "consumer binding could not be proven".to_string(),
                detail: None,
            }],
            ..Default::default()
        };
        let md = render_github_summary(&summary, 0);
        assert!(md.contains("### NEW INCONCLUSIVE"));
        assert!(!md.contains("### NEW FINDING"));
    }

    #[test]
    fn render_github_summary_truncation_note_only_appears_when_actually_truncated() {
        let mut summary = AuditDiffSummary { notable_total: 1, ..Default::default() };
        summary.notable.push(NotableChange {
            bucket: "new_finding",
            engine: "cdc",
            subject: "x".to_string(),
            code: None,
            message: "m".to_string(),
            detail: None,
        });
        assert!(!render_github_summary(&summary, 0).contains("more in the full artifact"));

        summary.notable_total = 15;
        let md = render_github_summary(&summary, 0);
        assert!(md.contains("_+14 more in the full artifact._"));
    }

    // =======================================================================
    // S1-F1: `resolve_within_root_if_exists` -- the real, purely offline
    // building block for audit-diff's own module-birth/death handling.
    // =======================================================================

    #[test]
    fn resolve_within_root_if_exists_none_for_a_real_missing_file() {
        let root = Path::new("fixtures/synthetic/audit-diff-module-lifecycle/before-empty");
        assert_eq!(resolve_within_root_if_exists(root, Path::new("module.nix")).unwrap(), None);
    }

    #[test]
    fn resolve_within_root_if_exists_some_for_a_real_present_file() {
        let root = Path::new("fixtures/synthetic/audit-diff-module-lifecycle/after-with-module");
        assert!(resolve_within_root_if_exists(root, Path::new("module.nix")).unwrap().is_some());
    }

    #[test]
    fn resolve_within_root_if_exists_rejects_an_absolute_path() {
        let root = Path::new("fixtures/synthetic/audit-diff-module-lifecycle/after-with-module");
        let err = resolve_within_root_if_exists(root, Path::new("/etc/passwd")).unwrap_err();
        assert!(format!("{err}").contains("must be relative"));
    }

    #[test]
    fn resolve_within_root_if_exists_rejects_a_real_root_escape() {
        // Same real fixture/relative-path pair `dotdot_escape_is_tool_error`
        // (tests/check_root.rs) already uses: --root fixtures/synthetic,
        // ../../Cargo.toml resolves to a real file two levels above it.
        let root = Path::new("fixtures/synthetic");
        let err = resolve_within_root_if_exists(root, Path::new("../../Cargo.toml")).unwrap_err();
        assert!(format!("{err}").contains("escapes --root"));
    }

    // =======================================================================
    // S1-F2/S1-F3: `added_bucket`/`removed_with_finding_bucket`/
    // `record_bucket` -- the honest Added/Removed-with-Finding semantics
    // and the resolved_finding/notable split.
    // =======================================================================

    #[test]
    fn added_bucket_classifies_a_brand_new_subjects_first_result() {
        assert_eq!(added_bucket(ResultVerdict::Finding), Some("new_finding"));
        assert_eq!(added_bucket(ResultVerdict::Inconclusive), Some("new_inconclusive"));
        assert_eq!(added_bucket(ResultVerdict::Pass), None);
    }

    #[test]
    fn removed_with_finding_bucket_only_fires_for_a_real_finding() {
        assert_eq!(
            removed_with_finding_bucket(ResultVerdict::Finding),
            Some("removed_subject_with_finding")
        );
        assert_eq!(removed_with_finding_bucket(ResultVerdict::Pass), None);
        assert_eq!(removed_with_finding_bucket(ResultVerdict::Inconclusive), None);
    }

    #[test]
    fn record_bucket_resolved_finding_counts_but_never_shows_in_notable() {
        // The one deliberate exception: resolved_finding is unambiguous
        // good news, a count is enough -- unlike every other real bucket
        // (including resolved_inconclusive, S1-F3's own point).
        let mut summary = AuditDiffSummary::default();
        let mut total = 0usize;
        let result = cdc_result("unpackerr", ResultVerdict::Pass, &["debug"]);
        record_bucket(&mut summary, &mut total, "resolved_finding", "cdc", "unpackerr".to_string(), &result);
        assert_eq!(summary.resolved_findings, 1);
        assert_eq!(total, 0);
        assert!(summary.notable.is_empty());
    }

    #[test]
    fn record_bucket_resolved_inconclusive_counts_and_shows_in_notable() {
        let mut summary = AuditDiffSummary::default();
        let mut total = 0usize;
        let result = cdc_result("unpackerr", ResultVerdict::Pass, &["debug"]);
        record_bucket(&mut summary, &mut total, "resolved_inconclusive", "cdc", "unpackerr".to_string(), &result);
        assert_eq!(summary.resolved_inconclusives, 1);
        assert_eq!(total, 1);
        assert_eq!(summary.notable.len(), 1);
        assert_eq!(summary.notable[0].bucket, "resolved_inconclusive");
    }

    #[test]
    fn record_bucket_removed_subject_with_finding_counts_and_shows_in_notable() {
        let mut summary = AuditDiffSummary::default();
        let mut total = 0usize;
        let result = cdc_result("unpackerr", ResultVerdict::Finding, &["debug", "upload_dir"]);
        record_bucket(
            &mut summary,
            &mut total,
            "removed_subject_with_finding",
            "cdc",
            "unpackerr".to_string(),
            &result,
        );
        assert_eq!(summary.removed_subjects_with_finding, 1);
        assert_eq!(total, 1);
        assert_eq!(summary.notable[0].bucket, "removed_subject_with_finding");
    }

    // =======================================================================
    // S1-F1: the real end-to-end pipeline, against real fixtures on disk
    // (fixtures/synthetic/audit-diff-module-lifecycle/), reproducing
    // exactly the shape S1 found hard-failing as TOOL_ERROR on 2/30 real
    // nixpkgs PRs.
    // =======================================================================

    #[test]
    fn run_audit_diff_module_birth_is_a_real_added_subject_not_a_tool_error() {
        let code = run_audit_diff(
            Path::new("fixtures/synthetic/audit-diff-module-lifecycle/before-empty"),
            Path::new("fixtures/synthetic/audit-diff-module-lifecycle/after-with-module"),
            Path::new("fixtures/synthetic/audit-diff-module-lifecycle/targets.toml"),
            AuditFormat::Json,
            None,
        );
        assert!(code.is_ok(), "expected a real comparison, got {code:?}");
        assert_eq!(code.unwrap(), 0);
    }

    #[test]
    fn run_audit_diff_module_death_is_a_real_removed_subject_with_finding() {
        let code = run_audit_diff(
            Path::new("fixtures/synthetic/audit-diff-module-lifecycle/after-with-module"),
            Path::new("fixtures/synthetic/audit-diff-module-lifecycle/before-empty"),
            Path::new("fixtures/synthetic/audit-diff-module-lifecycle/targets.toml"),
            AuditFormat::Json,
            None,
        );
        assert!(code.is_ok(), "expected a real comparison, got {code:?}");
        assert_eq!(code.unwrap(), 0);
    }

    #[test]
    fn run_audit_diff_module_missing_on_both_sides_is_a_real_tool_error() {
        let err = run_audit_diff(
            Path::new("fixtures/synthetic/audit-diff-module-lifecycle/before-empty"),
            Path::new("fixtures/synthetic/audit-diff-module-lifecycle/before-empty"),
            Path::new("fixtures/synthetic/audit-diff-module-lifecycle/targets.toml"),
            AuditFormat::Json,
            None,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("present under NEITHER"));
    }
}
