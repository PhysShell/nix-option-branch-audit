# S5 protocol — DRAFT, not authorized, no fresh draw performed

This is a draft written during the S5 design round (design-only
authorization, `8f1701a`). It proposes the protocol a future, separately
authorized S5 round would follow. **Nothing in this document authorizes
starting S5.** No fresh PR population has been drawn; no PR beyond the
already-seen S1-S4 corpus has been fetched or inspected for this round.

## Estimands (kept separate throughout — never pooled into one number)

**Estimand A — representative prevalence/applicability.** On a genuinely
representative random sample of eligible nixpkgs PRs: applicability
rate, PASS rate, actionable-transition rate, INCONCLUSIVE rate,
TOOL_ERROR rate, structural non-applicability, manual-effort
distribution. Answers "how often would this tool have something to say
in ordinary use." Not the precision-power cohort.

**Estimand B — actionable-presentation precision.** On a pre-declared,
result-blind, mechanically enriched population: what fraction of
maintainer-facing actionable presentations are correct on substance, PR
relevance, causal/origin framing, and rendered presentation. The
enrichment rule changes the population — this precision figure is
**never** presented as unconditional precision over all nixpkgs PRs.

**Estimand C — false-PASS safety.** A PASS materially relevant to a PR
transition is independently adjudicated regardless of which cohort (A
or B) it came from. A distinct safety question from B's precision; kept
in its own inventory, its own denominator, never mixed with B's.

## S5-A — representative cohort

- **Population**: fresh, non-overlapping nixpkgs PR population, drawn
  the same way S1-S4's own population funnels were (real GitHub commits
  API, path-filtered on `nixos/modules/services` and `nixos/tests`,
  screened for merge-window sanity / docs-only / mass-mechanical /
  exclusion-ledger match — reusing S4's own screening machinery, not
  reinventing it).
- **Sampling**: seeded shuffle over the survivor population, same
  convention as S1-S4 (`seed = int(<frozen binary short SHA>, 16)`).
- **Sample size**: chosen for Estimand A's own precision need, NOT
  forced to supply a minimum actionable count (S4's own stated design
  failure). See `planning-calculations.json`'s
  `representative_cohort_ci_widths` table:

  | n | rate | 95% CI half-width |
  |---:|---:|---:|
  | 100 | 10% | ±6.4pp |
  | 150 | 10% | ±5.1pp |
  | 200 | 10% | ±4.4pp |
  | 100 | 25% | ±8.9pp |
  | 150 | 25% | ±7.2pp |
  | 200 | 25% | ±6.2pp |

  **Recommendation: n=150.** At S4's own observed actionable-transition
  rate (7/120 ≈ 5.8%), n=150 gives a two-sided 95% CI half-width of
  roughly ±3.4-4pp around that rate — a real precision improvement over
  S4-A's own 120, at a proportionate (not runaway) manual-cost increase.
  n=200 buys only marginally tighter bounds for meaningfully more manual
  adjudication; n=100 is noticeably looser at the low rates this tool's
  own applicability/actionable rates actually sit at. This is a
  recommendation, not a forced round number — the actual choice belongs
  to S5's own separate authorization.
- **No minimum-actionable requirement.** S5-A's own stopping rule is
  simply "process the full frozen sample" — it is not required to reach
  any actionable-presentation count, mathematically justified by the
  above: its job is prevalence/applicability precision, a question a
  fixed-n sample answers regardless of how many actionable transitions
  happen to occur in it.

## S5-B — enriched precision cohort

- **Enrichment rule**: the frozen selector from
  `enrichment-rules.py` (design round's own recommendation: **R4,
  "option lifecycle" — select a PR if its diff touches an
  `mkOption`/`mkEnableOption` declaration line anywhere in a
  `nixos/modules/services/**` file** — see `design-report.md` for the
  full comparison against R1-R3/R5-R7 and why R4 is recommended over
  the higher-yield-but-lower-recall R7).
- **Result-blindness**: the rule is a pure function of
  `extract-features.py`'s own output, which never invokes `oba`, never
  reads any `oba` result file, and is proven so by
  `test_no_leakage.py` (6/6 passing self-tests, including a poisoned-row
  fail-closed check). Every S5-B selection decision must be
  accompanied by a `selection_record` (see the ledger schema below)
  naming the exact source-diff facts that caused selection.
- **Frozen before population selection.** The rule (its exact
  threshold/feature set) must not change after S5-B's real population
  is drawn, and must not be tuned in response to any interim S5-B
  result once real adjudication begins — the same "no Optional
  Stopping" discipline S4 itself enforced, extended to the SELECTOR,
  not just the stopping rule.
- **Historical selection rate** (S4 corpus, R4): 34/187 (18.2%) overall,
  22/120 (18.3%) within S4-A's own representative population alone.
- **Historical actionable yield** (S4 corpus, R4): 9/34 actionable
  presentations captured within selected PRs (0.26/selected-PR), a
  5.5x lift over the whole-corpus rate (9/187 = 0.048/PR); within S4-A
  alone, R4 captures **all 7** of S4-A's own real actionable
  presentations at 22/120 selected (100% historical recall on this
  corpus — see the recall caveat in `design-report.md`: this is a
  single-corpus measurement, not a guaranteed future recall rate).
- **Planning arithmetic** (never a guarantee — see
  `planning-calculations.json`): at R4's own observed 0.26
  actionable/selected-PR historical rate, reaching **30** actionable
  presentations would need roughly 30/0.26 ≈ **115 selected PRs**; at
  an 18.2% historical selection rate, that implies processing roughly
  115/0.182 ≈ **~630 candidate PRs** to draw enough selected ones —
  a real, large number, disclosed plainly rather than hidden. See
  `design-report.md` section 7 for the full range (20/30/40-actionable
  planning table) and why this number, while large, is still a real
  improvement over S4's own literal experience (187 processed PRs for
  9 actionable, i.e., a ~1-in-21 processed-PR yield vs R4's own
  ~1-in-4 selected-PR yield).
- **Stopping rule**: process at least `N_min` selected PRs, continue
  until at least 30 independently adjudicated actionable presentations
  (or a separately justified different threshold — see
  `planning-calculations.json`'s full precision-bound table for 20-49),
  capped at `N_max` selected PRs. If the cap is reached first:
  `INSUFFICIENT EVIDENCE`, not a forced continuation past the frozen
  cap and not a redraw. Exact `N_min`/`N_max` values are left to S5's
  own separate authorization (this design round provides the
  arithmetic, not the final numbers — matching the authorization's own
  instruction not to pre-select a "convenient round number").
- **False-PASS review (Estimand C)**: every PASS materially relevant to
  a PR transition, in EITHER cohort, gets the same two-reviewer
  independent adjudication S4 already used — never skipped for S5-B
  just because it's the "precision cohort."

## Gate (tier precedence, extending S4's own frozen structure)

**Tier 1 — hard correctness/safety failure → `FAIL`.** Any of: a false
PASS; a false finding in a maintainer-facing actionable presentation; a
PR-relevance error; a causal/origin-framing error; a materially wrong
rendered presentation; a `TOOL_ERROR` on ordinary supported input; an
unresolved adjudication disagreement. If Tier 1 occurs, S5 still
continues to its pre-registered stop point — no optional stopping,
exactly as S4 and S4-F1-R both already enforced.

**Tier 2 — insufficient evidence → `INSUFFICIENT EVIDENCE`.** Any of:
S5-B's cap reached before the actionable-presentation threshold;
S5-A's own sample incomplete; a required independent review missing;
the computed statistical bound below the pre-declared threshold because
the achieved denominator was too small.

**Tier 3 — precision/statistical gate → evaluated only if Tiers 1-2 are
clear.** Requires, at minimum: S5-B actionable precision = 100% on its
observed denominator; the one-sided exact 95% Clopper-Pearson lower
bound on S5-B's own actionable precision ≥ the pre-declared threshold
(≥90% requires n≥30 at 100% observed — see
`planning-calculations.json`); zero false PASS across BOTH cohorts;
zero unresolved disagreements. **S5-A and S5-B are never pooled into
one precision number** — S5-A serves Estimand A only, S5-B serves
Estimand B only; any pooled statistic shown is explicitly diagnostic,
never a gate input, unless a future protocol amendment explicitly
re-derives the estimand to support pooling (none does here).

## Independent review protocol

Carried forward verbatim from S4: two independent reviewers per
actionable presentation and per materially-relevant PASS, blind to each
other's judgment before submission, dispatched strictly in parallel;
disagreements preserve both original reviews, resolved by the
coordinator against raw evidence, an unresolved disagreement is never
silently counted as correct; reviewer forks are read-only with respect
to every source-of-truth artifact.

## Known capability gaps carried forward, not fixed here or by S5 itself

- Pixelfed nested/dotted option-declaration discovery gap
  (S4 report item 17, `#526840`).
- Cloudlog declaration-vs-predicate walker asymmetry
  (S4 report item 17, `#399257`).
- Wildcard `attrsOf(submodule)` discovery gap, confirmed recurring
  twice independently (S4 report item 17, `#411705`/`#507312`).
- Plain `oba diff`'s own analogous all-or-nothing missing-target
  behavior (noted, disclosed, untouched throughout S4-F1/S4-F1-R).

S5 measures the RELEASED v0.4.5 analyzer as it stands, gaps included —
these are real limitations that may suppress some real actionable
transitions from ever surfacing (a real risk to Estimand B's own
recall, disclosed explicitly, not hidden) — but fixing them is its own
separately authorized round, never folded into S5 itself.
