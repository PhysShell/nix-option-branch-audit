# GO-F preregistration amendment A1: prospective-window maturation only

**Parent**: `reports/go-f-prereg.md` (`22ff47d`), `reports/go-f-evaluation.md`
(`70957d3`, Phase F2 blocked). The original preregistration's own body
is **not edited** -- this is a forward amendment, same
historical-record-preservation convention this lineage has used for
every prior correction (R0A-R0D, the GO-C kimai correction). The
experiment is valid only as **GO-F as amended by A1**, not as "the
original GO-F, unchanged" -- stated explicitly per the authorizing
instruction, not glossed over.

## Why this amendment exists

`reports/go-f-prereg.md`'s own Phase F1.2 anchored a **prospective**
5-day window, `[2026-10-07T11:22:03Z, 2026-10-12T11:22:03Z)`. At the
time Phase F2 was reached, this window had not matured (only minutes
of real time had elapsed) -- `census()`'s own maturity guard
(inherited unchanged from S6-R0C) requires `now_utc >= upper` before
any query, so no query was even attempted. `reports/go-f-evaluation.md`
recorded this honestly as `S6_GOF_F2_BLOCKED_PENDING_WINDOW_MATURITY`
and explicitly declined to invent a substitution on its own authority.

**No Phase F2 work had begun**: no eligible-population enumeration, no
PR-count inspection for any candidate window, no manual browsing of
candidate PRs, no target construction, no `oba` scoring. (Confirmed
again, freshly, immediately before writing this amendment -- see
"Integrity check" below.)

## The amendment

Replace the prospective window with the immediately preceding
retrospective interval of exactly equal duration, chosen **mechanically**
as `[original LOWER_BOUND - 5 days, original LOWER_BOUND)` -- no
population inspection informed this choice:

- `WINDOW_START = 2026-10-02T11:22:03Z`
- `WINDOW_END = 2026-10-07T11:22:03Z`
- Boundary convention: `WINDOW_START <= event_time < WINDOW_END` --
  the same half-open convention `fetch_merged_prs_in_window` already
  uses throughout this lineage (`lower <= merged_at < upper`); no
  existing GO-F text specified a different one, so this is adopted
  consistently rather than introduced fresh.

This window is now already fully matured as of this amendment's own
commit time (`2026-10-08T02:26:59Z` is well past `2026-10-07T11:22:03Z`).

## Everything else frozen in GO-F, unchanged

Candidate commit `9ff7c048c500fc5939b5cf1da358383759e96676`; candidate
binary `work/go-f/oba-candidate-9ff7c04`, sha256
`406a27f54378c3a5b5b14c08fb9bec4d12bb23313fea73233d7154bb01834988`;
the 1351-entry exclusion set (`work/go-f/exclusion-set.json`);
population definition (repo, merged/open status, relevance filter);
deterministic enumeration/sharding; sampling procedure
(`CENSUS_CAP=750`, `NC1_THRESHOLD=20`, `systematic_sample`); seed
(`int("0a6f192",16)`); `N=15`; `NC2_MIN_DERIVED_PRS=9`,
`NC3_MIN_FRACTION`/`NC4_MIN_FRACTION=0.7`; the GO-D/GO-B-corrected
target-construction contract; the blind-then-reveal adjudication
protocol and its 4 escalation triggers; `KNOWN-LIMITATION-1`
(rename/move -> `KNOWN_UNSUPPORTED_RENAME_MOVE`). None of these are
touched by this amendment, regardless of what the retrospective
window's own population turns out to contain.

## Integrity check (performed before this amendment was frozen)

| Question | Answer |
|---|---|
| Eligible PR enumeration performed? | **NO** |
| PR-count inspection for any candidate window? | **NO** |
| Manual browsing of candidate PRs? | **NO** |
| Target construction performed? | **NO** |
| `oba` scoring performed? | **NO** |

Verified freshly, immediately before writing this document: `git log`
shows no commit since `70957d3` (the F2-blocked status commit); `git
status` shows only the already-known untracked candidate binary
(unchanged sha256, re-verified above); no new file under `work/go-f/`
beyond what `70957d3` already committed. No contamination occurred.

## Repository/candidate state confirmation

- `HEAD` before this amendment: `70957d3ea5fc90ba341451101ab6c1501cfb4e6d`.
- Candidate binary sha256: unchanged (`406a27f5...3498`), re-verified
  by direct `sha256sum` immediately before writing this document --
  identical to the value frozen at Phase F0.
- Tree clean except the (intentionally uncommitted) candidate binary.

**`S6_GOF_A1_PREREGISTERED`**

Phase F2 (population enumeration under the retrospective window) may
now begin, using this amendment's own window in place of the original
prospective one, with every other frozen rule unchanged.
