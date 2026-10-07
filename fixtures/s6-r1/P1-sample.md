# S6-R1 P1: sample drawn from the frozen relevant pool

**Parent**: `fixtures/s6-r1/P0-window1-result.md` (`734f522`) --
frozen window `[2026-09-24T05:46:05Z, 2026-10-07T02:40:18.634587Z)`,
26-PR relevant pool, NC1 PASS.

## Draw

`select_p1_sample(pool)` -- `random.Random(SEED).shuffle` on the
26-PR pool in its own chronological order (as encountered by
`census()`), `SEED = int("0a6f192", 16)`, same convention as S6-R0 and
historical S5 (the subject under test's identity, unchanged by R1).
`P1_SAMPLE_SIZE = 15`, inherited unchanged from `preregistration.md`.

**Pool, chronological order** (26): `565943, 566457, 564662, 566155,
562582, 556752, 443747, 481321, 566007, 561461, 508090, 561242,
567915, 563823, 568429, 564688, 566696, 568048, 568179, 568736,
569867, 568782, 563797, 568245, 528138, 541612`.

**P1 sample** (15, deterministic): `565943, 566007, 569867, 508090,
568429, 563823, 566696, 443747, 556752, 564688, 568782, 567915,
568245, 561242, 568048`.

**Not selected** (11, remain in the pool, unused): `566457, 564662,
566155, 562582, 481321, 561461, 568179, 568736, 563797, 528138,
541612`.

**Leakage check** (`check_leakage` against S5's 369-PR ledger): `[]`
-- no overlap.

## What did not happen

No target construction occurred (per
`target-construction-protocol.md`, inherited unchanged from S6-R0 --
each of these 15 PRs may yield 0, 1, or several targets, bounded by
`MAX_TARGETS_TOTAL=30`). No `oba` execution occurred. No adjudication
occurred. This document only records the sample draw itself.

**`S6_R1_P1_SAMPLE_DRAWN`**

**STOP.** Target construction and `oba` execution against these 15
PRs' real diff content requires a separate, explicit GO -- this is a
materially bigger step than the census work so far (first real
reading of PR content, first real `oba` runs against S6 data), not an
automatic continuation of this draw.
