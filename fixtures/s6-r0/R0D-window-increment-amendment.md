# S6-R0D: initial P0 window shrunk from 7 to 3 days

**Parent protocol commits**: S6-R0 preregistration
`218bf4727b86ba3234252bc636e4329dcdd22414`; S6-R0A amendment
`0cb69b0fb3b3a50643af6a406cf080dee657d705`; S6-R0B amendment
`ccdfbe91b1fdb7f668e413ce0e870ab2880fd35b`; S6-R0C amendment
`62bdec70cb76c86bb1708d9c95b632f8ff86f7ca`.

**Confirmation, stated explicitly and verified against the actual
session record**: no real S6 candidate population, PR content, or
result was queried or inspected at any point before this amendment.
Unlike R0A/R0B/R0C (each triggered by a bug or inconsistency found via
static review), R0D is a deliberate, reasoned PARAMETER CHOICE, made
explicitly before any S6 data exists to influence it -- exactly the
distinction that keeps it methodologically sound rather than a
post-hoc adjustment: this changes the COST of a cheap falsifier, not
the criterion itself, and not in response to any observed result.

**Effect on `218bf47`/`0cb69b0`/`ccdfbe9`/`62bdec7`**: all four remain
the immutable historical record, unedited beyond their own
forward-pointer banners. This document is a further amendment layer.

---

## The change

- **Initial P0 window**: `7 days -> 3 days`
  (`WINDOW_INCREMENT_DAYS = 3`).
- **Extension increment**: `+3 days` each step (was `+7`) -- the same
  clean arithmetic shape as before (`3 -> 6 -> 9 -> ... `), only the
  unit shrank.
- **Hard cap**: unchanged, `WINDOW_HARD_CAP_DAYS = 42` -- the overall
  "give up" ceiling is untouched; only the per-check granularity got
  cheaper (14 three-day steps to reach the cap, instead of 6 seven-day
  steps).
- **First eligible P0 execution time, recomputed**: `LOWER_BOUND +
  3 days` = `2026-09-27T05:46:05Z` (`2026-09-27 10:46:05` Almaty) --
  tomorrow morning relative to this round, not `2026-10-01`.

## Rationale

P0's own job is to answer exactly one cheap question: "does the fresh
post-release stream contain enough `potentially_relevant` volume to be
worth a 15-PR pilot?" A 7-day window overpays for that single density
check -- nixpkgs' own well-known scale means 7 days of raw merge volume
is far more than needed just to estimate a rate. A single day was
considered and rejected: it risks landing on an unrepresentative slice
of activity (a quiet weekend, a maintenance freeze, an unusually
active day from one large batch of related PRs) purely by chance. 3
days is the deliberately chosen middle ground -- enough to smooth over
a single unusual day, cheap enough that the first kill-gate check
completes tomorrow morning rather than in a week.

This is NOT a reaction to any observed density, rate, or content --
this round queried nothing. It is a reasoned adjustment to the
experiment's own COST structure, made while the experiment is still
provably blind, which is exactly the condition under which
"adjusting the methodology" remains legitimate rather than becoming
the failure mode preregistration exists to prevent.

## What changed in code

Exactly one constant, `population-and-sampling.py`'s own
`WINDOW_INCREMENT_DAYS` (`7 -> 3`). Every existing test in
`test_population_and_sampling.py` reads this constant symbolically
(`ps.WINDOW_INCREMENT_DAYS`), never a hardcoded `7` -- confirmed by
direct grep before this amendment was made, and by all 55 pre-existing
tests continuing to pass unmodified afterward. `WINDOW_HARD_CAP_DAYS`
(42) was not touched.

**Regression coverage** (3 new tests):
`test_window_increment_is_three_days_not_seven` (confirms both the new
value and that the hard cap is unchanged),
`test_first_window_upper_bound_is_three_days_after_lower_bound` (a
pure datetime-object comparison, both sides derived from
`ps.LOWER_BOUND` -- deliberately never a hand-typed date-string
literal for the real window boundary, to stay compliant with this same
file's own "no quoted S6-window date strings" blindness-preservation
check),
`test_extension_reaches_hard_cap_in_fourteen_three_day_steps`.

## What did NOT change

The scientific question, the temporal LOWER bound and its own
rationale, the maturity guard mechanism itself (R0C, unaffected --
still checks `now_utc < upper` regardless of what `upper` computes to),
the changed-file completeness guard, the cheap relevance filter, the
leakage-prevention mechanism, the sampling seed, the multi-target
derivation rule and its own budget cap, the NC2/NC3/NC4 denominators,
the expansion-batch gate/stopping rule, the blinded adjudication
procedure, the escalation rule, the error taxonomy, the decision not to
run a known-limitations challenge set, and the `STOP_DEFECT_FOUND`
decision-gate rule. **NC1's own threshold value (>=20 relevant PRs)
is unchanged** -- only how many CALENDAR DAYS are given to reach it
before the first kill-check fires.

## Scope confirmation

No analyzer source changed. No `oba` execution occurred. No real
post-release population was queried or inspected. No fresh nixpkgs PR
content was examined. No result files exist. No P1 target construction
occurred. No adjudication occurred. No residual-limitation work was
performed.

**`S6_R0D_WINDOW_AMENDMENT_COMPLETE`**

**STOP.** No P0 execution without a separate, explicit GO, and not
before `2026-09-27T05:46:05Z` (`2026-09-27 10:46:05` Almaty) -- the
newly frozen initial window's own upper bound -- has actually elapsed.
