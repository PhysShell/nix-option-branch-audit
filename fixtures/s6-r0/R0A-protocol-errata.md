# S6-R0A: protocol errata for S6-R0 (commit `218bf47`)

> **FURTHER AMENDED by S6-R0B** (`R0B-execution-tooling-errata.md`) --
> two execution-level bugs in the tooling THIS document's own fixes
> introduced (real Search API pagination was never actually
> implemented; the retry semantics this document froze in prose were
> never implemented in code) were found and corrected there, still
> before any S6 candidate data existed. This document's own scientific/
> sampling corrections (unit model, NC2-4 denominators, expansion-gate
> fix) are unaffected and remain in force. **Further amended by S6-R0C**
> (`R0C-final-pre-P0-guards.md`): a temporal-window maturity guard and a
> changed-file completeness guard (GitHub's own 3000-file
> `/pulls/{n}/files` cap) — execution preconditions/safeguards only.

**Original preregistration commit**: `218bf4727b86ba3234252bc636e4329dcdd22414`.

**Confirmation, stated explicitly and verified against the actual
session record**: no S6 candidate population was queried, no fresh
nixpkgs PR content was inspected, and no `oba` execution against the
post-`v0.5.0` window occurred between `218bf47` and this errata. Every
issue below was found by static re-review of the committed R0 protocol
and tooling text itself — the same discipline the R0 preregistration
itself was written to enforce, now applied to its own first draft
before any data existed to tempt a self-serving fix. This is exactly
what a kill-first, preregistered design is supposed to catch, and
catching it here (before P0) rather than in a published report's own
"limitations" section is the intended, working behavior of the
process, not an embarrassment to paper over.

**Effect on `218bf47`**: `218bf47` remains the immutable original
record. Nothing in it is rewritten. This document is an explicit
AMENDMENT layer; wherever it conflicts with `218bf47`'s own original
text, THIS document's own corrected rule governs going forward, and the
specific line below states exactly which original rule it replaces and
why.

---

## Issue 1: GitHub Search API result-count truncation

**Original rule** (`population-and-sampling.py`,
`fetch_merged_prs_in_window`, `218bf47`): issue one Search API query
covering the ENTIRE `[lower, upper)` window and trust `--paginate` to
return every match.

**Why this is unsafe**: GitHub's Search API caps retrievable results at
**1000 per query**, regardless of pagination — `total_count` in the
response can report a larger true match count, but only the first
1000 are ever actually retrievable. A window whose true population
exceeds 1000 merged PRs would silently yield a TRUNCATED population,
and every downstream "deterministic" sampling step would then be
deterministic over an already-biased, incomplete set — undetectable
from the sampling code's own output alone.

**Corrected rule**: a deterministic **temporal-sharding** procedure
(`fetch_merged_prs_in_window`, rewritten). The window is split into
whole-UTC-day shards (the finest granularity GitHub's own `merged:`
search qualifier supports). Each shard's own query response is
inspected for BOTH its `total_count` (must be `<= 1000`) and its
`incomplete_results` flag (must be `false`) before its own items are
ever trusted. A shard failing either check is bisected by TIME interval
only (never by content/relevance) and re-checked recursively. A
single-UTC-day shard that still cannot be proven complete raises
`PopulationIncompleteError` — there is no finer granularity available,
so this is the genuine floor of what the method can prove, and the
correct response is to fail closed, not to accept a possibly-truncated
result. An additional consistency check (retrieved item count must
equal the shard's own claimed `total_count`) catches any OTHER form of
silent under-retrieval the same way.

**Regression coverage**: `test_population_and_sampling.py`'s own
`test_result_well_below_cap_is_accepted_without_subdivision`,
`test_total_count_exactly_at_cap_is_complete_no_subdivision`,
`test_total_count_one_over_cap_requires_subdivision`,
`test_recursive_multi_level_subdivision`,
`test_incomplete_results_flag_forces_subdivision_regardless_of_total_count`,
`test_single_day_shard_still_incomplete_raises_population_incomplete_error`,
`test_retrieved_item_count_mismatching_total_count_also_fails_closed`,
`test_duplicate_pr_across_adjacent_shards_is_deduplicated`,
`test_exact_timestamp_bounds_are_applied_client_side`,
`test_final_ordering_is_merged_at_then_pr_number` — all synthetic,
mocked, zero real network calls, zero real S6-window dates (self-checked
by the same file's own `test_no_real_network_calls`/
`test_no_s6_candidate_window_dates_used`).

---

## Issue 2: the expansion stopping rule referenced an incoherent per-batch NC1

**Original rule** (`preregistration.md` section 11, `218bf47`): "stop
after ... 3 consecutive batches each independently satisfy NC1-4."

**Why this is incoherent**: NC1 is defined as "≥20 `potentially_relevant`
PRs among ≤100 systematically-sampled CENSUS records" — a condition
about raw, largely-irrelevant PR density in an UNFILTERED window
sample. An expansion batch is 15 PRs ALREADY drawn from the
`potentially_relevant` tier (the same tier NC1 itself measures the
size of). Applying NC1's own literal threshold to a batch of 15
already-filtered PRs is arithmetically unsatisfiable and was never a
meaningful per-batch check in the first place — the original text
conflated "does the WINDOW contain enough raw material" (a one-time,
window-establishment fact) with "did THIS BATCH perform well" (a
per-batch fact about derivation/yield/adjudicability, not density).

**Corrected rule**: NC1 is evaluated exactly once, at window
establishment (`find_final_window`), and never re-applied per batch.
A **per-batch expansion gate** (`pilot_accounting.
batch_passes_expansion_gates`) instead checks the batch-level analogs
of NC2/NC3/NC4 ONLY, at the same thresholds already frozen in `218bf47`
(≥9/15 PR-level derivation, ≥⌈0.7×n⌉ target-level substantive yield,
≥⌈0.7×n⌉ target-level adjudicability). The larger-S6 stopping rule
(`pilot_accounting.expansion_stopping_decision`) stops at whichever
comes first: **30 cumulative substantive TARGETS** (not PRs — see
Issue 3/4 below for why this unit matters) across all batches so far,
or **3 consecutive batches** that each independently pass the
corrected per-batch gate. This preserves the original design's own
intended shape (batches of 15; a 30-count-or-consecutive-success
stopping rule, mirroring historical S5's own "stop at 30 distinct
actionable PRs or the cap" precedent) while making every referenced
condition actually computable.

**Regression coverage**:
`test_batch_gate_ignores_nc1_and_uses_target_level_nc3_nc4`,
`test_expansion_stops_at_cumulative_thirty_substantive_targets`,
`test_expansion_stops_after_three_consecutive_passing_batches_below_cumulative_cap`,
`test_expansion_continues_when_neither_stop_condition_met`.

---

## Issues 3-5: PR-vs-target unit ambiguity, and the NC2/NC3/NC4 denominator mismatch

**Original rule** (`target-construction-protocol.md`, `218bf47`):
permitted multiple targets per PR ("every touched declaration...
becomes its own target"), while the surrounding preregistration text
simultaneously spoke of "P1 = 15 PRs," "≤15 primary reviewer
decisions," and "adjudicate every substantive result" as if PR and
target were the same countable unit. Compounding this, the target
protocol's own step 2 said a PR with no derivable target "still counts
toward the NC3/NC4 denominators as 'no substantive target,'" while the
main preregistration's own NC3/NC4 text defined those denominators as,
respectively, "PRs with a derived target" and "substantive-result
cases" — two different, inconsistent populations for the SAME named
condition.

**Why this matters**: without a single frozen unit model, "≤15 review
decisions" is not actually a bound (a 15-PR batch could legitimately
produce 40+ targets under the original multi-target rule, silently
blowing the stated budget), and NC3/NC4's own pass/fail numbers would
differ depending on which of the two inconsistent denominator
definitions a future executor happened to apply — exactly the kind of
ambiguity that invites picking whichever reading makes the pilot look
better AFTER seeing the data, which is precisely what preregistration
exists to foreclose.

**Corrected rule** (full detail in
`target-construction-protocol.md`'s own "Unit model and NC2/NC3/NC4,
restated precisely" section, and `pilot-accounting.py`):

- **Sampling unit**: PR (exactly 15, fixed, `sampled_pr_count`).
- **Analysis/adjudication unit**: target. A PR yields zero, one, or
  several, per the (unchanged) multi-target derivation rule.
- **Five explicit counters**, kept distinct everywhere:
  `sampled_pr_count` (15), `derived_pr_count`, `derived_target_count`,
  `substantive_target_count`, `adjudicated_target_count`.
- **Target-multiplicity budget cap**: `MAX_TARGETS_TOTAL = 30`
  (`pilot-accounting.py`), a pre-committed COST bound (not derived from
  any observed S6 data — chosen now as roughly "two independently
  touched declarations per sampled PR, on average," a deliberate,
  named, round-number budget). Targets beyond the cap are recorded as
  `target_construction_incomplete_due_to_budget_cap`, processed in a
  fixed order (frozen sample order across PRs, source-position order
  within a PR), never chosen by interest, and excluded from every NC
  count.
- **Reviewer-decision budget corrected accordingly**: up to
  `MAX_TARGETS_TOTAL` (30) primary review decisions, not 15 — the
  original "≤15" bound is retired; it assumed one target per PR, which
  the multi-target rule (correctly, and unchanged by this errata) never
  guaranteed.

**Corrected NC2/NC3/NC4 denominators** (Issue 5, resolved together with
3-4 since they share one root cause):

| Condition | Numerator | Denominator | KILL |
|---|---|---|---|
| NC2 | `derived_pr_count` | `sampled_pr_count` (=15, fixed) | `derived_pr_count < 9` |
| NC3 | `substantive_target_count` | `derived_target_count` | `substantive_target_count < ceil(0.7 * derived_target_count)` |
| NC4 | `adjudicated_target_count` | `substantive_target_count` | `adjudicated_target_count < ceil(0.7 * substantive_target_count)` |

A PR that fails to yield any target affects NC2's own denominator
only (it was sampled) — it contributes nothing to NC3/NC4, which are
now purely target-level and never see a "no substantive target"
placeholder entry the way `218bf47`'s own text incorrectly implied.

**Regression coverage**:
`test_target_cap_accepts_up_to_the_budget_then_records_the_rest_as_cut`,
`test_target_cap_no_cut_when_total_within_budget`,
`test_nc2_denominator_is_fixed_sampled_pr_count_not_target_count`,
`test_nc3_denominator_is_derived_target_count_not_pr_count`,
`test_nc4_denominator_is_substantive_target_count`,
`test_nc3_and_nc4_zero_denominator_is_a_defined_fail_not_a_crash`.

---

## Issue 6: exact KILL formulas restated with no interpretive prose

Per this errata's own item 6 requirement: every KILL condition above is
stated as a literal, computable formula (see the table in Issues 3-5,
and Issue 2's own batch-gate formulas) — no remaining "most cases" or
similarly vague language governs any KILL decision anywhere in the
amended protocol. `ceil(0.7 * n)` is computed via `math.ceil`
(`pilot-accounting.py`), with the `n = 0` edge case explicitly defined
as an automatic fail (`passed: False`) rather than a division error —
covered by `test_nc3_and_nc4_zero_denominator_is_a_defined_fail_not_a_crash`.

---

## Issue 7: fetch/infrastructure-discipline cleanup

**Original text** (`preregistration.md` section 14, `218bf47`)
contained a literal duplicated/broken sentence fragment ("...mark that
call's own subject `window_evaluation_incomplete` and continue with
the rest of the batch — never silently skip without recording the
reason. window and continue with the rest of the batch — never
silently skip.") — a genuine editing leftover, not a substantive design
choice.

**Corrected text**: four explicitly distinct incompleteness statuses,
never conflated:

- **`population_query_incomplete`** — a temporal shard cannot be
  proven complete even at one-UTC-day granularity
  (`PopulationIncompleteError`). Per this same item's own instruction:
  because this can BIAS WHO GETS SAMPLED, it fails the ENTIRE affected
  window-check, not merely the one shard — the census/window-extension
  loop (`find_final_window`) does not catch this exception itself; it
  propagates and aborts that window evaluation.
- **`changed_file_fetch_incomplete`** — a single PR's own changed-file
  list could not be retrieved after 3 attempts during census or target
  construction. Per-PR only; does not invalidate the rest of the
  census/batch. The PR itself is marked and stays in its own fixed
  denominator (a P0 census record, or a P1 sampled PR), never silently
  dropped or replaced.
- **`target_source_fetch_incomplete`** — a sampled PR's own base/head
  module or test source could not be retrieved after 3 attempts during
  target construction. Per-target; classified as
  `INPUT_OR_HARNESS_FAILURE` at adjudication time, per
  `adjudication-rubric.md`'s own existing taxonomy.
- **`oba_execution_incomplete`** — `oba` itself failed to run (crash,
  timeout, unexpected exit) against a frozen target's own real source.
  Per-target; also `INPUT_OR_HARNESS_FAILURE`.

Rate-limit handling (GitHub API): back off and retry per `gh`'s own
standard handling; after 3 attempts, mark the specific failing call's
own subject with whichever of the four statuses above applies to that
call, and continue with the rest of the batch/census — this part of
the original intent was correct and is restated cleanly, once, without
the duplicated fragment.

---

## Issue 8: regression tests for the protocol tooling

`fixtures/s6-r0/test_population_and_sampling.py`, 25 tests, all
synthetic/mocked, **0 real network calls, 0 real S6-candidate-window
dates** (self-verified by the file's own
`test_no_real_network_calls`/`test_no_s6_candidate_window_dates_used`,
which scan the file's own source with the check's own necessary
self-reference excluded). Full list of scenarios covered, matching
this round's own item 8 checklist one-to-one: result well below cap;
result exactly at cap (no subdivision); result one over cap (requires
subdivision); recursive multi-level subdivision; `incomplete_results`
forcing subdivision regardless of `total_count`; a single-day shard
still incomplete (fail-closed); a retrieved-count/`total_count`
mismatch (fail-closed); duplicate PR across adjacent shards
(deduplicated); exact lower/upper timestamp filtering (inclusive
lower, exclusive upper); deterministic final ordering
(`merged_at`, then PR number); deterministic seeded sample
(reproducible); leakage exclusion/replacement (deterministic, excludes
the leaked PR, does NOT over-promise incremental preservation the
original frozen text never asked for); multi-target PR accounting
(budget cap applied/recorded); NC2/NC3/NC4 denominator calculations
(each independently tested, including the zero-denominator edge);
expansion-batch stopping calculation (cumulative-count stop,
consecutive-passing-batches stop, and the "neither" continue case).

**Run**: `python3 fixtures/s6-r0/test_population_and_sampling.py` →
**25/25 passed**.

---

## What did NOT change

- The scientific question (`preregistration.md` section 1).
- The temporal lower bound, window-increment/hard-cap values, and NC1's
  own window-level threshold (20/100/7-day/42-day) — all untouched;
  Issue 2's own fix is about where NC1 is applied, never its own value.
- The cheap relevance filter (`nixos/modules/**`/`nixos/tests/**`).
- The leakage-prevention mechanism and its own exclusion list.
- The sampling seed (`int("0a6f192", 16)`).
- The multi-target derivation rule itself (every touched declaration
  becomes its own target) — only its own BUDGET and ACCOUNTING were
  previously unspecified/inconsistent, now fixed.
- The blinded adjudication procedure, escalation rule, and error
  taxonomy (`adjudication-rubric.md`), beyond the one PR→target
  terminology correction noted in that file itself.
- The decision not to run a known-limitations challenge set in this
  phase.
- The decision that a confirmed Tier-1-style defect halts expansion
  immediately (`STOP_DEFECT_FOUND`).

## Scope confirmation

No analyzer source changed. No `oba` execution occurred. No real
post-release population was queried or inspected. No fresh nixpkgs PR
content was examined. No result files exist. No target construction on
real PRs occurred. No adjudication occurred. No residual-limitation
work was performed.

**`S6_R0A_PROTOCOL_CORRECTED`**

**STOP.** No P0 execution without a separate, explicit GO.
