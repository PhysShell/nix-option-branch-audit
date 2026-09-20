# S4: population, screen, and both frozen cohort orders

Drawn mechanically per `protocol.md`, frozen here **before any of the
PRs below is individually inspected**. Nothing past this point
(adjudication) may change cohort membership or order.

The full frozen orders are the committed JSON files
(`s4a-frozen-order.json`, 120 entries; `s4b-frozen-order.json`, 67
entries) — this document summarizes the funnel and draw method, and
gives illustrative excerpts; the JSON files are the actual source of
truth the adjudication ledger must be built against, in order.

## Population funnel

```
raw commits (real GitHub commits API, path-filtered on
  nixos/modules/services and nixos/tests, 2026-06-01..2026-09-20)
unique PR numbers extracted from real commit messages             685
  detail-fetch failed (PR #538312, dropped)                          1
successfully detail-fetched                                        554
  excluded: already-examined PR number (S1 30 + S2 50 + S3 50)       0
  eligible (real path check)                                      554
  excluded: merge-window sanity                                      1
  excluded: purely docs/formatting-only                               5
  excluded: mass mechanical change                                  41
  excluded: exclusion-ledger match (name/module-path/test-path)    272
survived                                                            235
```

The window is deliberately wide (4 months) — S4's own contamination
control is identity-based (the machine-generated exclusion ledger:
130 PR numbers, 390 subject names, 183 module paths, 87 test paths),
not a non-overlapping time window, matching S2's own established
precedent that a wide, overlapping window is fine as long as
identity-level exclusion is real. Confirmed: 0 of the 554 fetched PRs
fell into the already-examined-PR-number bucket, meaning none of
S1/S2/S3's own 130 drawn PRs happen to also appear in this window's
own raw commit set — the identity-based exclusion was never actually
needed to reach zero overlap here, but was checked regardless, exactly
as the protocol requires.

**Exclusion-ledger quality**: the ledger (`exclusion-ledger.json`,
commit `e9bf65f`) was iteratively corrected against this real
population before the draw — three real classes of over-exclusion bugs
were found and fixed by direct inspection of the actual excluded
candidates (see that commit's own message for the full account):
generic nested filenames being wrongly promoted to subject names
(`nixos/tests/kubernetes/base.nix` → wrongly `"base"`), the same bug
in `pkgs/by-name/**` paths, old hand-compiled lists' own generic-term
entries being blindly unioned in, and exporter-census names with
generic-sounding-but-real subjects (`domain`, `json`, `mail`, etc.)
matched too broadly. Population size moved 125 → 133 → 163 → 198 →
231 → 235 survivors across these fixes — each verified against the
specific real PR that exposed the prior version's bug, not just a
before/after count.

## S4-A: random cohort

`seed = int("81e1131", 16) = 136188209` (the frozen `v0.4.4` commit's
own short SHA, matching S1/S2/S3's own established seeding
convention), `random.Random(seed).shuffle(...)` over the full 235
survivors, first 120 (the protocol's own upper cap) frozen as the
complete order — real adjudication stops early per the protocol's own
presentation-count stopping rule (≥60 processed AND ≥10 actionable
presentations), never reaching the full 120 unless needed.

First 10 in frozen order: `#537519, #528177, #511721, #535931,
#533299, #545582, #527732, #534299, #535094, #475455`
Last 5: `#540535, #543492, #523948, #527254, #548234`

## S4-B: stress cohort

Remaining pool after S4-A's own 120 are removed: 115 PRs.

The exact same 6 pre-registered mechanical stress categories S2-B/S3-B
already used, reused verbatim, never retuned:

```
category hit counts among the 67 stress-eligible candidates
(a PR can match more than one):
  nixos_tests_touched:          44
  package_version_source_dep:   24
  option_declaration:           15
  environment:                  14
  execstart_script_cmdline:      6
  generated_config:              5
```

67 of 115 remaining candidates are stress-eligible. `seed = int("81e1131",
16) + 1 = 136188210` (distinct from S4-A's own seed),
`random.Random(seed).shuffle(...)` over the 67 stress-eligible
candidates, **all 67 frozen as the complete order** — the pool itself
is smaller than the protocol's own 80-PR upper cap, so the entire
stress-eligible pool is the frozen order; real adjudication still
stops early per the protocol's own stopping rule (≥40 processed AND
S4-A+S4-B combined ≥30 actionable, ≥10 from S4-A).

First 10 in frozen order: `#538359, #528019, #550306, #529428,
#540256, #534381, #524004, #535633, #516530, #555078`
Last 5: `#485003, #535928, #549901, #547674, #533294`

## Zero individual inspection beyond mechanical selection

No PR in either frozen order has been read past what the mechanical
screen/stress-filter itself required (file path lists, diff
added/removed lines for the regex categories, title text for the
docs/mechanical/name checks). No module or test file content has been
fetched for any drawn PR. Adjudication has not started.
