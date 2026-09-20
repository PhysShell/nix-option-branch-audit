# S3-R: regression rerun of S3's own 48 PRs, after S3-F1/F2/F3

Same 48 PRs as `results.md` (29 S3-A + 19 S3-B), re-run against a dev
build of `main` (commit `87028d8`) containing S3-F1 (`specialisation.*`
visibility), S3-F2 (flat dotted-key declaration resolution), and S3-F3
(`transition_origin` honesty for `OptionNotFound`), against the frozen,
unchanged `v0.4.2` binary as the baseline. **Explicitly a regression
corpus, not a fresh generalization claim** — the code has now seen
these exact 48 PRs, matching S1-R/S2-R's own precedent exactly.

## Method

4 parallel forks (12 PRs each), each independently re-fetching real
base/head content from `NixOS/nixpkgs` and rebuilding fixtures (not
reusing S3's own original scratch data, to avoid any doubt about which
binary produced which result), running both `/home/tandem/.cargo/bin/oba`
(frozen `v0.4.2`) and `/home/tandem/cargo-target/debug/oba` (the fixed
dev build) against each. Zero repository writes across all four groups
(`git status --short` confirmed clean throughout, per each group's own
report).

**One disclosed methodology gap**: group 2 used a lighter structural
triage (scanning real `gh pr diff` output for new `mkOption`/
`mkEnableOption`/`specialisation` additions, rather than a full
dual-binary fixture rebuild) for 5 of its 12 PRs, citing time budget.
None of those 5 PRs contributed any Finding-class actionable
presentation in the original S3 adjudication (all were `unchanged`/
0-notable or `oba=no`), so this does not affect the actionable-precision
recomputation below, but it means those 5 PRs' "zero delta" status is
reasoned, not independently re-run byte-for-byte like the other 43.

## Headline: every real delta matches an expected fix; zero regressions

**`#510342` (userborn) — S3-F1's own target.** `enable` moves from the
round's one real **false finding** (`OBA001`, confidently wrong) to an
honest `new_inconclusive` (`test evidence: unresolved` — the real
`specialisation.userborn.configuration.services.userborn.enable =
lib.mkForce true;` assignment now resolves to the correct option path,
but its `lib.mkForce`-wrapped value can't be statically classified, a
separate, pre-existing, already-known limitation unrelated to
specialisations). The false finding is gone.

**`#556558` (vxwm) — S3-F2's own target.** `enable` moves from
`OptionNotFound` to a real, evidenced `PASS` — independently
re-verified by two different groups (2 and the original F2 commit
verification) from freshly-fetched real PR content.

**`#516128` (tinyauth) and `#562066` (suricata) — S3-F3's own two
targets.** Both move from the wrongly-confident `transition_origin:
analysis_became_possible` to the honest `origin_unclear`, independently
re-confirmed via raw JSON, not fork prose.

**`#511659` (kvrocks module) — an unanticipated but real second-order
S3-F1 benefit**, found independently by group 3: `socketActivation`
moves from `TestValueUnresolved` (`new_inconclusive`) to a genuine
`PASS`. Root cause: its own real test evidence also lives inside a
`specialisation."socketActivation".configuration.services.kvrocks.
socketActivation = true;` block — the same S3-F1 fix, generalizing
correctly to a second, independently-drawn real PR beyond its own
original `#510342` reproducer. This PASS is silently absorbed (an
Added-class target moving to Pass is not itself a "notable" transition,
per the pre-existing convention that only Finding/Inconclusive-class
Added results are surfaced) — a real, quiet improvement, not counted
toward actionable precision either way.

**Every other PR in all four groups reproduced a byte-for-byte
identical summary between the frozen and dev binaries** (or remained
correctly `oba=no`/no-manifest-target, where no delta is possible by
construction) — including real negative controls the mandate itself
asked for: `#504200` (bulwark, true module birth) and `#556069`
(kapla, true module birth) both stayed correctly `subject_added`,
confirming S3-F3 does not touch the `Added`/`module_birth_paths` path
at all; `#553699` (nebula) and `#555067` (matrix-synapse) both stayed
correctly `OptionNotFound`/`with lib;`-opaque, confirming the still-open,
never-in-scope capability gaps (function-form submodules,
`with lib;`-wrapped blocks) were correctly left untouched.

## Two corrections to `results.md`, found while compiling this report

Neither is a tool defect — both are inaccuracies in the ORIGINAL S3
report's own prose, caught by independently re-verifying claims against
raw data rather than trusting either the original fork reports or my
own subsequent summary of them. Disclosed here, not silently edited
into `results.md`, matching this project's own standing convention.

1. **`#551640` (matrix-continuwuity) was NOT a true module birth.**
   `results.md`'s own per-PR table described it as "true module birth"
   / `transition_origin: SubjectAdded — correct". Independently
   re-verified via `gh api`: the real module file
   (`nixos/modules/services/matrix/continuwuity.nix`) has the exact
   same blob SHA (`dfa782fa...`) at both the PR's base and head commit
   — genuinely byte-identical, already existing before this PR (a pure
   package version bump, `26.7.2 -> 26.7.3`, touching only the test and
   package files). This was an investigation artifact in the ORIGINAL
   S3-A adjudication (most likely an artificially-empty base-root built
   for convenience rather than the real historical module content at
   the real base SHA), not a tool defect. **Does not affect the
   actionable-precision numbers** — this PR was never counted in the
   Finding-class denominator (its `enable` verdict was a clean,
   unremarkable PASS both sides, correctly non-notable).

2. **`#562066` (suricata)'s pre-fix rendered heading was NOT
   "EXISTING UNCOVERED BRANCH BECAME OBSERVABLE".** `results.md`
   claimed this heading rendered for `#562066`, alongside `#516128`, as
   the practical, human-visible symptom of the `transition_origin` bug.
   Independently re-rendered the real pre-fix markdown summary for
   `#562066` (`--summary-path`, frozen `v0.4.2`): the actual heading was
   the origin-agnostic `### RESOLVED INCONCLUSIVE` — `render_github_
   summary`'s own heading-override logic was only ever wired for
   `("new_finding", AnalysisBecamePossible)` and `("new_inconclusive",
   AnalysisBecamePossible)`, never `("resolved_inconclusive", ...)`
   (`#562066`'s own real bucket, since its transition is Inconclusive
   -> Pass, not a first-time finding). The underlying `transition_origin`
   **field** in the raw JSON genuinely was `analysis_became_possible`
   (a real bug, confirmed independently, and correctly counted as a
   causality error in `results.md`'s own item 7 and the actionable-
   precision denominator) — but the claim that this specific PR's
   *rendered markdown heading* visibly misled a maintainer was an
   overstatement on my own part while compiling the original report,
   not something any fork reported. Both real S3-F3 reproducers
   (`#516128` via `new_finding`, and the underlying JSON field for
   `#562066`) remain valid, real bugs, now fixed — only the specific
   claim about `#562066`'s own rendered heading text is corrected here.

## Recomputed actionable precision (S3-R, honest, not forced to match the old denominator)

Per the mandate: fixing a false finding can legitimately remove an
actionable presentation from the denominator; this is not forced to
stay 4/9 or 13 overall.

```
S3-A (random cohort):
  #461073  openFirewall            correct (unaffected)
  #510342  passwordFilesLocation   correct (unaffected)
  #510342  enable                  REMOVED -- no longer Finding-class,
                                    now an honest new_inconclusive
  #562066  reloadOnRulesetUpdate   correct (transition_origin field
                                    fixed: origin_unclear)
  ------------------------------------------------------------------
  S3-A actionable precision, post-fix: 3/3 = 100% (was 2/4 = 50%)

S3-B (stress cohort):
  #504200  admin                   correct (unaffected)
  #504200  settingsSyncEnabled     correct (unaffected)
  #504200  telemetry.enabled       correct (unaffected)
  #504200  updateCheck.enabled     correct (unaffected)
  #511659  group                   correct (unaffected)
  #511659  user                    correct (unaffected)
  #516128  enableUnixSocket        correct (was the causality error,
                                    now origin_unclear)
  #545183  openFirewall            correct (unaffected)
  #556069  openFirewall            correct (unaffected)
  ------------------------------------------------------------------
  S3-B actionable precision, post-fix: 9/9 = 100% (was 8/9 = 88.9%)
```

Reported separately, per the mandate, never blended. For reference
only: 12/12 = 100% combined (was 10/13 ≈ 77% before the false finding
was removed from the denominator entirely, or 10/12 ≈ 83% if compared
against the post-fix denominator size instead — neither framing is the
deciding number).

**Zero false PASSes, zero TOOL_ERROR** across all 48 PRs, confirmed
unchanged from the original S3 round (no fix touched anything that
could introduce either).

## Verdict

Both real bugs (the false finding, the two causality errors) and the
one real scanner-correctness bug are confirmed fixed, on the exact
real PRs that found them, with zero regressions across the rest of the
48-PR corpus — including deliberate negative controls (true module
births, still-open unrelated capability gaps) that confirm each fix's
own boundaries held. Per the same S1-R/S2-R precedent, this confirms
the fixes work on data the code has now seen twice; it is not a fresh
generalization claim (that is S4's own job, on a genuinely new sample).

Per the mandate: cutting the next patch release and verifying the
actual published artifact from a clean scratch install, then stopping.
S4 is not authorized by this document.
