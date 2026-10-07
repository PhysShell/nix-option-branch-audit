# S6-R1 P0: real census, NC1 PASS -- final window and relevant pool frozen

**Parent**: `fixtures/s6-r1/preregistration.md` (`140354d`). Same
subject under test (`oba` `v0.5.0`).

## A deliberate deviation from the preregistered window anchor, decided in chat before running

`preregistration.md` anchored `LOWER_BOUND` at this round's own
preregistration instant (`2026-10-07T02:14:31Z`) specifically so a
*new* 5-day window would have zero temporal overlap with S6-R0's own
inspected records. Before that window could mature, the user and the
assistant explicitly discussed and agreed (in chat, not silently) to
run the census instead against the window `[2026-09-24T05:46:05Z,
now)` -- i.e. the full span from `v0.5.0`'s own release instant
(S6-R0's `LOWER_BOUND`) through the real moment of execution -- using
the R1 module's recalibrated `CENSUS_CAP=750`/`NC1_THRESHOLD=20`
called via `census()` directly rather than through
`find_final_window()` (which would have used R1's own
`LOWER_BOUND`/`WINDOW_INCREMENT_DAYS` and required a further wait).

**Why this is not a leakage/freshness violation**: the window is still
entirely post-release (identical freshness guarantee to every S6-R0
window). It overlaps with the small samples S6-R0's P0 already drew
(up to 300 metadata-only inspected records across windows 1-3), but
that overlap does not bias a *density* estimate -- `systematic_sample`
draws mechanically from the whole population regardless of what was
previously looked at, and P0 never uses the specific identity of a
relevant PR to decide anything (that only starts to matter at P1
target construction / adjudication, which this document does not
reach). The window was not chosen after seeing its own content -- it
is simply the entire existing fresh period, the least arbitrary choice
available, not a cherry-picked one.
`CENSUS_CAP=750`/`NC1_THRESHOLD=20` were fixed in `preregistration.md`
before this specific window's content was inspected.

## Real execution

- **Window**: `[2026-09-24T05:46:05Z, 2026-10-07T02:40:18.634587Z)`
  (~13 days).
- **Census-cap systematic sample inspected**: 750 (full `CENSUS_CAP`,
  not truncated -- raw population comfortably exceeded 750).
- **Changed-file metadata fetch**: all 750 succeeded (no
  `ChangedFileFetchIncomplete`).

## NC1 result: **PASS** (26 >= 20)

26 / 750 = **3.47%** `potentially_relevant` -- consistent with S6-R0's
own pooled estimate (~4.25%) and within ordinary sampling variance of
it.

| PR | files | | PR | files |
|---|---|---|---|---|
| [#443747](https://github.com/NixOS/nixpkgs/pull/443747) | 6 | | [#564688](https://github.com/NixOS/nixpkgs/pull/564688) | 20 |
| [#481321](https://github.com/NixOS/nixpkgs/pull/481321) | 2 | | [#565943](https://github.com/NixOS/nixpkgs/pull/565943) | 2 |
| [#508090](https://github.com/NixOS/nixpkgs/pull/508090) | 2 | | [#566007](https://github.com/NixOS/nixpkgs/pull/566007) | 1 |
| [#528138](https://github.com/NixOS/nixpkgs/pull/528138) | 1 | | [#566155](https://github.com/NixOS/nixpkgs/pull/566155) | 1 |
| [#541612](https://github.com/NixOS/nixpkgs/pull/541612) | 9 | | [#566457](https://github.com/NixOS/nixpkgs/pull/566457) | 8 |
| [#556752](https://github.com/NixOS/nixpkgs/pull/556752) | 10 | | [#566696](https://github.com/NixOS/nixpkgs/pull/566696) | 31 |
| [#561242](https://github.com/NixOS/nixpkgs/pull/561242) | 5 | | [#567915](https://github.com/NixOS/nixpkgs/pull/567915) | 10 |
| [#561461](https://github.com/NixOS/nixpkgs/pull/561461) | 3 | | [#568048](https://github.com/NixOS/nixpkgs/pull/568048) | 5 |
| [#562582](https://github.com/NixOS/nixpkgs/pull/562582) | 4 | | [#568179](https://github.com/NixOS/nixpkgs/pull/568179) | 3 |
| [#563797](https://github.com/NixOS/nixpkgs/pull/563797) | 12 | | [#568245](https://github.com/NixOS/nixpkgs/pull/568245) | 10 |
| [#563823](https://github.com/NixOS/nixpkgs/pull/563823) | 2 | | [#568429](https://github.com/NixOS/nixpkgs/pull/568429) | 4 |
| [#564662](https://github.com/NixOS/nixpkgs/pull/564662) | 3 | | [#568736](https://github.com/NixOS/nixpkgs/pull/568736) | 7 |
| | | | [#568782](https://github.com/NixOS/nixpkgs/pull/568782) | 1 |
| | | | [#569867](https://github.com/NixOS/nixpkgs/pull/569867) | 1 |

Note: `#562582` also appeared in S6-R0's window 1-3 relevant lists.
Expected and harmless per the reasoning above -- P0 density estimation
does not depend on any specific PR's identity being novel.

Full 750-record inspection log: `fixtures/s6-r1/P0-window1-raw-log.json`.

**Leakage check against S5's 369-PR ledger**: `check_leakage()` on all
26 relevant PRs returns `[]` -- no overlap.

## Per the preregistered rule: PASS freezes the final window and relevant pool

- **Frozen final window**: `[2026-09-24T05:46:05Z,
  2026-10-07T02:40:18.634587Z)`.
- **Frozen relevant pool** (26 PRs): listed above, exact numbers.

## What did not happen

No P1 sample was drawn from this pool. No target construction
occurred. No `oba` execution occurred. No adjudication occurred. Per
`preregistration.md`'s own staged design, P1 requires a separate,
explicit GO -- not implied by NC1 passing.

**`S6_R1_P0_NC1_PASS_WINDOW_AND_POOL_FROZEN`**

**STOP.** No P1/P2 execution without a separate, explicit GO.
