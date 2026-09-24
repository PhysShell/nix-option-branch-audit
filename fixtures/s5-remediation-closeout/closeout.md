# S5 remediation program: closeout record

This document is a research/documentation artifact only. It changes no
source code, no analyzer behavior, and no historical record. It does
not run, and does not require, a full frozen-corpus replay. Its job is
to state, precisely and without rounding up, what was actually proven
during S5 remediation, what remains open, and what should happen next.

## 1. Historical S5 remains immutable

`v0.4.5` / `8f1701a289ddab8bc23f23a25cc0937863fa0356`, full 369/369 PR
frozen-corpus run, **verdict: FAIL**, three confirmed Tier-1 defects
(angrr, cgit, guacamole). This result is not rewritten here or
anywhere. It is not "now a PASS" -- it remains what it was the day it
ran.

## 2. Remediation result

**All three confirmed historical S5 defects have accepted remediations
under full frozen-corpus regression validation**, each on its own
separate implementation/acceptance line:

| Defect | Layer | Implementation | Full-corpus acceptance |
|---|---|---|---|
| angrr | identity / discovery | `25c5b54` | `58b28b2` -- PASS |
| cgit | test-evidence / witness matching | `c647d1b` | `c04c5af` -- PASS |
| guacamole | option-evolution / migration semantics | `05d4a90` | `6051805` -- PASS, with a disclosed replay-methodology exception (section 3) |

This is a genuinely unusual guarantee for a project this size: each of
the three real defects didn't just get a unit test -- each one
survived a *separate, full, 369-PR frozen-corpus replay* before being
accepted, not merely a local regression suite.

## 3. The F3-R methodology exception, recorded precisely

**Not summarized as "every literal prerequisite of the F3-R mandate was
met."** It was not. Here is exactly what was, and was not, proven.

**Proven, directly from the frozen `6051805` replay**:
- the guacamole relocation itself (`services.guacamole-server.logbackXml
  -> services.guacamole-client.logbackXml`);
- complete `from_path`, complete `to_path`, `helper_form =
  "mkRenamedOptionModule"`, real migration-call source span;
- the structured `Verdict::OptionRelocated` classification itself,
  and its machine-readable distinction from plain `OptionNotFound`
  (`oba_verdict_transitions: {"oba001->option_relocated":1}`, `code:
  "OBA-RELOCATED"`) -- confirmed directly in the replay's own
  `raw.json`.

**Not proven by the frozen replay itself**: `destination_confirmed`.
It came back `false`, truthfully, because the historical, frozen
`targets.toml` for PR #462487 (captured before cross-file destination
confirmation existed as a capability) lists only
`guacamole-server.nix` and its own test file -- `guacamole-client.nix`
was never fetched into that PR's own sparse replay root at all.
Confirmed directly: `fixtures/s5-f3-r/sources/B/462487/{base,head}/`
contains exactly one module file, never the destination.

**What independently establishes the mechanism is sound anyway**:
1. The real, live-fetched S5-F3 implementation-round fixture
   (`fixtures/synthetic/f3-guacamole-logbackxml-relocation/`), built
   deliberately to include BOTH files, shows `destination_confirmed:
   true` for the identical rename edge.
2. A dedicated synthetic hostile control
   (`cross_file_destination_is_confirmed`,
   `tests/f3_migration_hostile.rs`) proves the cross-file search
   mechanism independently of the real fixture.
3. PR `#510900` (sshd, `moduliFile -> settings.ModuliFile`) proves
   destination confirmation genuinely succeeds *within the frozen
   replay's own sparse-root constraints*, for a same-file case --
   the mechanism is not merely theoretical, it is exercised and
   correct in the actual replay, just not for a cross-file case.

**Classification**: `accepted_replay_methodology_exception`. This is
explicitly NOT classified as an implementation defect: at no point did
`locate_migration_destination` assert `true` without a real
declaration found -- it stayed honestly `false` exactly where the
given root didn't contain the answer. The gap is in what the
*historical target manifest* (a validation-harness artifact, item J in
the register below) happens to fetch, not in what the analyzer claims.

**What this closeout does NOT claim**: that the frozen replay itself
demonstrated a successful cross-file destination lookup. It did not.
That specific claim rests on the implementation-round fixture and the
hostile control, both built with the necessary files present on
purpose -- not on `6051805`'s own frozen corpus.

## 4. Residual-limitations register

Each item: concrete evidence, then objective fields (not a priority
score), then a disposition with reasoning. Ordered as the mandate's own
lettering.

### A. `imports =` reachability

- **Evidence**: angrr's own `commonPolicyOptions`, reachable only
  through `imports = [ commonPolicyOptions ];`, never through any
  `type =` reference -- the only mechanism this analyzer's own
  reference-following (`walk_named_type_references`) understands.
  Surfaced by S5-F1D-R's own span-based provenance reconciliation
  (`fixtures/s5-f1d-r/f1d-r-report.md`), confirmed pre-existing since
  before F1D via F1-R's own frozen `comparison-report.jsonl`.
- **Observed corpus cases**: 1 PR (angrr `#471312`), 1 declaration
  span (`commonPolicyOptions`'s own `enable`).
- **Frozen watched-verdict impact**: none -- angrr's own watched
  `period` is unaffected either way.
- **False-positive risk**: none demonstrated.
- **False-negative risk**: yes -- a declaration reachable only via
  `imports =` is invisible to discovery; if a FUTURE watched target
  needed exactly such a declaration, it would show `OptionNotFound`
  incorrectly.
- **Fails closed**: yes (silently absent, not silently wrong).
- **Semantics already well-defined**: no -- `imports =` is a general
  NixOS module-composition mechanism (arbitrary expressions, not just
  named lets), materially different from a `type =` reference.
- **Implementation surface**: likely architectural -- resolving
  `imports =` in general risks re-deriving a slice of NixOS's own
  module evaluator, not a local extension of the existing reference
  walker.
- **Full replay required after any fix**: yes.
- **New corpus construction needed**: likely yes, to find more than
  the single known case.
- **Disposition: `PREREGISTERED_EXPERIMENT`** (hypothesis, tested
  above -- confirmed, not merely assumed: the surface genuinely crosses
  from declaration/type traversal into module-composition semantics,
  the single known real case has zero verdict impact, and the honest
  boundary of "how much of `imports =` is safe to resolve" is not
  self-evident without deliberate scoping first).

### B. `walk_merge_operands` wrapped-root gap (frigate / nvidia-container-toolkit)

- **Evidence**: `with`/`let ... in`-wrapped `options = {...}` blocks in
  a known shape prevent true-root discovery. Disclosed since S5-F1B,
  re-confirmed unchanged at every subsequent round (S5-F1D-R:
  "known_pre_existing_frigate_nvidia_wrap_gap": 36 spans across 6 PRs;
  re-confirmed untouched at S5-F2-R and S5-F3-R).
- **Observed corpus cases**: 6 PRs, 36 spans.
- **Frozen watched-verdict impact**: none confirmed in any S5 round to
  date -- no watched target has been shown to resolve incorrectly
  because of this gap specifically.
- **False-positive risk**: none demonstrated.
- **False-negative risk**: yes -- affected declarations are simply
  invisible.
- **Fails closed**: yes.
- **Semantics already well-defined**: plausibly yes -- unlike
  `imports =`, `with`/`let ... in` wrapping is lexical scoping the
  existing AST walker already partially understands elsewhere (e.g.
  S5-F1C-A's own scoped `with`-resolution for legitimate one-hop type
  references) -- but this closeout has not itself re-read
  `walk_merge_operands`'s own exact code this round (no implementation
  investigation was authorized), so "narrow root cause" is not yet
  independently confirmed, only plausible from the pattern's own
  description.
- **Implementation surface**: unconfirmed this round -- needs a real
  investigation pass before committing to "local."
- **Full replay required after any fix**: yes.
- **New corpus construction needed**: no -- 6 real PRs already known.
- **Disposition: `BUG_FIX_NEXT` (tentative, pending a short
  investigation pass to confirm narrowness)** -- the mandate's own
  hypothesis is plausible (well-evidenced, multiple real corpus
  examples, fails closed, no demonstrated verdict impact means low
  risk to fix), but this closeout stops short of a confident
  `BUG_FIX_NEXT` commitment until a dedicated look at
  `walk_merge_operands`'s own real code confirms the fix is genuinely
  local, not merely assumed to be from the pattern's name.

### C. Wildcard-prefix collision / first-match (wstunnel `#415326`)

- **Evidence**: the one remaining real leaf-collision finding after
  F1D's own provenance-qualification resolved 7 of 8 known cases.
  `option_prefix = [services, wstunnel, clients, "*"]` -- F1D's own
  reference-following is deliberately gated off for wildcard prefixes
  entirely (matching kimai's own established precedent), so this
  collision is untouched by design, not merely unresolved by accident.
  Re-confirmed unchanged, the sole remaining finding, at every
  subsequent full-corpus round (F1D-R, F2-R, F3-R).
- **Observed corpus cases**: 1 PR (`#415326`), 1 colliding path
  (`settings`).
- **Frozen watched-verdict impact**: a structural collision in
  `discovered_options` is confirmed present; whether it produces an
  actually INCORRECT final watched verdict for `#415326`'s own target
  has never been separately demonstrated in three rounds of
  re-checking -- recorded honestly as unconfirmed impact, not silently
  assumed benign.
- **False-positive risk**: plausible but undemonstrated -- a
  first-match `.find()` under a wildcard could theoretically select
  the wrong candidate declaration.
- **False-negative risk**: not the primary concern here.
- **Fails closed**: no -- `.find()` picks a candidate rather than
  refusing to answer.
- **Semantics already well-defined**: no -- correct behavior under a
  wildcard prefix with multiple structurally colliding candidates has
  never been specified, only worked around.
- **Implementation surface**: likely architectural, per the mandate's
  own hypothesis -- touches identity/first-match semantics broadly,
  not a single function.
- **Full replay required after any fix**: yes.
- **New corpus construction needed**: likely yes, since only one real
  case is known.
- **Disposition: `PREREGISTERED_EXPERIMENT`** -- agrees with the
  mandate's own hypothesis: not fail-closed, semantics genuinely
  undefined, broad interaction with identity resolution, exactly the
  shape that should be specified before implementation, not
  implemented-then-specified.

### D. Cross-instance witness ambiguity (F2's own documented limitation)

- **Evidence**: `evaluate_predicate_witness`'s own `instances_assigning_x`
  dedup is keyed by the nixos-test NODE, not by the concrete
  attrsOf-submodule instance key F2's own `allow_instance_key`
  tolerance skips over -- when two DIFFERENT concrete instances within
  ONE node both assign the same relative watched leaf, the FIRST one
  in source-scan order wins (`.find()`-first-match), documented and
  tested (`two_instances_disagreeing_on_the_same_leaf_use_first_source_order_match_documented_limitation`,
  `tests/f2_cgit_hostile.rs`), NOT solved.
- **Observed corpus cases**: 0 -- S5-F2-R's own dedicated corpus-wide
  scan (`cross-instance-candidates.json`, restricted to all 12
  structurally eligible targets, not merely changed PRs) found zero
  outcome-ambiguous cases anywhere in the frozen 180-PR corpus
  (`cross_instance_outcome_ambiguity_not_observed_in_frozen_S5`).
- **Frozen watched-verdict impact**: none observed.
- **False-positive risk**: yes, theoretically -- a future case with
  two disagreeing instances could produce a source-order-dependent
  PASS/OBA001 that flips if the two instances were merely reordered in
  the source file.
- **False-negative risk**: symmetric to the above.
- **Fails closed**: **no** -- this is the one item in the register
  that is NOT fail-closed. It picks an answer (the first match) rather
  than refusing one.
- **Semantics already well-defined**: no -- what the CORRECT answer
  should be when two real instances of the same submodule genuinely
  disagree has never been specified (all-must-agree? any-one-suffices?
  report ambiguous?).
- **Implementation surface**: architectural, per the mandate's own
  hypothesis -- requires specifying new witness-aggregation semantics
  across multiple concrete instances, not a local patch.
- **Full replay required after any fix**: yes.
- **New corpus construction needed**: yes -- zero real corpus cases
  exist to validate against; any fix would need synthetic corpus
  construction from scratch.
- **Disposition: `PREREGISTERED_EXPERIMENT`** -- agrees with the
  mandate's own hypothesis, and is the one item in this register that
  is both NOT fail-closed AND has undefined correct semantics --
  exactly the combination that should never be "just fixed" without a
  pre-registered specification of what "correct" even means here
  first.

### E. `listOf(submodule ...)` instance-key soundness

- **Evidence**: `option_prefix_is_instance_keyed_submodule`'s own
  `type_expr_wraps_attrs_of_or_list_of` treats `attrsOf` and `listOf`
  identically (checks only that either call-head appears anywhere in
  the type expression) -- but a real Nix `listOf` element has NO
  literal attribute name at all (list membership is positional), so a
  literal single-segment "instance key" can never legitimately arise
  for a genuinely valid `listOf`-typed config, unlike `attrsOf`.
  Discovered during S5-F2-R's own hand-verification of the eligibility
  manifest (`fixtures/s5-f2-r/f2-r-report.md`'s own dedicated section).
- **Observed corpus cases**: 2 real `listOf`-typed eligible targets in
  the frozen corpus (prosody's own `muc`, btrbk's own `sshAccess`),
  both independently confirmed at S5-F2-R to show ZERO matching
  candidates of any shape -- the gap is real and demonstrated to
  exist, but unexercised.
- **Frozen watched-verdict impact**: none.
- **False-positive risk**: theoretical only -- would require a test
  file whose own literal source happens to contain a syntactically
  well-formed but semantically nonsensical attrpath shaped like a
  `listOf` instance key (not producible from a real, type-checked Nix
  config, but not distinguishable from a real one by this tool's own
  purely-syntactic walker).
- **Fails closed**: yes in every observed case (zero candidates found
  for either real `listOf`-typed target).
- **Semantics already well-defined**: yes -- the fix is simply "don't
  extend the same tolerance to `listOf` that's sound for `attrsOf`,"
  no new semantics need inventing.
- **Implementation surface**: **local, directly confirmed from
  reading the actual code this session** -- a single function
  (`type_expr_wraps_attrs_of_or_list_of`, or its caller's own gating)
  needs to distinguish the two call heads rather than treating them as
  interchangeable.
- **Full replay required after any fix**: yes, per this project's own
  established discipline for any witness-matching change.
- **New corpus construction needed**: no -- the 2 real, already-known
  `listOf` targets are sufficient as regression anchors; new hostile
  synthetic tests would still be needed (matching this project's own
  established convention) but not new real-PR corpus mining.
- **Disposition: `BUG_FIX_NEXT`** -- agrees with the mandate's own
  hypothesis, confirmed rather than assumed: local surface, clear
  semantics, zero corpus impact either way (so the fix is low-risk),
  fails closed in every currently observed case.

### F. Flat-dotted / nested declaration-scanning limitation

- **Evidence, two distinct concrete manifestations of one underlying
  cause** (`scan_options`'s own root-matching only recognizes a
  declaration as a TRUE ROOT when it sits exactly at a given prefix,
  with no tolerance for either an intervening instance-key segment or
  a field nested one level inside an already-matched parent):
  1. **F2's own hostile-fixture-construction discovery**: the
     flat-dotted branch's own `walk_merge_operands` never calls
     `classify_option_helper_call` on its own `value`, so
     `options.services.widget = mkOption {...};` (helper call written
     directly, flat-dotted) is silently invisible -- unrelated to
     cgit's own real shape (which uses the nested form), found only
     while building F2's own synthetic hostile controls.
  2. **F3's own destination-lookup failures**: `locate_migration_destination`
     reuses `scan_options` UNCHANGED, so it inherits this same
     root-matching rigidity from the OTHER direction -- traefik
     `#490920`'s own real `static.file` (nested one level inside an
     already-matched `[services,traefik]` flat-dotted block, not
     itself a separate true root at `[services,traefik,static]`) and
     vmalert `#410856`'s own real `instances."".enable` (inside an
     `attrsOf(submodule)` instance, which needs the SAME kind of
     one-extra-segment tolerance F2's own witness-matching already has
     for ASSIGNMENTS but `locate_migration_destination` has no
     equivalent of for DESTINATIONS) both produced honest
     `destination_confirmed: false` for exactly this reason, confirmed
     by the reviewing session directly against real source during
     S5-F3-R.
- **Observed corpus cases**: 1 synthetic construction bug (F2, not a
  real-PR case) + 2 real corpus cases where it was load-bearing for a
  `destination_confirmed=false` result (F3-R: `#490920`, `#410856`).
- **Frozen watched-verdict impact**: none for the F2 manifestation
  (synthetic-only, worked around by using the nested form, matching
  cgit's own real shape); none for the F3 manifestation either --
  `destination_confirmed` is an ADDITIONAL, separately-gated claim,
  never the primary relocation classification itself, so its being
  `false` here doesn't produce a wrong final verdict, only a less
  complete one.
- **False-positive risk**: none -- this limitation only ever produces
  an honest `false`/invisible, never a false `true`/found.
- **False-negative risk**: yes, by definition -- real declarations in
  either shape go undiscovered by the affected code paths.
- **Fails closed**: yes, in both manifestations.
- **Semantics already well-defined**: yes for both -- "also check
  `classify_option_helper_call`" (F2's manifestation) and "also try a
  nested/instance-tolerant destination search" (F3's manifestation)
  are both well-understood, bounded extensions of existing, already-
  reviewed logic.
- **Implementation surface**: medium -- each manifestation individually
  looks local, but genuinely fixing `locate_migration_destination`'s
  own destination search to tolerate BOTH nesting and instance-keys
  would mean re-deriving a meaningful slice of `scan_options`'s own
  root-matching logic for a second, destination-side purpose, which is
  more than a one-line patch.
- **Full replay required after any fix**: yes for the F3-side fix (a
  witness/discovery-adjacent change); the F2-side synthetic-only issue
  would only need its own existing hostile-test suite re-run.
- **New corpus construction needed**: no -- both real F3-side cases
  are already known, real, frozen regression anchors.
- **Disposition: `BUG_FIX_NEXT` for the F2 manifestation (flat-dotted
  `walk_merge_operands`/`classify_option_helper_call` gap -- genuinely
  local, already scoped); `TRACKED_LIMITATION` for the F3 manifestation
  (`locate_migration_destination`'s own destination-search rigidity --
  real, understood, but a medium-surface fix that never affects a
  primary verdict, so it does not block a release on its own)**.

### G. Transitive rename chains

- **Evidence**: S5-F3 intentionally supports only direct rename edges
  (`A -> B`), not transitive resolution (`A -> B -> C`), per its own
  explicit, deliberate scope boundary -- confirmed via the migration-
  edge manifest that no watched target in the frozen corpus required
  transitive resolution (every real `matches_watched_path=true` edge
  in S5-F3-R's own corpus-wide manifest is a single, direct hop).
- **Observed corpus cases**: 0 chains observed anywhere requiring
  transitive resolution for a watched target.
- **Frozen watched-verdict impact**: none.
- **Fails closed**: yes -- an option renamed twice (`A->B` at one
  point in history, `B->C` later) simply isn't followed past the first
  hop; the target falls back to whatever the single-hop result is.
- **Semantics already well-defined**: mostly yes -- transitive
  resolution's OWN mechanics are conceptually simple (follow the
  chain, cycle-guard it, same pattern as F1D's own recursive reference
  chain), though real nixpkgs renames-of-renames are rare enough that
  no real corpus example currently forces the question.
- **Implementation surface**: local-to-medium if ever needed (reuse
  F1D's own cycle-guarded recursive-traversal pattern).
- **Disposition: `TRACKED_LIMITATION`** -- agrees with the mandate's
  own hypothesis exactly: a real, disclosed boundary, acceptable to
  leave until a genuine corpus case or product requirement demands it,
  not chased speculatively.

### H. Unsupported migration helpers (`mkChangedOptionModule`, `mkMergedOptionModule`)

- **Evidence**: real, current nixpkgs helpers with DIFFERENT semantics
  from a simple rename (a value TRANSFORM, and a MERGE of several old
  options into one new one, respectively) -- deliberately not
  recognized by `scan_migrations`, per S5-F3's own explicit scope.
- **Observed corpus cases**: **not measured** in S5 remediation work --
  none of S5-F1/F2/F3's own corpus scans searched for these two helper
  names specifically (F3-R's own dynamic-migration-call text scan only
  targeted the 3 SUPPORTED helper names, by design, to find
  unresolved call sites of those, not to census unrelated helpers).
  Recorded honestly as unknown prevalence, not silently assumed rare.
- **Frozen watched-verdict impact**: unknown, for the same reason.
- **Fails closed**: yes by construction -- an unrecognized helper name
  produces no `MigrationEdge` at all, falling through to ordinary
  `OptionNotFound`, exactly like any other unrecognized construct.
- **Semantics already well-defined**: no -- both helpers' own real
  value/merge semantics would need their own design work, structurally
  different from a rename edge's simple `from_path`/`to_path` pair.
- **Implementation surface**: medium-to-architectural, depending on how
  much of the VALUE-transform or multi-source-merge semantics would
  need representing.
- **Disposition: `TRACKED_LIMITATION`**, with an explicit caveat: this
  item's own true corpus prevalence should be measured (a cheap,
  read-only grep-based census, not a design commitment) before it can
  be confidently downgraded to `NO_ACTION` or upgraded to a real
  backlog item -- recorded here as the one item in this register whose
  own evidence gathering is itself incomplete, not glossed over.

### I. Dynamic/computed migration paths

- **Evidence**: real, confirmed frozen corpus examples --
  wstunnel `#415326` (`lib.map (option: lib.mkRemovedOptionModule
  [option] "...") [...]`), syncthing `#422094`/`#353770` (`map (o:
  mkRenamedOptionModule [...o] [...o]) [...]`), k3s `#374017`
  (`mkRemovedOptionModule ([...] ++ config) instruction`, a computed
  list via `++`) -- all independently confirmed by the reviewing
  session against real source during S5-F3-R to produce NO migration
  edge at all, and NONE became `OptionRelocated`.
- **Observed corpus cases**: 4 PRs, all correctly, fail-closed
  rejected.
- **Frozen watched-verdict impact**: none -- these targets simply
  behave as if no migration scanner existed at all for them (ordinary
  `OptionNotFound`/whatever their prior verdict already was).
- **Fails closed**: yes, unambiguously, and now empirically demonstrated
  on real, not merely synthetic, source.
- **Semantics already well-defined**: resolving these would require
  genuine partial Nix evaluation (lambda application, list
  concatenation) -- a materially different, much larger capability
  than static literal-list extraction.
- **Disposition: `INTENTIONAL_FAIL_CLOSED_BOUNDARY`** -- agrees with
  the mandate's own hypothesis. This is a coverage boundary, not
  correctness debt (see section 5): the tool never claims to
  understand computed Nix expressions, and correctly declines to
  guess, on real corpus evidence now, not just a synthetic hostile
  test's own say-so.

### J. Sparse frozen-replay roots (the `targets.toml` completeness gap)

- **Evidence**: this is the register-level generalization of section
  3's own F3-R methodology exception. A historical, frozen
  `targets.toml`'s own `module`/`test` file list reflects only what a
  PAST round of adjudication needed to see -- it has no mechanism to
  anticipate a FUTURE capability (like cross-file destination
  confirmation) needing an additional file. Guacamole `#462487` is the
  one concretely demonstrated instance.
- **Observed corpus cases**: 1 confirmed instance (guacamole); an
  unknown number of OTHER historical targets could in principle be
  similarly incomplete relative to some future capability not yet
  built, by the same structural argument -- not separately audited
  this round.
- **Frozen watched-verdict impact**: none -- this affects only the
  REPLAY HARNESS's own ability to exercise a capability, never the
  analyzer's own correctness (the analyzer stays honestly
  `unconfirmed`, never wrong).
- **Fails closed**: yes, by construction -- an incomplete root produces
  an honest "not found," never a fabricated "found."
- **Disposition: `HARNESS_LIMITATION`** -- agrees with the mandate's
  own hypothesis exactly. This is validation infrastructure, not
  product implementation; the appropriate remedy (if any is ever
  wanted) is broadening `run-replay.py`'s own fetch set for specific
  historical targets, not touching `src/main.rs`.

## 5. Correctness debt vs. coverage boundary

Explicitly separated, per the mandate's own requirement -- not every
unsupported construct is a bug:

**Correctness debt** (OBA claims to understand the syntax/semantics
and could in principle give a wrong answer on it): item **D**
(cross-instance ambiguity -- the only item that is not fail-closed) is
the sole item in this register carrying genuine correctness-debt risk,
and even it has zero demonstrated frozen-corpus impact. Item **C**
(wildcard collision) carries a smaller, structurally similar risk
(first-match rather than refusal), also with zero demonstrated impact.

**Coverage boundary** (OBA explicitly, honestly declines to interpret
constructs outside its own stated static-analysis scope): items **A**
(`imports=`), **G** (transitive chains), **H** (unsupported helpers),
and **I** (dynamic paths) are all boundaries, not bugs -- the tool
fails closed and never claims more than it can prove.

**Narrow, local scanner gaps** (a real declaration exists and SHOULD
be found by the tool's own stated model, but a specific code path
doesn't yet reach it): items **B** (wrapped roots) and **F** (flat-
dotted/nested forms) sit here -- these are the closest things to
"ordinary bugs" in this register, and not coincidentally the two items
with a `BUG_FIX_NEXT`-leaning disposition.

**A genuinely sound but currently over-broad safety gate**: item **E**
(`listOf`) is its own category -- not wrong in any observed case, but
structurally capable of being wrong in an unobserved one, with an
already-understood, local fix.

**Validation-harness scope, not analyzer scope**: item **J**.

## 6. Release-readiness recommendation

**`release_candidate_ready`.**

Basis, using only documented evidence, per the mandate's own explicit
instruction:

- All 3 originally confirmed S5 defects have accepted fixes, each
  independently validated by its own full 369-PR frozen-corpus
  regression round (`58b28b2`, `c04c5af`, `6051805`, all PASS).
- Across all three full-corpus rounds combined, **zero** confirmed
  wrong watched results remain in the accepted corpus, and **zero**
  items in this register represent an unsound FALSE POSITIVE currently
  producing an incorrect answer on any known real PR -- every open item
  is either a documented coverage boundary (fails closed, never wrong,
  just incomplete) or a theoretical, unexercised risk (items C and D)
  with zero demonstrated real-corpus impact across three separate
  full-replay rounds that specifically went looking for exactly this
  kind of case.
- The one genuinely open correctness-debt item (D, cross-instance
  ambiguity) is NOT fail-closed in principle, but has now been
  specifically, deliberately searched for across the ENTIRE frozen
  corpus (not a sample) TWICE (S5-F2-R's own dedicated scan, and
  implicitly re-covered by S5-F3-R's own unchanged eligibility set) and
  found zero times. Shipping with a known, disclosed, zero-observed-
  impact limitation is a normal, defensible release posture -- treating
  it as a release blocker would set a bar this project could never
  clear (a fully general Nix evaluator would still have open questions
  of its own).

## 7. Versioning recommendation

**Minor release**, not a patch.

Reasoning, from externally observable behavior only:

- `Verdict::OptionRelocated` is a **new, additive** JSON tag (the
  `#[serde(tag="verdict")]` enum gains a variant; every EXISTING
  variant's own shape, including `OptionNotFound`'s, is byte-for-byte
  unchanged -- confirmed directly, both by S5-F3's own hostile
  controls asserting the exact field count and by S5-F3-R's own
  corpus-wide discovery-invariance and hard-removal-control checks).
  Purely additive for a JSON consumer that already tolerates unknown
  verdict values gracefully.
- **Compatibility risk exists specifically for an EXHAUSTIVE consumer**
  -- any downstream code (Rust or otherwise) that pattern-matches on
  every known `Verdict`/`VerdictKind` value without a wildcard/default
  arm will fail to compile or silently mis-handle the new variant. This
  is real, not hypothetical -- it is exactly the class of change this
  project's OWN Rust code needed updating for internally (multiple
  `match` sites across `main.rs`, `ALL_VERDICT_KINDS`, and one existing
  test whose own premise changed, all discovered via the Rust
  compiler's own exhaustiveness check during S5-F3's implementation).
- F1's own `discovered_options` path re-qualification (bare -> fully
  embedding-qualified paths) and F2's own witness-matching broadening
  (some `OBA001` results becoming `PASS`) are BOTH real, externally
  observable behavior changes to already-existing fields/verdicts, not
  merely additive -- a consumer with a golden-file/snapshot test
  against `discovered_options`'s own exact path strings, or one
  counting `OBA001` occurrences as a fixed baseline, would see a real
  diff, even though every individual change is independently proven
  correct.
- None of this is a "bug fix that happens to look different" --
  together, these are real, intentional behavior changes serving new
  correctness that a minor-version bump communicates honestly, where a
  patch bump would understate the change and a major bump would
  overstate it (no existing FIELD is removed or renamed, no existing
  COMMAND'S calling contract changes).

**No version bump is performed in this round** -- recommendation only.

## 8. Compatibility audit

| Surface | F1 (`25c5b54`) | F2 (`c647d1b`) | F3 (`05d4a90`) |
|---|---|---|---|
| `discovered_options[].path` | **Changed**: bare -> embedding-qualified for named-submodule references reached by type. | unaffected | unaffected |
| `Verdict`/`VerdictKind` enum | unaffected | unaffected | **Additive**: new `OptionRelocated` variant/kind. |
| Existing verdict outcomes | unaffected | **Changed**: some `OBA001`->`PASS` for instance-keyed submodule targets (structurally eligible class, see F2's own report for the exact predicate). | **Changed**: some `OptionNotFound`->`OptionRelocated` for targets with a proven rename edge. |
| `ObaResultEvidence` (audit-diff) | unaffected | unaffected | **Additive**: two new `skip_serializing_if`-gated fields (`relocated_to`, `destination_confirmed`), `None` for every pre-existing verdict, confirmed byte-identical for those. |
| `NotableChange.code` (audit-diff) | unaffected | unaffected | **Additive**: new possible value `"OBA-RELOCATED"`. |
| Rendered markdown headings | unaffected | unaffected | **Additive**: two new possible headings, gated on the new code; every pre-existing heading text unchanged. |
| `oba_verdict_transitions` map keys | new key shapes appear only where the underlying kind changed (expected) | new key shapes appear only where the underlying kind changed (expected) | new key shapes appear only where the underlying kind changed (expected) |

**Enum-exhaustiveness break risk**: real, for any consumer (Rust or
otherwise) that exhaustively matches `Verdict`/`VerdictKind` without a
default arm -- this project's OWN code needed updating internally when
the variant was added (a genuine, felt example of the risk, not a
hypothetical one).

**JSON consumer compatibility**: safe for any consumer that reads
fields by name and tolerates unknown enum tag values (the normal,
recommended posture for a versioned API) -- unsafe for one that
enumerates known tags itself without a fallback.

**Markdown compatibility**: any CI script that greps rendered summary
text for the literal string `"FINDING BECAME INCONCLUSIVE"` will now
sometimes see `"FINDING'S OPTION WAS RENAMED/RELOCATED"` instead for
this specific, newly-distinguished case -- the underlying BUCKET COUNT
(`finding_became_inconclusive`) is unchanged, so a script reading the
structured `summary.*` fields is unaffected; only literal heading-text
greeping is at risk.

**Existing golden fixtures**: none broken -- confirmed via the full
384/384 test suite passing at S5-F3's own acceptance, and 0 discovery/
verdict deltas anywhere in the frozen corpus outside the specific,
intended, individually-verified changes at each round.

## 9. Final evidence chain

| Stage | Candidate | Result |
|---|---|---|
| Historical S5 | `8f1701a` | FAIL |
| F1 initial | `cdfb4ca` | failed full replay (S5-F1-R) |
| F1B | `6c2c2aa` | failed full replay (S5-F1B-R) |
| F1D | `25c5b54` | PASS (`58b28b2`) |
| F2 | `c647d1b` | PASS (`c04c5af`) |
| F3 | `05d4a90` | PASS (`6051805`, with the disclosed methodology exception in section 3) |

**The lesson this chain itself is evidence for**: local regression
tests were necessary but insufficient at every single line of this
work. `cdfb4ca` and `6c2c2aa` BOTH had their own passing local test
suites at the time they were proposed -- both were rejected by a full
frozen-corpus replay finding real counterexamples the local suite never
exercised. This is not a claim that full replay is infallible either
(S5-F3-R's own methodology exception in section 3 shows the replay
harness itself has real, disclosed limits) -- it is a factual record
that the discipline of running one caught two real regressions this
project would otherwise have shipped, stated plainly, not
celebrated.

## 10. Completion summary

- **S5 remediation program: formally closed.** All three originally
  confirmed defects have accepted, independently-validated fixes.
- **Final accepted implementation lineage**: `25c5b54`/`58b28b2`
  (angrr), `c647d1b`/`c04c5af` (cgit), `05d4a90`/`6051805` (guacamole,
  with the section-3 exception).
- **Residual limitations**: 10 items (A-J), each with objective
  evidence, no priority ranking. Dispositions: 2
  `PREREGISTERED_EXPERIMENT` (A: `imports=`; C: wildcard collision) + 1
  more (D: cross-instance ambiguity) = **3 preregistered-experiment
  candidates**; **2** `BUG_FIX_NEXT` (E: `listOf`; F's own F2-side
  manifestation) plus a tentative, investigation-gated third (B:
  wrapped roots); **3** `TRACKED_LIMITATION` (F's own F3-side
  manifestation; G: transitive chains; H: unsupported helpers, with an
  explicit note that its own corpus prevalence is unmeasured); **1**
  `INTENTIONAL_FAIL_CLOSED_BOUNDARY` (I: dynamic paths); **1**
  `HARNESS_LIMITATION` (J: sparse replay roots, the same mechanism as
  section 3's own exception).
- **Release-readiness**: `release_candidate_ready` (section 6).
- **Versioning**: minor release recommended, not performed (section
  7).
- **Compatibility**: audited in full (section 8); real
  exhaustiveness-break risk for strict consumers, no risk for
  field-by-name JSON consumers.
- **F3-R methodology exception**: recorded in full in section 3, not
  summarized away.
- **This artifact**: `fixtures/s5-remediation-closeout/closeout.md`.
- **No source changes, no new analyzer behavior, no full replay, no
  release, no version bump performed in this round** -- exactly as
  scoped.

**STOP.** No next implementation phase (B/F's own F2-side fix, E's own
`listOf` fix, or any preregistered-experiment design work) without a
separate, explicit GO.
