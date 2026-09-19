# S1-R: regression rerun of S1's own 30 PRs, after S1-F1/F2/F3

Same 30 PRs as `sample.md`, re-run against a dev build of `main`
(commit `f49a859`, containing the S1-F1/F2/F3 fixes), NOT the frozen
`v0.4.0` release binary S1 itself used. **This is explicitly a
regression corpus, not a beta-readiness claim** — the code has now
seen these exact 30 PRs, so a clean result here says "the fixes work
and nothing broke," never "the tool is validated." That question stays
open for S2, on a genuinely fresh sample.

## Headline: 30/30 match expectation

Every one of the 30 PRs produced exactly the result predicted before
the rerun. Zero surprises, zero new regressions, zero unintended side
effects on the 26 PRs the three fixes weren't specifically targeting.

## The three fixes, each confirmed directly on the real PRs that motivated them

**S1-F1 (TOOL_ERROR -> real comparison).** Both original PRs
(`#549506` kener, `#551955` yace-exporter) that hard-failed with
`TOOL_ERROR` now produce a real, honest `Added`/`new_inconclusive`
result (exit 2, not 3) instead. Beyond the two originals, **four more
PRs that S1 itself never even attempted** (`#519494` rosec, `#525702`
fleet-orbit, `#527821` iocaine, `#559588` netbird-relay — all skipped
in S1 once their base-side module was seen to be absent) were run for
real this time: all four produced a clean, real `Added` entry (three
resolved to a trivial `Pass`-class `enable` gate with no notable entry,
`#559588` correctly resolved a real `authSecretFile != null` alias and
witnessed it via the test's own real assignment). **The fix
generalizes past the exact two cases that found it.**

**S1-F2 (honest removal semantics).** `#492803` (ntfy) no longer shows
`resolved_findings=1`. The real transition is `oba001 ->
option_not_found`, and — a genuine, more precise finding than S1's own
original writeup implied — this is a **same-subject** transition
(`finding_became_inconclusive`), not a whole-target removal
(`removed_subject_with_finding`): `module.nix` itself still exists on
both sides; only the specific `user` option's own declaration vanished
within it via `mkRemovedOptionModule`. The classification is exactly
right for what actually happened — the earlier informal S1 writeup
speculated it *might* need the `removed_subject_with_finding` shape
instead; S1-R settles that it doesn't, for this specific real PR.

**S1-F3 (surface Inconclusive -> Pass).** Both real improvements S1
found being silently dropped (`#559627` btrfs autoScrub, `#561557`
modules/image enable-noop) now show `resolved_inconclusive` in
`.summary.notable`, confirmed in BOTH the raw JSON and the actual
rendered `summary.md` (a real `### RESOLVED INCONCLUSIVE` section
appears in each).

## Everything else: byte-for-byte unchanged

The remaining 24 PRs — persistent `OBA001`s (`#347823`, `#543675`,
`#553770`, `#555805`, `#556729`, `#557545`, `#564021`'s three),
persistent clean `Pass`es (`#556710`, `#561845`, `#564357`), persistent
`PredicateNotFound`/`TestConfigUnresolved` inconclusives (`#539076`,
`#553349`, `#554779`, `#558854`), and the genuinely-inapplicable
no-test-exists/no-module cases (`#550960`, `#558121`, `#560647`,
`#561669`, `#563958`) — all reproduced their exact original S1 shape,
with zero deltas in any summary field.

## One honest correction to S1's own original writeup

`#559055` (opentelemetry-collector): S1's own `results.md` described
this as "persistent OBA001 (`configFile`)". Re-derivation during S1-R
found this imprecise — the real option carrying the branch/default
logic is `validateConfigFile`, not `configFile` (whose own default is
a plain literal `null` on both sides, uninteresting), and
`validateConfigFile`'s real verdict class is `Inconclusive`
(`DefaultUnresolved`), not `OBA001`/Finding. The real, substantive
conclusion S1 drew — this PR's own genuine default-expression rewrite
(`isStorePath cfg.configFile` -> `cfg.configFile == null ||
isStorePath cfg.configFile`) stays invisible to `audit-diff` because
both old and new defaults are non-literal, the same `VerdictKind` on
both sides — is confirmed CORRECT and still unaffected by any of the
three fixes (a separate, still-open limitation:
`ChangeKind::EvidenceChanged` is still never populated for OBA). Only
the exact option name/verdict-class label in the earlier writeup was
imprecise; corrected here rather than left standing.

## Verdict

The three fixes work exactly as designed, generalize beyond the exact
PRs that motivated each one, and introduced zero regressions across
the full 30-PR corpus. **This is not a beta-readiness result** — per
the user's own explicit framing, a clean rerun on a corpus the code
has now literally seen proves the fixes, not the product. The next,
unauthorized step is a fresh, non-overlapping sample (S2) under a new
frozen release (`v0.4.1`), the only round that can actually answer the
still-open actionability question S1 itself could not.
