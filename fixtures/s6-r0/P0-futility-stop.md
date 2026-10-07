# S6 P0: early stop for futility -- `STOP_LOW_YIELD`

**Parents**: `fixtures/s6-r0/P0-window1-result.md`, `P0-window2-result.md`,
`P0-window3-result.md`. Same protocol commits (`218bf47`/`0cb69b0`/
`ccdfbe9`/`62bdec7`/`8da7653`), plus the three real window commits
(`7cc98b2`/`0c2f0b3`/`a2e091c`).

## What this is, and what it is not

This is a **decision to stop early**, not a change to NC1, to
`CENSUS_CAP`, to the window-increment rule, or to the population
source. No historical data was substituted for the S6 population at
any point. No PR outside the live post-release stream was used as
evidence for this decision. The three real census checks already run
(windows 1-3) and the already-disclosed historical diagnostic
(`P0-window3-result.md`, March 2026, run only to answer "is ~4% the
ordinary background rate" -- never as S6 population) are the entire
evidentiary basis.

## Why stop now instead of running to `WINDOW_HARD_CAP_DAYS = 42`

Three independent real checks on the live post-release stream:

| Window | Days | `potentially_relevant` / inspected |
|---|---|---|
| 1 | 3 | 3 / 100 (3%) |
| 2 | 6 | 4 / 100 (4%) |
| 3 | 9 | 7 / 100 (7%) |

Plus one historical diagnostic (March 2026, same functions, not S6
population): 3 / 100 (3%).

All four are consistent with one underlying rate of roughly 4%. As
established in `P0-window3-result.md`, `CENSUS_CAP` does not grow with
the window, so further extension (12, 15, ..., 42 days) does not
increase sample size -- it only narrows the estimate of the same rate.
Reaching `NC1_THRESHOLD = 20` (20%) from an observed ~4% would require
a true rate roughly 5x higher than every measurement taken so far, a
gap far outside normal sampling variance (binomial stderr at n=100,
p=0.04 is ~2 points; 20 is ~8 stderr above the pooled estimate).
Continuing the remaining ~10 mechanical extension steps (through
2026-11-05T05:46:05Z) would cost about another month of real-time
waiting to observe, with very high confidence, the same
`STOP_LOW_YIELD` outcome the hard cap was always going to produce.

This is a **futility stop**: the same methodological device as
stopping a trial early when an interim analysis makes the primary
endpoint statistically unreachable, not a redefinition of what counts
as evidence or where it comes from. The decision uses only the
already-observed real S6 measurements (windows 1-3) plus the already-
disclosed out-of-band diagnostic; it does not look at, or use, any
further data.

## Outcome

**`STOP_LOW_YIELD`** -- declared now, in place of the mechanical
`SystemExit` that `find_final_window` would eventually raise at the
hard cap. Equivalent outcome, reached by explicit early-stop reasoning
instead of by exhausting the remaining real-time wait.

**Finding for the record**: the fresh post-v0.5.0 nixpkgs stream's
`potentially_relevant` density (~4%, per the metadata-only
`nixos/modules/**`/`nixos/tests/**` filter inherited from S5) does not
match the ~20% density `NC1_THRESHOLD` assumed at S6-R0 preregistration
time. This is a property of the population, not of v0.5.0's own
correctness -- S6-R0's P0 never reached the point of inspecting any
PR's diff content, option declarations, or running `oba` against
anything. **No conclusion about v0.5.0's own correctness can be drawn
from this pilot.** It answered a narrower, prior question ("is there
enough raw volume to justify a P1/P2 pilot") in the negative.

## What did not happen

No P1 sample was drawn. No target construction occurred. No `oba`
execution occurred. No adjudication occurred. No historical PR was
used as S6 population or as a pilot target -- the March 2026 data
remains strictly a background-rate diagnostic, never promoted to
S6 evidence.

## Next step (not started here)

A future, separately preregistered round (working title S6-R1) may
recalibrate `NC1_THRESHOLD`/`CENSUS_CAP` using the ~4% rate observed
here as legitimate prior information, decided before that round's own
population is observed -- the same relationship an ordinary pilot
study has to the main study it informs. That round is out of scope
for this document and requires its own future, explicit GO.

**`S6_R0_P0_STOPPED_LOW_YIELD_BY_FUTILITY`**

**STOP.** S6-R0's P0 phase is concluded. No P1/P2 execution. No NC1/
protocol edit retroactively applied to R0-R0D. No historical data used
as S6 evidence beyond the already-disclosed diagnostic above.
