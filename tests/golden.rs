//! Golden + adversarial acceptance test for the Layer-1 OBA spike.
//!
//! Runs the actual compiled binary against the real commits from
//! PhysShell/nixpkgs branch fix/doctrine-unix-socket-param-name
//! (5530e24f2 = the historical bug, 37f81efa4 = the first fix commit) plus
//! four adversarial mutations, and asserts the exact verdict each one must
//! produce. This is the "does the analyzer die silently" backstop: every
//! assertion checks a *specific* verdict for a *specific* target, never
//! just "did it not crash".

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct GoldenRun {
    full: Value,
    exit_code: i32,
}

fn run_manifest(manifest: &str) -> GoldenRun {
    let output = Command::new(env!("CARGO_BIN_EXE_oba"))
        .current_dir(manifest_dir())
        .args(["--targets", manifest, "--json"])
        .output()
        .expect("failed to run oba binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let full: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "oba did not emit valid JSON: {e}\nstderr: {}\nstdout: {stdout}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    GoldenRun {
        full,
        exit_code: output
            .status
            .code()
            .expect("process exited via signal, not code"),
    }
}

/// For tool-error cases: stdout is empty (the error happens before any
/// printing) so there's nothing to parse as JSON -- just the exit code and
/// stderr matter.
fn run_manifest_raw(manifest: &str) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_oba"))
        .current_dir(manifest_dir())
        .args(["--targets", manifest, "--json"])
        .output()
        .expect("failed to run oba binary");
    (
        output
            .status
            .code()
            .expect("process exited via signal, not code"),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn run_golden() -> Vec<Value> {
    run_manifest("targets/golden.toml")
        .full
        .get("targets")
        .expect("top-level JSON must have a targets array")
        .as_array()
        .expect("targets is an array")
        .clone()
}

fn target<'a>(reports: &'a [Value], name: &str) -> &'a Value {
    reports
        .iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no report for target {name}"))
}

fn verdict_obj<'a>(report: &'a Value, option: &str) -> &'a Value {
    report["verdicts"]
        .as_array()
        .expect("verdicts array")
        .iter()
        .find(|v| v["option"] == option)
        .unwrap_or_else(|| panic!("no verdict for option {option} in {}", report["name"]))
}

fn verdict_kind(report: &Value, option: &str) -> String {
    verdict_obj(report, option)["verdict"]
        .as_str()
        .expect("verdict tag is a string")
        .to_string()
}

// --- Positive assertions: the scanner must actually have looked, not just
// returned nothing everywhere. A detector that silently stopped finding
// anything must fail loudly here, not read as "zero findings = all clean".

#[test]
fn positive_assertions_kimai_after_found_real_things() {
    let reports = run_golden();
    let r = target(&reports, "kimai-after");

    let options: Vec<String> = r["discovered_options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| {
            o["path"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap())
                .collect::<Vec<_>>()
                .join(".")
        })
        .collect();
    assert!(
        options.contains(&"database.socket".to_string()),
        "scanner must discover the database.socket option declaration; got {options:?}"
    );

    let predicates: Vec<String> = r["discovered_predicates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            p["path"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap())
                .collect::<Vec<_>>()
                .join(".")
        })
        .collect();
    assert!(
        predicates.contains(&"database.socket".to_string()),
        "scanner must discover the `cfg.database.socket != null` predicate; got {predicates:?}"
    );

    let assignments: Vec<String> = r["matched_test_assignments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            format!(
                "{}[{}]",
                a["path"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join("."),
                a["instance"].as_str().unwrap_or("?")
            )
        })
        .collect();
    assert!(
        assignments
            .iter()
            .any(|a| a.contains("database.socket") && a.contains("socketMachine")),
        "scanner must match socketMachine's database.socket assignment; got {assignments:?}"
    );
}

// --- A: reproduces the historical finding on the exact parent commit.

#[test]
fn golden_a_parent_commit_shows_oba001() {
    let reports = run_golden();
    let r = target(&reports, "kimai-before");
    assert_eq!(verdict_kind(r, "database.socket"), "OBA001");

    // General correctness check on a purely-diagnostic field that was
    // never asserted anywhere before this pass. NOTE: this does NOT by
    // itself kill the mutation-testing survivor on
    // `diagnostic_default_outcome`'s H1-side lookup (`predicates.iter()
    // .find(|p| p.path == watched_path)`, `==` flipped to `!=`) --
    // kimai's real module has several OTHER predicates for other
    // options, and the mutated lookup's `.and_then(...)` chain still
    // ends up `None` for most of them, falling through to the `.or_else`
    // H2 fallback, which independently re-derives the SAME correct
    // value for `database.socket` (H2 recognizes the identical `!= null`
    // predicate too) -- the fallback rescues the mutation exactly the
    // way H2 rescues H1's own gate-4 elsewhere in this file. See
    // `h2_case12_...` below for the fixture that actually isolates and
    // kills this specific lookup (H1 finds a classifiable predicate, H2
    // has ZERO candidates for the same option, so no fallback exists to
    // mask a broken H1 lookup).
    let v = verdict_obj(r, "database.socket");
    assert_eq!(
        v["default_outcome"],
        serde_json::json!(false),
        "diagnostic default_outcome must reflect this option's OWN default outcome, not an \
         unrelated predicate's"
    );
}

// --- B: clears on the exact fix commit, with real evidence attached.

#[test]
fn golden_b_fix_commit_shows_pass_with_evidence() {
    let reports = run_golden();
    let r = target(&reports, "kimai-after");
    assert_eq!(verdict_kind(r, "database.socket"), "PASS");

    let v = r["verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["option"] == "database.socket")
        .unwrap();
    let evidence = v["evidence"]
        .as_array()
        .expect("PASS verdict must carry evidence");
    assert!(!evidence.is_empty(), "PASS must not have empty evidence");
    assert_eq!(evidence[0]["instance"], "socketMachine");

    // Mutation-testing survivor: `run_target`'s H1-gate-4 attempt-
    // classification (`witnessed: if !opposite.is_empty() { Some(true) }
    // ...`) had its `!` deleted and survived, because no test checked
    // the H1-sourced (`PredicateRef::Unary`) entry in `predicate_attempts`
    // specifically -- only H2-sourced (`Resolved`) entries were ever
    // asserted on (see the mysqlLocal tests below). `database.socket`'s
    // real `!= null` predicate is independently found and evaluated by
    // BOTH H1 (`Unary`, distinguished by having a `kind` field, no `ir`)
    // and H2 (`Resolved`, has `ir`/`refs`) -- assert the H1 one
    // specifically reports `witnessed: true`, not just that SOME attempt
    // in the array does.
    let attempts = v["predicate_attempts"]
        .as_array()
        .expect("predicate_attempts array");
    let unary_attempt = attempts
        .iter()
        .find(|a| a["predicate"].get("kind").is_some())
        .expect("an H1 (Unary) predicate_attempts entry must exist for database.socket");
    assert_eq!(
        unary_attempt["witnessed"], true,
        "H1's own gate-4 opposite-outcome detection must report witnessed:true here"
    );
}

// --- davis: honest OptionNotFound, not a false PASS, on both commits.
// davis.nix uses the flat `options.services.davis = { ... };` attrpath
// form, which the (deliberately narrow) declaration scanner doesn't
// recognize -- so it fails at gate 1 (declaration) before ever reaching
// gate 2 (the `mysqlLocal` let-alias predicate, also out of MVP scope).
// Either gate failing first is fine; what matters is it's never PASS.

#[test]
fn davis_database_driver_passes_via_a_direct_predicate() {
    // H2 gate-1 fix (already landed) made database.driver findable at
    // all: `scan_options` resolves davis's flat-dotted
    // `options.services.davis = { ... };` root against option_prefix, so
    // gate 1 passes for both davis-before/after.
    //
    // The real davis.nix DATABASE_URL construction is an if/else-if
    // chain -- three *independent* NODE_IF_ELSE conditions, not one:
    // `if db.driver == "sqlite" then ... else if pgsqlLocal then ...
    // else if mysqlLocal then ...`. H2's multi-predicate gate 2/4
    // evaluates every predicate referencing database.driver, not just
    // the first one found in document order (an earlier version of this
    // gate did exactly that, and picked `db.driver == "sqlite"` --
    // reviewed as an accident of AST traversal order, not a real
    // semantic choice, and fixed). `machine1` in BOTH before and after
    // sets `database.driver = "postgresql"` (default is `"sqlite"`),
    // which is real, legitimate activation evidence for the simple
    // `db.driver == "sqlite"` predicate regardless of whether any mysql
    // scenario exists at all -- so both targets correctly PASS through
    // it. This is NOT the mysqlLocal/compound-alias proof (see the
    // dedicated test below for that) -- it's a different, equally real
    // predicate on the same watched option, and the point of this test
    // is specifically that H2 doesn't hide it.
    let reports = run_golden();
    for name in ["davis-before", "davis-after"] {
        let r = target(&reports, name);
        assert_eq!(
            verdict_kind(r, "database.driver"),
            "PASS",
            "target {name}: machine1's database.driver = \"postgresql\" is real evidence \
             against the db.driver == \"sqlite\" predicate regardless of any mysql scenario"
        );
    }
}

// The actual H2 acceptance case: davis's real `mysqlLocal = db.createLocally
// && db.driver == "mysql";` alias -- a compound expression over TWO
// options, reached only through `db -> cfg.database` select-alias
// resolution AND whole-expression alias resolution for the bare
// `mysqlLocal` identifier used as a condition. `predicate_attempts` (added
// specifically so a PASS via one predicate never hides what happened with
// others on the same option) makes this directly assertable: davis-after
// must show a *witnessed* attempt whose lowered IR is the mysqlLocal
// compound; davis-before -- same module, same predicate, only the test
// file differs -- must show that exact same predicate present but NOT
// witnessed, a real negative control proving the distinction isn't
// vacuous.

fn mysql_local_attempt(verdict: &Value) -> &Value {
    verdict["predicate_attempts"]
        .as_array()
        .expect("predicate_attempts array")
        .iter()
        .find(|a| {
            a["predicate"]["source"]
                .as_str()
                .map(|s| s.contains("mysqlLocal"))
                .unwrap_or(false)
        })
        .expect("a predicate_attempts entry for the mysqlLocal-sourced condition must exist")
}

#[test]
fn davis_after_witnesses_the_mysql_local_compound_alias() {
    let reports = run_golden();
    let r = target(&reports, "davis-after");
    let verdict = verdict_obj(r, "database.driver");
    let attempt = mysql_local_attempt(verdict);
    assert_eq!(
        attempt["witnessed"], true,
        "machine3 (driver=\"mysql\", createLocally defaults to true) must witness a real \
         false->true transition through the resolved mysqlLocal compound predicate; got {attempt:?}"
    );
    // And the IR itself, not just the fact that *a* condition happened to
    // be labeled "mysqlLocal" -- proves alias resolution actually
    // reconstructed the real compound expression, not something else
    // that merely mentions the name.
    let ir = &attempt["predicate"]["ir"];
    assert!(
        ir.get("And").is_some(),
        "the resolved mysqlLocal predicate's IR must be an And(...) compound; got {ir:?}"
    );
}

#[test]
fn davis_before_does_not_witness_the_mysql_local_compound_alias() {
    // Negative control: same module (mysqlLocal's definition is byte-for-
    // byte identical between before/after -- only the Doctrine DSN
    // parameter name differs elsewhere), but before/test.nix has no mysql
    // scenario at all. If this ever flipped to witnessed=true, the
    // counterfactual evaluator would be proving something no real test
    // assignment actually demonstrates.
    let reports = run_golden();
    let r = target(&reports, "davis-before");
    let verdict = verdict_obj(r, "database.driver");
    let attempt = mysql_local_attempt(verdict);
    assert_eq!(
        attempt["witnessed"], false,
        "davis-before has no mysql test scenario -- mysqlLocal must show real-but-unwitnessed \
         evidence (machine1/machine2 are both postgresql, so createLocally=true but \
         driver!=\"mysql\" every time), not a witness; got {attempt:?}"
    );
}

// --- H2 counterfactual gate adversarial cases 2-5, isolating davis's real
// mysqlLocal shape (a compound predicate over two options) from any
// real-corpus noise, so each specific failure mode has a fixture that
// exercises nothing else. Case 1 is davis itself, above.

#[test]
fn h2_case2_absorbed_watched_change_is_not_pass() {
    // createLocally=false, driver: sqlite(default) -> mysql(test). The
    // compound predicate is false both times (createLocally=false alone
    // already pins it) -- the watched option's own change is absorbed by
    // the other operand. Must be OBA001, not PASS: presence of
    // `driver = "mysql"` alone is never sufficient evidence.
    let reports = run_golden();
    let r = target(&reports, "h2-case2-absorbed-watched-change");
    assert_eq!(verdict_kind(r, "database.driver"), "OBA001");

    // General correctness check, same "never asserted before" gap as
    // golden_a above. NOTE: this does NOT by itself kill the mutation-
    // testing survivor on `diagnostic_default_outcome`'s H2-side fallback
    // lookup (`resolved_predicates.iter().find(|p| p.refs.iter().any(|r|
    // r == &watched_path))`, `==` flipped to `!=`) -- `p`'s `refs` here
    // is TWO entries (`createLocally` and `driver`, the compound
    // predicate's both operands), so `.any(|r| r != watched_path)` is
    // STILL true (via the OTHER ref) even under the mutation, and
    // `.find()` locates the exact same predicate either way. See
    // `h2_case13_...` below for the fixture that actually isolates and
    // kills this lookup (a SINGLE-ref H2-only predicate, where a broken
    // `!=` filter finds nothing at all).
    let v = verdict_obj(r, "database.driver");
    assert_eq!(
        v["default_outcome"],
        serde_json::json!(false),
        "diagnostic default_outcome must come from the actual H2 candidate referencing this \
         option, not an unrelated lookup mismatch"
    );
}

#[test]
fn h2_case3_symmetric_watch_is_pass() {
    // Same instance as case 2, watching the OTHER operand instead:
    // createLocally true(default) -> false(test), with driver=mysql held
    // fixed at this instance's own value throughout. true -> false is a
    // real transition -- proves the system attributes causation to
    // whichever option is actually watched, not just "the compound
    // predicate was true somewhere".
    let reports = run_golden();
    let r = target(&reports, "h2-case3-symmetric-watch");
    assert_eq!(verdict_kind(r, "database.createLocally"), "PASS");
}

#[test]
fn h2_case4_per_instance_isolation_prevents_cross_contamination() {
    // nodeA (driver=mysql, createLocally=false) and nodeB
    // (createLocally=true) are two independent machines. If nodeB's
    // createLocally=true ever leaked into nodeA's evaluation, this would
    // wrongly PASS (false&&false=false -> true&&true=true). Only nodeA
    // explicitly assigns database.driver, so only nodeA is considered,
    // and nodeA's OWN createLocally=false must be what's used -- giving
    // OBA001, the same absorbed-change shape as case 2.
    let reports = run_golden();
    let r = target(&reports, "h2-case4-per-instance-isolation");
    assert_eq!(
        verdict_kind(r, "database.driver"),
        "OBA001",
        "nodeB's createLocally=true must never leak into nodeA's own counterfactual \
         environment -- if it had, this would wrongly be PASS"
    );
}

#[test]
fn h2_case5_unknown_context_is_inconclusive_never_a_guess() {
    // driver=mysql is perfectly well known, but the predicate's OTHER
    // operand (flag) is unresolvable everywhere (never assigned, default
    // is a non-literal expression). Must weaken to inconclusive, never
    // silently become OBA001 (a false claim of "no evidence") or PASS (a
    // fabricated transition this tool has no basis for).
    let reports = run_golden();
    let r = target(&reports, "h2-case5-unknown-context");
    assert_eq!(
        verdict_kind(r, "database.driver"),
        "TestValueUnresolved",
        "an unresolvable co-operand must weaken the verdict, never be silently ignored"
    );
}

#[test]
fn h2_case6_explicit_opaque_co_operand_is_never_treated_as_its_own_default() {
    // H2.2 Finding 1: `createLocally` is explicitly assigned at this
    // instance, but to a non-literal (opaque) expression -- not left
    // unassigned. An earlier version of `evaluate_predicate_witness`
    // collapsed "explicitly assigned but unclassifiable" and "never
    // assigned at all" into the same lookup miss, silently falling back
    // to createLocally's OWN declared default (`true`) -- which would
    // fabricate `p = true && driver == "mysql"` and let driver's
    // sqlite->mysql transition wrongly PASS. The fix requires this to
    // weaken to TestValueUnresolved instead.
    let reports = run_golden();
    let r = target(&reports, "h2-case6-opaque-other-operand");
    assert_eq!(
        verdict_kind(r, "database.driver"),
        "TestValueUnresolved",
        "an explicit-but-opaque co-operand assignment must never be silently substituted \
         with that option's own declared default"
    );
}

#[test]
fn h2_case7_relevant_unresolved_site_blocks_a_false_oba001() {
    // H2.2 Finding 3: `watched`'s own predicate never flips in test.nix
    // (held at its own default) -- with no other consideration, this
    // resolves cleanly to OBA001. But the module also has a SEPARATE,
    // unrelated-looking `mkIf (cfg.items != [ ]) true` guard that is
    // both cfg-rooted and statically unlowerable (a list literal isn't a
    // supported ValueExpr). An earlier version of `scan_resolved_predicates`
    // silently dropped whatever `lower_pred` couldn't handle -- the same
    // "not found reads as genuinely absent" shape H1's own walker closed.
    // This must weaken to TestValueUnresolved, proving the
    // `unresolved_predicate_sites` gate actually engages for a real
    // cfg-mentioning site, not just exist as dead plumbing.
    let reports = run_golden();
    let r = target(&reports, "h2-case7-unresolved-relevant-site");
    assert_eq!(
        verdict_kind(r, "watched"),
        "TestValueUnresolved",
        "an unrelated but cfg-mentioning unresolvable predicate site must still weaken \
         the verdict, not be silently dropped into a false OBA001"
    );
}

#[test]
fn h2_case9_h2_candidate_rescues_a_false_default_unresolved() {
    // Hostile-review finding on H2.2 itself (invariant 2, post-commit):
    // H1's own gate 3 used to `continue` immediately whenever it
    // couldn't classify its OWN predicate's default outcome
    // (`DefaultUnresolved`), skipping the H2 loop entirely for that
    // option -- even when a SEPARATE H2 candidate on the exact same
    // option is fully resolvable. `flag`'s default is a plain string
    // ("sqlite"): H1's coarser ValueClass classifies it as
    // DefinitelyNonNull, which a `Truthy` predicate (H1's own, found
    // here) can't turn into a boolean outcome -- but H2's finer
    // KnownValue classifies the identical node as
    // Exact(Str("sqlite")), which the module's SEPARATE `flag ==
    // "mysql"` predicate resolves cleanly. Must be PASS, not a false
    // DefaultUnresolved that silently hides a real, evaluable H2
    // witness. The pre-existing `h1_unresolvable_default_is_inconclusive_not_guessed`
    // (target c7-unresolved-default) is the negative control: no H2
    // candidate exists there, so DefaultUnresolved is still exactly
    // correct and must remain unchanged.
    let reports = run_golden();
    let r = target(&reports, "h2-case9-default-unresolved-rescue");
    assert_eq!(
        verdict_kind(r, "flag"),
        "PASS",
        "an H1 predicate whose default can't classify must not block a separate, \
         fully H2-resolvable predicate on the same option from witnessing a real transition"
    );
}

#[test]
fn h2_case10_alias_hidden_two_hop_relevance_blocks_a_false_oba001() {
    // Mutation-testing survivor on `collect_reachable_refs`'s own
    // alias-following cycle-guard (`!chain.iter().any(|n| n == &name)`
    // mutated to use `!=`): the single-hop `h2-alias-hidden-unresolved`
    // fixture doesn't exercise this, because its own alias-following
    // step always starts with an EMPTY chain, where a broken guard
    // happens to coincide with the correct (permissive) answer. Two
    // hops (`hidden` -> `mid` -> `someUnsupportedHelper
    // cfg.database.driver`) is the minimum depth where the mutated
    // guard actually diverges: chain is non-empty by the second hop,
    // and the buggy version wrongly blocks resolving `mid` (any
    // *different* name already in the chain wrongly reads as "still
    // resolving," not "not yet visited"). Must be TestValueUnresolved,
    // not a false OBA001 from silently losing the reachable
    // `database.driver` reference two hops down.
    let reports = run_golden();
    let r = target(&reports, "h2-case10-alias-hidden-two-hop");
    assert_eq!(
        verdict_kind(r, "database.driver"),
        "TestValueUnresolved",
        "a cfg reference reachable only through a 2-hop alias chain must still be found by \
         collect_reachable_refs, not lost at the second hop"
    );
}

#[test]
fn h2_case11_h1_own_evidence_detection_is_not_dead_code() {
    // Mutation-testing survivor: `run_target`'s H1-gate-4 opposite-outcome
    // match guard (`o == !default_outcome`) mutated to a constant
    // `false` survived against the ENTIRE existing golden suite --
    // because every existing PASS-via-H1 fixture uses a bare `cfg.foo`
    // select, which H2's OWN scanner independently rediscovers and
    // re-evaluates as its own `ResolvedPredicate` (`Eq(Ref(foo), true)`)
    // -- so H2's redundant witness silently masked H1's own detection
    // logic being completely broken. This fixture isolates H1's gate 4:
    // the only predicate for `flag` is visible through a LOCALLY
    // shadowed `cfg` binding that H2's scope-aware `resolve_cfg_root`
    // correctly excludes from `resolved_predicates` (spelling matches,
    // scope doesn't) -- so H2 provides NO redundant coverage here at
    // all. If H1's own opposite-outcome detection were silently broken,
    // this target has nothing else to fall back on and would wrongly
    // report OBA001 instead of PASS.
    let reports = run_golden();
    let r = target(&reports, "h2-case11-scope-shadowed-h1-only");
    assert_eq!(
        verdict_kind(r, "flag"),
        "PASS",
        "H1's own gate-4 opposite-outcome detection must still work when no H2 candidate \
         exists to redundantly confirm the same transition"
    );
}

#[test]
fn h2_case12_diagnostic_default_outcome_h1_only_lookup() {
    // Mutation-testing survivor: `diagnostic_default_outcome`'s H1-side
    // lookup (`predicates.iter().find(|p| p.path == watched_path)`,
    // `==` flipped to `!=`) survived against `golden_a`'s kimai fixture
    // because kimai's OTHER predicates gave the mutated lookup something
    // to (wrongly) match, whose failure then fell through to an H2
    // `.or_else` fallback that independently re-derived the correct
    // answer anyway. This target reuses `h2-case11`'s module (the ONLY
    // predicate for `flag` is scope-shadowed away from H2 entirely, so
    // `resolved_predicates` has ZERO candidates for `flag` -- no
    // fallback exists here at all) but with `flag` held at its own
    // default (no transition) instead of case11's flipped value, so the
    // verdict is OBA001, not PASS, and we actually reach the diagnostic
    // computation. `flag`'s default is `false`
    // (`Truthy(Bool(false))=Some(false)`) -- computed PURELY through the
    // H1 lookup this mutant targets.
    let reports = run_golden();
    let r = target(&reports, "h2-case12-scope-shadowed-h1-only-no-transition");
    assert_eq!(verdict_kind(r, "flag"), "OBA001");
    let v = verdict_obj(r, "flag");
    assert_eq!(
        v["default_outcome"],
        serde_json::json!(false),
        "with zero H2 candidates for this option, diagnostic_default_outcome must still come \
         from H1's own (correctly-matched) predicate lookup"
    );
}

#[test]
fn h2_case13_diagnostic_default_outcome_h2_only_single_ref_lookup() {
    // Mutation-testing survivor: `diagnostic_default_outcome`'s H2-side
    // fallback lookup (`resolved_predicates.iter().find(|p| p.refs
    // .iter().any(|r| r == &watched_path))`, `==` flipped to `!=`)
    // survived against `h2_case2`'s compound (two-ref) predicate,
    // because a mutated `!=` filter still matches via the predicate's
    // OTHER ref. `x == true` is a SINGLE-ref predicate H1 doesn't
    // recognize at all (a general `==` comparison, not a null-check or
    // bare select) -- H1 finds nothing, and the mutated H2 lookup's
    // `.any(|r| r != watched_path)` over a ONE-element `refs` list is
    // unconditionally false, so `.find()` locates nothing and the
    // diagnostic value would silently become `None` under the mutation.
    // `x`'s default is `false`, held at default (no transition) -> OBA001,
    // `Eq(Ref(x), Literal(Bool(true)))` at `x=false` is `false`.
    let reports = run_golden();
    let r = target(&reports, "h2-case13-single-ref-h2-only");
    assert_eq!(verdict_kind(r, "x"), "OBA001");
    let v = verdict_obj(r, "x");
    assert_eq!(
        v["default_outcome"],
        serde_json::json!(false),
        "a single-ref H2-only predicate must still populate diagnostic_default_outcome \
         correctly, not silently become None"
    );
}

#[test]
fn h2_case8_alias_hidden_unresolved_relevance_blocks_a_false_oba001() {
    // H2.2 Finding 3, round 2: `direct`'s own predicate (driver != null)
    // never transitions (true both at default "sqlite" and test "mysql"),
    // so on its own this looks like honest evidence-no-transition. But a
    // SECOND branch, `hidden = someUnsupportedHelper cfg.database.driver;`,
    // used bare as a condition site (`hidden`, whose own syntax never
    // mentions `cfg` at all), genuinely depends on database.driver
    // through its alias binding and fails to lower. The first version of
    // Finding 3's fix (a purely syntactic "does the condition's own text
    // contain `cfg_ident`" check) would have missed this entirely --
    // `hidden`'s condition site literally never mentions "cfg" in its
    // own text, only in what it resolves to. `collect_reachable_refs`
    // must find `database.driver` reachable through the alias anyway.
    // Must be TestValueUnresolved, never a false OBA001.
    let reports = run_golden();
    let r = target(&reports, "h2-case8-alias-hidden-unresolved-relevance");
    assert_eq!(
        verdict_kind(r, "database.driver"),
        "TestValueUnresolved",
        "an unresolved predicate site reachable ONLY through alias resolution must still \
         block a false OBA001 for the option it actually references"
    );
}

// Positive assertion for the H2 gate-1 fix itself: without this, the test
// above would trivially pass for the wrong reason if `scan_options`
// regressed back to finding nothing at all (also `PredicateNotFound`... no,
// actually `OptionNotFound` -- but a positive assertion on the actual
// discovered path is still the only way to prove gate 1 specifically
// succeeded, not just that *some* inconclusive verdict came out).

#[test]
fn h2_gate1_davis_flat_option_root_is_discovered() {
    let reports = run_golden();
    for name in ["davis-before", "davis-after"] {
        let r = target(&reports, name);
        let options: Vec<String> = r["discovered_options"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| {
                o["path"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join(".")
            })
            .collect();
        assert!(
            options.contains(&"database.driver".to_string()),
            "target {name}: scan_options must discover database.driver via the flat \
             `options.services.davis = {{ ... }};` root now that it's resolved against \
             option_prefix; got {options:?}"
        );
    }
}

// --- C: survives adversarial mutations against the fixed module. Every one
// of these must stay OBA001 -- if any flips to PASS, the matcher is doing
// something dumber than it looks (naive grep-shaped false positive).

#[test]
fn golden_c_mutations_all_stay_oba001() {
    let reports = run_golden();
    for name in [
        "c2-remove-scenario",
        "c3-explicit-null",
        "c4-wrong-prefix",
        "c5-wrong-suffix",
    ] {
        let r = target(&reports, name);
        assert_eq!(
            verdict_kind(r, "database.socket"),
            "OBA001",
            "mutation {name} must not produce a false PASS"
        );
    }
}

// --- H1 review fixes ---------------------------------------------------
//
// Three real correctness issues, found by review of the pre-H1 code, not
// by this test suite (it was green the whole time): PredicateNotFound
// exited 0, is_default_class() checked textual equality to the default
// instead of predicate *outcome* (silently correct for kimai's null
// default only, wrong the moment a default is non-null), and PASS was
// reachable without the declaration scanner having found anything. Every
// fix below is pinned to the specific fixture that demonstrates it, not
// just re-asserted against the existing corpus.

// H1-1: PredicateNotFound (and friends) must make the whole run exit
// non-zero, distinctly from a genuine OBA001 finding -- a detector that
// couldn't evaluate something must never look like "all clear" to CI.

#[test]
fn h1_exit_codes_distinguish_clean_finding_and_inconclusive() {
    let clean = run_manifest("targets/clean.toml");
    assert_eq!(clean.exit_code, 0);
    assert_eq!(clean.full["summary"]["finding"], 0);
    assert_eq!(clean.full["summary"]["inconclusive"], 0);

    let findings = run_manifest("targets/findings-only.toml");
    assert_eq!(findings.exit_code, 1);
    assert_eq!(findings.full["summary"]["finding"], 1);
    assert_eq!(findings.full["summary"]["inconclusive"], 0);

    // golden.toml mixes real OBA001 findings with davis's honest
    // OptionNotFound -- inconclusive must take precedence over finding in
    // the exit code, per the explicit ask: a run that couldn't fully
    // evaluate everything has no business reporting itself as merely
    // "found some bugs, otherwise clean".
    let mixed = run_manifest("targets/golden.toml");
    assert_eq!(mixed.exit_code, 2);
    assert!(mixed.full["summary"]["finding"].as_u64().unwrap() > 0);
    assert!(mixed.full["summary"]["inconclusive"].as_u64().unwrap() > 0);
}

// H1-1b: parse errors specifically must fail the whole target closed --
// no per-watch verdict gets computed off a tree rnix patched together
// around the damage.

#[test]
fn h1_parse_errors_fail_closed() {
    let run = run_manifest("targets/parse-error.toml");
    assert_eq!(run.exit_code, 2);
    assert!(run.full["summary"]["inconclusive"].as_u64().unwrap() > 0);
    let targets = run.full["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 1);
    let parse_errors = targets[0]["parse_errors"].as_array().unwrap();
    assert!(
        !parse_errors.is_empty(),
        "malformed module.nix must produce parse_errors"
    );
    // and no verdict should be a PASS or OBA001 computed off the broken tree
    for v in targets[0]["verdicts"].as_array().unwrap() {
        assert!(
            v["verdict"] != "PASS" && v["verdict"] != "OBA001",
            "a target with parse errors must never produce a scan-derived verdict"
        );
    }
}

// H1-2: the outcome-transition model, not textual equality to the
// default. c6a is the exact scenario from the review: a non-null default
// (`"/run/default.sock"`), a test assignment of the *same* string. The
// old code read this as "not equal to null => non-default evidence" and
// would have reported PASS despite proving no branch transition at all.
// c6b is the positive control in the same module: assigning `null`
// (genuinely the opposite predicate outcome) must still produce PASS, so
// the fix isn't just "never PASS again".

#[test]
fn h1_outcome_transition_not_textual_equality() {
    let reports = run_golden();

    let same_as_default = target(&reports, "c6a-same-as-nonnull-default");
    assert_eq!(
        verdict_kind(same_as_default, "foo"),
        "OBA001",
        "assigning the option's own non-null default must not count as activation evidence"
    );

    let real_transition = target(&reports, "c6b-transitions-from-nonnull-default");
    assert_eq!(
        verdict_kind(real_transition, "foo"),
        "PASS",
        "assigning null against a non-null default IS a genuine predicate-outcome transition"
    );
}

// H1-2b: a default whose outcome can't be statically classified (not a
// literal null/true/false) must report DefaultUnresolved, not silently
// treat "unknown" as either "counts" or "doesn't count".

#[test]
fn h1_unresolvable_default_is_inconclusive_not_guessed() {
    let reports = run_golden();
    let r = target(&reports, "c7-unresolved-default");
    assert_eq!(verdict_kind(r, "bar"), "DefaultUnresolved");

    // Mutation-testing survivor: the H2.2-fixup re-lookup of H1's own
    // predicate for the `DefaultUnresolved` verdict's embedded
    // `predicate` field (`predicates.iter().find(|p| p.path ==
    // watched_path)`) had its `==` flipped to `!=` and survived, because
    // only the verdict TAG was ever asserted -- with `!=`, `.find()`
    // would silently pick a DIFFERENT option's predicate out of this
    // module's several (`foo`/`baz`/`qux` all have their own), not
    // `bar`'s. Assert the embedded predicate is actually `bar`'s own.
    let v = verdict_obj(r, "bar");
    assert_eq!(
        v["predicate"]["path"],
        serde_json::json!(["bar"]),
        "DefaultUnresolved's embedded predicate must be the watched option's own, not an \
         unrelated one found by a mismatched lookup"
    );
}

// H1-3: PASS must be structurally unreachable without the declaration
// scanner having found the option first -- watching a path with no
// mkOption at all must report OptionNotFound, the new first gate.

#[test]
fn h1_declaration_is_a_mandatory_gate() {
    let reports = run_golden();
    let r = target(&reports, "c8-option-not-found");
    assert_eq!(verdict_kind(r, "doesNotExist"), "OptionNotFound");
}

// --- H1.1 review fixes ---------------------------------------------------
//
// Four more correctness edge cases, again found by review of green code,
// not by this suite failing: gate 4's outcome filter silently dropped
// unresolvable test values into OBA001 (the exact fail-closed principle
// H1 applied to the *default* side, broken on the *test-value* side);
// the null-predicate outcome function was still comparing raw text
// instead of classifying the actual AST node, so a non-literal default
// or test value could be wrongly declared "definitely non-null"; an
// empty `watch = []` silently produced a clean PASS; and a missing
// module/test file (or any other manifest/I/O failure) exited through
// Rust's default error code, indistinguishable from a genuine FINDING.

// H1.1-1: an unresolvable test value must not silently vanish from gate 4's
// evidence search. qux's default is a *known* `false` (isolating this from
// the default-side bug below); the test assigns `builtins.elem "x" [ "x"
// "y" ]`, which is `true` at runtime but syntactically opaque.

#[test]
fn h1_1_unresolved_test_value_is_inconclusive_not_oba001() {
    let reports = run_golden();
    let r = target(&reports, "c11-unresolved-test-value");
    assert_eq!(
        verdict_kind(r, "qux"),
        "TestValueUnresolved",
        "an unresolvable matching test value must never be silently treated as 'no evidence'"
    );
}

// H1.1-2: predicate_outcome() for null predicates must classify the
// default's actual AST node, not compare its raw text to the string
// "null". baz's default is `if builtins.pathExists /etc/synth-baz then
// null else "/run/other.sock"` -- its text is never literally "null", but
// its ValueClass must still be Unknown (not "definitely non-null" from a
// naive text mismatch), so gate 3 must not silently resolve it.

#[test]
fn h1_1_null_predicate_default_is_ast_classified_not_textual() {
    let reports = run_golden();
    let r = target(&reports, "c10-null-expression-default");
    assert_eq!(
        verdict_kind(r, "baz"),
        "DefaultUnresolved",
        "a non-literal default under a null predicate must be Unknown, not guessed non-null from text"
    );
}

// H1.1-3: a target with watch = [] must never silently pass -- it checks
// nothing and would otherwise exit 0 with zero findings and zero
// inconclusive, the same "detector died, green light stayed on" failure
// mode `watch` itself exists to prevent, one level up the stack.

#[test]
fn h1_1_empty_watch_is_tool_error() {
    let (exit_code, stderr) = run_manifest_raw("targets/tool-error-empty-watch.toml");
    assert_eq!(
        exit_code, 3,
        "an empty watch list must be a tool/config error (3), never a passing analysis"
    );
    assert!(
        stderr.contains("watch"),
        "error message should name the actual problem; got: {stderr}"
    );
}

// H1.1-4: a missing module/test file must be TOOL_ERROR (3), never
// FINDING (1) -- before this fix, any `?`-propagated I/O error fell
// through to Rust's default error exit code, which happened to collide
// with this tool's own definition of "genuine OBA001 finding".

#[test]
fn h1_1_missing_file_is_tool_error_not_finding() {
    let (exit_code, stderr) = run_manifest_raw("targets/tool-error-missing-file.toml");
    assert_eq!(
        exit_code, 3,
        "a missing module/test file must be TOOL_ERROR (3), never FINDING (1)"
    );
    assert!(
        stderr.contains("this-file-does-not-exist"),
        "error should name the actual missing file; got: {stderr}"
    );
}

// --- H1.2 review fixes ---------------------------------------------------
//
// One more architectural gap, plus a clap exit-code contract leak and a
// verify-upstream.sh bug (not covered by Rust tests, checked separately).

/// Runs the binary with arbitrary raw CLI args (not the usual
/// `--targets <manifest> --json`) -- for testing clap-level parse failures
/// themselves, before any manifest is even looked at.
fn run_raw_args(args: &[&str]) -> i32 {
    let output = Command::new(env!("CARGO_BIN_EXE_oba"))
        .current_dir(manifest_dir())
        .args(args)
        .output()
        .expect("failed to run oba binary");
    output
        .status
        .code()
        .expect("process exited via signal, not code")
}

// H1.2-P0: a watched option that's never mentioned anywhere the walker CAN
// see must be TestConfigUnresolved when the test config also contains
// something the walker can't see into (imports, an alias) -- not silently
// treated as "not activated" just because nothing matching was *found*.
// The old code would OBA001 these exactly as if the option had genuinely
// never been touched, which is a different claim: "not observed in the
// part of the config I can read" vs "not activated". A census run over
// real modules would otherwise produce OBA001 findings that are actually
// just "config came in through `imports`".

#[test]
fn h1_2_imports_makes_result_inconclusive_not_oba001() {
    let reports = run_golden();
    let r = target(&reports, "c15-imports");
    assert_eq!(
        verdict_kind(r, "foo"),
        "TestConfigUnresolved",
        "an `imports` in the test config must not let OBA001 claim the option was never touched"
    );
}

#[test]
fn h1_2_instance_alias_makes_result_inconclusive_not_oba001() {
    let reports = run_golden();
    let r = target(&reports, "c16-instance-alias");
    assert_eq!(
        verdict_kind(r, "foo"),
        "TestConfigUnresolved",
        "`nodes.machine = someAlias;` must not let OBA001 claim the option was never touched"
    );
}

// Positive control: an explicit, provable opposite-outcome assignment
// found elsewhere in the same test file must win over an unrelated
// opacity site -- existential evidence beats incompleteness that has
// nothing to do with where the evidence was actually found.

#[test]
fn h1_2_explicit_opposite_evidence_wins_over_unrelated_import() {
    let reports = run_golden();
    let r = target(&reports, "c17-opposite-wins-over-import");
    assert_eq!(
        verdict_kind(r, "foo"),
        "PASS",
        "a provable transition must not be downgraded to inconclusive just because \
         an unrelated node elsewhere in the file also has an import"
    );
}

// H1.2-P1: clap's own Error::exit() (invoked internally by the old
// Cli::parse()) bypasses this tool's 4-state exit-code contract entirely --
// a bad CLI invocation exited with *clap's* code (2 for a usage error),
// indistinguishable from this tool's own INCONCLUSIVE (also 2), even
// though no analysis ever ran. Fixed with try_parse() mapping onto
// TOOL_ERROR (3) explicitly.

// Positive assertion for the opacity detector itself: c17 has both a real
// assignment AND an unrelated import, so `test_config_opacity` must
// actually be non-empty -- a detector that silently stopped finding
// opacity sites would make h1_2_explicit_opposite_evidence_wins_over_
// unrelated_import pass for the wrong reason (no opacity to lose to,
// rather than opposite evidence correctly outranking it).

#[test]
fn positive_assertions_c17_opacity_detector_found_the_import() {
    let reports = run_golden();
    let r = target(&reports, "c17-opposite-wins-over-import");
    let opacity = r["test_config_opacity"].as_array().unwrap();
    assert!(
        !opacity.is_empty(),
        "the opacity detector must have found the `other` node's import"
    );
    assert!(
        opacity.iter().any(|o| o["instance"] == "other"),
        "opacity should be attributed to the `other` instance; got {opacity:?}"
    );
}

#[test]
fn h1_2_cli_parse_failure_is_tool_error_not_inconclusive() {
    assert_eq!(
        run_raw_args(&["--bogus-flag-that-does-not-exist"]),
        3,
        "an unrecognized CLI flag must be TOOL_ERROR (3), not clap's own default exit code"
    );
    assert_eq!(
        run_raw_args(&[]),
        3,
        "a missing required --targets argument must be TOOL_ERROR (3)"
    );
}

// --- H1.3 review fixes ---------------------------------------------------
//
// The sharpest gap across all three review rounds: OBA001 only ever
// proved absence of *observed* evidence, not absence of activation. Two
// completely ordinary nixosTest patterns -- `imports`, and a `let`-bound
// alias for a node's config -- made the real assignment structurally
// invisible to the walker, which silently found nothing and OBA001'd
// exactly as if the option had never been touched. Confirmed against real
// nixpkgs, not just plausible synthetic scenarios: 559 files under
// nixos/tests use the nested `nodes = {` form, 95 use the
// `import ./make-test-python.nix (...)` wrapper -- a census would have
// produced industrial quantities of false findings. The walker is now an
// explicit three-context state machine (TestSpecRoot / ModuleRoot /
// ConfigTree, see the comment above `scan_test_assignments`) instead of
// one function accumulating special cases.

// Both forms of nodes/containers binding (flat, used everywhere else in
// this corpus, and nested `nodes = { machine = ...; };`) must produce
// identical verdicts for identical content.

#[test]
fn h1_3_nested_nodes_form_finds_opposite_evidence() {
    let reports = run_golden();
    let r = target(&reports, "c18-nested-nodes-opposite");
    assert_eq!(verdict_kind(r, "foo"), "PASS");
}

#[test]
fn h1_3_nested_nodes_form_still_oba001s_correctly() {
    let reports = run_golden();
    let r = target(&reports, "c19-nested-nodes-same");
    assert_eq!(verdict_kind(r, "foo"), "OBA001");
}

// An entirely unrecognized root wrapper must never let content it can't
// see into masquerade as "nothing happened here" -- TestConfigUnresolved,
// not OBA001, regardless of what's actually inside the wrapper.

#[test]
fn h1_3_unknown_root_wrapper_is_inconclusive_not_oba001() {
    let reports = run_golden();
    let r = target(&reports, "c20-unknown-root");
    assert_eq!(verdict_kind(r, "foo"), "TestConfigUnresolved");
}

// The dominant real-world nixosTest wrapper (`import ./make-test-python.nix
// (<lambda-or-attrset>)`) must be recognized and unwrapped, not treated
// as just another unknown root.

#[test]
fn h1_3_make_test_python_wrapper_is_unwrapped() {
    let reports = run_golden();
    let r = target(&reports, "c21-make-test-wrapper");
    assert_eq!(
        verdict_kind(r, "foo"),
        "PASS",
        "the import ./make-test-python.nix (...) wrapper must be transparently unwrapped, not treated as opaque"
    );
}

// Module-root `config = { ... };` must normalize into the SAME option-path
// namespace as top-level shorthand -- `services.synth.foo`, not
// `config.services.synth.foo`, which would never match anything.

#[test]
fn h1_3_module_root_config_key_normalizes_path() {
    let reports = run_golden();
    let r = target(&reports, "c22-module-root-config");
    assert_eq!(
        verdict_kind(r, "foo"),
        "PASS",
        "config = {{ ... }}'s contents must normalize into the same option-path namespace as top-level shorthand"
    );
}

// But `config = <opaque expression>;` (a function call, not a literal
// attrset) must still be treated as hiding anything -- `config` is only
// special because it's module-root syntax, not because the key happens
// to be named "config" anywhere in the tree.

#[test]
fn h1_3_module_root_opaque_config_is_inconclusive() {
    let reports = run_golden();
    let r = target(&reports, "c23-module-root-opaque-config");
    assert_eq!(verdict_kind(r, "foo"), "TestConfigUnresolved");
}

// `inherit` and a dynamic `${...}` attribute name are both genuinely
// untraceable by this walker -- neither should be silently skipped as if
// nothing was there.

#[test]
fn h1_3_inherit_and_dynamic_attrpath_are_opacity_not_silence() {
    let reports = run_golden();
    let r = target(&reports, "c24-inherit-and-dynamic");
    assert_eq!(
        verdict_kind(r, "foo"),
        "TestConfigUnresolved",
        "inherit / a dynamic attrpath must register as opacity, never silently skipped"
    );
    let opacity = r["test_config_opacity"].as_array().unwrap();
    assert!(
        opacity.len() >= 2,
        "both the inherit and the dynamic attrpath should be recorded; got {opacity:?}"
    );
}

// --- H1.3a review fixes ---------------------------------------------------
//
// The H1.3 walker's TestSpecRoot context still had one architectural hole:
// it inferred "not nodes/containers" implied "harmless harness metadata,
// safe to ignore" -- true for `name`/`meta`/`testScript`, false for
// anything else. A real-shaped adversarial example: a normal, fully-visible
// `nodes.machine` next to a second scenario (`hiddenScenario = runTest {
// nodes.other = ...; };`) that the walker can't see into at all. Because
// SOME nodes/containers binding was found, the old whole-file fallback
// (`found_any_instance`) never fired either -- the hidden scenario's real
// opposite-outcome assignment vanished with zero trace anywhere in the
// report. `walk_test_spec_root` now classifies every top-level key against
// an explicit, source-verified allowlist of confirmed-safe metadata keys
// (KNOWN_HARNESS_METADATA_KEYS) and opacity's anything else.

#[test]
fn h1_3a_unclassified_root_sibling_makes_result_inconclusive_not_oba001() {
    let reports = run_golden();
    let r = target(&reports, "c25-partial-root-opacity");
    assert_eq!(
        verdict_kind(r, "foo"),
        "TestConfigUnresolved",
        "an unrecognized test-spec-root entry next to a normal nodes.machine must not be \
         silently treated as harmless metadata just because nodes.machine was also found"
    );
    let opacity = r["test_config_opacity"].as_array().unwrap();
    assert!(
        opacity.iter().any(|o| o["reason"]
            .as_str()
            .unwrap_or("")
            .starts_with("unrecognized test-spec-root entry")),
        "the hiddenScenario entry must be recorded as opacity, not silently discarded; got {opacity:?}"
    );
}

// clap's `conflicts_with` must reject --targets and --census together as a
// CLI usage error (TOOL_ERROR, 3) -- previously `run()` silently preferred
// --census and just ignored --targets, which is a config bug wearing the
// costume of "it did something".

#[test]
fn h1_3a_targets_and_census_together_is_tool_error() {
    assert_eq!(
        run_raw_args(&["--targets", "targets/golden.toml", "--census", "fixtures"]),
        3,
        "passing both --targets and --census must be a CLI usage error, not a silent choice"
    );
}

// The census's own new metric, exercised directly (not just inferred from
// run_target's verdict): censusing c25's own fixture directory (NOT all of
// fixtures/synthetic -- that also contains c9-parse-error's deliberately
// malformed module.nix, which would make parse_errors > 0 for an unrelated
// reason) must surface hiddenScenario as an unclassified root entry, and
// must exit 0 -- this narrower directory has no unreadable files and no
// parse errors.

fn run_census_json(dir: &str) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_oba"))
        .current_dir(manifest_dir())
        .args(["--census", dir, "--json"])
        .output()
        .expect("failed to run oba binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "census did not emit valid JSON: {e}\nstderr: {}\nstdout: {stdout}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (
        output
            .status
            .code()
            .expect("process exited via signal, not code"),
        report,
    )
}

#[test]
fn h1_3a_census_counts_unclassified_root_entries() {
    let (exit_code, report) = run_census_json("fixtures/synthetic/c25-partial-root-opacity");
    assert_eq!(
        exit_code, 0,
        "a directory with no unreadable files and no parse errors must census as exit 0"
    );
    let unclassified = report["unclassified_root_entries"]
        .as_u64()
        .expect("unclassified_root_entries must be present in the census JSON");
    assert!(
        unclassified >= 1,
        "c25's hiddenScenario entry must be counted as an unclassified root entry; got {unclassified}"
    );
}

// --- H1.3b review fixes ---------------------------------------------------
//
// A close relative of the H1.3a bug, one level down: the NESTED
// `nodes = { ... };` form's own inner loop only handled a plain
// single-segment instance name (`enter_instance` on `inst_segs.len() ==
// 1`) and a dynamic ("${...}") instance name -- anything else (a
// multi-segment attrpath like `hidden.services.synth.foo = null;`, or an
// `inherit`) fell through the loop with neither an assignment nor an
// opacity record, even with a normal, correctly-recognized sibling
// instance (`machine`) right next to it. Unlike the H1.3a root-level bug,
// this one is invisible to BOTH `files_with_neither` (a real assignment
// was found, from `machine`) AND `unclassified_root_entries` (the
// unrecognized entry is already inside a recognized `nodes = { ... };`
// value, not at the spec root) -- it needed its own census counter,
// `unclassified_instance_entries`.

#[test]
fn h1_3b_nested_instance_multisegment_makes_result_inconclusive_not_oba001() {
    let reports = run_golden();
    let r = target(&reports, "c26-nested-instance-multisegment");
    assert_eq!(
        verdict_kind(r, "foo"),
        "TestConfigUnresolved",
        "a multi-segment attrpath under nested nodes = {{ ... }} must not be silently \
         dropped just because a normal sibling instance was also found"
    );
}

#[test]
fn h1_3b_nested_instance_inherit_makes_result_inconclusive_not_oba001() {
    let reports = run_golden();
    let r = target(&reports, "c27-nested-instance-inherit");
    assert_eq!(
        verdict_kind(r, "foo"),
        "TestConfigUnresolved",
        "an inherit binding under nested nodes = {{ ... }} must not be silently dropped"
    );
}

#[test]
fn h1_3b_census_counts_unclassified_instance_entries() {
    let (exit_code, report) =
        run_census_json("fixtures/synthetic/c26-nested-instance-multisegment");
    assert_eq!(exit_code, 0);
    let unclassified = report["unclassified_instance_entries"]
        .as_u64()
        .expect("unclassified_instance_entries must be present in the census JSON");
    assert!(
        unclassified >= 1,
        "c26's hidden.services.synth.foo entry must be counted; got {unclassified}"
    );
}

// --- E1/GAP-4 regression: nested-submodule declaration collision. Found
// by the E1 holdout audit as a real, demonstrated false-positive-CAPABLE
// bug (not just a coverage gap): a nested `options = {...}` block found
// inside another option's own `mkOption {...}` call used to be walked
// from a fresh, EMPTY path -- so a same-named leaf anywhere else in the
// file (`enable`, `host`, `port`, ...) could silently be matched by
// gate 1 instead of the real declaration. Fixed by scoping a nested
// submodule's own declarations under the outer option's real path.

fn discovered_option_paths(report: &Value) -> Vec<String> {
    report["discovered_options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| {
            o["path"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap())
                .collect::<Vec<_>>()
                .join(".")
        })
        .collect()
}

#[test]
fn h2_case14_nested_submodule_collision_no_longer_false_positives() {
    // The exact minimal E1 bisect reproducer. Before this fix: a real,
    // demonstrated false OBA001 -- gate 1 matched the unrelated nested
    // `nested.enable` (default true) instead of the real top-level
    // `enable` (mkEnableOption, itself still invisible per the separate
    // GAP-1 -- not this test's concern), so the tool's own fabricated
    // "default" disagreed with reality and it reported a transition that
    // never actually happened. After this fix: the collision-only entry
    // can no longer be matched at the bare, wrong path -- the correct,
    // honest result is `OptionNotFound` (P0 fixes the false-positive
    // CAPABILITY, not full coverage -- that's P1's `mkEnableOption`
    // support, a separate, later fix).
    let reports = run_golden();
    let r = target(&reports, "h2-case14-nested-submodule-collision");
    assert_eq!(
        verdict_kind(r, "enable"),
        "OptionNotFound",
        "must NOT be OBA001 (a false finding) or PASS (right-answer-wrong-mechanism) -- \
         the wrong, collision-sourced declaration must no longer be matchable at all"
    );
    let paths = discovered_option_paths(r);
    assert!(
        !paths.contains(&"enable".to_string()),
        "no entry should exist at the bare, collision-prone path any more; got {paths:?}"
    );
    assert!(
        paths.contains(&"services.bisect.nested.enable".to_string()),
        "the nested submodule's own `enable` must be recorded at its real, scoped path \
         instead; got {paths:?}"
    );
}

#[test]
fn h2_case15_xandikos_real_nested_submodule_collision_no_longer_false_positives() {
    // The real, in-the-wild case the E1 holdout audit actually found this
    // bug on: xandikos's `nginx = mkOption { type = types.submodule {
    // options = { enable = mkOption { default = false; ... }; ...}; };
    // };`. Before this fix: a real `PASS` on `enable`, but backed by the
    // WRONG declaration (`nginx.enable`, not `services.xandikos.enable`
    // itself) -- numerically harmless only by coincidence (both defaults
    // happened to be `false`), not because the mechanism was sound. After
    // this fix: same as the synthetic case above -- `OptionNotFound`,
    // honest and disclosed, not a right-answer-wrong-mechanism PASS.
    let reports = run_golden();
    let r = target(&reports, "h2-case15-xandikos-real-nested-submodule-collision");
    assert_eq!(
        verdict_kind(r, "enable"),
        "OptionNotFound",
        "must no longer be a PASS backed by the wrong (nginx.enable) declaration"
    );
    let paths = discovered_option_paths(r);
    assert!(
        !paths.contains(&"enable".to_string()),
        "no entry should exist at the bare, collision-prone path any more; got {paths:?}"
    );
    assert!(
        paths.contains(&"services.xandikos.nginx.enable".to_string()),
        "nginx's own real `enable` must be recorded at its real, scoped path instead; \
         got {paths:?}"
    );
}

// --- E1/P1 regression: mkEnableOption normalized as a declaration of a
// plain boolean option with a known default. Before this fix:
// `mkEnableOption` (nixpkgs's own standard helper for a service's
// `enable` toggle) was invisible to scan_options's declaration scanner
// entirely -- is_mk_option_call only recognized a literal `mkOption`
// call -- so every option declared this way was OptionNotFound
// regardless of whether a real predicate branched on it. E1 found this
// was the root cause behind 84% of its 32 real inconclusive holdout
// results.

#[test]
fn h2_case16_mkenableoption_bare_is_recognized_with_default_false() {
    let reports = run_golden();
    let r = target(&reports, "h2-case16-mkenableoption-bare");
    assert_eq!(
        verdict_kind(r, "enable"),
        "PASS",
        "a bare `mkEnableOption` declaration must now be found, with its real, known \
         default (false)"
    );
    let verdict = verdict_obj(r, "enable");
    assert_eq!(
        verdict["default_outcome"], false,
        "mkEnableOption's own synthesized default must be `false` -- got {verdict:?}"
    );
}

#[test]
fn h2_case17_mkenableoption_override_default_wins() {
    // The same real shape libinput's own module uses (`mkEnableOption
    // "..." // { default = config.services.xserver.enable; };`), here
    // with a literal override so the exact value can be asserted: the
    // `// {...}` merge's own `default = true;` must win over the
    // synthesized `false`, the same precedence real Nix's `//` operator
    // itself has.
    let reports = run_golden();
    let r = target(&reports, "h2-case17-mkenableoption-override-default-wins");
    assert_eq!(verdict_kind(r, "enable"), "PASS");
    let verdict = verdict_obj(r, "enable");
    assert_eq!(
        verdict["default_outcome"], true,
        "the `// {{ default = true; }}` override must win over mkEnableOption's own \
         synthesized `false`; got {verdict:?}"
    );
}

#[test]
fn h2_case18_nohang_real_mkenableoption_is_a_clean_pass() {
    // The real, in-the-wild confirmation: nohang's `enable =
    // mkEnableOption "...";`, flat-dotted declaration form (no GAP-2
    // involved), a single real `mkIf cfg.enable {...}` predicate, and a
    // real test flipping it to `true`. Before this fix: `OptionNotFound`
    // -- the option was never even visible to gate 1. After: a real,
    // correctly witnessed `PASS` on completely unfamiliar code.
    let reports = run_golden();
    let r = target(&reports, "h2-case18-nohang-real-mkenableoption");
    assert_eq!(verdict_kind(r, "enable"), "PASS");
    let verdict = verdict_obj(r, "enable");
    assert_eq!(verdict["default_outcome"], false);
}
