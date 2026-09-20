# S4: deployment-readiness evaluation — protocol

Pre-registered BEFORE any PR is drawn or individually inspected, per
this project's own standing discipline. Committed as its own commit,
separate from the exclusion ledger, the population/draw, the ledger
generator, and any result. The user's own explicit mandate for this
round is reproduced in full below the mechanics, since its clauses are
binding constraints on execution, not just framing.

## Frozen baseline

**`v0.4.4`, commit `81e1131c55be103431075ec3e82e58eb45fd92d5`** — the
real published artifact, downloaded into a clean scratch location and
checksum-verified against the release's own published checksum before
this document was written. **Not `v0.4.3`** — that release predates
S3-F1.1 (a real correctness gap found in S3-F1 after v0.4.3 was cut).

- No development build may be substituted for the frozen binary at any
  point during adjudication.
- No `src/` changes during S4.
- No fixes during S4.
- No capability expansion during S4.
- No `exporters.nix`, `with lib`, `mkPackageOption`, compound-predicate,
  multi-file, or other known-gap work during the measurement round.
- Any newly discovered problem is recorded and left unfixed until S4
  is closed.

## The question S4 answers

S4 is not another regression round — S3-R already showed the three
S3-F fixes hold on data the code has now been debugged against twice,
which is an answer key, not a generalization claim. S4 asks:

> Is frozen `v0.4.4` reliable enough, on genuinely unseen nixpkgs PRs,
> to move from internal shadow/dogfood use to a public, human-reviewed,
> maintainer-facing advisory stage?

It does **not** evaluate autonomous merge gating. It does **not**
authorize an unattended comment bot.

## Freshness / contamination control

A new population is built from fresh upstream data — S1/S2/S3's own
candidate pools, or their own unselected leftovers, are never reused.

Exclusion is mechanical and reproducible, generated from repository
evidence (`fixtures/s4-live-pr-shadow/build-exclusion-ledger.py` ->
`exclusion-ledger.json`, committed `79c811b`, before this document was
finalized), not constructed manually from memory:

- every PR number examined in S1, S2, or S3 (130, zero overlap)
- every normalized subject/app/service name examined in those rounds
  (367, each with its own provenance)
- every module path examined in those rounds (93)
- every test path examined in those rounds (87)
- the 93 exporter subjects already censused in S2-F3
- all synthetic/golden subjects already used by this repository

## Cohorts

Two cohorts stay distinct throughout — never blended into one headline
number, per the deployment gate below.

### S4-A — representative random cohort

A genuinely random eligible sample from the fresh population, real
commits touching `nixos/modules/services/**` or `nixos/tests/**` via
GitHub's own path-filtered commits API, the same real mechanical
screen (merge-window sanity, docs-only, mass-mechanical,
exclusion-ledger match) S1/S2/S3 already established.

The complete randomized ordering is frozen before any adjudication.

Process at least **60 PRs**. After 60, continue in the pre-frozen
order until either:
- S4-A has produced at least **10 actionable presentations**, or
- **120 PRs** have been adjudicated.

The stopping rule depends only on the number of actionable
presentations, never on whether those presentations are correct.

### S4-B — stress cohort

The same six pre-registered mechanical stress categories S2-B/S3-B
already used (option declaration; `nixos/tests/**` touched; ExecStart/
script/command line changed; environment/EnvironmentFile changed;
generated config changed; package version/source/dependency changed),
reused verbatim — never retuned after seeing S4-A.

The complete stress-eligible ordering is frozen before any
adjudication.

Process at least **40 PRs**. After 40, continue in the pre-frozen
order until either:
- S4-A + S4-B together have produced at least **30 actionable
  presentations**, with at least **10 coming from S4-A**, or
- S4-B reaches **80 PRs**.

Stopping depends only on presentation count, never correctness.

**If the caps are reached without enough actionable presentations,
the deployment-readiness result is `INSUFFICIENT EVIDENCE` — not pass
and not fail.**

## What counts as actionable

Exactly one pre-declared definition, fixed before any result is seen.

An actionable presentation is any maintainer-facing notable entry that
asks the reader to care about a new or changed analytical result,
including: new findings; newly surfaced inconclusives; resolved
inconclusives surfaced as notable; other bounded-summary entries
presented as PR-relevant changes.

A presentation is correct only if **all** of these are correct:
1. scanner/verdict substance;
2. PR relevance;
3. causal/origin framing;
4. rendered maintainer-facing presentation.

A wrong raw-JSON causal field is never silently forgiven merely
because one renderer happens not to display it (the exact
`#562066`-shaped gap this document's own project history already hit
once).

Ordinary unchanged/pass results do not count toward actionable
precision.

## False PASS: a separate hard-safety inventory

Actionable precision does not measure false PASS. For every applicable
PR, PASS verdicts are manually adjudicated sufficiently to determine
whether the claimed branch-activation evidence is genuine. A false
PASS is always reported individually, never hidden inside aggregate
precision.

## Adjudication pipeline

S4 must eliminate the report-transcription failure mode S3 exposed
repeatedly (the "~8%" arithmetic error, the `#551640` true-module-birth
mischaracterization, the `#562066` heading overstatement, the 5→6
triage-count error) — all real, all human, none a scanner defect, all
caught only by independently re-deriving ground truth rather than
trusting prose.

For every PR, machine-readable evidence is preserved: PR number, base
SHA, head SHA, module/test paths, target manifest, exact `oba` command,
raw JSON output, rendered summary, frozen binary version/hash,
adjudicated verdict, PR relevance, causal-framing judgment, false-PASS
status, adjudicator notes.

The adjudication ledger is stored as JSONL
(`fixtures/s4-live-pr-shadow/adjudication-ledger.jsonl`). **The final
Markdown tables and every metric numerator/denominator are generated
from that ledger by a checked script
(`fixtures/s4-live-pr-shadow/generate-report.py`), never manually
retyped.** The generator's own schema and formulas are frozen and
tested against synthetic dummy ledgers (see "Generator self-test"
below) before any real adjudication begins.

## Independent review requirement

Every actionable presentation is independently adjudicated by two
reviewers/forks that cannot see each other's adjudication before
submitting their own. Every PASS materially relevant to a PR
transition also receives independent verification. Disagreements are
recorded explicitly and resolved by the coordinator against raw
upstream content and raw `oba` output — an unresolved disagreement
never counts as correct.

Forks are strictly read-only. No worker may commit, push, alter cohort
membership, edit the protocol, edit the sample, edit another worker's
artifacts, or fix a discovered scanner problem — the coordinator alone
owns shared repository writes. (This constraint exists because a real
S3 fork violated exactly this boundary; it is repeated here
deliberately.)

## Deployment gate

S4-A and S4-B are reported separately, never replaced by one blended
headline. For the specific public-advisory decision, a pooled
statistical support calculation may additionally be reported, clearly
labelled as a gate statistic, never a population estimate.

Public human-reviewed maintainer-facing advisory is supported only if
**all** conditions hold:

1. Zero false PASSes.
2. Zero false findings.
3. Zero PR-relevance/causal-framing errors among actionable
   presentations.
4. At least 30 independently adjudicated actionable presentations
   total.
5. At least 10 actionable presentations come from S4-A.
6. S4-A actionable precision = 100% on its observed denominator.
7. S4-B actionable precision = 100% on its observed denominator.
8. The one-sided 95% exact binomial lower confidence bound for the
   combined actionable presentations is at least 90%.
9. Zero unresolved adjudication disagreements.
10. Zero `TOOL_ERROR` cases attributable to ordinary supported inputs.

(30/30 correct gives a one-sided 95% exact Clopper-Pearson lower bound
of ≈90.5% — condition 8 has an explicit statistical meaning fixed
before any result is seen, not a threshold chosen after the fact.)

Any false finding, false PASS, or framing/relevance error fails the
public-advisory gate for this round. The frozen evaluation continues
to its pre-declared stopping point even after a failure, so the
failure's prevalence and surrounding evidence are still measured. The
gate itself is never changed after seeing results.

## Additional metrics — diagnostic, not gates

Reported separately, never converted into post-hoc acceptance
criteria: applicability rate; actionable presentation rate;
`INCONCLUSIVE` rate; `INCONCLUSIVE` root causes; `origin_unclear`
frequency; manual adjudication effort; distribution of findings per
PR; structural non-applicability; known-gap hits versus genuinely new
gap classes.

## Generator self-test, before any real adjudication

`generate-report.py`'s own schema and gate arithmetic is exercised
against synthetic dummy ledgers before it ever touches real
adjudication data, confirming at minimum:

- 30/30 correct actionable presentations (≥10 from S4-A, ≥30 total,
  zero false PASS/finding/framing errors) → gate `PASS`.
- 29/30 correct (one framing error) → gate `FAIL`.
- one `false_pass: true` entry anywhere → gate `FAIL`, regardless of
  how high precision is otherwise.
- 9 actionable presentations from S4-A alone (below the 10-minimum),
  even if 9/9 correct → `INSUFFICIENT EVIDENCE`, never `PASS`.

These are committed as their own fixtures
(`fixtures/s4-live-pr-shadow/generator-selftest/`) with `#[test]`-style
assertions the generator script itself runs and reports on, so the
gate's own arithmetic is verified mechanically, not eyeballed — the
same discipline this project already applies to `oba` itself.

## Final S4 report — the user's own 18 items, fixed before any result is seen

1. Frozen binary identity and checksum.
2. Population-generation procedure.
3. Generated exclusion-set provenance.
4. Frozen S4-A and S4-B orders.
5. Exact stopping points and why they were reached.
6. Raw per-PR adjudication ledger.
7. S4-A actionable precision.
8. S4-B actionable precision.
9. Pooled gate-only confidence calculation.
10. False-PASS inventory.
11. False-finding inventory.
12. PR-relevance / causal-framing inventory.
13. `TOOL_ERROR` inventory.
14. `INCONCLUSIVE` root-cause breakdown.
15. Reviewer disagreement log.
16. Manual effort distribution.
17. Newly discovered gaps, not implemented.
18. Explicit deployment-gate result: `PASS` / `FAIL` /
    `INSUFFICIENT EVIDENCE`.

**If `PASS`**: stop and report that the evidence supports a separately
authorized public human-reviewed advisory pilot. Do **not** install a
bot, post on nixpkgs PRs, contact maintainers, or start a public pilot
automatically.

**If `FAIL`**: stop and identify concrete blockers. Do not fix them
inside S4.

**If `INSUFFICIENT EVIDENCE`**: stop. Do not lower the threshold or
opportunistically add hand-picked PRs.

No S5 and no external deployment without separate authorization.

## What does NOT happen during S4

- No `src/` changes of any kind.
- No fixing anything discovered mid-round.
- No re-tuning S4-B's own criteria after seeing S4-A.
- No GitHub Action, bot, or comment on any real external PR or
  repository.
- No blending S4-A's and S4-B's own metrics into one headline number.
- No individual inspection of drawn PR content beyond what is
  mechanically required for selection/filtering, before the frozen
  cohorts are committed.
- No changing the acceptance criterion after seeing the results.
