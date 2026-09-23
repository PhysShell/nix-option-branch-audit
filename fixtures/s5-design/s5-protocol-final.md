# S5 protocol — FROZEN, preregistration/freeze round only

This document is the operative S5 protocol, replacing the earlier
`s5-protocol-draft.md` (kept for history, now superseded). It was
written during the S5 preregistration/freeze round that followed the
design-only round committed at `51418df`. That round corrected two
real problems found in `51418df`'s own headline numbers (a latent
fail-open completeness bug in the historical feature extractor, and a
statistically invalid presentation-level unit for the precision
confidence bound), discovered a third real finding along the way (the
originally-planned population window was measured and found to supply
nowhere near enough real material, §5.1), and froze the exact
population, cohort orders, and gate this document describes.

**This document, and everything committed alongside it in this round,
authorizes NOTHING beyond the freeze itself.** No `oba` binary has been
invoked on any fresh S5 PR. No adjudication has occurred. A separate,
explicit GO is required before real S5-A/S5-B data gathering or
adjudication begins.

## 0. Baseline binary identity

The analyzer version S5 measures, frozen and immutable for the
duration of this round:

- **Release**: `v0.4.5` (published, GitHub Attestation-verified at
  release time)
- **Commit**: `8f1701a289ddab8bc23f23a25cc0937863fa0356`
- **Artifact**: `oba-x86_64-unknown-linux-musl.tar.xz`
- **SHA-256**: `c07bed6c37fa3e0f1885099bb3dfc7a7b741531e8a156dc4fa8a7a160bd45800`
  (from the release's own `sha256.sum` asset, fetched fresh via
  `gh release download v0.4.5`, not copied from memory)

## Estimands (kept separate throughout — never pooled into one number)

Unchanged from the draft:

- **Estimand A** — representative prevalence/applicability (S5-A).
- **Estimand B** — actionable-PR precision on a pre-declared,
  result-blind, mechanically enriched population (S5-B). **Never**
  presented as unconditional precision over all nixpkgs PRs.
- **Estimand C** — false-PASS safety, own denominator, never pooled
  with B.

## 1. Statistical-unit correction (this round's own headline fix)

The PRIMARY statistical unit for S5-B's precision confidence bound is
a **distinct actionable PR**, not an actionable presentation. S4's own
historical corpus produced 9 actionable presentations from only 6
distinct PRs (`#538802` alone contributed 4) — treating presentations
as independent Bernoulli trials for a Clopper-Pearson bound is invalid
given this real within-PR clustering.

- Every individual actionable presentation still receives full
  two-blind-reviewer adjudication, unchanged from S4.
- A single incorrect presentation anywhere is still an immediate
  Tier-1 FAIL, regardless of PR-level bookkeeping.
- An actionable PR counts as **one** correct PR-level observation only
  if **every** actionable presentation within it is independently
  adjudicated correct (`gate.py::ActionablePR.is_correct`).
- The confidence-bound denominator/numerator use this PR-level count,
  never the raw presentation count.

Mechanically regenerated historical reference values (never
hand-typed — `evaluate-rules.py` → `rule-comparison.jsonl`):
**R4 overall S4: 6 distinct actionable PRs / 34 selected PRs. R4
S4-A only: 4 distinct actionable PRs / 22 selected PRs.**

## 2. Frozen selector: `R4_mkoption_line_edit_v1`

Exact rule (`fixtures/s5-design/enrichment-rules.py` and the isolated
production implementation `fixtures/s5-design/selector/production_selector.py`):

> Select a PR iff at least one ADDED or REMOVED diff line in a
> `nixos/modules/services/**` file contains the literal token
> `mkOption` or `mkEnableOption`.

This is a **lexical line-edit detector**, not an option-lifecycle
detector: no AST or structural diffing, no add/remove/rename claim,
cannot distinguish a genuine semantic edit from a pure reformat. A
purely cosmetic edit to a line containing `mkOption`/`mkEnableOption`
selects under this rule — stated explicitly as **acceptable** (it only
changes which PRs get reviewed, never a verdict), not a defect.

Frozen before S5-B's real population was drawn. Not retuned after
real results begin arriving, exactly as `s5-protocol-draft.md`'s own
"no Optional Stopping, extended to the selector" clause already
required.

**Fail-closed completeness** (the actual bug fixed this round — the
historical `extract-features.py` used `f.get("patch", "")`, silently
treating a missing patch as "no edit"; confirmed empirically NOT to
have corrupted the historical 187-PR corpus, but a real risk for any
unvetted fresh PR): a relevant file with a missing/`None` `patch`, or
an unproven-complete 300-file truncation boundary, makes the whole
PR's selection `unresolved` — never silently coerced to `selected =
false`. An unresolved PR is excluded from the S5-B frozen order but
its `selection_record` is retained (real, auditable evidence of the
selector's own refusal to guess). See
`fixtures/s5-design/selector/test_production_selector.py` for the full
hostile-input test suite.

## 3. S5-A — representative cohort

- **Population**: real nixpkgs merge PRs touching
  `nixos/modules/services/**` or `nixos/tests/**`, window
  `2025-03-01` through `2026-09-22` — the satisfying window the
  deterministic backward-expansion procedure landed on (§5); NOT the
  originally-planned `2026-06-01` window, which real measurement found
  insufficient (94 survivors, capacity finding, row 1 of the capacity
  ledger).
- **Sampling**: `seed = int("8f1701a", 16)` (v0.4.5's own dereferenced
  commit short SHA, same convention S4 used with its own frozen
  binary's commit), `random.Random(seed).shuffle(survivors)`, first
  `min(150, len(survivors))`.
- **Target**: n=150. **Realized: 150 / 150** (exact match — the
  satisfying window was defined precisely as the first one reaching
  this).
- **Stopping rule**: process the full frozen sample. No
  actionable-count stopping condition — mathematically unnecessary,
  since S5-A's own estimand (prevalence) is answered by a fixed-n
  sample regardless of how many actionable transitions occur in it.

## 4. S5-B — enriched precision cohort

- **Selector**: frozen `R4_mkoption_line_edit_v1` (§2), applied via the
  isolated production CLI to every PR remaining after S5-A's picks are
  removed from the survivor pool (in the pool's own post-shuffle
  order).
- **Target**: **30 distinct actionable PRs** (§1's corrected unit —
  not 30 presentations).
- **Cap**: **219 selected-and-resolved PRs**, mechanically derived
  (`planning-calculations.py`'s `s5b_selected_pr_cap_planning`): the
  smallest N such that P(Binomial(N, p=6/34≈0.1765) ≥ 30) ≥ 95%,
  reusing `s4gen._binom_sf` (already verified against the 30/30 →
  ~0.9050 reference) rather than a new tail-probability
  implementation. At N=219, P≈95.14% — consistent with, and a
  mechanical confirmation of, this round's own initial ~220/~95%
  estimate.
  **Realized: 219 / 219** (the eligible R4-selected-and-resolved pool
  at the satisfying window was 233, so the cap itself — not pool
  exhaustion — determined the realized S5-B size; if the eligible pool
  had instead been smaller than 219, the realized cap would become the
  whole pool, same precedent as S4-B's own `min(80, 67) = 67`
  handling).
- **Sampling**: `seed = int("8f1701a", 16) + 1`, shuffled over the
  selected-and-resolved-eligible subset, first `min(219, len(eligible))`.
- **Stopping rule**: stop at 30 distinct actionable PRs or the cap,
  whichever comes first. If the cap is exhausted before 30 distinct
  actionable PRs: `INSUFFICIENT EVIDENCE`. **Correctness never affects
  stopping** — `gate.py::should_continue_sampling`'s own signature
  takes no correctness/Tier-1 input at all, by construction.
- **False-PASS review (Estimand C)**: every PASS materially relevant
  to a PR transition, in EITHER cohort, gets the same two-reviewer
  independent adjudication S4 already used.

## 5. Population construction

- **Exclusion set**: union of (a) `fixtures/s4-live-pr-shadow/exclusion-ledger.json`'s
  `excluded_pr_numbers`/`excluded_subject_names`/`excluded_module_paths`/`excluded_test_paths`
  (S1+S2+S3's 130 excluded PR numbers plus names/paths), reused
  verbatim, untouched; (b) every PR number S4 itself examined (187,
  derived mechanically from the S4 ledger's own `pr_summary` records
  in `fixtures/s5-design/population/build-exclusion-addendum.py`,
  cross-checked against `features.jsonl`'s independently-derived PR
  set before being trusted). Total exclusion set size: 317. **This
  exclusion policy is a separate, fixed degree of freedom from the
  window** — it stayed unchanged throughout the entire window-expansion
  search below.

### 5.1 The window-expansion finding

The originally-planned window (`2026-06-01`..`2026-09-22`, the same
window S1-S4 used, on the theory that PR-number exclusion alone
guarantees non-overlap regardless of calendar overlap) was measured
for real and found **insufficient**: only 94 survivors after the
mechanical screen — nowhere near enough to supply both S5-A (150) and
S5-B (needs its remainder pool to yield ≥219 R4-selected PRs). This is
a real, disclosed **population-capacity finding**, not an S4/S5 result
and not an `oba` failure: S1 through S4 had already collectively
sampled/excluded 317 PR numbers, plus a broad 390-name/183+87-path
contamination-exclusion ledger, from largely this same calendar
window, leaving comparatively little real untouched material in it.

Rather than silently redesigning the S5-A/S5-B split or discarding the
finding, the population window was **deterministically expanded
backward** under a frozen, pre-declared rule (`capacity-search.py`):

- End date fixed at `2026-09-22` throughout.
- Start date steps backward in fixed 3-calendar-month quarters from
  `2026-06-01`, with a **hard lookback cap at `2024-01-01`** (an
  explicit final step — a short ~2-month partial step from
  `2024-03-01`, evaluated regardless of whether `2024-03-01` itself
  satisfied the criterion, and never followed by anything earlier).
- All mechanical screens (docs-only, mass-mechanical, window sanity)
  and the exclusion policy stayed completely unchanged at every step —
  only the start date widened.
- At each candidate window: construct S5-A (`seed_a5`, first 150 of the
  shuffled survivor pool), compute the remainder, run the frozen
  isolated `production_selector.py` over it (real diffs, real
  fail-closed handling), and check whether `len(S5-A) == 150 AND
  count(R4-selected-and-resolved in remainder) >= 219`.
- **No `oba` invocation, no adjudication, no outcome data of any kind**
  at any step — pure population/selector mechanics, identical to every
  other boundary in this round.
- **Guardrail**: any real infrastructure failure (rate limit, API
  error) partway through a window's measurement is recorded as a
  distinct `status: "window_evaluation_incomplete"` ledger row, never
  silently coerced into `criterion_met: false` — the procedure would
  stop entirely rather than risk treating a broken measurement as a
  real negative result. (One transient GraphQL hiccup occurred in
  practice, at ample remaining quota; it was a retry-worthy network
  blip, not a capacity wall — fixed by adding the same retry/backoff
  discipline already used elsewhere in the script, then the search
  resumed from its own persistent cache with zero loss of already
  -verified work.)

**Full capacity ledger** (`fixtures/s5-design/population/capacity-ledger.jsonl`,
6 rows, every attempted window's own intermediate artifacts preserved
under `population/attempts/<window_start>/`):

| window_start | survivors | S5-A | R4-selected remainder | criterion met |
|---|---:|---:|---:|:---:|
| 2026-06-01 | 94 | 94 | 0 | no |
| 2026-03-01 | 367 | 150 | 30 | no |
| 2025-12-01 | 639 | 150 | 86 | no |
| 2025-09-01 | 908 | 150 | 126 | no |
| 2025-06-01 | 1154 | 150 | 181 | no |
| **2025-03-01** | **1424** | **150** | **233** | **yes** |

**Satisfying window: `2025-03-01` through `2026-09-22`** (~18.7 months,
well short of the 2024-01-01 hard cap). The user's own pre-search
sanity arithmetic (219/0.182 ≈ 1200 B-survivors + 150 for A ≈ ~1350
total, if the historical ~18.2% selection rate held) was close to the
real outcome (1424 survivors needed, 233 R4-selected in the remainder)
— disclosed as a sanity check that held up, not a target that was
engineered toward.

- **Fetch**: real GitHub commits API (path-filtered, paginated) for
  each newly-opened calendar slice, unioned into a persistent cache →
  real merge-PR resolution via **batched GraphQL**
  (`associatedPullRequests`, not one REST call per commit — an
  implementation-efficiency choice only, verified to produce the same
  field shape S4's own `population-raw.jsonl` used) → real PR
  detail+files via batched GraphQL, cached globally by PR number
  (fetched at most once across the entire multi-window search) →
  each PR's real Compare-API diff (needed for the selector's own
  line-level `patch` text, which GraphQL does not expose), also cached
  globally by PR number. `fixtures/s5-design/population/capacity-search.py`.
  This is a live, non-replayable population — the FROZEN artifacts are
  this run's own committed output (`population/raw.jsonl`,
  `population/attempts/*/raw.jsonl`), never re-fetched.
- **Screen**: same real mechanical screen S1-S4 used (merge-window
  sanity, docs-only, mass-mechanical, exclusion-ledger match), its pure
  functions imported (not reimplemented) from
  `fixtures/s5-design/population/screen.py` and re-run at every window
  step against that step's own window bounds.
- **S5-A/S5-B disjointness**: by construction (S5-B draws only from
  the pool remaining after S5-A's picks are removed) and mechanically
  re-verified by both `test_s5_protocol_machinery.py` and
  `reproduce-s5-freeze.py`.
- **Reproducibility**: `fixtures/s5-design/reproduce-s5-freeze.py`
  regenerates both frozen orders for the satisfying window
  (`2025-03-01`) from committed inputs alone (no network) and asserts
  byte-for-byte equality; this is the CI-safe check that actually runs
  on every push. It reproduces the FINAL window's freeze only, not the
  full multi-window search (which required live network access and is
  documented, not re-executed, via the capacity ledger).

## 6. Two-blind-reviewer protocol

Carried forward verbatim from S4: two independent reviewers per
actionable presentation and per materially-relevant PASS, blind to
each other's judgment before submission, dispatched strictly in
parallel; disagreements preserve both original reviews, resolved by
the coordinator against raw evidence; an unresolved disagreement is
never silently counted as correct; reviewer forks are read-only with
respect to every source-of-truth artifact.

## 7. Gate (tier precedence)

Implemented mechanically in `fixtures/s5-design/gate.py`, unit-tested
in `test_s5_protocol_machinery.py`.

**Tier 1 — hard correctness/safety failure → `FAIL`.** Any of: a false
PASS; a false finding in a maintainer-facing actionable presentation;
a PR-relevance error; a causal/origin-framing error; a materially
wrong rendered presentation; a `TOOL_ERROR` on ordinary supported
input; an unresolved adjudication disagreement. **If Tier 1 occurs,
S5-B still continues to its pre-registered stop point** — no optional
stopping, exactly as S4/S4-F1-R both enforced, and provable at the
type level here since `should_continue_sampling` never receives any
Tier-1 signal (§4).

**Tier 2 — insufficient evidence → `INSUFFICIENT EVIDENCE`.** Any of:
S5-B's cap reached before 30 distinct actionable PRs; S5-A's own
sample incomplete; a required independent review missing; the computed
one-sided lower bound below the pre-declared threshold because the
achieved PR-level denominator was too small.

**Tier 3 — precision/statistical gate → evaluated only if Tiers 1-2
are clear.** Requires: every distinct actionable PR correct at the
PR level (§1's aggregation rule); the one-sided exact 95%
Clopper-Pearson lower bound on the PR-level denominator ≥ 90%; zero
false PASS across both cohorts; zero unresolved disagreements.
**S5-A and S5-B are never pooled into one precision number.**

Reference bounds (`s4gen.clopper_pearson_lower_bound`, reused, cross
-checked in `test_s5_protocol_machinery.py`):
- **30/30 → 0.9050** (the pre-registered target, comfortable margin).
- **29/29 → 0.9019** (already clears ≥90% — mechanically verified;
  corrects this round's own initial hand-guess that 29/29 would fall
  short).
- **28/28 → 0.8985** (the real "just below the threshold" case).

## 8. Machine-readable ledger invariants

`fixtures/s5-design/ledger-schema-draft.json`'s `selection_record`
type (now including `unresolved`) and its `new_invariant_for_s5_b`
list are the operative schema for S5-B's own selection evidence.
`invariants_carried_forward_from_s4` remain unchanged: two-reviewer
requirement, presentation/PASS id validation, no false-finding/false
-pass field anywhere (always derived from the resolved judgment),
unresolved-disagreement handling, frozen-order PREFIX property.

## 9. Known capability gaps carried forward (not fixed here or by S5 itself)

- Pixelfed nested/dotted option-declaration discovery gap (S4 report
  item 17, `#526840`).
- Cloudlog declaration-vs-predicate walker asymmetry (S4 report item
  17, `#399257`).
- Wildcard `attrsOf(submodule)` discovery gap, confirmed recurring
  twice independently (S4 report item 17, `#411705`/`#507312`).
- Plain `oba diff`'s own analogous all-or-nothing missing-target
  behavior (disclosed throughout S4-F1/S4-F1-R, untouched).

S5 measures the released v0.4.5 analyzer as it stands, gaps included.
A PR that trips one of these discovery gaps could still be selected by
R4 yet produce no analyzable actionable presentation — a real,
disclosed risk to S5-B's own recall, not something this freeze round
resolves.

## Evidence boundaries

This document, and every file committed alongside it in this round,
authorizes the FREEZE only: the selector's own code, the exact
population, the exact frozen cohort orders, and the exact gate. It
does **not** authorize running `oba` on any fresh S5 PR, does **not**
authorize adjudication, does **not** constitute an S5 result of any
kind, and does **not** support any claim about the analyzer's current
precision, generalization, or deployment readiness. Only a separate,
explicit GO — and only the real S5-A/S5-B data-gathering and
adjudication that follows it — can produce that evidence.
