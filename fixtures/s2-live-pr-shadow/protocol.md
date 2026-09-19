# S2: live nixpkgs PR shadow evaluation — prevalence + stress cohorts, protocol

Pre-registered BEFORE any of the two cohorts is drawn or any PR is
individually inspected, per this project's own standing discipline.
Committed as its own commit, separate from the population/draw and
separate from any results.

## What this round is, and is not

S1 answered "how often is v0.4.0 applicable, and how much manual work
does a result take" on an honest but unlucky sample: zero notable
findings across all 30 PRs meant the actionability question stayed
completely open. S1-F1/F2/F3 then fixed the one real, disqualifying
bug S1 found (`TOOL_ERROR` on module-birth PRs) and refined the
transition taxonomy, confirmed via S1-R's own 30/30 regression match
on the SAME PRs — which is explicitly NOT a validation sample, since
the code has now seen those exact 30.

**S2 is the first round that can actually measure precision**, by
splitting into two cohorts whose results are NEVER blended into one
headline number:

> **S2-A (prevalence cohort)**: ~30 fresh, mechanically drawn, ordinary
> eligible PRs — answers "how often does `v0.4.1` say anything at all
> on the real nixpkgs PR stream."
>
> **S2-B (stress cohort)**: ~15–20 fresh PRs, mechanically selected
> (not eyeballed) for content likely to trigger a real transition —
> answers "when there IS a real chance of something to find, how
> accurate is it."

Keeping them separate is the whole point: a high finding rate on a
cherry-picked-by-content cohort proves nothing about ordinary PRs, and
a quiet ordinary cohort proves nothing about precision when something
does change. Blending them into one number would let either question
quietly answer the other.

## The freeze

- **Frozen at `v0.4.1`, commit `4ba459b`** — real-verified from a clean
  scratch-directory install of the actual published release artifact
  (not the local dev build), confirming both `--summary-path` and the
  real module-birth fix (`added=1, new_findings=1`, not `TOOL_ERROR`)
  work against the binary a real user would actually download.
- **Zero `src/` changes for the entire round.** This explicitly
  includes the CDC registry: anything `v0.4.1` doesn't understand
  becomes a real, honest `INCONCLUSIVE` with a disclosed reason — never
  a mid-round patch. Per the user's own words: "не повод в середине
  выборки выпустить v0.4.1.7-super-final."
- **Shadow means shadow.** No GitHub Action installed on, and no
  comment/review/reaction posted to, any real external PR or
  repository. A local worker only: fetch PR base/head → `oba
  audit-diff` → store the report → manual adjudication, all inside this
  repository's own `fixtures/`.

## Population (shared source for both cohorts)

Reuses S1's own already-vetted, already-mechanically-screened
candidate pool rather than re-deriving eligibility from scratch: S1's
own funnel (`fixtures/s1-live-pr-shadow/sample.md`) produced 162 real,
eligible, non-excluded survivors from 196 raw candidates; only 30 were
drawn. **The other 132 are equally valid and were never even looked
at** — reusing them is honest, not a shortcut, since the SAME real
mechanical screen (path-eligibility, docs/mechanical/already-used-name
exclusion, real merge-window sanity) already vetted every one of them
before S1's own draw ever happened.

Extended with a fresh top-up fetch (the same path-filtered
`GET /repos/NixOS/nixpkgs/commits?path=...` mechanism, no local clone,
covering the real commit window since S1's own population fetch) to
avoid an artificially stale pool and to grow it past what 132 alone
would support once the stress filter (below) narrows S2-B's own
candidates.

**Explicit non-overlap with S1, defensively, not just assumed from
"the leftover 132 were never drawn"**: the exact 30 S1 PR numbers are
excluded by number from the combined S2 population before either
cohort is drawn.

## S2-A: prevalence cohort

From the full non-overlapping population: a deterministic seeded
shuffle, `random.Random(seed).shuffle(...)`, `seed =
int("<v0.4.1's own short SHA>", 16)` — the same mechanism S1's own
draw used, with `v0.4.1`'s own freeze SHA in place of `v0.4.0`'s. First
**30** of the shuffled pool. Frozen before inspection.

## S2-B: stress cohort

A **mechanical content filter**, pre-registered here before any PR's
diff is read for classification purposes, applied to the remaining
population (S2-A's own 30 removed first, so the two cohorts are
disjoint by construction — no PR contributes to both cohorts' own
separate metrics):

A candidate is **stress-eligible** iff its real unified diff (`gh pr
diff <N>`, added/removed lines only, not unchanged context) matches at
least one of:

1. **Option declaration** — a real `mkOption`/`mkEnableOption` call is
   added, removed, or changed.
2. **`nixos/tests/**` substantively touched** — at least one changed
   path is under `nixos/tests/` (already common but NOT universal in
   this population — several real S1 candidates touched only the
   module, never the test).
3. **`ExecStart`/script/command line** — a diff line containing
   `ExecStart`, `script =`, or an invoked command/argv changes.
4. **environment/`EnvironmentFile`** — a diff line containing
   `environment`, `EnvironmentFile`, or `environmentFile` changes.
5. **Generated config** — a diff line referencing `writeText`,
   `toYAML`, `toJSON`, `settingsFormat`, `format.generate`, or
   `configFile` changes.
6. **Package version/source/dependency** — a diff line matching
   `version =`, `src = fetch`, `rev =`, or a hash literal changes.

**Disclosed up front, not discovered after the fact and quietly
tightened**: category 6 is expected to match a large fraction of this
population on its own (most real nixpkgs service PRs bump a version
somewhere in the same diff) — the six categories are used exactly as
given, OR-ed together, with no post-hoc narrowing. If this makes the
"stress" filter close to non-selective in practice, that is itself
reported honestly in the results, not corrected by tightening the
criteria after seeing who qualifies.

From the stress-eligible sub-pool: the same deterministic seeded draw
mechanism (a fresh, distinct seed derived the same way, to avoid
correlating with S2-A's own draw order) selects the first **15–20**
(bounded by how many stress-eligible candidates actually exist after
the filter — the real count, never padded).

## Recording schema (per PR, either cohort — same schema, tagged by cohort)

```
cohort: A | B
pr: <number>
title: <string>
base_sha / head_sha: <shas>
applicable_engines: oba=yes|no (<reason>), cdc=no (structural, as in S1)
runtime_seconds: <real wall-clock>
exit_code: <real CLI exit code>
result:
  tool_error: yes|no
  new_finding / resolved_finding / finding_became_inconclusive /
    new_inconclusive / resolved_inconclusive: <counts>
  added_subject / removed_subject: <counts>
  persistent_findings: <count, out of headline metrics per S1's own precedent>
for each notable entry:
  correct: yes|no
  pr_relevant: yes|no
  already_noticed_by_reviewer_or_author: yes|no
  verification_time: "<2min" | "2-10min" | ">10min"
  notes: <real reasoning>
```

## Metrics — frozen before any result is seen, computed PER COHORT, never blended

```
per cohort:
  total, applicable, tool_error,
  new_finding, resolved_finding, finding_became_inconclusive,
  new_inconclusive, resolved_inconclusive,
  added_subject, removed_subject

correctness precision = correct notable entries / inspected notable entries
actionable precision  = (correct AND pr_relevant) notable entries / inspected notable entries
```

Computed separately for S2-A and S2-B — a combined "overall precision"
number is explicitly never reported, since it would let a noisy stress
cohort inflate (or a quiet prevalence cohort dilute) the other's own
real signal.

## Decision rule — pre-registered, qualitative, not a decimal

Per the user's own explicit instruction: no `precision >= 96.7%`-style
number this early — a small sample dressed up with a decimal point
usually decorates an absence of data, not a real threshold.

```
0 ordinary PR shapes producing TOOL_ERROR           -- required
0 confirmed false findings                          -- required
no hidden false-PASS (manual review of the           -- required, STOP
  applicable area finds an obvious missed issue)        and investigate
  if this happens
notable output is actually PR-relevant, not mostly    -- required
  dredging up old/persistent junk
most notable entries verify in minutes, not           -- required
  archaeology
runtime fits an advisory PR workflow                  -- required
S2-A shows the tool can stay quiet on ordinary PRs     -- required
S2-B gives ENOUGH real events to actually measure      -- required for a
  precision                                               beta decision
```

**If S2-B again gives close to zero transitions**, that is ALSO a
real, complete, useful result — not a failed round. It would suggest
PR-local OBA/CDC-relevant changes are genuinely rare even under a
content-targeted filter, and the honest product conclusion becomes
"position this as a targeted checker for specific classes of change,
not a universal per-PR reviewer" — a real positioning answer, not a
null result.

Three possible outcomes, named explicitly, none of them a failure to
reach: **ship advisory beta**, **fix concrete issues found and
re-run**, or **reconsider positioning** (if the stress cohort shows the
signal is real but rare, or the applicable population itself turns out
too narrow to be a universal reviewer).

## What does NOT happen during S2

- No new CDC registry candidates, regardless of what a real PR's own
  shadow run surfaces — a genuine `INCONCLUSIVE: reason = X` is the
  correct, honest output for anything `v0.4.1` doesn't understand.
- No GitHub Action, bot, or comment on any real external PR or
  repository.
- No `src/` changes of any kind against the frozen `v0.4.1`/`4ba459b`.
- No tightening S2-B's own content-filter criteria after seeing which
  PRs qualify.
- No blending S2-A's and S2-B's own metrics into one headline number.
