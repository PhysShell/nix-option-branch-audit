# S6 P0: window 2 census result (real execution)

**Parent**: `fixtures/s6-r0/P0-window1-result.md` (window 1, NC1 FAIL
3/100, extension immature). Same protocol commits
(`218bf47`/`0cb69b0`/`ccdfbe9`/`62bdec7`/`8da7653`).

Executed 2026-09-30, after the 6-day window's own upper bound
(`2026-09-30T05:46:05Z`) had actually elapsed. `find_final_window`
re-evaluates from scratch, so this run re-censused window 1 before
reaching window 2 — see "Window 1 redo" below for why that's expected
and not a scope violation.

## Window 1 redo (sanity check)

Same bounds as before, `[2026-09-24T05:46:05Z, 2026-09-27T05:46:05Z)`.
`find_final_window` always restarts from `LOWER_BOUND +
WINDOW_INCREMENT_DAYS`, so it re-ran window 1's census rather than
resuming from window 2 directly -- this is the frozen algorithm's own
behavior, not a deviation. Result: **identical** to the first run --
100 inspected, exactly the same 3 `potentially_relevant` PRs
(#566270, #552123, #562582). This is expected: window 1's time range
is fixed in the past, so the merged-PR population within it cannot
change. Useful as an unplanned determinism check on the tooling
itself.

## Window 2

- **Bounds**: `[2026-09-24T05:46:05Z, 2026-09-30T05:46:05Z)` -- the
  6-day extension (`+3` from window 1).
- **Raw day-granularity population** (`merged:2026-09-24..2026-09-30`,
  inclusive-date): `total_count = 1263`, `incomplete_results = false`.
- **Census-cap systematic sample inspected**: 100 (fresh sample of the
  whole 6-day window, not merely "the previous 100 plus 3 more days" --
  per `systematic_sample`'s whole-window re-sampling semantics; no
  overlap with the window-1 sample).
- **Changed-file metadata fetch**: all 100 succeeded.

## NC1 result: **FAIL** (4 < 20)

| PR | changed files |
|---|---|
| [#547371](https://github.com/NixOS/nixpkgs/pull/547371) | 1 |
| [#563434](https://github.com/NixOS/nixpkgs/pull/563434) | 3 |
| [#560908](https://github.com/NixOS/nixpkgs/pull/560908) | 1 |
| [#565369](https://github.com/NixOS/nixpkgs/pull/565369) | 2 |

`4 potentially_relevant` among `100` inspected -- below
`NC1_THRESHOLD = 20`. Extension to the next window triggered.

Full 200-record inspection log (window-1 redo + window-2):
`fixtures/s6-r0/P0-window2-raw-log.json`.

## Extension attempt: window `[2026-09-24T05:46:05Z, 2026-10-03T05:46:05Z)`

`find_final_window` extended `upper` by `WINDOW_INCREMENT_DAYS` (3) to
`2026-10-03T05:46:05Z` (9-day window) and called `census` again. The
maturity guard fired before any network call:

```
window_not_matured: now=2026-09-30T10:55:07.831395+00:00 < required
upper bound 2026-10-03T05:46:05+00:00 -- no GitHub candidate query
was performed
```

## Next eligible check

**`2026-10-03T05:46:05Z`** (`2026-10-03 10:46:05` Almaty) -- the 9-day
window's own upper bound.

## Scope confirmation

Same 8 permitted P0 actions, nothing else: full population fetched,
systematic 100-sample, changed-file metadata only, `potentially_relevant`
computed, NC1 applied (FAIL both times), extension attempted and found
immature, no OBA/target construction/manual reading. No P1 sample
drawn.

**`S6_P0_WINDOW2_CENSUS_COMPLETE_NC1_FAIL_EXTENDED_IMMATURE`**

**STOP.** No further P0 execution before `2026-10-03T05:46:05Z` has
actually elapsed. No P1/P2 execution without a separate, explicit GO.
