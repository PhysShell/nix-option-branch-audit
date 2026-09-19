# S2: prevalence (A) + stress (B) cohorts — results

50 PRs total (30 S2-A + 20 S2-B) from `sample.md`, shadow-audited
against the frozen `v0.4.1` binary (`4ba459b`), manually adjudicated.
Zero `src/` changes, zero new CDC registry entries, zero GitHub writes
anywhere in this round. Metrics computed and reported **separately per
cohort throughout — never blended into one headline number**, per the
protocol's own explicit requirement.

## Headline numbers

```
                          S2-A (prevalence, 30)   S2-B (stress, 20)
applicable (oba=yes)              22 (73%)              14 (70%)
TOOL_ERROR                          0                      0
new_finding                         0                      2
resolved_finding                    0                      0
finding_became_inconclusive         0                      0
new_inconclusive                    0                      3
resolved_inconclusive               1                      0
added_subject (plain, no finding)   1                      1
removed_subject                     1                      1
persistent findings (out of         5                      1
  headline metrics, as in S1)
notable entries (inspected)         2                      5
```

**`TOOL_ERROR` is 0/50 across both cohorts, on a completely fresh,
non-overlapping sample.** This is the clean, real confirmation S1-F1
needed: the module-birth/death fix isn't just patching the two PRs
that originally broke it, it holds on genuinely new data — including
two real module births in this round (`#532540`, `#552038`), one real
test-file birth against a pre-existing module (`#469112`, plus a
second variant in `#559239`), and two real module deaths
(`#563778` pghero, `#559393` go-neb).

## Correctness and actionable precision — the number S1 couldn't produce

```
S2-A: 2 notable entries inspected
  correct:      1 clearly yes, 1 "uncertain" (fail-closed, message
                 text technically misleading — see #559009 below)
  pr_relevant:  1 yes, 1 "partial"
  -> too small (n=2) for a meaningful percentage; reported as raw
     counts, not a decimal, per the protocol's own decision rule

S2-B: 5 notable entries inspected
  correct:      5 / 5  (100%)
  pr_relevant:  3 / 5  (60%)

  correctness precision (S2-B) = 5/5  = 100%
  actionable precision  (S2-B) = 3/5  =  60%
```

**Zero false findings across the entire round, both cohorts, all 7
notable entries** — the honest, single most important precision result:
`v0.4.1` never claimed something was wrong when it wasn't, on 50 real,
freshly-drawn PRs including 20 mechanically selected specifically to
raise the odds of a transition. But **actionable precision is
meaningfully lower than correctness precision (60% vs 100% in S2-B)** —
two of the five notable entries were technically correct but not about
what the PR itself changed. This is the real, disclosed gap S1's own
quiet sample couldn't have shown.

## The two PR-irrelevant findings, in detail — the real product gap

**`#547038`** (nginx: add missing MIME types to compression configs):
a real `new_finding` on `recommendedBrotliSettings`, correct (the
branch genuinely is untested) — but the PR itself never touches
brotli at all. The real mechanism: this PR's own diff includes a
brand-new test file (`nginx-compression.nix`, absent at `base_sha`),
so the whole nginx target becomes `Added` relative to that PR — and
`added_bucket` correctly reports every Finding/Inconclusive-class
option the head-side test doesn't cover, including ones that predate
the PR by years. **A real, disclosed modeling nuance, not a bug**:
`Added` correctly means "this subject just became analyzable," but a
maintainer reading "new finding" would reasonably assume "the PR
introduced this," not "the PR happened to be the first one with a
test file for this pre-existing option."

**`#554495`** (hyphanet: rename from freenet): a real `new_inconclusive`
on `nice`, correct (no real predicate exists for a bare
`Nice = cfg.nice;` passthrough) — but this PR's own real new logic
(`useNewNames = versionAtLeast stateVersion "26.11"`) is gated on
`config.system.stateVersion`, entirely outside any `services.hyphanet.*`
option OBA's `option_prefix`-scoped model can see. A real, structural
scope boundary (OBA watches named options, never arbitrary internal
branch logic), not a false result.

## Real, newly-discovered limitations (not previously documented anywhere in this project)

**The `exporters.nix` shared-framework `extraOpts` convention hides
declarations from every Prometheus exporter in nixpkgs.**
(`#552038`, snowflake exporter init) — `privateKeyFile` and
`environmentFile` are both real `mkOption` declarations, but nested
inside the shared `exporters.nix` framework's own `extraOpts = {...}`
merge pattern rather than a direct `options.services.X = {...}` block.
Both correctly `new_inconclusive` (fail-closed, not a false PASS), but
this is a structural gap affecting a whole FAMILY of modules — every
`services.prometheus.exporters.<name>` in nixpkgs uses this same
convention — the single most consequential gap this round found, by
breadth.

**nginx's own option declaration and its real predicate live in
different files.** (`#549553`, `useGrpcErrorPages`) —
`location-options.nix` declares the option; `nginx/default.nix`
contains the real `optionalString config.useGrpcErrorPages ...` gate
that uses it. `analyze()` reads one named `module` file per target; it
cannot span nginx's own real multi-file module split. Confirmed
directly (probing `default.nix` alone as the module still produces a
real `OPTION_NOT_FOUND` for an option that genuinely exists, just
elsewhere). A second real declaration-nesting variant, structurally
related to the `exporters.nix` gap above but distinct in shape.

**A likely-related nested-declaration gap**: `#559009`
(gitea-actions-runner test init) — `token` is declared inside a real
`instances.<name>` `attrsOf(submodule)` pattern; the scanner reports
`OptionNotFound` even though the declaration genuinely exists. Adjudicated
`correct=uncertain` rather than a clean yes/no — the verdict is
fail-closed and never wrong in the dangerous direction, but the
message text ("no declaration found") is technically imprecise for
what's actually a declaration-VISIBILITY gap, not a true absence.

**One real, unexplained anomaly, not root-caused**: `#558149`
(firewall-nftables race-condition test fix) — both `enable` and
`logRefusedPackets`, declared via what looks like the exact same
canonical flat-dotted `options = { networking.firewall = { ... }; }`
shape that resolves cleanly elsewhere in this same round (e.g.
`#558091`'s `resolved.enable`), both return `OptionNotFound`. Verified
twice with different watch lists by the investigating fork; no
`parse_errors` reported. Flagged honestly as unresolved rather than
guessed at.

**A second, independent real confirmation of the already-known
"`EvidenceChanged` never populated for OBA" limitation** (first found
in S1 on `#559055`): `#528118` (kmscon version bump) — the PR's own
diff genuinely flips `libseat`'s default from `mkOption{default=true}`
to `mkEnableOption` (default `false`), a real, PR-introduced
default-outcome change — invisible to `audit-diff` because both old
and new defaults land on the same `VerdictKind`
(`TestConfigUnresolved`). Two independent real PRs now, across two
separate rounds, hitting the identical class of gap.

**A fresh real-world recurrence of an already-documented gap**:
`#562104` (hickory-dns) hits the exact `with lib; { ... }`
wrapper-opacity limitation E2's own residual census already named
(previously found on jitsi-meet/i2pd) — not new, but confirms the gap
generalizes beyond the two candidates that originally found it.

## Two clean negative confirmations of S1-F2's own new bucket

`#563778` (pghero: drop, a real whole-module removal) and `#559393`
(go-neb: drop, module gutted to a `mkRemovedOptionModule` stub + real
test-file deletion) both correctly report `removed_subjects_with_finding=0`
— the removed subject's own base-side verdict was a clean `Pass` in
both cases, and the new bucket correctly stayed silent rather than
over-firing on every removal. **No PR in this 50-PR round positively
triggered `removed_subject_with_finding`** — the bucket exists,
is correctly negative-tested twice, but hasn't yet been positively
observed on real data (S1-R's own `#492803` triggered
`finding_became_inconclusive` instead, a same-subject shape, not this
one). Honestly reported as untested-positive, not assumed working.

## Decision rule, applied

```
0 ordinary PR shapes -> TOOL_ERROR              MET.  0/50, confirmed
                                                  on fresh, non-
                                                  overlapping data.
0 confirmed false findings                       MET.  7/7 notable
                                                  entries correct or
                                                  fail-closed-uncertain,
                                                  never a false claim.
no hidden false-PASS                             MET, as far as
                                                  reported -- no group
                                                  found an obvious
                                                  missed issue in an
                                                  applicable area.
notable output is PR-relevant, not mostly         PARTIALLY MET.
  dredging up old/persistent junk                 3/7 notable entries
                                                  (43%) were correct
                                                  but not about what
                                                  the PR itself
                                                  changed -- a real,
                                                  disclosed gap, not a
                                                  clean pass.
most notable entries verify in minutes            MET.  Every one
                                                  <2min or 2-10min,
                                                  none >10min.
runtime fits an advisory workflow                 MET.  0.005s-0.11s
                                                  per real run.
S2-A shows the tool can stay quiet                MET.  28/30 (93%)
                                                  completely silent.
S2-B gives enough events to measure precision     MET.  7 real notable
                                                  entries across 50
                                                  PRs (vs. S1's zero
                                                  across 30) -- a real,
                                                  if still small,
                                                  precision signal.
```

## Verdict

**Fix concrete issues found, then re-run — not ship beta, not
reconsider positioning.** The round answered its own central question
decisively: real applicability (~70-73% of an eligible population
across both cohorts), zero false findings, trivial runtime, and a tool
that stays quiet on ordinary PRs while genuinely finding real things
under a content-targeted filter. That combination is a strong, real
foundation — but the "notable output is PR-relevant" criterion is only
partially met (60% actionable precision in S2-B), and this round found
one real, structurally significant new gap (`exporters.nix`'s
`extraOpts` pattern, affecting every Prometheus exporter module in
nixpkgs) plus one unresolved anomaly (`#558149`) that should be
understood before any beta claim.

**Concrete, scoped follow-ups this round names** (none fixed here, per
S2's own freeze):

1. Investigate and fix `#558149`'s unexplained `OptionNotFound` on an
   apparently-standard declaration shape — possible real scanner bug.
2. Consider a product-level answer for the `Added`-via-new-test-file
   PR-relevance nuance (`#547038`) — a maintainer reading "new finding"
   on a test-birth PR may reasonably assume the PR introduced the gap,
   when it may only have introduced the FIRST test for a much older
   option.
3. Name the `exporters.nix extraOpts` gap (and its likely relative,
   nested `attrsOf(submodule)` declarations like `#559009`'s `token`)
   as the next real candidate for scanner work, given its breadth
   across nixpkgs — but not undertaken during this round, matching the
   user's own explicit "no new adapters/scanner work until after a
   positioning decision" instruction.

**A fresh S2-style re-run after these fixes**, not a third from-scratch
sample, would be the natural next check — the population/eligibility/
exclusion/cohort-split machinery built here is fully reusable.
