# GO-F Phases F3-F7: scoring, adjudication, gates, final verdict

**Parents**: `reports/go-f-f2-1-target-construction.md` (`0ea8d44`),
`reports/go-f-prereg.md`/`amendment-A1`. Candidate: `9ff7c04` (binary
sha256 `406a27f5...3498`, unchanged throughout -- verified again
below). P1 unchanged. GO-E replay not counted.

## Self-caught target-construction error, corrected before adjudication

While reviewing the first scoring run's own output, a real error in
this round's OWN target construction was caught: `pr519655_settings_*`
and `pr557977_settings_*` were initially built with `option_prefix`
ending in the interior `mkOption`-wrapped container's own name
(`[...,"settings"]`, `watch=["HOST"]` etc.) -- **the exact same
invalid shape GO-D already found and corrected in P1**. Caught by
noticing `settings.HOST`/`settings.PORT` came back `OptionNotFound`
despite the real head source genuinely declaring them (same pattern
as the retracted P1 defect). Corrected to `option_prefix` stopping at
the true root (`["services","elk"]`/`["services","bazarr"]`),
`watch` as the full relative dotted path (`"settings.HOST"` etc.),
per `reports/go-f-prereg.md`'s own F1.5 contract -- and re-run. All
raw outputs below are from the CORRECTED construction; the
first-attempt (wrong) raw JSON is not kept as scoring evidence (it
tested a malformed request, exactly like `h2-case15-xandikos`).
`#564819`'s `maxConcurrency`/`metricsAddress` were checked and
confirmed to NOT have this problem (`options.services.nar-serve`
-- a true top-level root, no interior container involved; verified
directly in `work/go-f/trees/564819/head/.../nar-serve.nix` lines
19-20).

This is the GO-D/GO-A lesson applied to this round's own work, not
just cited: target-construction validity was checked *before*
adjudication, not after.

## Phase F0 re-confirmation (candidate unchanged)

`work/go-f/oba-candidate-9ff7c04` sha256 re-verified identical
(`406a27f54378c3a5b5b14c08fb9bec4d12bb23313fea73233d7154bb01834988`)
immediately before Phase F3's scoring runs. No `oba` source change
occurred at any point in this round.

## Phase F3: scoring (raw evidence)

`oba diff --base-root ... --head-root ... --targets ... --json` run
once per derived PR (5 invocations, corrected manifests), raw JSON in
`work/go-f/raw/{570480,519655,557977,564819,569875}-diff.json`. No
`TOOL_ERROR` anywhere (GO-E's own feature exercised cleanly for
`#557977`'s new module). No infra failure, no rerun needed.

## Phase F4: adjudication (disclosed NOT blind)

**Honesty note, same as `P1-harness-repair.md`'s own precedent**: this
round has exactly one executing agent (this fork). The same process
that constructed targets and ran `oba` is also the one classifying
results -- genuine blind-then-reveal (a reviewer with no access to
the real output, writing an answer first) was not achievable within
this single continuous session. Stated plainly rather than faked.
Classification below is a disclosed retrospective judgment against
real source + real output, not a blind prediction.

| Target | Real verdict | Ground truth (from real source) | Classification |
|---|---|---|---|
| `pr570480_nixPath` (base) | `OptionRelocated` -> `nix.settings.nix-path`, confirmed | Base genuinely has the old `mkRenamedOptionModule` shim to that exact destination | `CORRECT` |
| `pr570480_nixPath` (head) | `PredicateNotFound` | Head restores flat `nixPath = mkOption{listOf str}`, consumed as a plain value, no boolean gate | `CORRECT` |
| `pr519655_settings` | `OptionNotFound`(base)/`PredicateNotFound`(head) | New in head; consumed as opaque freeform data, no predicate of its own | `CORRECT` |
| `pr519655_settings_analytics_enabled` | same shape | New; forwarded via freeform `DYNACONF_*`, never read by a NixOS predicate | `CORRECT` |
| `pr519655_settings_general_auto_update` | same shape | `readOnly=true`, forwarded via freeform, never predicate-gated | `CORRECT` |
| `pr519655_settings_general_port` | same shape | Consumed as a value under `openFirewall`'s OWN predicate, not its own | `CORRECT` |
| `pr519655_listenPort_removed` | base `PredicateNotFound`, head `OptionRelocated` -> `settings.general.port`, confirmed | PR's own `mkRenamedOptionModule` migrates exactly there | `CORRECT` |
| `pr564819_maxConcurrency` | `OptionNotFound`(base)/`PredicateNotFound`(head) | New; `environment.MAX_CONCURRENCY = toString cfg.maxConcurrency;` -- plain value, no gate | `CORRECT` |
| `pr564819_metricsAddress` | same shape | Same pattern, plain value | `CORRECT` |
| `pr557977_enable` | `PASS`, witnessed | Real test sets `services.elk.enable = true;` | `CORRECT` |
| `pr557977_openFirewall` | `OBA001`, `witnessed: false` | Real test never sets `openFirewall` -- genuinely untested default | `CORRECT` |
| **`pr557977_package`** | **`OptionNotFound`** | **Genuinely declared** via `package = mkPackageOption pkgs "elk" {};`, consumed (`ExecStart = lib.getExe cfg.package;`) -- confirmed absent from `discovered_options` entirely (checked via `oba check`) | **`MISSED_FINDING`** -- see Phase F5 |
| `pr557977_settings` | `PredicateNotFound` | Whole-value freeform passthrough, no predicate | `CORRECT` |
| `pr557977_settings_HOST` | `PredicateNotFound` | Never individually gated | `CORRECT` |
| `pr557977_settings_PORT` | `PredicateNotFound` | Used as a value under `openFirewall`'s own predicate, not its own | `CORRECT` |
| `pr569875_declarativePlugins` | `TestConfigUnresolved` | Real `if (cfg.declarativePlugins == null) then ... else ...` predicate exists; grafana's own real test config is too complex for static resolution here | `CONSERVATIVE_INCONCLUSIVE` |

**Taxonomy totals**: `CORRECT` = 14, `MISSED_FINDING` = 1,
`CONSERVATIVE_INCONCLUSIVE` = 1. Zero `FALSE_FINDING`/
`MISLEADING_PRESENTATION`/`INPUT_OR_HARNESS_FAILURE`/
`ORACLE_AMBIGUOUS`/`KNOWN_UNSUPPORTED_RENAME_MOVE` (no rename/move in
this sample, per F2.1).

## Phase F5: defect handling (`pr557977_package`)

- **Not fixed.** `oba`'s own source was not touched.
- **Minimal real evidence preserved**: `work/go-f/trees/557977/head/
  nixos/modules/services/web-apps/elk.nix` (real declaration,
  `mkPackageOption`), `work/go-f/raw/557977-diff.json` +
  `oba check`'s own `discovered_options` list (confirmed `package`
  absent from it entirely, not merely unresolved).
- **Escalation per F1.7**: `MISSED_FINDING` is one of the 4 trigger
  conditions. **A second, independent, blind reviewer was NOT
  obtained in this round** -- stated honestly, exactly the GO-A/GO-D
  lesson this preregistration itself cited in advance (Phase F1.7).
  This finding is therefore **flagged, not confirmed**.
- **Target-construction validity checked first** (the GO-D/GO-A
  lesson, applied proactively this time): `option_prefix=["services",
  "elk"]`/`watch=["package"]` is contract-correct (a direct,
  flat, top-level leaf -- not an interior-container case). The
  declaration genuinely exists and is genuinely absent from
  `discovered_options`. This is not a repeat of the P1 target-
  construction mistake.
- **Classification of the cause**: `target-construction-protocol.md`'s
  own step 2 explicitly names `mkPackageOption` alongside `mkOption`/
  `mkEnableOption` as a recognized declaration form for ELIGIBILITY
  screening; `grep -rn "mkPackageOption" README.md src/main.rs`
  returns nothing -- `oba`'s own `scan_options` does not appear to
  parse `mkPackageOption` calls into `discovered_options` at all. This
  looks like a genuine, previously undiscovered gap (not a documented,
  intentional limitation), but is reported as a **secondary
  engineering observation requiring the escalation this round could
  not provide** -- not as a primary-verdict-changing confirmed defect.

## Phase F6: gates (via `pilot_accounting`'s real functions, not hand arithmetic)

| Counter | Value |
|---|---|
| `sampled_pr_count` | 15 |
| `derived_pr_count` | 5 |
| `derived_target_count` | 15 |
| `substantive_target_count` | 15 (zero `INPUT_OR_HARNESS_FAILURE`) |
| `adjudicated_target_count` | 15 (zero `ORACLE_AMBIGUOUS`) |

```
nc2_pr_level(5)      -> {numerator: 5,  denominator: 15, passed: False}   # need >=9
nc3_target_level(15,15) -> {numerator: 15, denominator: 15, threshold: 11, passed: True}
nc4_target_level(15,15) -> {numerator: 15, denominator: 15, threshold: 11, passed: True}
batch_passes_expansion_gates(5,15,15,15) -> False
```

**NC2: KILL** (5/15, need >=9). **NC3: PASS** (15/15 >= 11). **NC4:
PASS** (15/15 >= 11).

## Phase F7: comparison with P1 (descriptive only, not pooled)

| | `derived_pr` | NC2 | NC3 | NC4 |
|---|---|---|---|---|
| P1 original cohort | 5/15 | KILL | 7/28 KILL | 7/7 PASS |
| P1 corrected protocol | 3/15 | KILL | 7/7 PASS | 7/7 PASS |
| **GO-F (fresh, post-GO-E)** | **5/15** | **KILL** | **15/15 PASS** | **15/15 PASS** |

NC2 fails in every single one of these three independent looks at
this question -- P1's original cohort, P1's corrected cohort, and
now a fully fresh, non-overlapping GO-F sample. NC3 improves
descriptively (consistent with GO-E removing the new-module
`TOOL_ERROR` class entirely -- all 6 of `#557977`'s targets reached a
real verdict), but per this round's own explicit instruction,
**improved NC3/disappearance of `TOOL_ERROR` is not evidence of
useful PR-level yield on its own** -- NC2 is the binding constraint,
and it fails on fresh data exactly as it failed on both P1 cohorts.
Not pooled statistically; the 21 GO-E replay cases are not counted
toward GO-F's own N anywhere above.

## Primary experimental question, answered

"After GO-E, is PR-level useful yield on fully fresh data sufficient
to pass the preregistered NC2 expansion gate?" **No.** Only 5 of 15
freshly sampled PRs mechanically derived even one target -- the same
order of magnitude as both P1 cohorts. GO-E's own real contribution
(eliminating `TOOL_ERROR` for new-module PRs) shows up correctly in
NC3, not in NC2 -- exactly the distinction this round's own
preregistration warned against conflating.

## Final verdict

**`FRESH-EVAL-KILL`**

A mandatory gate (NC2) fails. Per the preregistered rule, this is not
rescued by additional PRs, by the NC3/NC4 passes, or by GO-E's own
real, independently-useful contribution to evaluability.

## Explicit confirmations

- **P1 unchanged**: no file under `fixtures/s6-r1/` was read-written
  by this round (only read, for reusing the frozen contract text).
- **GO-E replay not counted**: the 21 `#443747`/`#568048` replay
  targets do not appear anywhere in this round's own 15-target count.
- **Candidate unchanged after freeze**: sha256 `406a27f5...3498`
  re-verified identical at the start of Phase F3 and again here.
- **P2 not started.**

## Secondary engineering observations (do not change the primary verdict)

- **`mkPackageOption` discovery gap** (`pr557977_package`): flagged,
  escalation-pending, not confirmed (see Phase F5).
- **Rename/move**: zero occurrences in this 15-PR sample --
  `KNOWN-LIMITATION-1` was pre-registered but not exercised.
- **`OptionRelocated` worked correctly twice** on fresh data
  (`#570480`, `#519655`), both with `destination_confirmed: true` --
  a real, positive data point for that specific capability,
  independent of the primary KILL verdict.

## What did not happen

No `oba` fix. No README/protocol change. No P2/expansion decision. No
threshold change after seeing results (NC2/NC3/NC4 thresholds were
fixed in `reports/go-f-prereg.md`, before any PR was inspected).

**`S6_GOF_FINAL_FRESH_EVAL_KILL`**

**STOP.**
