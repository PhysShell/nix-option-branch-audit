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
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
struct Cli {
    /// Path to a TOML target manifest (see targets/*.toml). Required
    /// unless --census is given.
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
    /// both were given -- clap now rejects the combination outright).
    #[arg(long, conflicts_with = "targets")]
    census: Option<PathBuf>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct TargetFile {
    target: Vec<Target>,
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
    if m.target.is_empty() {
        return Err("manifest has no [[target]] entries".to_string());
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

#[derive(Serialize, Debug, Clone)]
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

#[derive(Serialize, Debug, Clone)]
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
fn scan_options(
    file: &str,
    src: &str,
    root: &SyntaxNode,
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
        if value.kind() != NODE_ATTR_SET {
            continue;
        }

        if segs == ["options"] {
            walk_options_block(file, src, &value, &mut Vec::new(), &mut out);
            continue;
        }

        if prefix_is_concrete
            && !option_prefix.is_empty()
            && segs.len() == option_prefix.len() + 1
            && segs[0] == "options"
            && segs[1..] == option_prefix[..]
        {
            walk_options_block(file, src, &value, &mut Vec::new(), &mut out);
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
) {
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

        if is_mk_option_call(&value) {
            let default_node = mk_option_field(&value, "default");
            let default_source = default_node.as_ref().map(|n| n.text().to_string());
            let default_class = default_node.as_ref().map(classify_value);
            out.push(OptionDecl {
                path: path.clone(),
                default_source,
                default_class,
                span: span_of(file, src, &entry),
            });
        } else if value.kind() == NODE_ATTR_SET {
            walk_options_block(file, src, &value, path, out);
        }

        for _ in 0..segs.len() {
            path.pop();
        }
    }
}

fn is_mk_option_call(node: &SyntaxNode) -> bool {
    if node.kind() != NODE_APPLY {
        return false;
    }
    let (head, _args) = flatten_apply(node);
    call_head_name(&head).as_deref() == Some("mkOption")
}

fn mk_option_field(apply_node: &SyntaxNode, field: &str) -> Option<SyntaxNode> {
    let (_head, args) = flatten_apply(apply_node);
    let arg = args.first()?;
    if arg.kind() != NODE_ATTR_SET {
        return None;
    }
    for entry in arg.children() {
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
// does. Every lowering function below returns `Option<_>`, not a guess:
// `None` means "this syntactic shape isn't representable in this pure
// IR", propagated by `?` rather than silently discarded -- the same
// fail-closed idiom `ValueClass`/`predicate_outcome` already use
// throughout H1.
// ---------------------------------------------------------------------

/// A dotted option path, relative to `cfg_ident` -- e.g. `cfg.database.
/// driver` lowers to `["database", "driver"]`. Same representation
/// `Predicate.path`/`OptionDecl.path` already use; aliased here for
/// readability in the IR types below.
type OptionPath = Vec<String>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Scalar {
    Null,
    Bool(bool),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ValueExpr {
    /// A `cfg_ident`-rooted select, e.g. `cfg.database.driver`.
    Ref(OptionPath),
    Literal(Scalar),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Pred {
    Eq(ValueExpr, ValueExpr),
    Not(Box<Pred>),
    And(Box<Pred>, Box<Pred>),
    Or(Box<Pred>, Box<Pred>),
}

/// Lowers a value-position expression into the pure IR: a `cfg_ident`-
/// rooted select becomes `Ref`; the literals `null`/`true`/`false`/a
/// plain string become `Literal`. `None` for anything else -- this
/// commit has no alias/scope resolution yet (that lands in the next
/// commit), so a bare identifier referring to some other binding (e.g.
/// `db` in `db.driver`) is honestly unresolved here, never assumed to be
/// absent or to evaluate to any particular thing.
fn lower_value_expr(node: &SyntaxNode, cfg_ident: &str) -> Option<ValueExpr> {
    if let Some((root_name, path)) = as_select(node) {
        return if root_name == cfg_ident {
            Some(ValueExpr::Ref(path))
        } else {
            None
        };
    }
    match node.kind() {
        NODE_IDENT => match ident_text(node).as_deref() {
            Some("null") => Some(ValueExpr::Literal(Scalar::Null)),
            Some("true") => Some(ValueExpr::Literal(Scalar::Bool(true))),
            Some("false") => Some(ValueExpr::Literal(Scalar::Bool(false))),
            _ => None,
        },
        NODE_STRING => string_text(node).map(|s| ValueExpr::Literal(Scalar::Str(s))),
        _ => None,
    }
}

/// Lowers a boolean-valued expression into the pure `Pred` IR: `a != b`
/// (`Not(Eq(a,b))`), `a == b` (`Eq(a,b)`), a bare `cfg_ident`-rooted
/// reference used directly as a condition (`Eq(ref, true)`), `!p`
/// (`Not(p)`), `a && b` / `a || b` (`And`/`Or`). Uses rnix's typed
/// `ast::BinOp`/`ast::UnaryOp` operator classification (see the reuse
/// survey above) instead of hand-matching tokens. `None` for anything
/// unrepresentable -- a bare alias identifier used directly as a
/// condition (unresolved until the next commit's alias resolution), a
/// helper call, a non-boolean-shaped expression, or an operator this IR
/// doesn't model (`<`, string ops, arithmetic, ...).
fn lower_pred(node: &SyntaxNode, cfg_ident: &str) -> Option<Pred> {
    let node = unwrap_paren(node.clone());
    if let Some(bin) = rnix::ast::BinOp::cast(node.clone()) {
        let op = bin.operator()?;
        let lhs_node = bin.lhs()?.syntax().clone();
        let rhs_node = bin.rhs()?.syntax().clone();
        return match op {
            rnix::ast::BinOpKind::And => Some(Pred::And(
                Box::new(lower_pred(&lhs_node, cfg_ident)?),
                Box::new(lower_pred(&rhs_node, cfg_ident)?),
            )),
            rnix::ast::BinOpKind::Or => Some(Pred::Or(
                Box::new(lower_pred(&lhs_node, cfg_ident)?),
                Box::new(lower_pred(&rhs_node, cfg_ident)?),
            )),
            rnix::ast::BinOpKind::Equal => Some(Pred::Eq(
                lower_value_expr(&lhs_node, cfg_ident)?,
                lower_value_expr(&rhs_node, cfg_ident)?,
            )),
            rnix::ast::BinOpKind::NotEqual => Some(Pred::Not(Box::new(Pred::Eq(
                lower_value_expr(&lhs_node, cfg_ident)?,
                lower_value_expr(&rhs_node, cfg_ident)?,
            )))),
            _ => None,
        };
    }
    if let Some(un) = rnix::ast::UnaryOp::cast(node.clone()) {
        return if un.operator()? == rnix::ast::UnaryOpKind::Invert {
            let inner = un.expr()?.syntax().clone();
            Some(Pred::Not(Box::new(lower_pred(&inner, cfg_ident)?)))
        } else {
            None
        };
    }
    // A bare reference used directly as a condition: `if cfg.foo then
    // ...` lowers to `cfg.foo == true`. Only a `Ref` counts here -- a
    // bare `Literal` (e.g. a stray `if true then ...`) isn't a real
    // option-branch predicate, so it's left unresolved rather than
    // fabricating trivial evidence.
    match lower_value_expr(&node, cfg_ident) {
        Some(v @ ValueExpr::Ref(_)) => Some(Pred::Eq(v, ValueExpr::Literal(Scalar::Bool(true)))),
        _ => None,
    }
}

/// Evaluates a `ValueExpr` against a concrete environment: `env` supplies
/// the currently-known `Scalar` for every option path this evaluation
/// cares about. `None` means "this environment doesn't (yet) say", not
/// "false" -- callers must not conflate an absent lookup with a negative
/// result.
fn eval_value_expr(
    v: &ValueExpr,
    env: &std::collections::HashMap<OptionPath, Scalar>,
) -> Option<Scalar> {
    match v {
        ValueExpr::Literal(s) => Some(s.clone()),
        ValueExpr::Ref(path) => env.get(path).cloned(),
    }
}

/// Evaluates a `Pred` against a concrete environment. `None` propagates
/// fail-closed through every combinator: `And`/`Or` do NOT short-circuit
/// on a known operand the way Nix's own `&&`/`||` would at the *value*
/// level, because at the *evidence* level an unresolved operand means
/// this pass genuinely doesn't know whether the real Nix evaluation would
/// have short-circuited past it or not -- treating `And(unknown, false)`
/// as `Some(false)` would be assuming the right operand was never forced,
/// which this static pass has no basis for claiming.
fn eval_pred(p: &Pred, env: &std::collections::HashMap<OptionPath, Scalar>) -> Option<bool> {
    match p {
        Pred::Eq(a, b) => {
            let a = eval_value_expr(a, env)?;
            let b = eval_value_expr(b, env)?;
            Some(a == b)
        }
        Pred::Not(inner) => eval_pred(inner, env).map(|b| !b),
        Pred::And(a, b) => {
            let a = eval_pred(a, env)?;
            let b = eval_pred(b, env)?;
            Some(a && b)
        }
        Pred::Or(a, b) => {
            let a = eval_pred(a, env)?;
            let b = eval_pred(b, env)?;
            Some(a || b)
        }
    }
}

// ---------------------------------------------------------------------
// Test-file leaf assignment discovery (test.nix): walks AttrSets,
// transparently unwrapping Lambda bodies (containers.X = { ... }: { ... };),
// accumulating full dotted paths, recording every non-attrset leaf.
// ---------------------------------------------------------------------

#[derive(Serialize, Debug, Clone)]
struct TestAssignment {
    path: Vec<String>,
    value_source: String,
    value_class: ValueClass,
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

#[derive(Serialize, Debug)]
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
        predicate: Predicate,
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
    TestValueUnresolved {
        option: String,
        predicate: Predicate,
        default_outcome: bool,
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
        predicate: Predicate,
        default_outcome: bool,
    },
    /// A predicate and a classifiable default were found, but no test
    /// assignment's value provably flips the predicate's outcome away from
    /// what the default produces.
    #[serde(rename = "OBA001")]
    Oba001 {
        option: String,
        predicate: Predicate,
        default_outcome: bool,
    },
    /// A predicate and a classifiable default were found, and at least one
    /// test assignment's value provably evaluates the predicate to the
    /// *opposite* outcome from the default -- i.e. the branch is proven to
    /// have been taken down a different path than it would with no
    /// configuration at all.
    #[serde(rename = "PASS")]
    Pass {
        option: String,
        predicate: Predicate,
        default_outcome: bool,
        evidence: Vec<TestAssignment>,
    },
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
}

#[derive(Serialize, Debug)]
struct TargetReport {
    name: String,
    /// Non-empty only when the module or test file failed to parse
    /// cleanly. Fail closed: a target with parse errors gets no per-watch
    /// verdicts at all (they'd be scanning a tree rnix patched together
    /// around damage, not the real one) -- the parse errors themselves
    /// count as inconclusive.
    parse_errors: Vec<String>,
    discovered_options: Vec<OptionDecl>,
    discovered_predicates: Vec<Predicate>,
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

fn run_target(t: &Target) -> anyhow::Result<TargetReport> {
    let module_src = fs::read_to_string(&t.module).map_err(|e| {
        anyhow::anyhow!(
            "target {}: reading module {}: {e}",
            t.name,
            t.module.display()
        )
    })?;
    let test_src = fs::read_to_string(&t.test).map_err(|e| {
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
            parse_errors,
            discovered_options: Vec::new(),
            discovered_predicates: Vec::new(),
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
        &t.option_prefix,
    );
    let predicates = scan_predicates(
        &module_file,
        &module_src,
        module_root.syntax(),
        &t.cfg_ident,
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

        // Gate 2: a direct branch predicate referencing it.
        let Some(pred) = predicates.iter().find(|p| p.path == watched_path) else {
            verdicts.push(Verdict::PredicateNotFound {
                option: watched.clone(),
            });
            continue;
        };

        // Gate 3: the default's outcome under this predicate must be
        // statically classifiable, or there's no baseline to prove a
        // transition away from. Classified from the actual default AST
        // node (ValueClass), not a textual comparison -- see H1.1: a
        // `!= null` default that's an `if` expression, an alias select, or
        // `lib.mkDefault null` has no text equal to "null" even when its
        // runtime value definitely is, so a text check would wrongly call
        // it "definitely non-null" instead of honestly Unknown.
        let Some(default_outcome) = decl
            .default_class
            .and_then(|c| predicate_outcome(&pred.kind, c))
        else {
            verdicts.push(Verdict::DefaultUnresolved {
                option: watched.clone(),
                predicate: pred.clone(),
            });
            continue;
        };

        // Gate 4: a structurally-bound test assignment whose value's
        // outcome under this predicate is the *opposite* of the default's.
        // Three-way split, not a binary filter: a matching assignment
        // whose own outcome is Unknown (e.g. `builtins.elem "x" [ "x" "y"
        // ]` -- statically opaque, but genuinely `true` at runtime) must
        // NOT be silently treated as "no evidence" just because it fails
        // an `== Some(opposite)` filter. That's the same fail-closed
        // principle this tool applies to the default (gate 3) not being
        // applied to the test side -- caught by the H1.1 review. A known
        // opposite-outcome assignment takes precedence over an unresolved
        // one if both are present: real evidence beats an unrelated
        // ambiguity elsewhere in the same test file.
        let matches: Vec<TestAssignment> = assignments
            .iter()
            .filter(|a| path_matches_prefix(&a.path, &t.option_prefix, &pred.path))
            .cloned()
            .collect();
        matched_assignments.extend(matches.clone());

        let mut opposite = Vec::new();
        let mut has_unresolved = false;
        for a in &matches {
            match predicate_outcome(&pred.kind, a.value_class) {
                Some(o) if o == !default_outcome => opposite.push(a.clone()),
                Some(_) => {} // known, but same outcome as the default -- no evidence, but not ambiguous either
                None => has_unresolved = true,
            }
        }

        // Does any part of the test config that could structurally contain
        // this option live in a region the walker couldn't see into
        // (imports, an alias, a function call)? Checked AFTER opposite
        // evidence, never before: an explicit, provable transition found
        // elsewhere in the file is a stronger claim than "some unrelated
        // opacity also exists" (c17). Checked before TestValueUnresolved:
        // "an entire region of config was invisible" is a more fundamental
        // gap than "one specific value we did see was ambiguous".
        let target_path_opaque = opacity
            .iter()
            .any(|o| path_is_prefix_of_target(&o.path, &t.option_prefix, &pred.path));

        if !opposite.is_empty() {
            verdicts.push(Verdict::Pass {
                option: watched.clone(),
                predicate: pred.clone(),
                default_outcome,
                evidence: opposite,
            });
        } else if target_path_opaque {
            verdicts.push(Verdict::TestConfigUnresolved {
                option: watched.clone(),
                predicate: pred.clone(),
                default_outcome,
            });
        } else if has_unresolved {
            verdicts.push(Verdict::TestValueUnresolved {
                option: watched.clone(),
                predicate: pred.clone(),
                default_outcome,
            });
        } else {
            verdicts.push(Verdict::Oba001 {
                option: watched.clone(),
                predicate: pred.clone(),
                default_outcome,
            });
        }
    }

    Ok(TargetReport {
        name: t.name.clone(),
        parse_errors,
        discovered_options: options,
        discovered_predicates: predicates,
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

#[derive(Serialize, Debug)]
struct Summary {
    /// "PASS" only if every target resolved cleanly with no findings.
    /// "FINDING" if at least one OBA001 and nothing inconclusive.
    /// "INCONCLUSIVE" takes precedence over FINDING: a run that couldn't
    /// fully evaluate some watched option has no business claiming to have
    /// swept the rest cleanly, regardless of what else it found.
    status: &'static str,
    findings: usize,
    inconclusive: usize,
}

#[derive(Serialize, Debug)]
struct FullReport {
    summary: Summary,
    targets: Vec<TargetReport>,
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
    if let Some(dir) = &cli.census {
        return run_census(dir, cli.json);
    }

    let targets_path = cli
        .targets
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("either --targets or --census is required"))?;

    let manifest_src = fs::read_to_string(targets_path)
        .map_err(|e| anyhow::anyhow!("reading targets manifest {}: {e}", targets_path.display()))?;
    let manifest: TargetFile = toml::from_str(&manifest_src)
        .map_err(|e| anyhow::anyhow!("parsing targets manifest {}: {e}", targets_path.display()))?;
    validate_manifest(&manifest).map_err(|e| anyhow::anyhow!("invalid targets manifest: {e}"))?;

    let mut reports = Vec::new();
    for t in &manifest.target {
        // run_target's own `?`s (missing module/test file, etc.) bubble up
        // here as a genuine tool error too -- a target naming a file that
        // doesn't exist is a manifest problem, not an OBA001 finding.
        reports.push(run_target(t)?);
    }

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

    if cli.json {
        let full = FullReport {
            summary: Summary {
                status,
                findings,
                inconclusive,
            },
            targets: reports,
        };
        println!("{}", serde_json::to_string_pretty(&full)?);
    } else {
        println!("=== summary: {status}  findings={findings}  inconclusive={inconclusive} ===");
        for r in &reports {
            print_human(r);
        }
    }

    Ok(exit_code)
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
                "  TEST_VALUE_UNRESOLVED  {option}  default_outcome={default_outcome}  (a matching test assignment's value isn't statically classifiable, and no other match is a known opposite outcome)"
            ),
            Verdict::Oba001 {
                option,
                predicate,
                default_outcome,
            } => println!(
                "  OBA001  {option}  default_outcome={default_outcome}  [{}:{}]  `{}`",
                predicate.span.line, predicate.span.col, predicate.source
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

#[cfg(test)]
mod tests {
    use super::*;

    fn path_eq(a: &[String], b: &[&str]) -> bool {
        a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x == y)
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
            Some(Pred::Not(Box::new(Pred::Eq(
                ValueExpr::Ref(vec!["foo".into()]),
                ValueExpr::Literal(Scalar::Null),
            ))))
        );
        assert_eq!(
            lower_pred(&parse_expr("cfg.foo == null"), "cfg"),
            Some(Pred::Eq(
                ValueExpr::Ref(vec!["foo".into()]),
                ValueExpr::Literal(Scalar::Null),
            ))
        );
        // A bare cfg-rooted reference used directly as a condition (`if
        // cfg.foo then ...`) lowers to `cfg.foo == true`, matching H1's
        // `Truthy` semantics.
        assert_eq!(
            lower_pred(&parse_expr("cfg.foo"), "cfg"),
            Some(Pred::Eq(
                ValueExpr::Ref(vec!["foo".into()]),
                ValueExpr::Literal(Scalar::Bool(true)),
            ))
        );
        assert_eq!(
            lower_pred(&parse_expr("!cfg.foo"), "cfg"),
            Some(Pred::Not(Box::new(Pred::Eq(
                ValueExpr::Ref(vec!["foo".into()]),
                ValueExpr::Literal(Scalar::Bool(true)),
            ))))
        );
    }

    #[test]
    fn lower_pred_handles_compound_forms_new_in_h2() {
        // The exact real-world davis.nix chain this commit exists to move
        // towards: `db.createLocally && db.driver == "mysql"` -- here with
        // `db` already replaced by `cfg` directly (no alias resolution
        // yet, that's the next commit), isolating just the `&&`/string-
        // literal-equality lowering.
        assert_eq!(
            lower_pred(
                &parse_expr(r#"cfg.createLocally && cfg.driver == "mysql""#),
                "cfg"
            ),
            Some(Pred::And(
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
            Some(Pred::Or(
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
    fn lower_pred_is_none_for_unsupported_shapes_not_silently_something_else() {
        // A helper call: not a shape this IR represents.
        assert_eq!(
            lower_pred(&parse_expr(r#"builtins.elem "x" [ "x" "y" ]"#), "cfg"),
            None
        );
        // A bare alias identifier used directly as a condition: genuinely
        // unresolved until the next commit's alias resolution exists --
        // must never be silently treated as `false`/absent.
        assert_eq!(lower_pred(&parse_expr("mysqlLocal"), "cfg"), None);
        // An operator this IR doesn't model.
        assert_eq!(lower_pred(&parse_expr("cfg.a < cfg.b"), "cfg"), None);
    }

    // --- H1/H2 compatibility: the new IR+evaluator must reproduce H1's
    // existing predicate_outcome() table for every unary case that table
    // actually covers. `ValueClass::DefinitelyNonNull` is represented in
    // the environment as a concrete placeholder scalar (`Scalar::Str`) --
    // any concrete non-null value proves a null-comparison's outcome,
    // which is exactly what `DefinitelyNonNull` meant in H1 too.
    // `ValueClass::Unknown` is represented as the path being *absent* from
    // the environment, matching `eval_value_expr`'s own "unresolved, not a
    // guess" semantics. One case is deliberately NOT covered here:
    // `Truthy`/`NegTruthy` combined with `DefinitelyNonNull` returned
    // `None` in H1's table, but H1 never actually reached that state
    // through any real predicate + default/test-value combination in the
    // golden suite -- a `Truthy` predicate's operand must be a real Nix
    // bool for evaluation to succeed at all, so "known non-null, exact
    // value withheld" was already a degenerate corner of the old model,
    // not a case this IR needs to preserve bit-for-bit.

    fn env_of(pairs: &[(&[&str], Scalar)]) -> std::collections::HashMap<OptionPath, Scalar> {
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
            eval_pred(&neq, &env_of(&[(&["foo"], Scalar::Null)])),
            predicate_outcome(&PredicateKind::NullNeq, ValueClass::Null),
        );
        assert_eq!(
            eval_pred(&neq, &env_of(&[(&["foo"], Scalar::Bool(true))])),
            predicate_outcome(&PredicateKind::NullNeq, ValueClass::Bool(true)),
        );
        assert_eq!(
            eval_pred(
                &neq,
                &env_of(&[(&["foo"], Scalar::Str("placeholder".into()))])
            ),
            predicate_outcome(&PredicateKind::NullNeq, ValueClass::DefinitelyNonNull),
        );
        assert_eq!(
            eval_pred(&neq, &env_of(&[])),
            predicate_outcome(&PredicateKind::NullNeq, ValueClass::Unknown),
        );

        // NullEq (cfg.foo == null) -- same environments, the other predicate.
        assert_eq!(
            eval_pred(&eq, &env_of(&[(&["foo"], Scalar::Null)])),
            predicate_outcome(&PredicateKind::NullEq, ValueClass::Null),
        );
        assert_eq!(
            eval_pred(&eq, &env_of(&[(&["foo"], Scalar::Bool(true))])),
            predicate_outcome(&PredicateKind::NullEq, ValueClass::Bool(true)),
        );
        assert_eq!(
            eval_pred(
                &eq,
                &env_of(&[(&["foo"], Scalar::Str("placeholder".into()))])
            ),
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
                eval_pred(&truthy, &env_of(&[(&["foo"], Scalar::Bool(b))])),
                predicate_outcome(&PredicateKind::Truthy("if".into()), ValueClass::Bool(b)),
            );
            assert_eq!(
                eval_pred(&neg_truthy, &env_of(&[(&["foo"], Scalar::Bool(b))])),
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

    proptest::proptest! {
        #[test]
        fn not_not_is_identity(p in arb_pred(), env in arb_value_expr()) {
            // `env` here just forces proptest to exercise a handful of
            // concrete single-path environments as a cheap source of
            // variety; `Not(Not(p)) == p` provably holds for *any*
            // environment (double negation of an Option<bool> via `.map`
            // is the identity), full, partial, or empty alike, so a full
            // arbitrary HashMap generator adds no extra coverage here.
            let env: std::collections::HashMap<OptionPath, Scalar> = match env {
                ValueExpr::Ref(path) => [(path, Scalar::Bool(true))].into_iter().collect(),
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

            let full_env: std::collections::HashMap<OptionPath, Scalar> = paths
                .iter()
                .cloned()
                .map(|path| (path, Scalar::Bool(true)))
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
}
