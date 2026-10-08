# GO-F Phase F2: fresh corpus census and sample (under amendment A1)

**Parent**: `reports/go-f-prereg.md` (`22ff47d`), `reports/go-f-prereg-amendment-A1.md`
(`7cbb2be`). Window per A1: `[2026-10-02T11:22:03Z, 2026-10-07T11:22:03Z)`.

## Census

- Raw population before exclusion (whole-day-sharded, deterministic):
  **1292**.
- Population after subtracting the frozen 1351-entry exclusion set:
  **975** (317 of the raw population were already-seen PR numbers --
  expected, since this retrospective window partially overlaps the
  period S6-R1's own 750-record census already covered).
- Systematic sample inspected (`CENSUS_CAP=750`, unchanged):
  **750**, all changed-file metadata fetches succeeded.
- **`potentially_relevant`: 30/750 (4.0%)** -- consistent with every
  prior density estimate in this lineage (S6-R0: 3%/4%/7%; March
  diagnostic: 3%; S6-R1: 3.47%).
- **NC1: PASS** (30 >= 20). Full inspection log:
  `work/go-f/census-result.json`.

## Relevant pool (30, chronological order as encountered)

`568883, 567691, 559742, 569772, 569841, 556686, 565592, 565899,
565712, 561392, 510072, 569875, 519655, 569962, 569876, 557977,
570480, 541149, 563198, 570195, 565935, 483650, 557353, 565901,
554709, 564819, 570438, 566204, 493369, 571163`.

## GO-F sample (15, deterministic, `SEED = int("0a6f192", 16)`, same seed convention)

`570480, 565935, 483650, 568883, 510072, 565712, 519655, 569962,
569876, 564819, 569875, 566204, 565592, 556686, 557977`.

**Unused (15, remain in the pool)**: `567691, 559742, 569772, 569841,
565899, 561392, 541149, 563198, 570195, 557353, 565901, 554709,
570438, 493369, 571163`.

**Leakage check** (`check_leakage` against S5's 369-PR ledger, the
same function every prior round used): `[]` -- no overlap (expected;
the broader 1351-entry exclusion set already subsumes the 369-PR
ledger, applied earlier at population level).

## What did not happen

No target construction yet (Phase F2.1/F3 next). No `oba` invocation
against any of these 15 PRs yet. No adjudication. No gate computation.

**`S6_GOF_F2_CENSUS_AND_SAMPLE_COMPLETE_NC1_PASS`**
