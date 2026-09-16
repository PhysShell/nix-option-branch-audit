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
//! Every watched option's verdict comes from a hard 4-gate chain — see
//! `run_target` — where any gate failing short-circuits to an inconclusive
//! verdict rather than falling through to a guess: declaration found →
//! predicate found → default's ValueClass resolves to a predicate outcome
//! → a structurally-matching test assignment provably flips that outcome.
//! Exit code is 4-state: 0 clean / 1 FINDING / 2 INCONCLUSIVE (takes
//! precedence over FINDING) / 3 TOOL_ERROR (the analysis never ran at all
//! — bad manifest, missing files — distinct from INCONCLUSIVE, where it
//! ran but couldn't prove something).

use rnix::SyntaxKind::*;
use rnix::{SyntaxNode, SyntaxToken};
use rowan::ast::AstNode;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
struct Cli {
    /// Path to a TOML target manifest (see targets/*.toml).
    #[arg(long)]
    targets: PathBuf,
    /// Emit machine-readable JSON instead of the human report.
    #[arg(long)]
    json: bool,
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
        if t.cfg_ident.trim().is_empty() {
            return Err(format!("target {}: cfg_ident must not be empty", t.name));
        }
        if t.option_prefix.is_empty() {
            return Err(format!(
                "target {}: option_prefix must not be empty",
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
            if !seen_watch.insert(w.as_str()) {
                return Err(format!("target {}: duplicate watch entry {w}", t.name));
            }
        }
    }
    Ok(())
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

fn scan_options(file: &str, src: &str, root: &SyntaxNode) -> Vec<OptionDecl> {
    let mut out = Vec::new();
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
        // KNOWN GAP, deliberately not patched: nixpkgs modules use two
        // conventions for the options block -- nested (`options = { foo =
        // ...; };`, matched here) and flat-dotted (`options.services.davis =
        // { ... };`, e.g. davis.nix). The flat form isn't handled: naively
        // seeding the walk with the trailing segments ("services","davis")
        // would make discovered option paths inconsistent with how the
        // predicate scanner reports paths (always relative to cfg_ident,
        // which for davis is bound at exactly the "services.davis" level --
        // aligning the two requires knowing that `cfg = config.services.
        // davis` binding, i.e. real alias/scope resolution, which is the
        // same out-of-scope machinery as the mysqlLocal case). Reporting
        // nothing here is more honest than reporting paths that would
        // silently fail to correlate with predicates later.
        if segs != vec!["options".to_string()] {
            continue;
        }
        let Some(value) = children.next() else {
            continue;
        };
        if value.kind() != NODE_ATTR_SET {
            continue;
        }
        walk_options_block(file, src, &value, &mut Vec::new(), &mut out);
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

fn scan_test_assignments(file: &str, src: &str, root: &SyntaxNode) -> Vec<TestAssignment> {
    let mut out = Vec::new();
    // root is a Lambda `{ lib, ... }: { ... }` for every fixture in this corpus
    let body = unwrap_lambda_chain(root.clone());
    if body.kind() == NODE_ATTR_SET {
        walk_test_block(file, src, &body, &mut Vec::new(), None, &mut out);
    }
    out
}

fn unwrap_lambda_chain(node: SyntaxNode) -> SyntaxNode {
    if node.kind() == NODE_ROOT {
        if let Some(child) = node.children().next() {
            return unwrap_lambda_chain(child);
        }
    }
    if node.kind() == NODE_LAMBDA {
        if let Some(body) = node.children().last() {
            return unwrap_lambda_chain(body);
        }
    }
    node
}

fn walk_test_block(
    file: &str,
    src: &str,
    attrset: &SyntaxNode,
    path: &mut Vec<String>,
    instance: Option<String>,
    out: &mut Vec<TestAssignment>,
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

        let is_instance_binding =
            path.is_empty() && segs.len() == 2 && (segs[0] == "containers" || segs[0] == "nodes");

        if is_instance_binding {
            // `containers.<name>` / `nodes.<name>` are nixosTest scaffolding,
            // not part of the NixOS option namespace: everything nested
            // inside is a fresh per-node config, so the accumulated dotted
            // path resets to empty here rather than carrying "containers.X"
            // as a prefix (that prefix would never match any real option
            // path and silently drop every assignment -- caught by the
            // kimai-after golden case coming back OBA001 when it should
            // PASS, exactly the "detector died" failure mode to guard
            // against).
            let body = unwrap_lambda_chain(value.clone());
            if body.kind() == NODE_ATTR_SET {
                let mut fresh_path = Vec::new();
                walk_test_block(
                    file,
                    src,
                    &body,
                    &mut fresh_path,
                    Some(segs[1].clone()),
                    out,
                );
            }
            continue;
        }

        path.extend(segs.clone());
        let body = unwrap_lambda_chain(value.clone());

        if body.kind() == NODE_ATTR_SET {
            walk_test_block(file, src, &body, path, instance.clone(), out);
        } else {
            out.push(TestAssignment {
                path: path.clone(),
                value_source: body.text().to_string(),
                value_class: classify_value(&body),
                instance: instance.clone(),
                span: span_of(file, src, &entry),
            });
        }

        for _ in 0..segs.len() {
            path.pop();
        }
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
            verdicts: t
                .watch
                .iter()
                .map(|w| Verdict::OptionNotFound { option: w.clone() })
                .collect(),
        });
    }

    let module_root = module_parse.tree();
    let test_root = test_parse.tree();

    let options = scan_options(&module_file, &module_src, module_root.syntax());
    let predicates = scan_predicates(
        &module_file,
        &module_src,
        module_root.syntax(),
        &t.cfg_ident,
    );
    let assignments = scan_test_assignments(&test_file, &test_src, test_root.syntax());

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

        if !opposite.is_empty() {
            verdicts.push(Verdict::Pass {
                option: watched.clone(),
                predicate: pred.clone(),
                default_outcome,
                evidence: opposite,
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
        verdicts,
    })
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
    let cli = Cli::parse();
    match run(&cli) {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(e) => {
            eprintln!("TOOL_ERROR: {e:#}");
            std::process::exit(3);
        }
    }
}

fn run(cli: &Cli) -> anyhow::Result<i32> {
    let manifest_src = fs::read_to_string(&cli.targets)
        .map_err(|e| anyhow::anyhow!("reading targets manifest {}: {e}", cli.targets.display()))?;
    let manifest: TargetFile = toml::from_str(&manifest_src)
        .map_err(|e| anyhow::anyhow!("parsing targets manifest {}: {e}", cli.targets.display()))?;
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
