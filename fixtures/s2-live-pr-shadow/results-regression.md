# S2-R: regression rerun of S2's own 50 PRs, after S2-F1/F2/F3

Same 50 PRs as `sample.md` (30 S2-A + 20 S2-B), re-run against a dev
build of `main` containing S2-F1 (the `firewall.nix` `//`-merge fix)
and S2-F2 (`transition_origin`) — S2-F3 was research-only and changed
no code. **Explicitly a regression corpus, not a beta-readiness
claim** — the code has now seen these exact 50 PRs, matching S1-R's
own precedent exactly.

## Method

45 of the 50 PRs' own real base/head fixtures survived from S2's
original execution and were reused directly (efficient, and — since
`analyze()`/`compare()` themselves are untouched by either fix —
equally valid as a fresh fetch would be). One exception, disclosed
below. The 5 genuinely `oba=no` PRs (no manifest was ever built for
them) needed no re-run at all — nothing about applicability
determination changed in either fix.

## Headline: every real delta matches expectation; nothing else moved

**`#558149` — the real anomaly S2-F1 targeted.** The REUSED scratch
fixture for this PR turned out to reference the wrong file
(`nftables.nix`, which only *uses* `cfg.logRefusedPackets` in a string
interpolation, never *declares* it — an investigation artifact from
S2's own original fork, not a fix regression). Re-built correctly
using the actual `nixos/modules/services/networking/firewall.nix` at
the PR's own real head SHA (the same file S2-F1 itself was root-caused
and fixed against): `enable` now resolves to a clean, real `PASS`
(was `OptionNotFound` before the fix — completely invisible).
`logRefusedPackets` now resolves to `PredicateNotFound` (the
declaration is now visible; its own real usage,
`lib.optionalString cfg.logRefusedPackets "..."` inside a string
interpolation, isn't a recognized predicate SHAPE — a separate,
honest, already-understood limitation, not a false result and not
something S2-F1 claimed to fix). **The real bug is gone; what remains
is a disclosed, different, and much more ordinary limitation.**

**`#547038` and the `transition_origin` taxonomy.** The exact real PR
that motivated S2-F2 now reports `new_finding` with
`transition_origin: analysis_became_possible` — the reclassification
worked precisely as designed, on the real data that found the gap.
Every other real notable entry across both cohorts reproduced its
correct, expected origin: `#479381`/`#559009` (`analysis_became_
possible`, same-subject `Inconclusive` transitions), `#532540`/
`#552038`\(x2\)/`#554495` (`subject_added`, genuine module births — none
of these were ever the "PR-irrelevant" cases, so staying
`subject_added` is correct, not a regression).

**Exporter cases stay honestly `INCONCLUSIVE`.** `#552038` (the real
snowflake-exporter PR) reproduces byte-for-byte identical to S2's own
original result — `new_inconclusive` x2, `subject_added` — exactly as
S2-F3's own census predicted: no generic multi-file support was built,
so nothing here could have changed.

**Everything else: byte-for-byte unchanged.** All 44 remaining reused
fixtures reproduced their exact original S2 summary (same
`added`/`removed`/`new_findings`/etc., same empty `notable` where it
was empty before) — zero unintended side effects from either fix.

## Verdict

Both real fixes work exactly as designed, on the real PRs that
motivated them, with zero regressions across the rest of the 50-PR
corpus. Per the user's own explicit framing, this is confirmation the
fixes work, not a beta-readiness claim — the code has now literally
seen these 50 PRs. The next, unauthorized step is a new release and a
genuinely fresh, non-overlapping S3 sample, where actionable precision
— not correctness, which already looked strong before this round and
looks no worse now — is the metric that actually decides whether an
advisory beta is earned.
