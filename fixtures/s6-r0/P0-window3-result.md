# S6 P0: window 3 real census result, plus a disclosed structural finding about NC1

**Parents**: `fixtures/s6-r0/P0-window1-result.md`, `P0-window2-result.md`.
Same protocol commits (`218bf47`/`0cb69b0`/`ccdfbe9`/`62bdec7`/`8da7653`).

Executed 2026-10-03, after the 9-day window's own upper bound
(`2026-10-03T05:46:05Z`) had elapsed.

## Windows 1 and 2 redo (sanity check)

`find_final_window` always restarts from `LOWER_BOUND +
WINDOW_INCREMENT_DAYS`. Both re-censused identically to their first
runs: window 1 -- 3/100 (#566270, #552123, #562582); window 2 -- 4/100
(#547371, #563434, #560908, #565369). Determinism reconfirmed a second
time.

## Window 3

- **Bounds**: `[2026-09-24T05:46:05Z, 2026-10-03T05:46:05Z)`, 9 days.
- **Raw day-granularity population** (`merged:2026-09-24..2026-10-03`):
  `total_count = 2044`.
- **Census-cap systematic sample inspected**: 100 (fresh sample of
  the whole 9-day window).

## NC1 result: **FAIL** (7 < 20)

| PR | changed files |
|---|---|
| [#566108](https://github.com/NixOS/nixpkgs/pull/566108) | 2 |
| [#536237](https://github.com/NixOS/nixpkgs/pull/536237) | 2 |
| [#566964](https://github.com/NixOS/nixpkgs/pull/566964) | 4 |
| [#565501](https://github.com/NixOS/nixpkgs/pull/565501) | 2 |
| [#567238](https://github.com/NixOS/nixpkgs/pull/567238) | 6 |
| [#569026](https://github.com/NixOS/nixpkgs/pull/569026) | 1 |
| [#564258](https://github.com/NixOS/nixpkgs/pull/564258) | 1 |

7/100 (7%) -- still below `NC1_THRESHOLD = 20`, though higher than
window 1 (3%) and window 2 (4%). Consistent with ordinary sampling
noise around a true rate of ~4% (binomial stderr on n=100 at p=0.04
is ~2; 7 is about 1.5 sigma above 4, not an outlier). Extension to
the 12-day window attempted, hit the maturity guard
(`2026-10-06T05:46:05Z` not yet elapsed). Full 300-record log:
`fixtures/s6-r0/P0-window3-raw-log.json`.

## Disclosed side-finding: NC1's own design does not scale with window size (diagnostic, NOT an S6 protocol change)

Between window 2 and window 3, the user and I discussed, in chat, why
three consecutive real checks (3%, 4%, 7%) sit so far below the 20%
`NC1_THRESHOLD`. The structural reason: `CENSUS_CAP = 100` is fixed
regardless of window length, and `systematic_sample` always draws
exactly 100 points spread across the whole window -- so extending the
calendar window does **not** increase the sample size or the expected
*proportion* found relevant; it only reduces estimation variance on
the same underlying rate. NC1 can only pass if the true
`potentially_relevant` rate in the live stream is itself near 20% --
which these three checks suggest it is not.

To tell whether this is a temporary post-release dip or the ordinary
background rate, a **separate, one-off historical diagnostic** was
run (NOT part of the S6 population, NOT used as pilot targets, NOT
committed as an S6 sample): the same sampling/classification
functions (`fetch_merged_prs_in_window`, `systematic_sample`,
`obviously_irrelevant_or_potentially_relevant`), unmodified, over
`[2026-03-01T00:00:00Z, 2026-03-31T00:00:00Z)` -- a month chosen only
for being far from both the release and the S5 corpus period, with no
other selection criterion. Result: raw population 6328, 100-PR
systematic sample, **3/100 (3%) relevant** (#491275, #499398,
#497612; full log `fixtures/s6-r0/P0-window3-historical-diagnostic-march2026.json`).
This matches the live post-release rate closely -- the ~3-4% density
is the ordinary nixpkgs background rate, not an artifact of the
current window.

**Decision, made explicitly and recorded here rather than silently
acted on**: this is a real flaw in NC1's design (it assumes a ~20%
background rate that does not hold), discovered only after observing
real S6 data plus this historical diagnostic -- so fixing NC1 *inside*
the already-touched S6-R0 now would be a straightforward post-hoc
methodology change, exactly what R0A-R0D existed to prevent. The
chosen path instead: leave NC1/`CENSUS_CAP`/the window-extension
mechanism completely untouched here, let `find_final_window` continue
mechanically to the frozen `WINDOW_HARD_CAP_DAYS = 42`, and accept
whatever it produces (almost certainly `STOP_LOW_YIELD` /
`SystemExit`, given the rates observed so far) as a legitimate,
disclosed finding of this pilot -- "the fresh post-release stream's
`potentially_relevant` density did not match the density S6-R0's own
NC1 assumed." Any recalibration of NC1/CENSUS_CAP belongs to a
separate, freshly preregistered future round (working title S6-R1),
informed by this rate as legitimate prior information for a design
decision made before *that* round's own population is observed --
not as a retroactive edit to this one.

## Next eligible check

**`2026-10-06T05:46:05Z`** (`2026-10-06 10:46:05` Almaty) -- the
12-day window's own upper bound.

## Scope confirmation

Same 8 permitted P0 actions on the S6 population, nothing else. The
historical diagnostic is explicitly out-of-band: separate window, not
drawn from the S6 population, not used for P1/target construction, not
treated as part of the S6 sample.

**`S6_P0_WINDOW3_CENSUS_COMPLETE_NC1_FAIL_EXTENDED_IMMATURE_NC1_DESIGN_FLAW_DISCLOSED`**

**STOP.** No further P0 execution before `2026-10-06T05:46:05Z` has
elapsed. No NC1/protocol edit in this round. No P1/P2 execution
without a separate, explicit GO.
