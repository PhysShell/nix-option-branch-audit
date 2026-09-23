# S5-F1D-R: full frozen-corpus acceptance replay of `25c5b54`

This document is a validation artifact only. It does not rewrite,
supersede, or reinterpret any prior historical result. Historical S5
(`28a1d58`, FAIL), S5-F1 (`cdfb4ca`), S5-F1-R (`baaaec9`/`f0a54a3`,
FAIL), S5-F1B (`6c2c2aa`), S5-F1B-R (`44edf38`, FAIL), and S5-F1C
(`d272ccf`/`2054cbb`/`26acad9`/`f94959f`) all remain exactly as
committed. This is the first full acceptance replay after replacing
exclusion/hop-count heuristics with provenance-qualified recursive
discovery.

## Final verdict

```json
"final_verdict": "PASS"
```

`25c5b54` (the F1D implementation commit -- the binary under replay was
built from this commit specifically, not from `892231a`, the
documentation-only commit that follows it; `git diff 25c5b54 892231a --
src/` is empty, confirmed) satisfies the frozen S5 acceptance corpus
under the new provenance-qualified discovery model.

**This PASS does not mean**: complete Nix semantics are supported;
wildcard collision policy is solved; cgit/guacamole are fixed;
`run_target`'s `.find()` is universally correct. All four are
explicitly out of scope and unaddressed, disclosed below.

## Candidate provenance

- Source commit: `25c5b5468c94f87231eed115e6eba5ad6118e9ff`.
- Built from a genuinely clean checkout: fresh `git clone` to
  `/home/tandem/f1dr-clean-checkout/repo` (outside `/tmp`'s tmpfs, after
  a real disk-exhaustion failure earlier in this line of work), `git
  checkout 25c5b54`, confirmed `git status --short` empty.
- Toolchain: `rustc 1.100.0-nightly (8925ea358 2026-08-20)`,
  `nightly-2026-08-21-x86_64-unknown-linux-gnu`.
- Build command: `CARGO_TARGET_DIR=/home/tandem/f1dr-clean-checkout/target
  rustup run nightly-2026-08-21-x86_64-unknown-linux-gnu cargo build --release`.
- Binary SHA-256: `3e2b180f07f3b8352f3faf668ac4d93c7198992e52c7cbff47494d5df07fb8f9`.
- No `cp`-based restoration was used anywhere in this round's own
  build or in any A/B comparison -- the clean-checkout binary above was
  used directly for every comparison. Full details:
  `candidate-provenance.json`.

## Coverage: 369/369, derived and verified

The 189/180 partition was independently re-derived from the raw ledger
(`derive-applicable-manifest.py`) -- 369 total, 180 applicable, 189
non-applicable, byte-identical to the frozen S5-F1-R/F1B-R selection
(same PR set, same ordering, same frozen identities). All 180
applicable PRs were fully replayed against the clean-checkout `25c5b54`
binary: real content re-fetched at the exact frozen `base_sha`/
`head_sha`, each PR's original committed `targets.toml` reused
verbatim. **0 fetch failures, 0 `window_evaluation_incomplete`.**

## Comparison result

- **163 / 180 byte-identical** to the historical v0.4.5 output.
- **17 / 180 changed** -- down from 22 under `6c2c2aa` (S5-F1B-R/F1C):
  `#506644`, `#508427`, `#428153`, `#427260` (4 of the 7 former
  regressions) and `#397967` (the already-adjudicated fedimintd
  finding) are now fully byte-identical to historical, including
  `discovered_options`, not merely the watched verdict.

## The comparison methodology: representation change is not automatically regression

Per this round's own explicit requirement, raw byte identity of
`discovered_options` is NOT the acceptance bar -- F1D deliberately
changes a promoted declaration's own recorded `path` from an
incorrect bare form to one qualified by its real embedding. A new
script, `reconcile-discoveries.py`, performs **declaration-level
reconciliation keyed on source SPAN** (file, line, col) rather than
path: the same literal `mkOption {...}` call produces the exact same
span in both the historical and the candidate output (both parse the
exact same, frozen, live-fetched source text), regardless of what path
either binary recorded for it. Every span present on both sides is
classified as `same_identity` (same path too) or `requalified`
(different path, same source declaration -- F1D's own fix doing its
job); every span present on only one side is `newly_reachable`,
`removed_false_duplicate`, `unexplained_loss`, or `unexplained_gain`.

Result across the 17 changed PRs' own 34 sides:

- **450 spans**: `same_identity`.
- **401 spans**: `requalified` -- proven, not assumed, to be the exact
  same source declaration, now correctly embedded.
- **0 spans**: `newly_reachable` -- expected: historical v0.4.5, having
  NO exclusion mechanism at all, already discovered every named
  binding's own content unconditionally, so there is no such thing as
  "newly reachable relative to v0.4.5" (only relative to a
  hop-capped candidate like `6c2c2aa`, which this round does not
  compare against directly).
- **0 spans**: `removed_false_duplicate`, `unexplained_gain`.
- **37 spans**: mechanically flagged `unexplained_loss` -- resolved to
  **0 genuinely unexplained** after hand tracing (below).

## Provenance-conservation reconciliation: the 37 flagged spans

All 37 trace to exactly one of two disclosed mechanisms, documented in
`classification-notes.json`:

**36 spans, 6 PRs (`#438285 #433539 #423934 #401840 #398993 #480839`)**:
the already-known, pre-existing frigate.nix/nvidia-container-toolkit
`walk_merge_operands`-wrap gap (a file's own true-root `options =`
value wrapped in `with`/`let...in`, unhandled since before F1 existed
at all -- Pass 1 never discovers a true-root declaration in these
files, so F1D's own reference-following, which starts FROM a true
root, never runs for them either). Disclosed in S5-F1B's own
completion report; confirmed unchanged by F1D specifically.

**1 span, PR `#471312` (angrr)**: a **genuinely new finding**, surfaced
by this round's own more rigorous span-based reconciliation, that the
earlier, cruder path-SET comparison (used in S5-F1-R/F1B-R) could not
see. `commonPolicyOptions` (angrr.nix line 95) is referenced ONLY via
`imports = [ commonPolicyOptions ];` (from both `temporaryRootPolicyOptions`
and `profilePolicyOptions`) -- never via any `type =` field. NONE of
F1/F1B/F1C/F1D's own named-binding reference-following mechanism ever
looks at `imports =` (a separate NixOS module-merge mechanism, distinct
from a type reference) -- this content has been unreachable since
S5-F1's own first fix (`cdfb4ca`), confirmed by checking F1-R's own
frozen `comparison-report.jsonl` for this exact PR, whose own
`check-head.json` diff already shows this same gap, three rounds ago.
It was previously invisible specifically because angrr's own real
top-level `enable` (always discovered) and `commonPolicyOptions`'s own
`enable` (now missing) reduce to the SAME bare path -- a plain
path-SET diff cannot distinguish "the one that's still there" from
"the different one that's now gone." Zero impact on angrr's own
watched verdict (`period`, unaffected) or any other PR in this corpus
(verified: no target in the frozen 180-PR set watches anything
reachable only via `imports =`). Classified `known_pre_existing_limitation`
-- disclosed as a genuinely new investigation finding, not silently
folded into "already known."

**Result: 0 genuinely unexplained span-level deltas remain.** No lost,
invented, merged, or substituted declaration went unaccounted for.

## Mandatory anchors

**A. Angrr `#471312`** -- corrected: `predicate_not_found ->
option_not_found`, Changed. Verified directly from the replay evidence
that `temporaryRootPolicyOptions.period` IS present in
`discovered_options`, at its own real, qualified path
(`settings.temporary-root-policies.period`) -- it does not remain
correct because the nested declaration was hidden; it is correct
because its qualified identity can never equal the bare watched query.
This is the decisive proof of F1D's intended mechanism, confirmed from
the live replay, not merely the committed synthetic fixture.

**B. Seven former F1 regressions** -- all restored: `#431289`,
`#440660` (requalified, zero unexplained deltas); `#506644`, `#508427`,
`#428153`, `#427260`, `#397967` (fully byte-identical to historical).
`#397967`'s own already-adjudicated finding confirmed visible.

**C. Rspamd second hop** -- recovered, confirmed directly in the live
replay for `#484133`: `workers.bindSockets.socket`/`.mode`
(`bindSocketOpts`'s own leaves) newly present at their own qualified
path.

**D. Tayga second hop** -- recovered, confirmed directly in the live
replay for `#432528`: `ipv4.pool.prefixLength` and
`ipv6.pool.prefixLength` (`addrOpts`'s own leaves) newly present at
their own qualified path -- not merely a unit-test claim.

**E. Scoped `with`** -- intact (part of the same replay mechanism
that restores `#429967`/`#494314`/`#431289`/`#440660`/`#260551`/`#484133`).

**F. Portmaster alias chain** -- intact, confirmed via `#557329`: all 7
of `packageMatchType`'s own leaves, reached through the
`profilePackageType` alias step, fully requalified, zero unexplained
deltas.

**G. K3s ordering** -- confirmed directly: both historical and
candidate `discovered_options` arrays for `#374017` are in
span-sorted (source-position) order, same length (23), same content.
The pre-existing `enable` collision this PR exercised is now GONE
(requalified to distinct paths, not merely reordered) -- per this
round's own explicit instruction, order among declarations that no
longer share a comparison domain is not required to be "preserved"
(there is nothing left to order relative to each other); order among
everything else holds exactly.

## Fourteen former discovery deltas -- individually re-classified

| PR | classification |
|---|---|
| `#429967` `#494314` `#484133` `#557329` `#374017` `#260551` `#432528` `#463443` | `provenance_requalification` (8) |
| `#438285` `#423934` `#401840` `#398993` `#480839` `#433539` | `known_pre_existing_limitation` (6) |

Every one individually traced (`classification-notes.json`): source
declaration -> reference chain -> accumulated path -> historical vs.
F1D identity -> effect on watched query. None labeled cosmetic without
declaration-level (span) evidence; no "same root cause as angrr"
hand-waving applied anywhere.

## Eight real leaf-collision PRs -- re-checked from the full replay

Confirmed via `scan-leaf-collisions.py` run against the **complete
180-PR replay** (not a targeted spot-check): **exactly 1** collision
finding remains in the entire corpus.

| PR | historical | F1D-R |
|---|---|---|
| `#429967` `#494314` `#431289` `#440660` `#260551` `#463443` `#374017` | collision | **resolved** (7/7) |
| `#415326` (wstunnel) | collision | **unresolved**, `pre_existing=true` |

For all seven resolved cases, the reconciliation data proves resolution
is caused by distinct qualified embeddings (each colliding
declaration's own content now prefixed by its own real referencing
option's name) -- not a new ranking/order policy. `run_target`'s own
`.find()` is byte-for-byte unmodified in this round.

`#415326` remains unresolved because its own `option_prefix`
(`[services, wstunnel, clients, "*"]`) is a wildcard prefix -- F1D's
own reference-following mechanism is deliberately gated off for
wildcard prefixes (matching kimai's own established S5-F1B precedent).
F1D did not create this collision (confirmed `pre_existing=true`,
identical to historical) and did not worsen it. Kimai's own wildcard
behavior is independently confirmed intact
(`two_roots_are_analyzed_independently_real_kimai_transition` passes).
This remains a separate, disclosed, unresolved architectural question,
not claimed solved.

## Cgit and guacamole

Both confirmed **byte-identical to historical** in the full 180-PR
replay (not a targeted spot-check) -- `#475112` and `#462487` are
`complete`/`identical: true` in `comparison-report.jsonl`. Deliberately
untouched by F1D, as required.

## Fatal/error behavior

Unchanged: 0 `parse_errors` entries across all 180 PRs x 2 sides (360
check files scanned); only `"changed"`/`"unchanged"` diff kinds
observed across all 180 `raw.json` outputs (0 error/fatal/tool-error
kinds), matching historical's own 0-error record exactly.

## Reproducibility

- `applicable-manifest.json` independently re-derived from the raw
  ledger, byte-identical to the frozen selection.
- `compare-replay.py`, `reconcile-discoveries.py`, and
  `classify-deltas.py` each re-run a second time against the same
  committed replay data: byte-identical output on every file
  (`comparison-report.jsonl`, `provenance-reconciliation.json`,
  `delta-classification.json`) confirmed via direct diff.
- `scan-leaf-collisions.py` re-run, stable.
- New CI workflow `.github/workflows/s5-f1d-r.yml` enforces all of the
  above reproducibility checks on every future push touching this
  directory.

## Full repository test suite / CI

`cargo test --release` (working tree, guaranteed fresh build): 356/356
passing, 0 new warnings. CI for this round's own artifact commit:
reported after push.

## Files added / changed this round

`fixtures/s5-f1d-r/{applicable-manifest.json, candidate-provenance.json,
derive-applicable-manifest.py, run-replay.py, compare-replay.py,
reconcile-discoveries.py, classify-deltas.py, scan-leaf-collisions.py,
classification-notes.json, comparison-report.jsonl,
provenance-reconciliation.json, delta-classification.json,
leaf-collision-scan.json, f1d-r-summary.json, f1d-r-report.md (this
file), replay/<cohort>/<pr>/{raw.json,check-base.json,check-head.json,
summary.md,command.txt} for all 180 applicable PRs}`,
`.github/workflows/s5-f1d-r.yml`. No historical or frozen artifact
(`s5-live-pr-shadow/`, `s5-f1-r/`, `s5-f1b-r/`, `s5-f1c/`, `s5-f1d/`)
touched.

## Unresolved limitations (explicitly stated, not silently dropped)

- The `imports =`-based reference gap (angrr's own `commonPolicyOptions`)
  is real and, while inert for this frozen corpus, not fixed -- no
  mechanism in F1/F1B/F1C/F1D follows module-system `imports =`
  chains, only `type =` field references.
- The frigate.nix/nvidia-container-toolkit `walk_merge_operands` wrap
  gap (`with`/`let...in`-wrapped true-root `options =` values) remains
  unfixed, unrelated to F1D, disclosed since S5-F1B.
- The wildcard-prefix collision/first-match question (`#415326`)
  remains open, unaddressed, explicitly not claimed solved.
- Cgit `#475112` and guacamole `#462487` remain deliberately unfixed.
- General Nix evaluation, dynamic/computed attribute access, and any
  reachability analysis beyond static lexical reference-following
  remain entirely out of scope.

## Scope discipline

No implementation change, no fixture rewriting, no historical ledger/
adjudication mutation, no cgit fix, no guacamole fix, no wildcard-policy
redesign, no version bump, no release performed in this round. Only
replay machinery, immutable replay artifacts, reconciliation/reporting,
and integrity tests for that machinery were added, per this round's own
scope.

**STOP.** No implementation changes, release, cgit/guacamole fix,
wildcard-policy redesign, or next phase without a separate, explicit GO.
