# S5-F1B-R: full-corpus acceptance replay of candidate `6c2c2aa`

This document is a validation artifact only. It does not rewrite,
supersede, or reinterpret any prior historical result. Historical S5
(verdict **FAIL**, `28a1d58`), S5-F1 (`cdfb4ca`), S5-F1-R
(`baaaec9`/`f0a54a3`, verdict **FAIL**), and S5-F1B (`6c2c2aa`, targeted
verification complete, CI green) all remain exactly as committed. This
round is an **acceptance** replay, not a development round: no
implementation change was made during it.

**Purpose**: determine whether `6c2c2aa` fixes the angrr identity
collision without introducing semantic regressions anywhere in the
complete frozen S5 corpus.

## Baselines and provenance

- Historical S5 under test: `v0.4.5` / `8f1701a289ddab8bc23f23a25cc0937863fa0356`, verdict **FAIL**.
- S5-R0: `28a1d58`.
- First F1 candidate: `cdfb4ca`. S5-F1-R verdict: **FAIL** (retained as diagnostic context only; not re-executed this round).
- Acceptance candidate: `6c2c2aa71f0c9764c8fb7c1bb76473ae5918c5af`.
- Built from a genuinely clean checkout (`fixtures/s5-f1b-r/candidate-provenance.json`): fresh `git clone`, `git checkout 6c2c2aa...`, confirmed `git status --short` empty and `HEAD` matches before building.
- Binary SHA-256: `db3032775432ad276d6db4debd42265477aa70fe5cd7bb48a546fff2c995cf17`.
- Self-reported `tool.version`: `0.4.5` (unchanged, as expected -- identity is established by commit + binary hash, not the version string).

## Coverage: 369/369, derived and verified

The 189/180 non-applicable/applicable partition was **independently
re-derived** from the raw ledger (`fixtures/s5-live-pr-shadow/adjudication-ledger.jsonl`'s
own `pr_summary.applicable` field, not assumed from the S5-F1-R
manifest) via `fixtures/s5-f1b-r/derive-applicable-manifest.py`: 369
total records, 180 applicable, 189 non-applicable, and the re-derived
180-PR set is **byte-identical** to `fixtures/s5-f1-r/applicable-manifest.json`'s
own set -- same PR selection, same ordering, same frozen identities, as
required.

All 180 applicable PRs were fully replayed: real content re-fetched at
the exact frozen `base_sha`/`head_sha`, each PR's original committed
`targets.toml` reused verbatim, run against the clean-checkout `6c2c2aa`
binary. **0 fetch failures, 0 `window_evaluation_incomplete`.** Full
coverage achieved; nothing declared from partial data.

## Comparison result

- **158 / 180 byte-identical** to the historical v0.4.5 output.
- **22 / 180 changed** -- the *exact same 22 PRs* S5-F1-R found changed
  under `cdfb4ca` (`fixtures/s5-f1-r/delta-classification.json`'s own PR
  set, confirmed identical by direct set comparison). Every one of these
  22 PRs sits on the code paths S5-F1/S5-F1B's own mechanism touches;
  no PR outside that set shows any delta at all.

## Delta classification (every one of the 22 changed PRs individually traced)

| Classification | Count | PRs |
|---|---:|---|
| **expected target correction** | 1 | `#471312` |
| **demonstrated correction, same root-cause class** | 8 | `#431289 #506644 #508427 #440660 #428153 #397967 #427260 #463443` |
| **known pre-existing, unrelated limitation** | 6 | `#438285 #423934 #401840 #398993 #480839 #433539` |
| **unexpected candidate semantic delta** | **7** | `#429967 #494314 #484133 #557329 #374017 #260551 #432528` |

Full per-PR mechanism traces: `fixtures/s5-f1b-r/classification-notes.json`.

### `#471312` (angrr) -- corrected as intended

Re-verified from the frozen replay itself: head's watched `period`
transitions `PredicateNotFound` (historical, false match) ->
`OptionNotFound` (candidate, correct). Final result **Changed**, not
Unchanged.

### The 7 real F1-R watched-verdict regressions -- all restored

`#431289`, `#506644`, `#508427`, `#440660`, `#428153`, `#397967`,
`#427260` all restored to their exact historical transition. `#397967`
(fedimintd `api_ws.openFirewall`, the already-adjudicated real S5
finding) is confirmed **visible** under `6c2c2aa`
(`option_not_found->oba001`, matching historical exactly). Not inferred
from aggregate counts -- each verified individually against its own
`verdicts` field.

### The 14 former F1-R discovery-only deltas -- re-evaluated individually

All 14 have a **historical-identical watched verdict** (confirmed
individually, including the two already-adjudicated findings this
corpus carries in this set: `#260551` prosody `checkConfig`
`option_not_found->oba001`, and `#432528` tayga `wkpfStrict`
`option_not_found->pass`). `#463443` additionally turns out to be one of
the 7 verdict-level corrections (a `predicate_not_found` restoration),
reclassified accordingly. Their own `discovered_options` field, however,
splits into two genuinely different groups on closer trace (see below)
-- **6 are the already-disclosed, pre-existing limitation; 7 are a
newly-identified, F1B-specific gap.** This is a real correction to this
round's own understanding: the S5-F1B completion round (`fixtures/s5-f1b/f1b-report.md`)
verified these 14 only at the watched-verdict level and attributed the
whole residual `discovered_options` gap to the single pre-existing
`walk_merge_operands` limitation already known at that time. This
round's own field-level, per-path tracing (required by the mandate,
"do not merely infer restoration from aggregate counts") found that
attribution was incomplete.

#### 6 -- known pre-existing limitation (`#438285 #423934 #401840 #398993 #480839 #433539`)

frigate.nix (`#438285`, `#433539`) and nvidia-container-toolkit's
`default.nix` (`#423934`, `#401840`, `#398993`, `#480839`): the file's
own TRUE-ROOT `options =` value is itself wrapped in a form
`walk_merge_operands` does not unwrap (`with types; {...}` for frigate,
`let mountType = ...; in {...}` for nvidia-container-toolkit) --
Pass 1 never even discovers a true-root declaration in these files, so
F1B's own type-reference machinery never runs at all. This predates F1
entirely and was already disclosed in the S5-F1B completion report.
Confirmed unchanged this round.

#### 7 -- unexpected candidate semantic delta (`#429967 #494314 #484133 #557329 #374017 #260551 #432528`)

Two genuinely new mechanisms, both inside F1B's own new type-reference
resolution, neither previously disclosed:

**(a) Local `with` wrapping a type reference** (`#429967`, `#494314`,
`#260551`, `#440660`*, `#431289`*, `#484133`, `#432528` -- *`#440660`/`#431289`
also restore their own watched verdict correctly, see above; the
residual delta is separate content in the same file). `resolve_ident_binding`
already, pre-existingly, refuses to resolve any identifier reached
through a `with` scope. F1B's own `resolve_type_reference` reuses this
function directly -- so whenever the REFERENCE ITSELF (not the file, not
necessarily the target) is locally wrapped (`type = with types; attrsOf
(submodule X);`), the named submodule is never promoted, **even at one
hop**, even though the reference is completely legitimate. Confirmed in:
prosody's `vHostOpts` (`virtualHosts = mkOption { type = with types;
attrsOf (submodule vHostOpts); ...};`, hop 1) and, cascading from it,
`httpFileShareOpts`; rspamd's `workerOpts` (`workers = mkOption { type =
with types; attrsOf (submodule workerOpts); ...};`, hop 1), and
`bindSocketOpts` nested inside it (never even reached, since `workerOpts`
itself never promotes); tayga's `addrOpts` (`type = with types; nullOr
(submodule (addrOpts v));`, a function-call-wrapped reference, same
mechanism). This is genuinely new: F1's own blanket exclusion also hid
this content, but for an unrelated reason (it excluded everything);
F1B was specifically designed not to do that, and for this specific,
real mechanism, does not succeed.

**(b) Non-recursive alias-chain reference** (`#557329`, portmaster.nix).
`profiles`'s own inline-submodule `packages` option has `type = listOf
profilePackageType`; `profilePackageType` is a named `let`-bound type
alias (`types.coercedTo types.package (package: {inherit package;})
packageMatchType`) that itself references `packageMatchType`, the real
named submodule declaring `directory`/`name`/`package`/`storeNameRegex`/
`strictHead`/`strictLast`/`wrapped`. `collect_type_reference_ranges`
resolves `profilePackageType`'s own reference but does not recursively
scan `profilePackageType`'s own value for further nested references, so
`packageMatchType` is never discovered. This is the closest match in
this corpus to the mandate's own literal "true root -> legitimate
submodule A -> legitimate submodule B" scenario -- though the mechanism
is a non-recursive alias scan, not the one-hop cap itself (see "Critical
single-hop check" below).

**(c) Pure reordering of pre-existing content** (`#374017`, k3s.nix):
covered separately under "leaf-collision edge case" below -- no content
is hidden or invented, but a pre-existing duplicate-path collision's own
relative order changes, which the mandate's own instructions treat as
delta-worthy in the presence of ambiguity, not cosmetic.

None of these 7 changes any watched verdict anywhere in this corpus --
confirmed individually, not inferred. But per the mandate ("do not call
something cosmetic merely because the currently watched leaf survives"),
they are classified as unexpected candidate deltas, not folded into the
pre-existing-limitation category.

## Mandatory anchors

**A. angrr** -- corrected as intended, established from the frozen
replay (not only the committed fixture). Confirmed above.

**B. Seven F1 regressions** -- all restored, each compared explicitly
against its own historical result, not inferred from aggregate counts.
Confirmed above, including `#397967`'s continued visibility.

**C. Fourteen F1 discovery deltas** -- all re-evaluated individually;
every one has a historical-identical final verdict; `discovered_options`
identity is reported separately per PR above, split into the 6
pre-existing (unchanged this round) and 7 newly-identified unexpected
deltas -- not folded together.

**D. cgit** (`#475112`) -- unaffected. Not in the 22-PR changed list;
`comparison-report.jsonl` confirms byte-identical to historical.

**E. guacamole** (`#462487`) -- unaffected. Not in the 22-PR changed
list; confirmed byte-identical to historical.

**F. Fatal/error controls** -- unchanged. 0 `parse_errors` entries found
across all 180 PRs x 2 sides (360 check files scanned); only
`"changed"`/`"unchanged"` diff kinds observed across all 180 `raw.json`
outputs (0 error/fatal kinds), matching historical's own 0-error record
exactly.

## Critical single-hop check

Actively searched for a legitimate `true root -> named submodule A ->
named submodule B` chain lost **solely** to the one-hop cap, as
required. **No clean, isolated instance was found** in this corpus: every
multi-hop-shaped case observed is confounded by an independent local
`with`-scope resolution failure at one of the hops (angrr's own
`settingsOptions -> temporaryRootPolicyOptions`, and rspamd's own
`workerOpts -> bindSocketOpts`, both blocked at their FIRST hop by a
local `with`, so the cap's own marginal contribution at the second hop
cannot be isolated from the with-refusal in either case). The closest
real match is the portmaster alias-chain case above (`(b)`), which is
structurally similar but mechanistically distinct (non-recursive alias
scanning, not hop-counting).

**The one-hop cap itself was not widened during this round.** Its own
correctness relative to a genuinely clean two-hop options-block chain
remains untested by this corpus -- this is disclosed as an open
question, not resolved either way.

## Newly disclosed leaf-collision edge case

**Observed in the frozen corpus**: yes, 8 PRs exercise it (`#415326`
wstunnel `settings`, `#374017` k3s `enable`, `#463443` `profiles`,
`#429967`/`#494314`/`#431289`/`#260551`/`#440660` prosody `domain`/
`extraConfig`/`ssl`) -- `fixtures/s5-f1b-r/leaf-collision-scan.json`,
found via a dedicated full-corpus scan (all 180 PRs x both sides, not
just the 22 changed ones).

**Pre-existing in every case**: yes. Verified directly (not just via the
per-PR "identical" classification): for every one of the 15 (PR, side)
occurrences, the candidate's own colliding-path set is a **strict
subset** of the historical v0.4.5 baseline's own (equal or larger)
collision set. `6c2c2aa` never introduces a new colliding path anywhere
in this corpus; it only ever reduces the degree of a collision that
already existed, identically, before F1 or F1B ever existed. Not folded
into the angrr root-cause class -- this is `run_target`'s own pre-existing
first-match gate-1 model, unrelated to and unmodified by either fix.

**One additional wrinkle found this round**: `#374017` (k3s) shows that
F1B's two-pass architecture can change the **relative order** of a
pre-existing collision's own entries (promoted content is appended after
all true-root content, rather than interleaved by source position) --
without adding or removing any entry. This does not affect this PR's own
watched verdict (`autoDeployCharts`, unrelated to the colliding
`enable`), but it is new evidence that this architecture can, in the
presence of an existing ambiguity, change which duplicate `run_target`'s
gate-1 `.find()` would select for a different, hypothetical query. Not
fixed here.

**No change made to this behavior.** Disclosed and evidenced, exactly as
scoped.

## Reproducibility / integrity

- `applicable-manifest.json` independently re-derived from the raw
  ledger and verified byte-identical to the frozen selection
  (`derive-applicable-manifest.py`).
- `compare-replay.py` and `classify-deltas.py` (adapted verbatim from
  their S5-F1-R counterparts) reproduce `comparison-report.jsonl` and
  `delta-classification.json` deterministically from the committed
  `replay/` directory + `classification-notes.json`'s own hand-verified
  causal notes.
- `scan-leaf-collisions.py` reproduces `leaf-collision-scan.json`
  deterministically from the same committed replay data.
- New CI workflow `.github/workflows/s5-f1b-r.yml` enforces all three
  reproducibility checks on every future push touching this directory.

## Repository CI

Green (see PR/commit checks after this artifact is committed and
pushed).

## Gate

Per the frozen mandate: S5-F1B-R may PASS only if, among other
conditions, "every output delta is explained and classified" AND "no
unexplained candidate-induced discovery delta." Every one of the 22
deltas IS explained and classified here -- but 7 of them are classified
as **unexpected candidate semantic delta**, a category the mandate
explicitly does not treat as PASS-compatible ("Do not call something
cosmetic merely because the currently watched leaf survives"). Two
genuinely new mechanisms were found and traced this round, neither
previously disclosed: (a) local `with`-wrapped type references blocking
F1B's own resolution machinery even at a single, otherwise-legitimate
hop; (b) a non-recursive alias-chain gap losing a legitimately-reachable
submodule through an intermediate type alias. Neither causes any watched
verdict anywhere in this frozen corpus to differ from historical, and
neither hides any real adjudicated finding -- but per the mandate's own
explicit instruction, an unexpected semantic delta, once found and
traced, is not patched, not silently reclassified as pre-existing, and
drives the verdict.

```json
"final_verdict": "FAIL"
```

**A PASS does not prove that one-hop provenance is universally
sufficient; this result does not prove it is insufficient either.**
`6c2c2aa` correctly fixes angrr and correctly restores all 7 real
verdict-level regressions and the already-adjudicated `#397967` finding
-- the acceptance corpus's own verdict-level requirements (anchors A, B,
D, E, F) are fully satisfied. The gate fails on a stricter bar: full
field-level discovery-delta classification, which this round's mandate
specifically required rather than inferring from watched-verdict safety
alone. Per the mandate's own failure procedure: this delta set is
recorded fully, the implementation is **not patched** during this round,
the replay corpus is **not mutated**, and no rerun follows an ad-hoc
correction. `6c2c2aa` is not reverted or altered by this replay; it
remains exactly as committed, now with this replay's evidence attached.

## Scope discipline

No source fix, no test-driven implementation change, no fixture
rewriting, no historical ledger/adjudication modification, no cgit fix,
no guacamole fix, no version bump, no release, and no next-phase work
were performed during this round. Only replay machinery, immutable
replay artifacts, reconciliation/reporting, and integrity tests for that
machinery were added, per the mandate's scope restriction.

**STOP.** No implementation changes, release, or subsequent phase
without a separate, explicit GO.
