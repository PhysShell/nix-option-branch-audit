# S6-R0C: final pre-P0 execution guards for S6-R0/R0A/R0B

> **FURTHER AMENDED by S6-R0D** (`R0D-window-increment-amendment.md`):
> the initial P0 window this document's own maturity guard governs
> shrunk from 7 days to 3 (a reasoned cost adjustment, not a bug fix,
> made while still blind to any S6 data) — first eligible P0 execution
> time is now `2026-09-27T05:46:05Z`, not `2026-10-01T05:46:05Z`. The
> maturity guard MECHANISM itself (`census`'s own `now_utc < upper`
> check) is unaffected.

**Parent protocol commits**: S6-R0 preregistration
`218bf4727b86ba3234252bc636e4329dcdd22414`; S6-R0A amendment
`0cb69b0fb3b3a50643af6a406cf080dee657d705`; S6-R0B amendment
`ccdfbe91b1fdb7f668e413ce0e870ab2880fd35b`.

**Confirmation, stated explicitly and verified against the actual
session record**: no real S6 candidate population, PR content, or
result was queried or inspected at any point between `ccdfbe9` and
this errata. Both issues below were found by static re-review of the
already-corrected R0B tooling — the maturity issue is a genuinely
calendar-level fact (the frozen window's own upper bound has not
elapsed yet as of this round), and the changed-files issue is a real,
previously-unaudited GitHub API limit distinct from the Search API cap
R0B already fixed.

**Effect on `218bf47`/`0cb69b0`/`ccdfbe9`**: all three remain the
immutable historical record, unedited beyond their own forward-pointer
banners. This document is a further amendment layer.

---

## Issue 1-2: temporal-window maturity guard

**Problem**: the frozen initial window is `[2026-09-24T05:46:05Z,
2026-10-01T05:46:05Z)`. As of this round, the current date is
2026-09-26 — the window's own upper bound has not elapsed. Running a
population query against this window NOW would not measure "the frozen
7-day window" at all; it would measure whatever subset of that window
happens to have occurred so far, silently mislabeled as the complete
thing. This is a different failure mode from GitHub API truncation
(R0B's own concern) — it is a fact about CALENDAR TIME, not about API
result limits, and no amount of correct pagination or retry logic
fixes it.

**Correction**: `census()` now checks `now_utc < upper` as its very
FIRST action, before any network call of any kind (population or
otherwise), and raises `WindowNotMatured(now_utc, required_upper_bound)`
if the window has not matured. This is a THIRD, structurally distinct
outcome from `STOP_LOW_YIELD` (a genuine low-density result) and
`WindowEvaluationIncomplete` (missing classification data for an
already-fully-existing population) — an immature window is neither; it
simply cannot be evaluated yet, and the correct response is to wait,
not to substitute `now` for the frozen upper bound or shorten the
window.

Because `find_final_window`'s own extension loop calls `census` again
for each successively larger `upper` (7, 14, 21, ... days), the SAME
single guard inside `census` automatically covers every extension step
too — no separate extension-specific logic was needed. If the first
(7-day) window matures and genuinely shows low yield, but the SECOND
(14-day) extension's own upper bound hasn't matured yet, `census`
raises `WindowNotMatured` carrying THAT 14-day upper bound the moment
`find_final_window` tries it — never silently reinterpreted as
`STOP_LOW_YIELD` for the first window, and never skipped ahead to a
later, also-immature window.

`now_fn` is injectable (defaults to `datetime.now(timezone.utc)`);
every test in this round passes a fixed, explicit `now_fn`, so no test
depends on real wall-clock time or on when it happens to be run.

**Regression coverage**:
`test_now_before_upper_is_blocked_with_zero_population_calls` (asserts
the injected `search_fn` is never even called),
`test_now_exactly_at_upper_is_permitted`,
`test_now_after_upper_is_permitted`,
`test_extension_window_immature_does_not_reinterpret_earlier_result_as_low_yield`
(the first window's own real, low-yield census DOES run and produce
zero relevant PRs; the test asserts the final observable outcome is
still `WindowNotMatured` for the second window, never `SystemExit`/
`STOP_LOW_YIELD` for the first).

**Current first eligible P0 execution time**: the frozen initial
window's own upper bound, `2026-10-01T05:46:05Z` (`2026-10-01
10:46:05` Almaty, UTC+5). P0 cannot mechanically succeed before this
instant — enforced by `census`'s own maturity guard, not merely by
operator discipline.

---

## Issue 3-5: GitHub's own 3000-file `/pulls/{n}/files` cap

**Problem**: `GET /repos/{owner}/{repo}/pulls/{pull_number}/files`
cannot represent more than 3000 files for a single PR, regardless of
pagination (a documented GitHub REST API limit, structurally different
from the Search API's own 1000-RESULT-across-a-QUERY cap R0B already
addressed — here there is only ONE PR; its own file count is what it
is, and no query subdivision is possible). Before this round,
`fetch_changed_files` trusted whatever `--paginate` returned as
complete, with no independent check against the PR's own true file
count — a real PR with more than 3000 changed files (rare, but not
impossible in nixpkgs) could have its own real
`nixos/modules/**`/`nixos/tests/**` changes silently invisible to the
cheap relevance filter, which would then incorrectly classify it
`obviously_irrelevant`.

**Correction**: `fetch_changed_files` now fetches the PR resource's
own authoritative `changed_files` metadata field (`GET
/repos/{owner}/{repo}/pulls/{pull_number}`, a single object, no
pagination) FIRST, bounded-retried the same as every other call
(`MAX_FETCH_ATTEMPTS = 3`, no new retry policy). This field is an
independently-computed diff-stat integer, NOT itself subject to the
`/files` listing endpoint's own 3000-item cap — which is exactly why
comparing it against the actually-retrieved file-list length is a
valid, authoritative completeness check, not a second instance of the
same problem.

- If the reported count exceeds `GITHUB_PR_FILES_HARD_CAP` (3000), the
  PR fails closed IMMEDIATELY (`ChangedFileFetchIncomplete`, cause
  `reported_changed_files_exceeds_github_3000_file_api_limit`) —
  the `/files` listing is never even attempted for such a PR (item 6's
  own network-cost discipline: no point paying for a fetch that cannot
  possibly be complete).
- Otherwise, the file list is fetched (bounded-retried, fully paginated
  per R0B) and `len(retrieved) == reported_count` is required; any
  mismatch is ALSO `ChangedFileFetchIncomplete`.
- Never classified `obviously_irrelevant` in either failure mode — the
  existing `census()`-level handling (R0B) already treats every
  `ChangedFileFetchIncomplete`, regardless of its own specific cause,
  identically: recorded per-PR, and the whole window-check raised as
  `WindowEvaluationIncomplete` if any occur among the systematically-
  sampled census records.

**Regression coverage**:
`test_changed_files_reported_zero_retrieved_zero`,
`test_changed_files_reported_one_retrieved_one`,
`test_changed_files_reported_2999_retrieved_2999`,
`test_changed_files_reported_exactly_3000_retrieved_3000`,
`test_changed_files_reported_3001_fails_closed_without_attempting_file_list_fetch`
(asserts the `/files` fetch is never even attempted),
`test_changed_files_count_mismatch_is_incomplete`,
`test_changed_files_metadata_transient_failure_then_success`,
`test_changed_files_metadata_fails_all_attempts`,
`test_incomplete_file_list_case_does_not_become_obviously_irrelevant_r0c`
(drives the SAME `census()`-level guard through the NEW failure mode
specifically, confirming `census` never inspects `.cause` to decide
relevance).

---

## Endpoint-completeness audit (item 8), scoped to P0 only

| Endpoint | Documented cap | Completeness established by | Fail-closed behavior |
|---|---|---|---|
| `search/issues` (population discovery) | 1000 results per query, regardless of pagination | Temporal sharding to whole-UTC-day granularity + `total_count`/`incomplete_results` check per shard + retrieved-vs-`total_count` consistency check (R0B) | `PopulationIncompleteError`/`PopulationQueryIncomplete` — fails the whole window evaluation |
| `GET /pulls/{n}` (PR metadata, `changed_files` field) | None relevant — a single object, not a paginated listing | N/A (not itself capped) | `ChangedFileFetchIncomplete` on retry exhaustion (R0C) |
| `GET /pulls/{n}/files` (changed-file listing) | 3000 files per PR, regardless of pagination | Cross-checked against the PR's own authoritative `changed_files` metadata count (R0C) | `ChangedFileFetchIncomplete` (cap exceeded, or count mismatch) |

**No other P0 endpoint exists.** `census`/`fetch_merged_prs_in_window`/
`fetch_changed_files` are the complete set of network-touching
functions in this tooling. **No further undocumented/unchecked hard
truncation was found.** This audit does not extend to P1's own future
tooling (target construction, `oba` execution) — out of this round's
own explicit scope.

---

## What did NOT change

Every design decision frozen in `218bf47`, `0cb69b0`, and `ccdfbe9` —
the scientific question, the window INCREMENT/hard-cap VALUES (7/42
days — only the MATURITY precondition for evaluating a window is new,
never the window sizes themselves), the cheap relevance filter, the
leakage-prevention mechanism, the sampling seed, the multi-target
derivation rule and its own budget cap, the NC2/NC3/NC4 denominators,
the corrected expansion-batch gate/stopping rule, the blinded
adjudication procedure, the escalation rule, the error taxonomy, the
decision not to run a known-limitations challenge set, and the
`STOP_DEFECT_FOUND` decision-gate rule — all unchanged.

**Classification, as anticipated by this round's own mandate**:
temporal maturity is an EXECUTION PRECONDITION implicit in a
future-complete frozen window, now made explicit and mechanically
enforced (not a new scientific rule); changed-file completeness is an
IMPLEMENTATION SAFEGUARD for the already-frozen metadata-only relevance
classifier (not a new relevance criterion — the filter itself,
`nixos/modules/**`/`nixos/tests/**` path prefixes, is byte-for-byte
unchanged). **No outcome-dependent sampling semantics changed.**

## Scope confirmation

No analyzer source changed. No `oba` execution occurred. No real
post-release population was queried or inspected. No fresh nixpkgs PR
content was examined. No result files exist. No P1 target construction
occurred. No adjudication occurred. No residual-limitation work was
performed.

**`S6_R0C_PRE_P0_GUARDS_COMPLETE`**

**STOP.** No P0 execution without a separate, explicit GO, and not
before `2026-10-01T05:46:05Z` (`2026-10-01 10:46:05` Almaty) — the
frozen initial window's own upper bound — has actually elapsed.
