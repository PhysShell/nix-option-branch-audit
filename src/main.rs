//! oba — Option Branch Activation evidence for NixOS modules.
//!
//! Layer 1 only, per the explicit non-goals agreed on:
//!   - This answers "was this option's branch ever exercised with a
//!     non-default value by a test", NOT "does the branch's value reach a
//!     consumer with the right name" (that is CDC, a separate tool) and NOT
//!     "does the value actually change observed runtime behavior" (that is
//!     ROB — the pdo_mysql.default_socket confound from this session is
//!     exactly why OBA PASS must never be read as "works end-to-end").
//!   - No VM tests are run. No upstream consumer is consulted. The tool does
//!     not know what Doctrine is.
//!   - cfg-alias resolution (`foo = cfg.x; if foo != null then ...`) is out
//!     of scope for this MVP and reported as PREDICATE_NOT_FOUND rather than
//!     silently skipped, so a target with an aliased predicate (davis in
//!     this corpus) doesn't masquerade as a clean PASS.

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
    #[serde(default)]
    watch: Vec<String>,
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
            let default_source = mk_option_field(&value, "default").map(|n| n.text().to_string());
            out.push(OptionDecl {
                path: path.clone(),
                default_source,
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

/// The boolean outcome of evaluating `kind`'s predicate expression with the
/// literal `value_source` substituted in, when that's staticaly decidable
/// from the raw source text alone -- `None` means genuinely unknown, not
/// "assume it doesn't count" and not "assume it does". This is deliberately
/// narrow: only the literal tokens `null`/`true`/`false` are understood.
/// Anything else (an interpolated string, a let-bound alias, an arbitrary
/// expression) is honestly unresolvable rather than guessed at.
fn predicate_outcome(kind: &PredicateKind, value_source: &str) -> Option<bool> {
    let v = value_source.trim();
    match kind {
        PredicateKind::NullNeq => Some(v != "null"),
        PredicateKind::NullEq => Some(v == "null"),
        PredicateKind::Truthy(_) => match v {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        },
        PredicateKind::NegTruthy => match v {
            "true" => Some(false),
            "false" => Some(true),
            _ => None,
        },
    }
}

fn run_target(t: &Target) -> anyhow::Result<TargetReport> {
    let module_src = fs::read_to_string(&t.module)?;
    let test_src = fs::read_to_string(&t.test)?;
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
        // transition away from.
        let Some(default_outcome) = decl
            .default_source
            .as_deref()
            .and_then(|d| predicate_outcome(&pred.kind, d))
        else {
            verdicts.push(Verdict::DefaultUnresolved {
                option: watched.clone(),
                predicate: pred.clone(),
            });
            continue;
        };

        // Gate 4: a structurally-bound test assignment whose value's
        // outcome under this predicate is the *opposite* of the default's
        // -- not merely "not textually equal to the default", which is a
        // different (and, for non-null defaults, wrong) question. See the
        // H1 review: `default = "/run/default.sock"; cfg.foo != null` with
        // a test assignment of the exact same string used to read as
        // "non-default evidence" under a textual-equality check, even
        // though both default and test land in the *same* predicate
        // outcome (true) and no branch transition is proven at all.
        let matches: Vec<TestAssignment> = assignments
            .iter()
            .filter(|a| path_matches_prefix(&a.path, &t.option_prefix, &pred.path))
            .cloned()
            .collect();
        matched_assignments.extend(matches.clone());

        let transitions: Vec<TestAssignment> = matches
            .into_iter()
            .filter(|a| predicate_outcome(&pred.kind, &a.value_source) == Some(!default_outcome))
            .collect();

        if transitions.is_empty() {
            verdicts.push(Verdict::Oba001 {
                option: watched.clone(),
                predicate: pred.clone(),
                default_outcome,
            });
        } else {
            verdicts.push(Verdict::Pass {
                option: watched.clone(),
                predicate: pred.clone(),
                default_outcome,
                evidence: transitions,
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let manifest: TargetFile = toml::from_str(&fs::read_to_string(&cli.targets)?)?;

    let mut reports = Vec::new();
    for t in &manifest.target {
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

    std::process::exit(exit_code);
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
