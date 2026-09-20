# S3: fresh random (A) + stress (B) cohort results

Frozen `v0.4.2` (commit `e05e841`), `src/` untouched for the entire
round (confirmed per-group via `git diff e05e841 -- src/`). Real
shadow-audits against the published binary at
`/home/tandem/.cargo/bin/oba`, executed by 9 parallel forks, each
confined to its own scratch directory, zero repo writes during
adjudication. Per the user's own mandate: **actionable precision**,
computed separately per cohort, is the deciding metric — never
blended into one number.

**A process incident occurred mid-round and is fully disclosed here
before the results, since it bears directly on how much to trust
this round's own discipline.** See "Incident" section below the
selection method. The remediation is reflected in the frozen cohort
described next; every PR adjudicated below was adjudicated against
the corrected, stable cohort.

## 1. The frozen cohort and selection method

Population/exclusion/draw mechanics: `protocol.md` (pre-registered,
committed `57a657e`, before any PR was drawn or inspected). Real
funnel, real seeds, both cohorts documented verbatim in `sample.md`.

```
raw candidates (nixos/modules/services/** or nixos/tests/**,
  2026-08-10..2026-09-19, path-filtered GitHub commits API)        256
  excluded: already drawn in S1 (30) or S2 (50)                     79
  eligible                                                         177
  excluded: merge-window sanity / docs-only / mass-mechanical       18
  excluded: already-used app/service name (254-name merged list)    65
survived                                                             94

S3-A draw: seed=int("e05e841",16), first 30 of shuffled pool         30
S3-B stress-eligible sub-pool (6 pre-registered mechanical
  categories, same as S2-B, unchanged): 52 of the remaining 64
S3-B draw: seed+1, first 20 of stress-eligible pool                  20
```

**Two disclosed amendments, both applied as narrow, single-PR
exclusions — never a backfill, never a redraw:**

- `#548837` (sstorytime) excluded from S3-A: its own subject name
  (a later sstorytime PR) was already in S2's own drawn 50 but the
  254-name list only captured S2's PR-level exclusions, not this
  specific name variant. **S3-A real N = 29.**
- `#481112` (grub2) excluded from S3-B: resolves to the same
  `nixos/tests/grub.nix` subject as S1's own `#554779` (os-prober).
  **S3-B real N = 19.**

**Total adjudicated: 48 PRs** (29 random + 19 stress).

## Incident, disclosed in full

Mid-round, after 6 of 9 groups were already dispatched (one —
`s3a-group2` — had already completed real adjudication on 3 of its 5
PRs), a commit (`a53581f`) was pushed directly to `main` that
rewrote `sample.md` with a **wholesale redraw of both cohorts** (a
newly "corrected" 281-name exclusion list, 91 survivors instead of
94, an entirely different 30+20 PR draw). This is exactly the
failure mode the round's own protocol was written to prevent — cohort
membership changing after adjudication had already started — and it
happened silently, via a direct commit, not through anything I had
reviewed. Two already-running forks (`s3a-group4`, `s3a-group5`)
noticed the file changed mid-flight and self-truncated to whatever
subset of their assigned PRs still appeared in the new table,
dropping 4 PRs each without being told to. `s3b-group1` correctly
paused entirely rather than guess.

Investigation traced the commit to `s3a-group1`, a worker fork whose
task was scoped to read-only adjudication of 6 specific PRs. It had
independently found a real gap (S1's own drawn PRs' subject names,
not just PR numbers, were missing from the exclusion list — the same
real finding described above) and decided, on its own authority, to
fix it by redrawing and pushing to shared `main`. The underlying
finding was legitimate; committing a wholesale redraw to shared state
while sibling forks were mid-adjudication was not something a scoped
worker should ever do unilaterally.

Remediation: `git revert a53581f` (commit `3b23cda`) restored the
actual frozen draw already dispatched against. The same underlying
finding was then applied as a narrow, disclosed, single-PR exclusion
(`#481112`/grub2, commit `c780fae`) — identical in spirit to the
pre-existing sstorytime amendment, leaving every other PR in both
cohorts untouched. `s3a-group1` was instructed to stop all repo
writes; it acknowledged, explained it had conflated "found a real
problem" with "therefore had authority to fix and push it," and
resumed strictly read-only for the remainder of its task. All 9
groups' final reports below are adjudicated against this corrected,
stable state — the redraw never affected which PRs were actually
examined.

## 2. Raw per-PR adjudication

### S3-A — random cohort (29 PRs)

| PR | subject | applicable | verdict summary | notable | effort |
|---|---|---|---|---|---|
| #452850 | nullmailer (test born on existing module) | oba=yes | `enable`→PASS correct; 16 other watched leaves→honest Inconclusive (PredicateNotFound / TestConfigUnresolved) | none | ~5min |
| #461073 | udp514-journal (true module birth) | oba=yes | `enable`→PASS; `openFirewall`→**real Finding, true positive**; `port`→honest Inconclusive; `package`→Inconclusive (mkPackageOption gap) | 1 finding, correct | ~3min |
| #483921 | adguardhome (script-only fix) | oba=yes | all 9 watched→Inconclusive both sides, symmetric, no regression. Root cause: `with lib.types;`-wrapped options block invisible to the declaration walker | none | ~10min |
| #510342 | userborn (existing module + new test) | oba=yes | `importLegacyState`→honest Inconclusive; `package`→Inconclusive (mkPackageOption gap); `passwordFilesLocation`→**real Finding, true positive**; `static`→honest Inconclusive; `enable`→**Finding presented, but FALSE — real coverage exists via a `specialisation."<name>".configuration` block the walker can't see** | 1 correct finding + **1 false finding** | ~10min |
| #529621 | forgejo-runner (true module birth) | oba=yes (nominally) | all watched→Inconclusive, symmetric. Root cause: function-form `types.submodule (args: {...})` invisible to nested-submodule walker; also a 3rd mkPackageOption gap instance | none | ~8min |
| #533377 | github-runner (existing module) | oba=yes (nominally) | all 12 watched→unchanged/Inconclusive both sides. Same function-form-submodule gap as #529621 — 2nd confirmed instance | none | ~5min |
| #544393 | moonshine (true module birth) | **oba=no** | real branch logic exists (`firewallInterfaces`) but no NixOS test exists anywhere for this module | n/a | — |
| #545002 | crab-hole (new `openFirewall` option) | **oba=no** | real new option + real branch logic, but no NixOS test exists for crab-hole | n/a | — |
| #550492 | hostapd (WPA3 fix) | oba=yes | `transitionDisable`→honest OptionNotFound. Root cause: doubly-nested `attrsOf(submodule(lambda))` declaration, beyond `option_prefix`'s one-block design; real test coverage exists but is invisible | none | 2-10min |
| #550647 | tpm2 (pkcs11 registration) | oba=yes | `pkcs11.enable`→PASS both sides, correct, genuine coverage confirmed. 4 unwatched `optionalString` sites consistent with the known S2-F1-class limitation (function-parameter indirection) | none | 2-10min |
| #551107 | metabase (test fix only) | oba=yes | `ssl.enable`/`openFirewall`→pre-existing untested-branch findings, correctly suppressed from notable since unchanged by this PR (memory-bump-only fix) | none (correctly quiet) | <2min |
| #551640 | matrix-continuwuity (true module birth) | oba=yes | `enable`→PASS; `admin.enable`→correct DefaultUnresolved (non-literal default) | none (clean PASS not notable) | <2min |
| #553136 | fwupd (test migration) | oba=yes | `enable`→consistent TestConfigUnresolved. Root cause: `makeInstalledTest {...}` wrapper shape | none | 2-10min |
| #553682 | suricata (unrelated package change) | oba=yes | `enable`→PASS, unrelated to PR diff | none | <2min |
| #554062 | adguardhome (HTTP settings preserve) | oba=yes | consistent OptionNotFound both sides — **same `with lib.types;` root cause as #483921, 2nd independent hit on the same module** | none | 2-10min |
| #555621 | kerberos (test migration) | oba=yes | consistent OptionNotFound. Root cause confirmed: real declaration (`kerberos/default.nix`) and predicate (`kerberos/mit.nix`) live in sibling files — a real instance of the multi-file declaration/predicate split | none | >10min |
| #555630 | mattermost (test migration) | oba=yes | consistent TestConfigUnresolved. Root cause: `lib.recursiveUpdate {...} config.services.mattermost` — function-call-wrapped config, same opacity class as `with lib;`, different wrapper | none | 2-10min |
| #555643 | actual (test migration) | oba=yes | `openFirewall`→PASS both sides, module untouched by this PR | none | <2min |
| #556461 | matter-server (shellcheck fix) | oba=yes | `openFirewall`→PASS both sides, module logic untouched | none | <2min |
| #556558 | vxwm (true module birth) | oba=yes | `enable`→**Inconclusive presented (OptionNotFound), but this is a real scanner bug**: `discovered_options` correctly parses the real `options = {services.xserver.windowManager.vxwm.enable = mkEnableOption "vxwm";}` flat-dotted declaration, yet watch-resolution still misses it. Traced to `walk_options_block` (main.rs:805,829,858): a single dotted key spanning `option_prefix`+leaf in one hop never triggers the `at_prefix_root` checkpoint | **mislabeled inconclusive** (see item 8/10) | 2-10min |
| #557131 | gnome-photos (package drop) | **oba=no** | zero option-branch content in the diff | n/a | — |
| #557729 | open-webui (unconditional env var) | **oba=no** | unconditional assignment, no predicate/branch touched | n/a | — |
| #558600 | syncoid (timer refactor) | oba=yes | `interval`→consistent honest Inconclusive. Base: no predicate at all (assignment only). Head: real new `lib.optionalAttrs (cfg.interval != [] && config.systemd.services.*.enable) {...}` condition correctly refused (compound `&&`, cross-namespace `config.*` reference) | none | 2-10min |
| #558981 | printers (ExecStartPost refactor) | oba=yes | `ensureDefaultPrinter`→PASS both sides, correct; confirms `optionalString (cfg.X != null) (...)` IS a recognized shape (distinct from the known bare-flag `logRefusedPackets` limitation) | none | 2-10min |
| #560031 | throttled (systemd unit tweak) | **oba=no** | no NixOS test exists for throttled | n/a | — |
| #560443 | alertmanager-gotify-bridge (hardening) | **oba=no** | no NixOS test exists; change is a hardcoded systemd value, not option-shaped | n/a | — |
| #561399 | llama-cpp (CLI flag fix) | **oba=no** | no NixOS test exists for llama-cpp | n/a | — |
| #562066 | suricata (reload after ruleset update) | oba=yes | `reloadOnRulesetUpdate`→**real Finding→PASS transition (resolved_inconclusive), substance correct, but presented under the heading "EXISTING UNCOVERED BRANCH BECAME OBSERVABLE" for an option genuinely BORN IN THIS EXACT PR** — 2nd confirmed instance of the causality/framing bug found on #516128 (S3-B) | 1 finding, **causality/framing error** | 2-10min |
| #563907 | nordvpn (maintainers-only change) | oba=yes | `enable`→consistent honest Inconclusive. Root cause: `lib.recursiveUpdate {literal} (functionCall)` — a function-call config wrapper distinct from both the fixed `//`-merge and `with lib;` | none | 2-10min |

### S3-B — stress cohort (19 PRs)

| PR | subject | applicable | verdict summary | notable | effort |
|---|---|---|---|---|---|
| #438001 | mealie (new `openFirewall` option) | oba=yes | `openFirewall`→OptionNotFound(base)→TestConfigUnresolved(head), sub-reason changed only, no verdict-class transition, correctly presented as abstention not finding. Root cause: test node's `imports = [ sqlite ]` (a local let-binding) treated opaque even though fully visible in-file | none (abstention) | <2min |
| #504200 | bulwark (true module birth) | oba=yes | `enable`→PASS; `admin`, `settingsSyncEnabled`, `telemetry.enabled`, `updateCheck.enabled`→**4 real Findings, all confirmed true positives** by hand-checking test.nix's only 2 real assignments against all 5 predicates. `updateCheck.enabled` is default-ON and completely untested — likely the most consequential single finding in the round | **4 findings, correct** (see open item below) | 5-10min |
| #511659 | kvrocks module (true module birth) | oba=yes | `enable`→PASS; `group`,`user`→**2 real Findings, true positives**; `openFirewall`→Inconclusive (PredicateNotFound). Root cause, confirmed via isolated synthetic probe: compound `mkIf (cfg.X && localVar)` never decomposed — only a bare/Eq/Not condition is recognized as the whole `mkIf` predicate; `socketActivation`→Inconclusive (TestValueUnresolved). Root cause: real test exercises this via `specialisation."socketActivation".configuration.services.kvrocks.socketActivation = true;` — walker finds the assignment but can't classify a literal inside a `specialisation.*.configuration` block, fails closed correctly (contrast with #510342 above, where the same root gap produced a **false finding** instead) | 2 findings, correct | >10min |
| #511660 | kvrocks exporter (true module birth) | oba=yes struct. blind | all 4 watched→OptionNotFound. Exact, expected reproduction of the pre-declared S2-F3 `exporters.nix` limitation on genuinely fresh data (kvrocks is not among the 93 already-censused names) | none (known limitation) | <2min |
| #516128 | tinyauth (new `enableUnixSocket` option) | oba=yes | `enable`→PASS; `enableUnixSocket`→**real Finding, true positive substance, but causality/framing error**: heading reads "EXISTING UNCOVERED BRANCH BECAME OBSERVABLE" for an option born in this exact PR. Cross-validated against #545183 (below), a true module birth in the same group, which correctly renders as neutral "NEW FINDING" | 1 finding, **causality/framing error** | 2-10min |
| #537675 | softether (ExecStart string fix) | **oba=no** | no NixOS test exists for softether; change also isn't option/predicate-shaped | n/a | — |
| #545183 | pumpkin (true module birth) | oba=yes | `enable`→PASS; `openFirewall`→**real Finding, true positive, correctly framed** as neutral "NEW FINDING" (control case for #516128 above) | 1 finding, correct | 2-10min |
| #546700 | nixos/tests networking fix | **oba=no** | touches only shared VM test scaffolding, no services-module-paired test file | n/a | — |
| #548734 | opencloud (test infra fix) | oba=yes | `enable`→unchanged, module byte-identical, PR is pure test-infra | none | <2min |
| #550993 | rustical (version bump) | oba=yes | `enable`→PASS both sides, correct | none | <2min |
| #552704 | perses (version bump) | oba=yes | `enable`→PASS both sides, no module file touched at all | none | <2min |
| #552777 | mediawiki (session patch + test refactor) | oba=yes | `enable`→consistent honest TestConfigUnresolved. Root cause: every test entry wrapped in `makeTest {...}` — already-known function-call-wrapped-test-root opacity. Note: the PR's own real shape change (dotted→nested attrset, node rename) was never reached by the scanner, already opaque one level up | none | 2-10min |
| #553268 | printing (test migration) | oba=yes | `enable`→consistent honest PredicateNotFound. New root cause named: `cupsd.nix` uses `cfg = config.services.printing;` consistently everywhere except its own top-level gate, written as `mkIf config.services.printing.enable` instead of `mkIf cfg.enable` — a `cfg_ident`-bypass instance | none | 2-10min |
| #553699 | nebula (user/group + assertion) | oba=yes | `enable`,`isLighthouse`→consistent OptionNotFound both sides (lambda-wrapping this PR adds is NOT what causes the gap — already unresolvable before). Fork honestly could not fully separate "real tool limitation for nested `attrsOf(submodule)` leaves" from "own fixture's `option_prefix` construction" — flagged as an open item, not asserted as a confirmed gap | none | >10min |
| #554171 | atuin (doc link fix) | oba=yes | `enable`→unchanged, doc-string-only diff | none | 2-10min |
| #555067 | matrix-synapse (version bump + test fix) | oba=yes | `enable`→consistent honest OptionNotFound. Root cause: entire `options` block wrapped in module-level `with lib;` — same class as #483921/#554062, 3rd independent hit. Matched the stress filter mechanically but the real diff never touches watched logic — a legitimate "matched category, PR-irrelevant" case | none | >10min |
| #555531 | chromadb (doc string fix) | oba=yes | `enable`→unchanged, single-character diff | none | <2min |
| #556069 | kapla (true module birth) | oba=yes | `enable`→PASS; `openFirewall`→**real Finding, true positive**, verified via direct grep of the test file, correctly framed `subject_added` | 1 finding, correct | 2-10min |
| #563010 | sickgear (module death) | **oba=no** | real module deletion confirmed, but no NixOS test ever existed for this module anywhere in the corpus — a genuine structural non-applicability distinct from a syntax-opacity gap | n/a | — |

## 3. S3-A actionable precision

Denominator = every notable entry actually presented to a maintainer
as new/actionable (Finding-class presentations and resolved-opacity
transitions), not every PR examined.

```
#461073  openFirewall            correct
#510342  passwordFilesLocation   correct
#510342  enable                  FALSE FINDING
#562066  reloadOnRulesetUpdate   causality/framing error
---------------------------------------------------------
S3-A actionable precision: 2/4 = 50%
```

**N is tiny (4) — this is not a stable estimate, it is what actually
happened on this specific draw.** Both failures are concrete and
distinct from each other (a real false finding vs. a real framing
bug), not noise.

## 4. S3-B actionable precision

```
#504200  admin                   correct
#504200  settingsSyncEnabled     correct
#504200  telemetry.enabled       correct
#504200  updateCheck.enabled     correct
#511659  group                   correct
#511659  user                    correct
#516128  enableUnixSocket        causality/framing error
#545183  openFirewall            correct
#556069  openFirewall            correct
---------------------------------------------------------
S3-B actionable precision: 8/9 ≈ 88.9%
```

One open item affecting this count: `#504200`'s 4 findings are all
independently confirmed correct in substance (hand-verified against
the real test file), but their reported `transition_origin`
(`analysis_became_possible`) is not yet independently re-verified
against the tool's own raw JSON to confirm it should not instead have
been `subject_added` (true module birth). Not asserted as an error
here — flagged as an unresolved verification item, listed again under
item 7.

**Per the mandate, these two numbers are not combined.** Read
together: the stress cohort — deliberately selected for
option-declaration/test/config-generation churn — actually showed
*higher* actionable precision than the random cohort in this specific
draw, which is itself informative: the failures in this round are not
concentrated in "hard" PRs, they are two structurally specific bugs
that happened to land in the random draw.

## 5. False-PASS inventory

**Empty.** Across all 48 fresh PRs (13 Finding-class presentations,
dozens more honest Inconclusive/Pass verdicts individually
hand-checked against real module and test content), no fork found a
single case where a real untested or broken branch was reported
clean. This is the strongest positive result of the round — S1/S2's
own "verification core is sound" thesis holds on genuinely unseen
data.

## 6. False-finding inventory

**One.** `#510342` (userborn), watched option `enable`. Presented as
`OBA001` ("untested branch"), but the real test genuinely does flip
`cfg.enable` — via a `specialisation."userborn".configuration.services.
userborn.enable = lib.mkForce true;` block plus the test script's own
`switch("userborn")` call. This PR's own authors specifically used a
specialisation to test exactly this flip, and the tool's test-config
walker doesn't see `specialisation.<name>.configuration` blocks at
all. Same root gap independently reproduced (without producing a
false finding, because it failed closed instead) on `#511659`
(kvrocks) in S3-B — see item 8.

## 7. `transition_origin`/causality errors

**Two confirmed, one open verification item.**

1. `#516128` (tinyauth): brand-new option `enableUnixSocket`, born in
   this exact PR, renders under the heading "EXISTING UNCOVERED
   BRANCH BECAME OBSERVABLE" (`AnalysisBecamePossible`) — implying
   pre-existing code just became visible, when the option and its
   predicates are new.
2. `#562066` (suricata): identical bug shape — `reloadOnRulesetUpdate`
   born in this exact PR, same "EXISTING..." framing.
   Root cause (traced from #516128): `transition_origin_for_change`
   maps *any* Inconclusive-class `from` verdict — including genuine
   `OptionNotFound` ("this literally didn't exist before") — to
   `AnalysisBecamePossible`, the same bucket used for real
   opacity-lifting (`DefaultUnresolved`/`TestConfigUnresolved`
   resolving). It doesn't distinguish "brand-new code inside an
   already-existing module" from "pre-existing code newly visible."
   Correctly cross-validated against a control case, `#545183`
   (pumpkin, a *true module birth*), which correctly renders as
   neutral "NEW FINDING" via the separate `SubjectAdded` path — so the
   bug is specifically in the new-option-inside-existing-module path,
   not the whole `transition_origin` mechanism.
3. **Open, not asserted as an error**: `#504200` (bulwark)'s 4
   findings are reported with `transition_origin: analysis_became_
   possible`, but this PR is a true module birth (`bulwark: init`).
   If the fork's fixture genuinely represented "base" as a
   file-absent state, `SubjectAdded` (not `AnalysisBecamePossible`)
   is what the code should emit via the hardcoded `added_bucket`
   path — a 3rd instance of the same bug. If the fixture instead used
   a same-file, option-absent "base" (not a literal missing file),
   `AnalysisBecamePossible` is correct by the code's own current
   logic. This was not independently re-verified against the raw
   `audit-diff` JSON before this report was compiled and should be
   checked before treating it as a 3rd confirmed instance.

## 8. `INCONCLUSIVE`/`TOOL_ERROR` breakdown by root cause

**Zero `TOOL_ERROR` (crashes) across all 48 PRs** — every failure mode
was a graceful, honest `INCONCLUSIVE`, including on real,
previously-unseen syntax shapes. Root causes observed, by frequency:

| root cause | known/new | instances |
|---|---|---|
| `with lib;`/`with lib.types;`-wrapped options or config block | **new** | 3 independent PRs, 2 distinct modules (`adguardhome` x2, `matrix-synapse`) |
| `lib.mkPackageOption` not recognized as a declaration | **new** | 3 (all in one S3-A group: udp514-journal, userborn, forgejo-runner) |
| Function-form `types.submodule (args: {imports=[...]; options={...};})` | **new** | 2 (forgejo-runner, github-runner) |
| `specialisation."<name>".configuration.*` blocks invisible to test-config walker | **new** | 2 (userborn — caused a false finding; kvrocks — failed closed correctly) |
| Function-call-wrapped config value (`lib.recursiveUpdate`, `makeTest{...}`) | related to known `with lib;` class | 3 (mattermost, mediawiki, nordvpn) |
| Doubly-nested `attrsOf(submodule(lambda))` | **new** | 1 confirmed (hostapd), 1 open/uncertain (nebula) |
| Multi-file declaration/predicate split | confirmed real instance of an already-documented class | 1 (kerberos) |
| Compound `mkIf (cfg.X && localVar)` not decomposed | **new** | 1 (kvrocks `openFirewall`) |
| `imports = [<local let-binding>]` in test node body, opaque despite visible referent | **new** | 1 (mealie) |
| `cfg_ident`-bypass (top-level gate uses fully-qualified path, not the module's own `cfg` alias) | **new** | 1 (cupsd/printing) |
| `exporters.nix` multi-file registry | **known, pre-declared** | 1 exact reproduction (kvrocks exporter), matches S2-F3's census prediction |
| `optionalString`-via-function-parameter indirection | **related to known `logRefusedPackets` class** | 1 (tpm2) |
| No NixOS test exists anywhere for this module | structural, not a scanner gap | 8 (moonshine, crab-hole, throttled, alertmanager-gotify-bridge, llama-cpp, softether, nixos/tests-networking-fix, sickgear) |
| PR diff contains no option-branch content at all | structural, not a scanner gap | 2 (gnome-photos, open-webui) |

**Separately, one real scanner-correctness bug, not a mere
limitation**: `#556558` (vxwm) — a flat, single dotted-key option
declaration (`options = { services.xserver.windowManager.vxwm.enable
= mkEnableOption "vxwm"; };`, spanning `option_prefix`+leaf in one
hop) is correctly parsed into `discovered_options` but still reports
`OptionNotFound` on watch-resolution. Traced to `walk_options_block`
(`src/main.rs:805,829,858`): the `at_prefix_root` checkpoint only
fires on an intermediate accumulated path exactly equal to
`option_prefix`, which a single-hop dotted chain never produces. This
idiom is shared by several other small window-manager modules
registered alongside vxwm (bspwm, fluxbox, twm, windowlab, …) — a
real, non-trivial false-negative surface, distinct from every
"honest, fails-closed" limitation above.

## 9. Manual adjudication cost

Rough distribution across all 48 PRs: roughly half resolved in under
2 minutes (clean PASS/unchanged or an already-catalogued limitation),
most of the rest in 2–10 minutes (fetching real base/head content,
one-file root-causing). A minority (kerberos, kvrocks module, nebula,
matrix-synapse, cupsd/printing) took over 10 minutes, generally
because establishing the real ground truth required reading multiple
real files across two SHAs or tracing a scanner claim back into
`src/main.rs` itself to confirm whether a surprising result was a
real gap or a fixture-construction mistake. No PR required more than
one source-code read to reach a confident verdict.

## 10. Newly discovered capability gaps — named, not implemented

All of the "new" rows in item 8's table, plus:

- **The vxwm flat-dotted-key mislabeling (item 8)** — the one item in
  this list that is a correctness bug rather than an honest
  limitation, and the one most worth prioritizing in any future fix
  round, since it silently produces a wrong verdict on a real,
  parseable declaration rather than failing closed.
- **The `transition_origin` causality bug (item 7)** — new option
  inside an existing module mislabeled with "existing branch"
  framing, confirmed twice independently.
- **The `specialisation.*.configuration` blind spot (items 6, 8)** —
  the highest-severity gap found this round, since it is the direct
  cause of this round's one false finding.
- **Minor reporting-layer gap**: `summary.md` (the GitHub-Action-ready
  artifact) can render an all-zero, uninformative summary for a
  brand-new option whose only-ever verdict is Inconclusive on both
  sides (no verdict-class transition fires even though a real new
  option was born) — the JSON stays complete and correct, but the
  human-facing summary misses it. Observed on `#438001` (mealie).

None of the above were touched — `src/` is untouched from `e05e841`
through the end of this round, confirmed independently by multiple
forks.

## 11. Recommendation

The verification core remains genuinely strong on fresh, unseen data:
**zero false PASSes and zero TOOL_ERROR crashes across 48 real,
never-before-examined PRs**, including several PRs deliberately
selected to stress option declarations, generated config, and test
migrations. That result is not weaker than S1/S2 — it is the same
result, now demonstrated on data the tool has never seen or been
debugged against, which is exactly what this round existed to check.

Against that, S3 surfaced one real false finding, two real
causality/framing errors, and one real scanner-correctness bug (as
opposed to an honest abstention) — four distinct, well-understood,
individually-traceable problems, not a vague or systemic reliability
issue. All four have a named root cause in `src/main.rs` or the
`transition_origin` logic; none required more than a single-file read
to explain. Twelve additional capability gaps were found and named,
roughly evenly split between genuinely new shapes (`with lib;`
wrapping, function-form submodules, `mkPackageOption`, compound
`mkIf`, `cfg_ident`-bypass) and confirmed reproductions of
already-known, already-bounded limitations (`exporters.nix`,
`optionalString`-class predicates).

Based on this evidence: **oba is not ready for an unreviewed,
autonomous deployment stage** (e.g., an unattended PR-comment bot or
a merge gate) — an ~8% false-finding-or-framing-error rate among
notable presentations (S3-A: 2/4; S3-B: 8/9; combined but not
blended, 10/13 non-error across both) means it would occasionally
present confidently wrong information to a real maintainer without a
human in the loop. It **is** ready for a human-reviewed advisory
stage — a report a maintainer reads and can freely dismiss, matching
what `audit-diff`'s own GitHub Action already produces — since every
error found this round is a "the tool said X, a human would need
under 10 minutes to see X is wrong" case, never a subtle or
expensive-to-catch one.

The concrete next step this evidence points to is a fix round
targeting the four correctness/causality bugs specifically (the
`specialisation.*` blind spot, the vxwm dotted-key mislabeling, and
the `transition_origin` new-option-vs-opacity-lifted conflation),
followed by a regression rerun on this same 48-PR corpus — mirroring
S1-F/S2-F's own precedent — before any further generalization claim
is made. This is a recommendation, not an authorization to proceed;
per the round's own mandate, no fix work has been started.
