# S6-R0B: execution-tooling errata for S6-R0/S6-R0A

> **FURTHER AMENDED by S6-R0C** (`R0C-final-pre-P0-guards.md`): a
> temporal-window maturity guard and a changed-file completeness guard
> (GitHub's own 3000-file `/pulls/{n}/files` cap, distinct from the
> Search API's own 1000-result cap this document fixed) — both
> execution preconditions/safeguards, found still before any S6
> candidate data existed. This document's own two fixes (pagination,
> retry semantics) are unaffected and remain in force. **Further
> amended by S6-R0D** (`R0D-window-increment-amendment.md`): initial P0
> window shrunk 7 days -> 3 (a reasoned cost adjustment, not a bug fix).

**Parent protocol commits**: S6-R0 preregistration
`218bf4727b86ba3234252bc636e4329dcdd22414`; S6-R0A amendment
`0cb69b0fb3b3a50643af6a406cf080dee657d705`.

**Confirmation, stated explicitly and verified against the actual
session record**: no real S6 candidate population, PR content, or
result was queried or inspected at any point between `0cb69b0` and
this errata. Both issues below were found by static re-review of the
R0A tooling's own SOURCE CODE against R0A's own PROSE CONTRACT — the
same discipline applied one more time, still before P0.

**Effect on `218bf47`/`0cb69b0`**: both remain the immutable historical
record, unedited. This document is a further amendment layer on top of
`R0A-protocol-errata.md`.

---

## Issue 1: real GitHub Search pagination was never implemented

**Bug**: `population-and-sampling.py`'s own `_real_github_search`
(`0cb69b0`) issued exactly ONE request (`per_page=100`, no `page=`
parameter, no `--paginate`) and returned whatever that single call gave
back — while the function's own surrounding doc comment claimed "the
real implementation uses `--paginate`." For any shard whose real
`total_count` exceeded 100 (a routine case for nixpkgs even over a
single day), this would silently retrieve only the first 100 items,
then the EXISTING consistency check (`len(items) == total_count`)
would correctly detect the shortfall and raise
`PopulationIncompleteError` — so the bug was fail-safe in its own
effect (it could never silently accept a truncated population as
complete), but it would falsely `KILL`/abort a perfectly healthy P0
census the moment any real shard had more than 100 merged PRs, which
is not a rare edge case for nixpkgs but the ordinary case.

**Correction**: `default_paginated_search` (replacing
`_real_github_search`), a genuine two-phase implementation:

- **Phase 1** (cheap): fetch page 1 only, to learn `total_count`/
  `incomplete_results` before committing to anything further.
- **Phase 2** (only for a shard already proven `total_count <=
  SEARCH_API_RESULT_CAP` and not `incomplete_results`): fetch every
  remaining page (`ceil(total_count / 100)` pages total), each through
  the same bounded retry wrapper as phase 1.

For an oversized or already-flagged-incomplete shard, phase 2 is never
entered at all — no page beyond page 1 is ever requested merely to
confirm a shard is too large, matching R0B's own explicit cost
requirement (item 2). The existing shard-level consistency check
(`fetch_merged_prs_in_window`'s own `len(unique_numbers) ==
total_count`) is unchanged and unweakened; it now correctly receives a
FULLY retrieved item set for any shard phase 2 actually ran on, so it
tests what it was always meant to test, rather than papering over a
pagination bug that happened to trigger the same code path.

**Regression coverage**: `test_total_count_zero_returns_empty_no_extra_pages`,
`test_total_count_one_single_page`,
`test_total_count_exactly_one_page_size_no_second_page_fetched`,
`test_total_count_one_over_page_size_fetches_second_page`,
`test_multi_page_350_fetches_four_pages_in_order`,
`test_total_count_exactly_at_cap_fetches_all_ten_pages`,
`test_total_count_one_over_cap_never_fetches_a_second_page`,
`test_incomplete_results_on_page_one_never_fetches_further_pages`,
`test_page_mismatch_is_caught_by_the_shard_level_consistency_check`,
`test_duplicate_item_within_one_shards_own_pages_is_caught`,
`test_later_page_failure_after_retries_raises_population_query_incomplete`.

---

## Issue 2: retry semantics were prose-only, not implemented

**Bug**: R0A's own text (`preregistration.md` section 14) froze a
3-attempt retry budget for population and changed-file fetches, but
`_real_github_search`/`fetch_changed_files` (`0cb69b0`) each issued
exactly one `subprocess.run(..., check=True)` call with no retry logic
at all. A single transient GitHub API failure would raise an uncaught
`CalledProcessError`, crashing the whole census/pilot run instead of
being absorbed as the single documented, per-call incomplete status.

**Correction**: one shared, bounded retry primitive,
`with_retry(fn, max_attempts=MAX_FETCH_ATTEMPTS, sleep_fn=None)` —
`MAX_FETCH_ATTEMPTS = 3`, frozen, used identically by both:

- `default_paginated_search`'s own per-page fetches — exhausting the
  budget on ANY page raises `PopulationQueryIncomplete` (a
  `PopulationIncompleteError` subclass, so every existing caller that
  already handles the base class handles this identically — it fails
  the whole affected window evaluation, exactly like the pre-existing
  day-granularity-floor case).
- `fetch_changed_files` — exhausting the budget raises
  `ChangedFileFetchIncomplete(pr_number, cause)`, a PER-PR exception
  `census` is now responsible for catching (see Issue 2's own NC1
  sub-item below) — it never propagates as a raw, uncaught
  `subprocess` error, and it never silently disappears.

`sleep_fn` is an injectable backoff hook (every test in this round
passes `None`, so no test ever sleeps in real time — "test injection
must make attempt counts observable," per this round's own item 5,
satisfied via each mock's own call-count tracking, asserted directly
by the relevant test).

**Regression coverage**:
`test_population_query_transient_failure_then_success`,
`test_population_query_fails_all_attempts_raises_population_query_incomplete`,
`test_changed_file_transient_failure_then_success`,
`test_changed_file_fails_all_attempts_raises_changed_file_fetch_incomplete`.

---

## Amendment: NC1 behavior under changed-file incompleteness (item 4)

R0A's own text did not fully specify this case — flagged here visibly,
as its own amendment, per this round's own instruction, rather than
silently folded into "just an implementation detail."

**Frozen rule**: if ANY systematically-sampled P0 census record's own
changed-file fetch exhausts its retry budget
(`ChangedFileFetchIncomplete`), the entire current window-check raises
`WindowEvaluationIncomplete` (`census`, then propagated uncaught
through `find_final_window`) — it is NEVER silently classified as
`obviously_irrelevant` (an unknown is not the same fact as an
irrelevant), and it NEVER triggers the ordinary NC1 "not enough
relevant PRs yet, extend the window" path (`SystemExit`/
`STOP_LOW_YIELD`) — those two outcomes are structurally distinct
exception types in the corrected implementation, so a future P0 runner
cannot accidentally conflate "genuinely low density" with "we don't
actually know the density because some fetches failed."

**Regression coverage**:
`test_failed_changed_file_classification_does_not_become_obviously_irrelevant`,
`test_incomplete_census_propagates_through_find_final_window_not_extended_as_low_yield`
(the latter explicitly asserts that catching `SystemExit` instead of
`WindowEvaluationIncomplete` is itself a test failure — i.e., it proves
the two paths cannot be confused, not merely that one of them fires).

---

## Re-audit against R0/R0A (item 7)

Mechanically re-checked, this round, after the two fixes above:

- **Pagination/completeness**: now genuinely two-phase, retry-wrapped,
  consistency-checked (both cross-shard AND within-shard-across-pages
  duplicates now caught — the within-shard case, `test_duplicate_item_
  within_one_shards_own_pages_is_caught`, was not previously
  exercisable since pagination itself didn't exist yet).
- **Retry count**: `MAX_FETCH_ATTEMPTS = 3` used consistently by both
  call sites; no other real network call exists in this file.
- **Incompleteness statuses**: `population_query_incomplete` and
  `changed_file_fetch_incomplete` now have real, tested code
  (`PopulationQueryIncomplete`, `ChangedFileFetchIncomplete`).
  `target_source_fetch_incomplete` and `oba_execution_incomplete`
  (R0A's own remaining two statuses) remain CONCEPTUAL/protocol-level
  only — correctly so, since no P1 target-construction or `oba`-
  execution tooling exists yet to implement them in code, and building
  that tooling is explicitly out of this round's own scope (item 9;
  it belongs to P1 itself, under its own future GO).
- **P0 NC1 handling**: confirmed structurally impossible to conflate
  with `WindowEvaluationIncomplete`, per the amendment above.
- **Deterministic sampling**: `select_sample`/`select_p1_sample`/
  `check_leakage` untouched by this round's own fixes, all 3 of their
  own R0A-era tests still pass verbatim.
- **Leakage handling**: untouched, unaffected.

**No further execution-blocking mismatch found.** This round does not
report a third issue requiring its own errata.

---

## What did NOT change

Every design decision frozen in `218bf47` and `0cb69b0` — the
scientific question, temporal bounds, window-extension VALUE (7/42
days; only WHERE `NC1` is checked was corrected, in R0A, not touched
again here), the cheap relevance filter, the leakage-prevention
mechanism, the sampling seed, the multi-target derivation rule and its
own `MAX_TARGETS_TOTAL=30` budget, the NC2/NC3/NC4 denominators, the
corrected expansion-batch gate/stopping rule, the blinded adjudication
procedure, the escalation rule, the error taxonomy, the decision not to
run a known-limitations challenge set, and the `STOP_DEFECT_FOUND`
decision-gate rule — all unchanged. **No scientific/sampling semantics
changed in this round; the implementation was brought into conformity
with already-frozen R0A rules**, with the one explicit exception noted
above (NC1-under-changed-file-incompleteness), which R0A's own text
had not fully specified and which is now frozen here, visibly, as its
own amendment rather than silently assumed.

## Scope confirmation

No analyzer source changed. No `oba` execution occurred. No real
post-release population was queried or inspected. No fresh nixpkgs PR
content was examined. No result files exist. No P1 target construction
occurred. No adjudication occurred. No residual-limitation work was
performed.

**`S6_R0B_EXECUTION_TOOLING_CORRECTED`**

**STOP.** No P0 execution without a separate, explicit GO.
