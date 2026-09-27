# S6 P0: window 1 census result (real execution)

**Parent protocol commits**: S6-R0 `218bf4727b86ba3234252bc636e4329dcdd22414`;
S6-R0A `0cb69b0fb3b3a50643af6a406cf080dee657d705`; S6-R0B
`ccdfbe91b1fdb7f668e413ce0e870ab2880fd35b`; S6-R0C
`62bdec70cb76c86bb1708d9c95b632f8ff86f7ca`; S6-R0D
`8da7653ce08cdf69836dc6bf4fb53a4dcc1ab03c`.

**This is the first real execution of `population-and-sampling.py`
against the live post-v0.5.0 candidate window.** Executed
2026-09-27, after the frozen initial window's own upper bound
(`2026-09-27T05:46:05Z`) had actually elapsed, per an explicit GO
following the already-detailed P0 scope the user specified before the
window matured (permitted actions: full population fetch, systematic
sample ≤100, changed-file metadata only, `potentially_relevant`
classification, NC1 check, freeze-on-PASS, extend-on-FAIL, no OBA/
target construction/manual PR reading — all observed exactly, see
"Scope confirmation" below).

## Window 1

- **Bounds**: `[2026-09-24T05:46:05Z, 2026-09-27T05:46:05Z)` — the
  frozen initial (post-R0D) 3-day window.
- **Raw day-granularity population** (GitHub `merged:2026-09-24..2026-09-27`,
  inclusive-date, pre exact-timestamp-trim): `total_count = 543`,
  `incomplete_results = false`.
- **Census-cap systematic sample inspected**: `100` (`CENSUS_CAP`) —
  evenly spaced across the exact-timestamp-filtered, deduplicated,
  sorted merged-PR list, per `systematic_sample`.
- **Changed-file metadata fetch**: all 100 succeeded (no
  `ChangedFileFetchIncomplete`, no `WindowEvaluationIncomplete`).

## NC1 result: **FAIL** (3 < 20)

`potentially_relevant` (via the S5-inherited `nixos/modules/**`/
`nixos/tests/**` path-prefix filter, metadata-only):

| PR | changed files |
|---|---|
| [#566270](https://github.com/NixOS/nixpkgs/pull/566270) | 24 |
| [#552123](https://github.com/NixOS/nixpkgs/pull/552123) | 2 |
| [#562582](https://github.com/NixOS/nixpkgs/pull/562582) | 4 |

`3 potentially_relevant` among `100` inspected — below
`NC1_THRESHOLD = 20`. Per the frozen `find_final_window` rule, this
does **not** freeze a final window/relevant pool; it triggers the
`+3 day` extension.

Full 100-record inspection log (PR number, file count, classification):
`fixtures/s6-r0/P0-window1-raw-log.json`.

## Extension attempt: window `[2026-09-24T05:46:05Z, 2026-09-30T05:46:05Z)`

`find_final_window` extended `upper` by `WINDOW_INCREMENT_DAYS` (3) to
`2026-09-30T05:46:05Z` and called `census` again for this 6-day
window. The maturity guard (S6-R0C) fired **before any network call**:

```
window_not_matured: now=2026-09-27T06:14:45.254395+00:00 < required
upper bound 2026-09-30T05:46:05+00:00 -- no GitHub candidate query
was performed
```

This is the correct, expected outcome per the user's own item 7
("при <20 перейти к следующему 6-дневному matured window") — the
transition was attempted mechanically; the next window is simply not
mature yet. `WindowNotMatured` propagated uncaught, exactly as
`find_final_window`'s own contract specifies (never reinterpreted as
`STOP_LOW_YIELD` for window 1, never skipped ahead).

## Next eligible check

**`2026-09-30T05:46:05Z`** (`2026-09-30 10:46:05` Almaty) — the 6-day
window's own upper bound. At that point, `census(LOWER_BOUND,
2026-09-30T05:46:05Z, ...)` may run for real: a **fresh, full 100-PR
systematic sample of the (now larger) 6-day population** — not merely
"add 3 more days' worth of PRs to the existing 3 relevant found," per
`systematic_sample`'s own whole-window re-sampling semantics. The 3
PRs identified above are not privileged; they may or may not recur in
the window-2 sample.

## Scope confirmation

Exactly the 8 permitted P0 actions were performed against real data:
(1) full population fetched for the window, (2) systematic sample of
100, (3) changed-file metadata only (no diff content, no titles, no
labels read), (4) `potentially_relevant` computed, (5) NC1 `>=20`
applied, (6) not reached (FAIL), (7) extension to the next 6-day
window attempted and found immature, (8) no OBA execution, no target
construction, no manual reading of PR content beyond the frozen
metadata-only filter. No P1 sample was drawn. No adjudication
occurred.

**`S6_P0_WINDOW1_CENSUS_COMPLETE_NC1_FAIL_EXTENDED_IMMATURE`**

**STOP.** No further P0 execution before `2026-09-30T05:46:05Z` has
actually elapsed. No P1/P2 execution without a separate, explicit GO.
