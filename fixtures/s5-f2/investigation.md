# S5-F2: investigation and root-cause trace (cgit PR #475112)

Required by the mandate: a full end-to-end trace of the false OBA001 on
`gitHttpBackend.enable`, base `308f82252459d32a68cfbbec3af672e5867ff994`,
head `a722a999d1af37d9f3760680dc50fc598fce01a6`, presentation id
`B:475112:oba:gitHttpBackend.enable:new_finding:1`, identifying the
EXACT point of invariant violation, classified into one of the 7
enumerated failure-mode categories, with actual values on both sides of
the failed comparison -- not stopped at the first suspicious function
name.

## Real shape, as read from the fetched source (not assumed)

`nixos/modules/services/networking/cgit.nix`:

```
options = {
  services.cgit = lib.mkOption {
    type = lib.types.attrsOf (lib.types.submodule ({ config, ... }: {
      options = {
        ...
        gitHttpBackend.enable = lib.mkOption { type = lib.types.bool; default = true; };
        ...
      };
    }));
  };
};
```

`config`, later in the same file:

```
services.nginx.virtualHosts = lib.mkMerge (
  lib.mapAttrsToList (name: cfg: {
    ${cfg.nginx.virtualHost} = {
      locations = (...)
        // lib.optionalAttrs cfg.gitHttpBackend.enable { ... };
    };
  }) cfgs
);
```

`cfg` here is a LAMBDA PARAMETER of `lib.mapAttrsToList (name: cfg:
...) cfgs` -- not a `let`-alias, not the submodule's own internal
`config`. `scan_predicates`/`scan_resolved_predicates` do not need to
resolve `cfg`'s own binding to find this predicate: they match
`cfg_ident.<path>` (`cfg.gitHttpBackend.enable`) wherever it textually
appears, given `cfg_ident = "cfg"` from `targets.toml`, regardless of
what actually binds `cfg`. Confirmed empirically: the predicate IS
found (`PredicateKind::Truthy("optionalAttrs")`, `path =
["gitHttpBackend","enable"]`) even though `cfg` is never a simple
alias here.

`nixos/tests/cgit.nix`, the real, deliberate assignment (line 58):

```
services.cgit."no-git-http-backend.localhost" = {
  enable = true;
  scanPath = "/tmp/git";
  settings = { strict-export = "git-daemon-export-ok"; };
  gitHttpBackend.enable = false;
};
```

`gitHttpBackend.enable`'s own declared default is `true`; the test
instance flips it to `false`; the testScript later exercises exactly
the disabled branch (`server.fail("git clone
http://no-git-http-backend.localhost/some-repo ...")` after re-enabling
it, and the corresponding `succeed` while it's still enabled) -- a
real, deliberate test of both branches of this exact predicate.

## Trace, step by step, with actual values

1. **Source assignment**: `services.cgit."no-git-http-backend.localhost".gitHttpBackend.enable = false;`, line 58, confirmed present verbatim.
2. **Parsed AST**: `rnix` parses the test file's `nodes.server` config tree without error -- confirmed via successful `scan_test_assignments` traversal (no `ResolveFailure`/parse-error branch taken).
3. **Extracted test assignment/fact**: instrumented `walk_module_root` directly (temporary `eprintln!`, later fully reverted) -- confirmed `attrpath_segments` on the quoted node returns `["services","cgit","no-git-http-backend.localhost"]` as `path_before` -- the quoted component, despite containing two literal dots, is ONE segment, not three. Not the bug.
4. **Extraction continues via `walk_config_tree`**: instrumented directly -- confirmed it correctly reaches `WCT path_before=["services","cgit","no-git-http-backend.localhost"] segs=["gitHttpBackend","enable"]` and records a `TestAssignment { path: ["services","cgit","no-git-http-backend.localhost","gitHttpBackend","enable"], value_source: "false", known_value: Some(Bool(false)), instance: Some("server"), span: {line: 58, ...} }`. Extraction is entirely correct -- ruled out failure modes "never extracted", "extracted under wrong path", and "value semantics wrong" (the boolean literal `false` is classified correctly).
5. **Normalized option path / watched declaration identity**: `option_prefix = ["services","cgit"]` (from `targets.toml`, matching the real historical adjudication record), `watch = ["gitHttpBackend.enable"]`. `scan_options` correctly discovers the declaration at `["services","cgit","gitHttpBackend","enable"]` (verified present in `discovered_options`) -- discovery itself is not the bug.
6. **Candidate witness matching -- THE FAILURE POINT**: `evaluate_predicate_witness` (pre-fix) calls `path_matches_prefix(&a.path, &t.option_prefix, watched_path, /* no allow_instance_key parameter existed yet */)`. Pre-fix `path_matches_prefix` required, unconditionally:
   ```
   full.len() == prefix.len() + suffix.len()
   ```
   Actual values: `full = ["services","cgit","no-git-http-backend.localhost","gitHttpBackend","enable"]` (len 5), `prefix = ["services","cgit"]` (len 2), `suffix = ["gitHttpBackend","enable"]` (len 2). `2 + 2 = 4 != 5` -> the function returns `false` IMMEDIATELY, without ever comparing a single segment's content. Confirmed via direct instrumentation: `matched_test_assignments: []`, completely empty, for this target -- not a partial or fuzzy failure, a total one.
7. **Predicate-attempt evaluation**: with zero matched assignments, `instances_assigning_x` is empty, the `for instance in &instances_assigning_x` loop never executes, `has_unresolved` stays `false`, and the function falls through to `PredicateWitnessOutcome::EvidenceNoTransition` -- which `run_target`'s own aggregation reports as `witnessed: false` in `predicate_attempts`, and ultimately as `OBA001` (no evidence of a real transition), despite a real, deliberate, correctly-classified test assignment existing the entire time.

**Classification**: category 3, "fails instance-submodule matching" -- the real assignment is extracted correctly, under the correct path, with correctly-resolved value semantics; the declaration/predicate resolution is also correct; the failure is specifically in the STATIC-STRUCTURE comparison between a concrete test-instance path and the declaration/predicate-relative path, which has no representation at all for "a submodule instance key may legitimately sit between the option_prefix and the watched suffix."

## The representation gap, named precisely

OBA's `option_prefix` + `watched_path` model implicitly assumes the
concrete test-instance path is exactly `option_prefix ++ watched_path`.
That is true for a directly-declared option. It is FALSE whenever
`option_prefix` itself resolves to an `attrsOf`/`listOf (submodule
...)`-typed declaration: every real instance of such a declaration
necessarily inserts its own instance key between `option_prefix` and
any of the submodule's own leaf paths, because that is literally what
"a set of instances of a submodule" means in Nix. The predicate's own
reference (`cfg.gitHttpBackend.enable`), scanned from inside the
submodule's own `config` block via `mapAttrsToList`, is GENERIC across
every instance -- it has no instance key in it at all, by construction,
since it's evaluated once per instance with `cfg` bound to that
instance's own attrset. So the predicate side is instance-key-free and
the assignment side is instance-key-bearing, and the pre-fix matching
function had no way to reconcile the two: an invariant ("the assignment
path is `prefix ++ suffix` with nothing in between") that is only ever
true for non-instance-keyed declarations was applied unconditionally to
BOTH shapes.

## Why a hostname-specific fix was rejected before writing any code

The mandate explicitly forbade curing this by hard-coding anything
resembling `no-git-http-backend.localhost`, `cgit`, `services`, or any
other cgit-specific spelling. Consistent with that: nothing in the fix
below references cgit, `services`, hostnames, or vhosts by name
anywhere. The fix is expressed purely in terms of the STRUCTURAL
relationship already available to `oba` -- whether `option_prefix`'s
own true-root declaration (already discovered by `scan_options`, not
re-derived) is `attrsOf`/`listOf`-typed.

## The fix

`path_matches_prefix` (`src/main.rs:3445`) gains a fourth parameter,
`allow_instance_key: bool`. The pre-existing exact-length branch is
completely unchanged (byte-for-byte, same comparisons, same order). A
NEW branch, taken only when `allow_instance_key` is `true` AND
`full.len() == prefix.len() + 1 + suffix.len()`, still requires prefix
and suffix to match content-for-content at their own fixed positions;
only the ONE segment at `full[prefix.len()]` (the concrete instance
key) is accepted regardless of its own value.

`allow_instance_key` itself is never assumed or guessed -- it is
computed ONCE per target, in `run_target` (`src/main.rs:4347`), by a
new function `option_prefix_is_instance_keyed_submodule`
(`src/main.rs:3497`): re-locate `option_prefix`'s own already-discovered
`OptionDecl` (by `options.iter().find(|o| o.path == option_prefix)`,
reusing `scan_options`'s own output -- no second, independent
declaration search), re-find the exact AST node by matching its
recorded `span` (a flat `root.descendants()` scan, the same
span-matching idiom used elsewhere in this codebase), extract its own
`type =` field via the pre-existing `mk_option_field`, and check
(`type_expr_wraps_attrs_of_or_list_of`, `src/main.rs:3534`, reusing the
pre-existing `is_call_named` helper) whether `attrsOf`/`listOf` appears
anywhere in that type expression. This returns `false` for a
`submodule` NOT wrapped in `attrsOf`/`listOf` (a single, unkeyed
instance, e.g. angrr's real `settings = mkOption { type =
types.submodule settingsOptions; };`) -- no instance key tolerance is
ever granted there, deliberately, since none is structurally possible.

`evaluate_predicate_witness` (`src/main.rs:3649`) and `run_target`'s own
H1 gate-4 site (`src/main.rs:4447`) both take the SAME
`allow_instance_key` value, computed once, and thread it to every
`path_matches_prefix` call site they own -- there is no predicate-form
branch anywhere in the fix (see "Predicate-form breadth" below).

## Why the broader, unconditional design was considered and rejected

An earlier design considered making `path_matches_prefix` tolerant of
one extra segment UNCONDITIONALLY, regardless of `option_prefix`'s own
shape. Rejected before writing any code: an unrelated, genuinely
NESTED, non-instance-keyed option sharing the same suffix under a
directly-declared (non-`attrsOf`/`listOf`) structure would then wrongly
match -- e.g. `services.foo.sub.gitHttpBackend.enable` colliding with a
watched `services.foo.gitHttpBackend.enable` purely because both are
one segment away from matching, with no structural justification at
all. `option_prefix_is_instance_keyed_submodule`'s own independent,
structural verification is exactly the guard that prevents this --
covered directly by hostile controls (3) and (4) below, both of which
use non-instance-keyed collision shapes and confirm no false match.

## Cross-instance ambiguity: investigated, not solved, explicitly documented

When TWO DIFFERENT attrsOf-submodule instances WITHIN THE SAME NixOS
test node both assign the same relative watched option to DIFFERENT
values, `instances_assigning_x`'s own dedup-by-node (keyed on
`TestAssignment.instance`, the NixOS-test MACHINE name -- NOT the
submodule instance key `path_matches_prefix`'s own new tolerance skips
over) means only the FIRST matching assignment in source-scan order is
actually used, via `.find()`-first-match semantics. This matches this
project's own existing precedent elsewhere (`run_target`'s own gate-1
`.find()` lookup). Per the mandate's own explicit instruction ("should
be handled according to OBA's intended test-coverage semantics;
document that intended semantics explicitly rather than assuming it"),
this is documented, not silently assumed, and asserted as the ACTUAL
current behavior by
`two_instances_disagreeing_on_the_same_leaf_use_first_source_order_match_documented_limitation`
in `tests/f2_cgit_hostile.rs`. Not fixed, not claimed to be fixed --
S5-F2's mandate scopes fixing the cgit false-finding defect
specifically, not redesigning cross-instance aggregation semantics.

## Predicate-form breadth (H1 Truthy/NegTruthy, H2 Eq/NotEq)

Traced structurally, then confirmed empirically, that the fix is
predicate-form-agnostic BY CONSTRUCTION:

- `PredicateKind::Truthy`/`NegTruthy`/`NullEq`/`NullNeq` (H1's own
  representation of `cfg.foo`, `!cfg.foo`, `cfg.foo == null`, `cfg.foo
  != null`) all flow through the SAME single call site in `run_target`
  (`src/main.rs:4447`), which takes the SAME `allow_instance_key`
  value.
- H2's own `Pred` IR (`lower_pred_chained`, `src/main.rs:2049`) already
  normalizes `a == b` / `a != b` / a bare `cfg.foo` used as a condition
  / `!p` / `a && b` / `a || b` into `Pred::Eq` / `Pred::Not(Eq)` /
  `Pred::And` / `Pred::Or` BEFORE `evaluate_predicate_witness` ever
  runs -- there is no surface-form branch left by the time witness
  matching happens. All 3 of `evaluate_predicate_witness`'s own
  `path_matches_prefix` call sites take the identical
  `allow_instance_key` value.

There is no code path anywhere that branches on predicate surface form
when deciding whether to tolerate an instance key. Confirmed
empirically, not merely by code reading, with two new targeted tests
(`tests/f2_cgit_hostile.rs`):
`negated_predicate_form_on_instance_keyed_submodule_is_also_witnessed`
(H1 `NegTruthy`, `if !cfg.foo.enable then ...`) and
`equality_predicate_form_on_instance_keyed_submodule_is_also_witnessed`
(H2 `Eq`, `cfg.foo.mode == "on"`), both PASS on an instance-keyed
submodule target via the same mechanism as the real cgit fixture's own
`Truthy("optionalAttrs")` form.

## A separate, unrelated, pre-existing limitation found during synthetic-fixture construction

While building the hostile controls, synthetic fixtures written in the
FLAT-DOTTED declaration form (`options.services.widget = lib.mkOption
{...};`) produced `discovered_options: []` -- NOT a symptom of the
cgit fix. Root-caused via temporary instrumentation directly in
`scan_options`'s flat-dotted branch: the condition matches correctly,
but `walk_merge_operands` (called on `value` in that branch) only
unwraps a literal `NODE_ATTR_SET` or a `//`-merge -- it never calls
`classify_option_helper_call` on `value` itself, so an `mkOption{...}`
call written directly as the flat-dotted value is silently invisible
to it. The REAL cgit file avoids this entirely because it uses the
NESTED form (`options = { services.cgit = mkOption {...}; };`), which
goes through `walk_options_block`'s own generic entry loop (which DOES
check `classify_option_helper_call` first, unconditionally). This is a
real, pre-existing, UNRELATED limitation of the flat-dotted branch's
own design -- confirmed out of S5-F2's own scope (it does not affect
cgit, and fixing it is not part of this mandate), NOT touched. Worked
around by rewriting the affected synthetic fixtures to the nested form,
matching cgit's own real structure.

## Regression corpus re-checked (targeted, not full replay, per the mandate)

- `scan_options_angrr_two_hop_period_is_discovered*` -- unaffected (angrr's `settings` is a bare `submodule`, not `attrsOf`/`listOf`; `option_prefix_is_instance_keyed_submodule` returns `false` for it).
- `scan_options_resolves_an_arbitrary_depth_chain*` (F1D's own recursive chain) -- unaffected.
- `two_roots_are_analyzed_independently_real_kimai_transition` (kimai wildcard) -- unaffected; S5-F2's new tolerance is gated on a CONCRETE `option_prefix` resolving to a real `OptionDecl` via `options.iter().find(|o| o.path == option_prefix)`, which never matches a wildcarded prefix.
- Full existing suite: 356/356 passing before this round's own new tests were added; 367/367 (356 + 2 real-fixture + 9 hostile) after, zero regressions, zero new warnings beyond the 2 pre-existing, unrelated ones.
