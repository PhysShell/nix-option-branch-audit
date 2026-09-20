# S3-R: regression rerun of S3's own 48 PRs, after S3-F1/F1.1/F2/F3

Same 48 PRs as `results.md` (29 S3-A + 19 S3-B), re-run against a dev
build of `main` (commit `5e7fd71`, the final S3-F head — see
"S3-F1.1" below) containing S3-F1 (`specialisation.*` visibility),
S3-F1.1 (narrowing S3-F1 to the real top-level marker position only),
S3-F2 (flat dotted-key declaration resolution), and S3-F3
(`transition_origin` honesty for `OptionNotFound`), against the frozen,
unchanged `v0.4.2` binary as the baseline. **Explicitly a regression
corpus, not a fresh generalization claim** — the code has now seen
these exact 48 PRs, matching S1-R/S2-R's own precedent exactly.

## S3-F1.1: a real correctness gap in S3-F1, found on independent review, fixed before this document was finalized

S3-F1's original version scanned the WHOLE accumulated test-config
path for the literal `"specialisation"`/*/`"configuration"` token
sequence anywhere, not just at the real NixOS semantic position (a
top-level-only option, per `nixos/modules/system/boot/
specialisation.nix` — nixpkgs's own module documents nested
specialisations as ignored). A hostile but entirely legitimate
counterexample — `services.foo.specialisation.bar.configuration.enable`,
an ordinary submodule option that merely contains the same three
tokens in sequence, not at the real top-level position — was wrongly
collapsed into `services.foo.enable`, manufacturing false evidence for
a completely different, unrelated option. Confirmed as a real,
reproducible bug (a dedicated hostile regression test failed against
the pre-fix code, `commit 87028d8`), fixed by narrowing the check to
the root of the accumulated path only, stripped at most once, never
recursively (`commit 5e7fd71`). All 8 pre-existing S3-F1 tests, plus
the real `#510342`/`#511659` reproducers, independently re-verified
unaffected by the narrowing (both genuinely have their real
specialisation blocks at the top-level position). Full account in the
commit itself; every result in this document reflects the narrowed,
corrected fix, not the original overly-broad version.

## Method

4 parallel forks (12 PRs each), each independently re-fetching real
base/head content from `NixOS/nixpkgs` and rebuilding fixtures (not
reusing S3's own original scratch data, to avoid any doubt about which
binary produced which result), running both `/home/tandem/.cargo/bin/oba`
(frozen `v0.4.2`) and `/home/tandem/cargo-target/debug/oba` (the fixed
dev build) against each. Zero repository writes across all four groups
(`git status --short` confirmed clean throughout, per each group's own
report).

**Correction to the first S3-R draft**: it stated that 5 Group-2 PRs
received structural-only triage. Re-checking the original group
assignment shows the actual count was 6: `#554062`, `#555621`,
`#555643`, `#556461`, `#557729`, `#558600`. This was a report-counting
error, not a tool result — the same class of mistake as the "~8%"
arithmetic error and the `#551640`/`#562066` inaccuracies found
earlier in this same document's own history, all human transcription
errors in Markdown, never a scanner defect. All six have since
received full independent dual-binary reruns (see below), so the
final 48/48 parity claim is unaffected.

**A disclosed methodology gap, since closed.** Group 2 initially used a
lighter structural triage (scanning real `gh pr diff` output for new
`mkOption`/`mkEnableOption`/`specialisation` additions, rather than a
full dual-binary fixture rebuild) for 6 of its 12 PRs (`#554062`,
`#555621`, `#555643`, `#556461`, `#557729`, `#558600`), citing time
budget — a real gap against the mandate's own explicit requirement to
re-run the exact same 48 PRs, not a subset, since `#511659` (S3-B) had
already independently demonstrated that a PR with no original
actionable presentation can still be materially affected by S3-F1.
**Closed directly**, not left as an accepted shortcut: all 6 were
independently re-fetched and re-run against both binaries. Two
(`#555621`/kerberos, `#555643`/actual) have real modules confirmed
byte-identical at base/head via blob-SHA comparison (both are pure
test-migration PRs); all 6 reproduced byte-for-byte identical
`audit-diff` summaries between the frozen and dev binaries, real
evidence, not reasoning. **All 48 PRs now have a genuine, independent
dual-binary rerun — zero remaining reasoned-only entries.**

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
real PRs that found them, with zero regressions across **all 48 PRs,
every one independently re-run against both binaries** — including
deliberate negative controls (true module births, still-open unrelated
capability gaps) that confirm each fix's own boundaries held, and
including a real, independently-found correctness gap in S3-F1 itself
(S3-F1.1) caught and closed before this document was finalized, not
discovered after the fact by someone else. Per the same S1-R/S2-R
precedent, this confirms the fixes work on data the code has now seen
twice; it is **not** a fresh generalization claim — 100% actionable
precision on a corpus the code has now been debugged against twice is
an answer key, not evidence of generalization. That question is S4's
own job, on a genuinely new sample the code has never seen.

v0.4.3 (commit `43dd651`) was cut before S3-F1.1 was found and does
**not** contain that fix — a real, disclosed gap in the published
artifact. `v0.4.4` supersedes it, containing S3-F1.1, cut and
independently verified from a clean scratch install after this
document was finalized. S4 is not authorized by this document.
