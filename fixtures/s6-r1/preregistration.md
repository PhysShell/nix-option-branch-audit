# S6-R1: NC1 recalibration, preregistered before any R1 population is observed

**Parent**: the complete S6-R0 chain -- preregistration `218bf47`,
amendments `0cb69b0`/`ccdfbe9`/`62bdec7`/`8da7653`, real P0 execution
`7cc98b2`/`0c2f0b3`/`a2e091c`, futility stop `65e7783`. S6-R0's own P0
phase is concluded (`STOP_LOW_YIELD`) and is **not reopened** by this
document -- nothing here edits R0-R0D's history or results.

**Subject under test**: unchanged -- the same released `oba` `v0.5.0`
(`0a6f1928c650c240ae0f709d413abc6ec6b65688`).

**What changed, and why**: S6-R0's three real census checks (3%, 4%,
7% `potentially_relevant`, pooled with a March 2026 background-rate
diagnostic at 3%) showed the live nixpkgs stream's density is ~4%, not
the ~20% `NC1_THRESHOLD=20` implicitly assumed when `CENSUS_CAP=100`
was frozen in `218bf47`. `CENSUS_CAP` did not scale with window size,
so extending the window could only narrow the estimate of the same
~4% rate, never raise it -- R0's own `P0-futility-stop.md` disclosed
this and explicitly deferred any fix to a future, separately
preregistered round. This is that round. It uses the ~4% rate
established in R0 as **prior information for a design decision**,
exactly the way a pilot study's results inform a main study's sample
size -- not as a retroactive edit to R0's own frozen numbers.

## The recalibration

**Pooled rate estimate** (R0's own 4 independent measurements, equal
weight, no cherry-picking): `(3+4+7+3)/(100+100+100+100) = 17/400 =
4.25%`.

**`NC1_THRESHOLD` stays `20`, unchanged.** Its role was never a
percentage -- it is an absolute floor on how many real,
`potentially_relevant` candidates must exist before trusting the pool
to supply `P1_SAMPLE_SIZE=15` (the original 20/15 ratio is a ~1.33x
margin for later exclusions, not-adjudicable cases, etc.). That
reasoning does not depend on how rare `potentially_relevant` records
are in the underlying stream, so there is no statistical reason to
move it.

**`CENSUS_CAP` becomes `750`** (from `100`). Modeling the count of
`potentially_relevant` records among `n` inspected as approximately
Binomial(`n`, `p=0.0425`):

| `n` (CENSUS_CAP) | mean | stderr | `(mean-20)/stderr` | approx. `P(X>=20)` |
|---|---|---|---|---|
| 100 (R0's own value) | 4.25 | 2.02 | -7.8 | ~0% |
| 500 | 21.3 | 4.52 | 0.29 | ~62% |
| 750 | 31.9 | 5.53 | 2.15 | ~98% |
| 1000 | 42.5 | 6.38 | 3.53 | ~99.98% |

`750` was chosen (by explicit agreement, not unilaterally) as the
value giving a ~98% chance of clearing the unchanged `NC1_THRESHOLD`
if the true rate matches R0's pooled estimate -- comfortably safe
without inspecting an order of magnitude more than necessary. This is
the single free parameter in this recalibration; the table above is
recorded so a different margin could be chosen by inspection alone,
without rederiving the arithmetic.

## Temporal anchor: a new `LOWER_BOUND`, not a reuse or an edit of R0's window

`LOWER_BOUND_R1 = 2026-10-07T02:14:31Z` -- the moment this document
was written (verifiable against this round's own commit timestamp).
This is **not** the same anchor as R0's `LOWER_BOUND`
(`2026-09-24T05:46:05Z`, `v0.5.0`'s own publish time). Reasoning:

- The subject under test (`oba` `v0.5.0`) is unchanged, so there is no
  requirement that the population start exactly at the release
  instant -- only that it start at or after it, so the population
  could not have influenced the analyzer's own development.
  `2026-10-07` satisfies this trivially, being two weeks later.
- R0's P0 already inspected (metadata-only: PR number, file count,
  relevance classification -- never diff content, never option
  declarations) up to 300 records across its three real windows, all
  within `[2026-09-24T05:46:05Z, 2026-10-03T05:46:05Z)`. Anchoring
  `LOWER_BOUND_R1` at `2026-10-07T02:14:31Z` -- strictly after that
  entire range -- means **R1's population has zero temporal overlap**
  with anything R0 already looked at. No PR-number exclusion list is
  needed for this reason (unlike S5's 369-PR ledger, which overlaps in
  content/time and does need the existing exclusion mechanism, kept
  below).
- This also means R1 answers a slightly fresher question than R0 did
  ("is there enough signal from Oct 7 onward"), which is a strict
  improvement for the freshness goal, not a compromise of it.

## What is inherited unchanged

Everything not named above, verbatim: the cheap relevance filter
(`nixos/modules/**`/`nixos/tests/**` path-prefix, metadata-only); the
two-phase paginated search with the 1000-result Search API cap
handling; `MAX_FETCH_ATTEMPTS=3` and its retry semantics; the temporal
maturity guard (`now_utc < upper` checked before any network call);
the changed-file completeness guard (`GITHUB_PR_FILES_HARD_CAP=3000`,
cross-checked against the PR's own `changed_files` metadata field);
`systematic_sample`'s whole-window re-sampling behavior;
`P1_SAMPLE_SIZE=15`; `MAX_TARGETS_TOTAL=30` (a pilot-wide cost bound on
P1 target derivation, independent of `CENSUS_CAP`); the seed
convention `seed = int(RELEASE_COMMIT_SHORT, 16)` (the subject under
test, hence its identity, is unchanged); `check_leakage()` against the
existing `excluded-pr-identities.json` (S5's 369-PR ledger) --
retained as a defense-in-depth check even though R1's forward-anchored
window makes overlap with it very unlikely; the staged P0->P1->P2
design; the kill-first decision gate; the blinded adjudication
procedure; the error taxonomy; the decision not to run a
known-limitations challenge set.

## Window increment and hard cap

Observed raw merged-PR volume in R0 was ~180-230 PRs/day. An initial
window needs to comfortably exceed `CENSUS_CAP=750` in raw population
so the first real check actually draws a full 750-record sample
(`systematic_sample` returns the whole population unsampled if it is
smaller than the cap, which would silently under-power the very
margin just computed). `WINDOW_INCREMENT_DAYS_R1 = 5` (expected raw
population ~900-1150, safely above 750). Extension step: `+5` days,
same clean arithmetic shape as R0's own `+3`. `WINDOW_HARD_CAP_DAYS_R1
= 30` (6 steps) -- a generous fallback that should rarely be needed
given the 98% single-check pass probability computed above; kept for
the same reason R0 kept one, not because the design expects to need
it.

**First eligible P0 execution time: `LOWER_BOUND_R1 +
WINDOW_INCREMENT_DAYS_R1` = `2026-10-12T02:14:31Z`.**

## Confirmation

No R1 population has been queried, sampled, or inspected as of this
document. `LOWER_BOUND_R1` is a future instant relative to this
commit's own timestamp (window has not even started accruing a full 5
days of data yet), so even a maturity-guard-bypassing accident could
not have produced a real result before this preregistration existed.

**`S6_R1_PREREGISTERED`**

**STOP.** No P0 execution without a separate, explicit GO, and not
before `2026-10-12T02:14:31Z` has actually elapsed.
