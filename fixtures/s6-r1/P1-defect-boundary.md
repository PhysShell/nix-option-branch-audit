# S6-R1 P1: defect boundary characterization -- `BOUNDARY_IDENTIFIED`

**Parent**: `fixtures/s6-r1/P1-defect-confirmation.md` (`416d2ce`),
treated as closed fact per this round's own explicit scope -- NOT
re-adjudicated here, real-world corpus NOT expanded. This document
only adds synthetic, black-box boundary work.

**Scope boundary honored**: no change to `oba`, no edit to
`README.md`, no P2 work, no automatic move into GO-B (harness-repair)
-- this document stops at the boundary finding itself.

## Method discipline (why the order matters)

Every fixture below was built and run as a black box FIRST. `src/main.rs`
was not opened until after step 6 (the minimal PASS/FAIL pair) was
already isolated and recorded -- avoiding exactly the "see the code,
get the insight, build confirming experiments" failure mode the user
flagged explicitly. The behavioral matrix in the next section is in
the actual chronological order the experiments were run.

## Setup

Reused the same sha256-verified `oba` `v0.5.0` binary already verified
in `fixtures/s6-r1/P1-defect-confirmation-evidence/`. Built fresh
synthetic fixtures (not reusing or polluting that directory) under
`fixtures/s6-r1/P1-defect-boundary-fixtures/`, modeled on the project's
own existing minimal synthetic style (`fixtures/synthetic/
h2-nested-submodule-collision/`).

## Behavioral matrix (chronological order run)

| # | Fixture | `option_prefix` | `watch` | `freeformType` | Result |
|---|---|---|---|---|---|
| M0 | `outer = mkOption{submodule{options.inner=mkOption{...};};}` | `[...,"outer"]` | `["inner"]` | absent | **FAIL** -- `discovered_options: []`, `OptionNotFound` |
| M2 | flat dotted, NOT inside any submodule at all: `outer.inner = mkOption{...}` directly in the top `options={}` block | `[...,"outer"]` | `["inner"]` | n/a | **FAIL** -- identical to M0 |
| sanity | same module as M0/M2 | `[...]` (stops before `outer`) | n/a (raw scan) | absent | `discovered_options` **correctly lists** `['outer','inner']` -- the walker itself finds the leaf fine |
| M3 | same module as M0 | `[...]` (stops before `outer`) | `["outer.inner"]` (one dotted string) | absent | **PASS** -- real `OBA001` finding, declaration discovered |
| M6 | same module as M0 | `[...]` (stops before `outer`) | `["outer","inner"]` (TWO array elements) | absent | **FAIL**, but differently -- each element evaluated as its OWN independent leaf (`outer`->`PredicateNotFound`, `inner`->`OptionNotFound`); confirms `watch` items are independent lookups, not a path to concatenate |
| M1-deep | M0's module + `freeformType` added | `[...,"outer"]` | `["inner"]` | **present** | **FAIL** -- identical to M0 (freeformType changes nothing) |
| M1-shallow | same module as M1-deep | `[...]` (stops before `outer`) | `["outer.inner"]` | **present** | **PASS** -- identical to M3 (freeformType changes nothing here either) |

**The `freeformType` hypothesis from the prior round is REFUTED**: M0
(no `freeformType`) already fails identically to M1-deep (`freeformType`
present); M3 (no `freeformType`) already passes identically to
M1-shallow (`freeformType` present). `freeformType` has no observable
effect in this matrix.

## Minimal PASS/FAIL pair

- **FAIL**: `option_prefix` ends in the submodule-wrapped container
  option's own name (`"outer"`), `watch = ["inner"]` (bare leaf).
- **PASS**: `option_prefix` stops ONE level earlier (before `"outer"`),
  `watch = ["outer.inner"]` (one dotted string spanning the container's
  own name AND the leaf).

These two differ by exactly one factor: **where the `option_prefix`/
`watch` split point is placed relative to the submodule-wrapped
container option's own name** -- not nesting depth, not `freeformType`,
not the type combinator, not whether the module uses the flat-dotted
or nested `options={}` idiom.

## Mechanism (inspected only after the pair above was fixed)

`scan_options` (`src/main.rs`) only treats a node as a walkable "root"
for a given `option_prefix` in two cases: (1) a nested `options = {...}`
block whose accumulated path, reached through `walk_options_block`'s
own recursion, exactly equals `option_prefix`, or (2) a flat top-level
declaration `options.<...> = {...}` whose own segments, stripped of the
leading `"options"`, exactly equal `option_prefix`
(`segs[1..] == option_prefix`). Neither case matches an `option_prefix`
that names an INTERIOR option declared via its own `mkOption{...}` call
with a submodule type (`"outer"`/the real `"settings"`) -- that name is
not itself a literal `options={}` root, it is a LEAF the walker only
reaches by first finding the true root (`services.synthBoundary`) and
then recursing into `find_nested_options_block`'s own result. Because
no root ever matches the over-long `option_prefix`, `scan_options`
returns nothing for that `option_prefix` at all (confirmed:
`discovered_options: []` in the FAIL cases is not "found but didn't
match `watch`" -- it is "found nothing under this `option_prefix` to
even search"). `option_prefix_relative_path`'s own doc comment already
anticipates exactly the shape that DOES work (`option_prefix` + a
single dotted key spanning the leaf in one hop) -- it does not, and by
its own stated design was never meant to, cover `option_prefix` itself
terminating mid-way through an interior submodule-typed option's name.

**This is not a nesting-depth or type-combinator limitation of the
discovery walker itself** -- the walker demonstrably finds `outer.inner`
fine (see the `sanity` row). It is a constraint on how `option_prefix`
must be constructed: it must name an actual options-block root, never
an interior submodule-wrapped option's own name.

## Real-world precedent already in this repository (found while mapping to GAP-4, not part of the synthetic matrix itself)

This project's own existing, long-frozen real targets already follow
the PASS-side convention, not the FAIL-side one, for structurally
identical real-world 2-hop submodule leaves -- e.g. kimai's real
`database.socket` (`targets/clean.toml`, `targets/findings-only.toml`,
`targets/golden.toml`, `targets/s4f1-*.toml`): `option_prefix = [
"services", "kimai", "sites", "*"]` (stopping BEFORE kimai's own
`database` option), `watch = ["database.socket"]` (one dotted string
spanning the container's name and the leaf) -- exactly the PASS-side
shape from this matrix, not the FAIL-side shape
`fixtures/s6-r1/P1-adjudication-and-accounting.md`'s own target
construction used for `#568429`/`#508090`
(`targets/s6-r1-p1.toml`: `option_prefix = [..., "settings"]`,
`watch = ["server.port"]` -- the FAIL-side shape in this matrix).

**Stated plainly, as a factual observation this stage is responsible
for surfacing, not for acting on**: the P1 target construction for the
4 `MISSED_FINDING` targets used an `option_prefix`/`watch` split that
diverges from this project's own already-established, already-working
convention for the exact same structural shape (a submodule-wrapped
container option, itself declared via `mkOption`, containing a further
nested leaf). Both the first and the independent second reviewer
adjudicated the SAME already-frozen target record (Phase A target
construction happened once, before either adjudication) -- so neither
review re-derived or re-checked the `option_prefix`/`watch` split
itself; both reviews were "independent" only with respect to judging a
given fixed target against a given fixed tool output, not with respect
to whether that target was itself constructed correctly.

**What this document does NOT do**: it does not re-adjudicate
`#568429`/`#508090`, does not retroactively change `CONFIRMED_NEW_DEFECT`,
does not touch `target-construction-protocol.md` or any other GO-B-scoped
file, and does not test whether reconstructing those two real PRs'
targets with the kimai-style split would actually discover the real
declarations (that would be expanding/re-touching the real-world
corpus, explicitly out of this round's own scope). It only establishes,
on synthetic fixtures plus an existing-precedent citation, that such a
reconstruction is a live, well-evidenced possibility that the next
stage should weigh.

## GAP-4 mapping

None of the three offered categories (narrower GAP-4 subclass; broader
nested-options defect GAP-4 partially covers; a separate class
README incorrectly lumps with GAP-4) fit cleanly, because the evidence
here points at a **target-construction convention mismatch**, not
necessarily a gap in GAP-4's own fix at all -- the walker's own
discovery mechanism, exercised correctly (per this repo's own existing
precedent), finds exactly the shape GAP-4 claims to fix, with no
`freeformType`-related or depth-related exception found in this
matrix. This document does not conclude that GAP-4 is fully correct in
general (only this specific minimal shape and its `freeformType`
variant were tested) -- only that THIS specific confirmed finding's
own root cause, as far as synthetic evidence can establish it, is not
yet distinguishable from "the target was constructed wrong," and that
distinguishing the two conclusively would require revisiting the real
`#568429`/`#508090` targets -- real-world-corpus work this round was
explicitly told not to do.

## Final verdict

**`BOUNDARY_IDENTIFIED`** -- a minimal, single-factor-differing
PASS/FAIL pair was found and the mechanism behind it explained from
source, after the fact, as instructed. The GAP-4 mapping itself is
inconclusive for the reason stated above, and that inconclusiveness is
itself the headline result of this stage, not a failure to produce
one.

## What did not happen

No fix attempted. No `README.md` edit. No change to `target-construction-protocol.md`,
`pilot-accounting.py`, or `population-and-sampling.py`. No re-adjudication
of `#568429`/`#508090`. No real-world corpus expansion. No automatic
transition into GO-B (harness-repair) -- deliberately kept orthogonal,
per the user's own explicit instruction, even though this finding is
obviously relevant to it.

**`S6_R1_P1_BOUNDARY_IDENTIFIED_TARGET_CONSTRUCTION_CONVENTION_IMPLICATED`**

**STOP.** No fix. No README edit. No P2. No automatic harness-repair.
Next steps (GO-B harness-repair, and/or revisiting whether `#568429`/
`#508090` were constructed per this project's own established
convention) each require their own separate, later, explicit GO.
