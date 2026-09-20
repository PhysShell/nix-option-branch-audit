# S3: fresh random + stress cohorts on genuinely unseen data — protocol

Pre-registered BEFORE any of the two cohorts is drawn or any PR is
individually inspected, per this project's own standing discipline.
Committed as its own commit, separate from the population/draw and
separate from any results. The user's own explicit mandate for this
round is reproduced in full below the mechanics, since several of its
clauses are binding constraints on execution, not just framing.

## What this round is, and is not

S1/S1-F/S1-R and S2/S2-F/S2-R together answered "did we break
anything" and "do the fixes work on the data that found them" — real,
necessary questions, but ones where the same ~80 PRs (S1's 30 + S2's
50) have now been read, debugged against, and regression-tested
multiple times. **That corpus is contaminated for generalization
purposes.** It can prove "we didn't break it"; it cannot prove "this
is useful on data the tool, and the people building it, have never
seen." S3 is the first round built specifically to answer that
question, with **actionable precision** — not correctness, which
already looks strong and unchanged — as the deciding metric.

## The freeze

- **Frozen at the exact published `v0.4.2` release** — real-verified
  from a clean scratch-dir install of the actual published artifact
  (confirmed both the `#558149`-shaped `//`-merge fix and
  `transition_origin` work against the real downloaded binary, not
  just the local build, before this round started).
- **Zero `src/` changes for the entire round.** This explicitly
  includes scanner semantics, classification rules, `transition_origin`,
  and `exporters.nix` support — S2-F3's own 93-module census
  established that the `exporters.nix` gap is real and large, NOT that
  fixing it is authorized inside a measurement round. Newly discovered
  gaps are recorded, never fixed, during S3; classifying proposed
  fixes into a later round happens only after the frozen evaluation is
  complete.
- **Shadow means shadow.** No GitHub Action installed on, and no
  comment/review/reaction posted to, any real external PR or
  repository. A local worker only.

## Population: genuinely fresh, not S1/S2's own leftover pool

Unlike S2 (which legitimately reused S1's own never-drawn survivors),
S3 draws an entirely NEW population from a fresh commits-API fetch —
not S1's or S2's own leftover candidate pool, even the parts never
individually inspected. This is a deliberately stricter standard than
S1→S2's own reuse, matching the user's own explicit concern that
"contamination" means more than "was this exact PR looked at" — it
means the whole population-generation exercise should not lean on
machinery already exercised against the same historical window.

Mechanism (the same real, disclosed one S1/S2 already established):
real commits touching `nixos/modules/services/**` or `nixos/tests/**`,
pulled via GitHub's own path-filtered commits API (no local nixpkgs
clone), most recent window as of this round's own start
(2026-09-20).

**Exclusion, three layers**:

1. The same real screen S1/S2 already used: real merge/state-window
   sanity, purely docs/formatting-only, mass-mechanical changes.
2. **Every PR number already drawn in S1 (30) or S2 (50)** — 80 exact
   numbers, excluded explicitly, not just assumed disjoint from a
   different time window.
3. **A merged, real app/service-name exclusion list, 253 names** —
   S1's own original 113-name list, PLUS every real subject examined
   across S2's own drawn 50 PRs, PLUS all 93 real Prometheus exporter
   module names S2-F3's own census fetched and read directly
   (`fixtures/s3-live-pr-shadow/exclusion-name-list-253.txt`, the
   real merged, deduplicated list).

## S3-A: random cohort (~30)

The same deterministic seeded draw mechanism as every prior round:
`random.Random(seed).shuffle(...)`, `seed = int("<v0.4.2's own short
SHA>", 16)`. First 30 of the shuffled, screened pool. Frozen before
inspection.

## S3-B: stress cohort (~20) — criteria fixed BEFORE the draw, never revisited after seeing S3-A

Per the user's own explicit, emphatic instruction: **the stress
cohort is not a second chance after seeing the random cohort's own
results.** Its selection criteria are exactly the same six,
pre-registered, mechanical, content-based categories S2-B already
used — reused verbatim, not re-tuned, specifically so no one can later
say the criteria were adjusted once it was clear where the tool
struggles:

1. Option declaration added/removed/changed (`mkOption`/
   `mkEnableOption` in the real diff).
2. `nixos/tests/**` substantively touched.
3. `ExecStart`/script/command line changed.
4. `environment`/`EnvironmentFile` changed.
5. Generated config (`writeText`/`toYAML`/`toJSON`/`settingsFormat`/
   `format.generate`/`configFile`) changed.
6. Package version/source/dependency changed.

Applied mechanically (real `gh pr diff`, added/removed lines only, OR
across all six, no post-hoc narrowing) to the remaining pool after
S3-A's own 30 are removed. A distinct seed draws the first ~20 of the
stress-eligible sub-pool.

## Recording schema — the user's own 11-point adjudication list, per PR

```
cohort: A | B
pr: <number>, title, base_sha, head_sha
applicable_engines: oba=yes|no (reason), cdc=no (structural)
runtime_seconds, exit_code
scanner_verdict_correctness: for each watched option, is the real
  verdict (Pass/Finding/Inconclusive/TOOL_ERROR) actually right given
  the real module+test content?
pr_relevance_and_transition_origin_correctness: for each notable
  entry, is transition_origin (SubjectAdded/AnalysisBecamePossible/
  VerdictChanged) the honest, correct explanation for why this looks
  new? Does the finding relate to what the PR itself changed?
actionable_vs_non_actionable: would a real maintainer reading this
  entry find it worth acting on, for THIS PR specifically?
false_findings: any Finding-class verdict that is, on manual
  inspection, actually wrong?
false_passes: any Pass-class verdict (or Unchanged/no-finding
  presentation) that manual inspection shows should have been a real
  Finding? -- surfaced individually, never folded into an aggregate.
honest_inconclusive_or_tool_error: cases correctly refusing to guess,
  with the real root cause named.
unsupported_syntax_or_architecture: any real shape the scanner
  genuinely cannot parse/model, named precisely.
manual_verification_effort: "<2min" | "2-10min" | ">10min" -- real
  time to reach a confident ground-truth judgment.
```

## The deciding metric

```
actionable precision = (scanner-correct AND PR-relevant AND
                         correctly-framed) notable entries
                        / all notable entries presented as actionable
```

Computed and reported SEPARATELY for S3-A and S3-B — never combined
into one flattering percentage. A false PASS is reported as its own
named inventory item, never hidden inside an aggregate precision
number. A technically-correct finding whose framing (`transition_
origin`) incorrectly implies the PR introduced the problem is recorded
as its own causality/framing failure, distinct from a false finding.

## Known limitations, explicitly not defects S3 must chase

Per the user's own explicit instruction: `PredicateNotFound` on a
`logRefusedPackets`-shaped `optionalString`-based predicate use, and
`exporters.nix`'s own real, census-confirmed multi-file gap, are
**known limitations of `v0.4.2`**, not new defects this round is
obligated to fix. If S3 hits either shape again on fresh data, it is
recorded as a real, disclosed `INCONCLUSIVE`-root-cause data point —
exactly what this round exists to measure the real frequency and cost
of, not something to patch mid-round.

## Report structure — the user's own exact 11 items, fixed before any result is seen

1. The frozen cohort and selection method.
2. Raw per-PR adjudication.
3. Random-cohort (S3-A) actionable precision.
4. Stress-cohort (S3-B) actionable precision.
5. False-PASS inventory.
6. False-finding inventory.
7. `transition_origin`/causality errors.
8. `INCONCLUSIVE`/`TOOL_ERROR` breakdown by root cause.
9. Manual adjudication cost.
10. Newly discovered capability gaps, named, NOT implemented.
11. A recommendation for the next adoption stage, based on the
    measured evidence.

**The acceptance criterion is not changed after seeing the results.**

## What does NOT happen during S3

- No `src/` changes of any kind, including scanner semantics,
  `transition_origin`, or `exporters.nix` support — "the 93-module
  census establishes that the limitation is important; it does not
  authorize evaluator-like functionality inside this measurement
  round" (the user's own words).
- No fixing anything discovered mid-round — record, finish the frozen
  evaluation, THEN classify proposed fixes into a later round.
- No re-tuning S3-B's own criteria after seeing S3-A's results.
- No GitHub Action, bot, or comment on any real external PR or
  repository.
- No blending S3-A's and S3-B's own metrics into one headline number.
