# S5-F1B: investigation before implementation

Required by the S5-F1B mandate before any code change: a real-source trace
of angrr's own defect plus one prosody, one drupal, and one fedimintd
regression (all four found by S5-F1-R, `fixtures/s5-f1-r/classification-notes.json`),
each covering: watched prefix, the relevant `mkOption`, the named binding,
the AST/reference relationship, pre-F1 discovery, `cdfb4ca` (F1) exclusion,
and provenance-path yes/no.

## angrr PR #471312 -- must stay EXCLUDED

- **Watched prefix**: `option_prefix = [services, angrr]`, `watch = [period]`.
- **Relevant `mkOption`**: `services.angrr.settings = mkOption { type =
  lib.types.submodule settingsOptions; ... };` -- a true-root declaration,
  directly at `option_prefix`.
- **Named binding**: two hops deep, not one. `settingsOptions` (a named
  `let`-bound value) declares `temporary-root-policies = lib.mkOption {
  type = with lib.types; attrsOf (submodule temporaryRootPolicyOptions);
  ... };`; `temporaryRootPolicyOptions` (a second, different named
  `let`-bound value) is where the real (unrelated) `period` leaf lives.
- **AST/reference relationship**: `settings`'s own `type =` field
  references `settingsOptions` directly (dotted, no `with`) -- ONE hop
  from a true-root declaration, so `settingsOptions` itself IS promoted.
  Reaching `temporaryRootPolicyOptions` requires a SECOND hop, from
  `temporary-root-policies` (itself only discovered because
  `settingsOptions` was promoted, i.e. not a true-root declaration) --
  and that second hop's own `type =` field is wrapped in `with lib.types;`,
  which `resolve_ident_binding` already, independently, refuses to resolve
  (`ResolveFailure::UnsupportedScope("with")`).
- **Pre-F1 discovery**: pre-F1, every named `let`/`rec`-bound `options =
  {...}` block was walked unconditionally, with a fresh bare path --
  `temporaryRootPolicyOptions`'s own `period` was bare-recorded as
  `["period"]`, colliding with the real, removed top-level
  `services.angrr.period`. This is the real S5 defect: a genuinely
  UNRELATED `period` (a per-policy retention string, nothing to do with
  the removed top-level option) satisfied `run_target`'s gate-1
  exact-path lookup, producing a false `Unchanged`.
- **`cdfb4ca` (F1) exclusion**: `is_nested_inside_named_let_or_rec_binding`
  excludes ANY named-binding-nested `options = {...}` block under a
  concrete `option_prefix`, with no further distinction -- correctly
  excludes this case, but (per S5-F1-R) also wrongly excludes prosody's/
  drupal's/fedimintd's real, legitimate cases below.
- **S5-F1B provenance-path**: **NO.** `settingsOptions` is promoted (one
  hop, real provenance), but `temporaryRootPolicyOptions` is not: S5-F1B
  caps promotion at exactly one hop from a true-root declaration (see
  "Why one hop, not a fixpoint" below), so a second-hop candidate is
  never promoted regardless of whether its own reference happens to
  resolve. `period` stays excluded, exactly as F1 already had it.

## Why one hop, not a fixpoint (a real design correction made during this investigation)

The first S5-F1B implementation used an unbounded fixpoint: a promoted
candidate's own declarations could discover further real `type =`
references, promoting further named bindings, iterated to a fixpoint.
Tracing angrr's own real module (above) surfaced a real problem with
that design: `settingsOptions` promotes correctly (one hop), and while
walking its own content, `temporary-root-policies`'s `type =` field
(`with lib.types; attrsOf (submodule temporaryRootPolicyOptions)`) would
normally register a second-hop reference to `temporaryRootPolicyOptions`
-- except that `resolve_ident_binding` already refuses `with`-scoped
identifiers, so nothing gets registered and the cascade stops.

That the fixpoint DIDN'T reintroduce angrr's bug here is coincidental,
not structural: the semantically identical `lib.types.attrsOf
(lib.types.submodule temporaryRootPolicyOptions)` (no `with`) would
resolve just fine, and under an unbounded fixpoint would promote
`temporaryRootPolicyOptions`, reintroducing angrr's exact original
false-`Unchanged` defect one hop deeper. Relying on angrr's own specific
coding style (its use of `with`) to keep its defect excluded is not an
acceptable invariant.

Every real regression S5-F1-R found (prosody/drupal/fedimintd, all three
traced below) is exactly ONE hop from a true-root declaration. Nothing in
the S5 corpus needs more. S5-F1B therefore caps promotion at one hop:
`referenced` is frozen after Pass 1 (true-root declarations only) before
Pass 2 decides which deferred candidates to promote, and a promoted
candidate's own walk records its declarations but does not feed new
references back for further promotion (its own `type =` scan writes into
a throwaway, discarded set). This closes the two-hop reintroduction
structurally, independent of `with`-opacity, while still covering every
real demonstrated case.

## prosody PR #431289 -- must be INCLUDED

- **Watched prefix**: `option_prefix = [services, prosody, muc]`, `watch
  = [moderation]`.
- **Relevant `mkOption`**: `muc = mkOption { type = types.listOf (types.submodule
  mucOpts); ... };`, declared directly at `option_prefix`'s own concrete
  path (this `muc` declaration itself IS the true-root match for this
  target).
- **Named binding**: `mucOpts`, a named `let`-bound value; its own
  `options = {...}` block declares `moderation` (the watched leaf) among
  others.
- **AST/reference relationship**: `muc`'s own `type =` field references
  `mucOpts` directly (dotted `types.submodule mucOpts`, no `with`) -- ONE
  hop from a true-root declaration.
- **Pre-F1 discovery**: bare-recorded, `moderation` found at `["moderation"]`,
  matching the watch -- correct, historical `PredicateNotFound` (found,
  not witnessed by any test).
- **`cdfb4ca` (F1) exclusion**: wrongly excluded (same predicate as
  angrr's case, no distinction made) -- `moderation` becomes undiscoverable,
  collapsing the real `option_not_found->predicate_not_found` transition
  into a false `Unchanged`. This is one of S5-F1-R's 7 real watched-verdict
  regressions.
- **S5-F1B provenance-path**: **YES**, one hop, dotted, no unsupported
  scope. `mucOpts` is promoted; `moderation` is bare-recorded again,
  restoring the historical `predicate_not_found` transition exactly
  (re-verified this session against real re-fetched PR #431289 content).

## drupal PR #506644 -- must be INCLUDED

- **Watched prefix**: `option_prefix = [services, drupal, sites]`, `watch
  = [webRoot]`.
- **Relevant `mkOption`**: `sites = mkOption { type = types.attrsOf
  (types.submodule siteOpts); ... };`, declared directly at
  `option_prefix`'s own concrete path (again, this declaration itself IS
  the true-root match).
- **Named binding**: `siteOpts`, a named `let`-bound value declaring
  `webRoot` among others.
- **AST/reference relationship**: `sites`'s own `type =` field references
  `siteOpts` directly (dotted `types.submodule siteOpts`, no `with`) --
  ONE hop from a true-root declaration.
- **Pre-F1 discovery**: bare-recorded, historical `option_not_found ->
  predicate_not_found`.
- **`cdfb4ca` (F1) exclusion**: wrongly excluded, same mechanism as
  prosody -- one of S5-F1-R's 7 real watched-verdict regressions
  (`webRoot`, plus siblings `configRoot`/`#508427`, `configSyncDir`/`#428153`,
  `privateFilesDir`/`#427260`, all four the same `siteOpts` mechanism).
- **S5-F1B provenance-path**: **YES**, one hop, dotted, no unsupported
  scope. Re-verified this session against real re-fetched content for
  all four drupal PRs (`#506644`, `#508427`, `#428153`, `#427260`) --
  all four restored to their exact historical transition.

## fedimintd PR #397967 -- must be INCLUDED (the critical case)

- **Watched prefix**: `option_prefix = [services, fedimintd]`, `watch =
  [api_ws.openFirewall]`.
- **Relevant `mkOption`**: `services.fedimintd = mkOption { type =
  types.attrsOf (types.submodule fedimintdOpts); ... };` -- the WHOLE
  target's own type, declared directly at `option_prefix`.
- **Named binding**: `fedimintdOpts`, a named `let`-bound value whose own
  `options = {...}` declares `p2p`, `api_ws = { openFirewall = mkOption
  {...}; ...}`, among others.
- **AST/reference relationship**: `services.fedimintd`'s own `type =`
  field references `fedimintdOpts` directly (dotted, no `with`) -- ONE
  hop from a true-root declaration.
- **Pre-F1 discovery**: bare-recorded; historical `option_not_found ->
  oba001` -- a REAL, already-adjudicated actionable finding from S5
  itself (two independent blind reviewers confirmed it correct during
  the original S5 round).
- **`cdfb4ca` (F1) exclusion**: wrongly excluded -- `api_ws.openFirewall`
  becomes undiscoverable, collapsing the real, already-confirmed finding
  into a false `Unchanged`. This is the single most consequential
  instance of S5-F1-R's 7 regressions: F1, left as committed, would make
  an already-adjudicated real S5 finding invisible.
- **S5-F1B provenance-path**: **YES**, one hop, dotted, no unsupported
  scope. Re-verified this session against real re-fetched PR #397967
  content -- restored to the exact historical `option_not_found->oba001`
  transition.

## Summary of the required invariant

Not "syntactic nesting inside a named `let`/`rec` binding" (F1's own
predicate, too broad) and not "reachable via any chain of `type =`
references" (an unbounded fixpoint, which angrr's own real module shows
is too permissive once dependent on `with`-opacity as its only brake).
The invariant S5-F1B implements: a named `let`/`rec`-bound `options =
{...}` block is included if and only if it is referenced, by name,
directly (not `with`-scoped, not `rec`-scoped -- `resolve_ident_binding`
already refuses both, conservatively) from a real, already-anchored
TRUE-ROOT declaration's own `type =` field, i.e. exactly one hop away.
Every real S5-F1-R regression is exactly one hop; angrr's real defect is
two.
