# C-E1.2c: `GeneratedConfigArtifact` soundness audit -- protocol

Pre-registered BEFORE any attack is run or any holdout candidate is
inspected, per this project's own standing discipline. Committed as its
own commit, separate from any results.

## What this round is, and is not

C-E1.2a proved the abstraction works on 4 heterogeneous anchors.
C-E1.2b proved it transfers onto 8 more real candidates without
app-specific branching (6 clean PASS, 1 real FINDING, 1 real
INCONCLUSIVE). Both were about whether the model FITS. **This round is
about whether the model can be TRUSTED** -- a hostile audit measuring
soundness (false PASS, false FINDING) rather than coverage, run against
a vertical that is now substantial enough to be worth trying to break.

- **Frozen at commit `984b366`** (C-E1.2b implementation's own closing
  commit, confirmed green on all 5 CI workflows).
- **Production code stays frozen for the whole audit**: `acquire_*`,
  `extract_*`, `compare_config_contract`, `flatten_structured_value`,
  every `ArtifactBindingEvidence`/`ConfigFormat` variant -- none of it
  changes during this round, regardless of what's found. "Production
  code" means the analysis engine itself. Writing NEW `#[ignore]`
  adversarial/mutation test functions that exercise the FROZEN engine
  is this round's own actual output and does not violate the freeze --
  but if a test finds a genuine bug, the FIX is explicitly NOT part of
  this round (matching the `vault`-stays-`INCONCLUSIVE` precedent) --
  it becomes a scoped, named follow-up item in this round's own
  decision, for the user to authorize separately.
- Every real candidate this round touches was already real-proven by
  an earlier round -- no re-deriving A/B/C/D from scratch except for
  the fresh holdout (which needs it fully, being genuinely new).

## Population

**11 already-implemented real candidates** (C-E1.2a's 4 anchors +
C-E1.2b's 7): `unpackerr`, `unbound`, `mobilizon`,
`nebula-lighthouse-service`, `privoxy`, `misskey`, `kavita`,
`transmission`, `i2pd`, `spacecookie`, `akkoma`. `vault` stays excluded
(its own real `INCONCLUSIVE` from C-E1.2b stands, not re-litigated
here).

**2 negative controls**: `libinput`, `nohang` -- C-E1.1's own real
`NO_PRODUCER_PROOF` cases, untouched by any implementation round since.
Re-run against the NOW-EXPANDED locator/extractor set (11 candidates'
worth of new acquire functions) to confirm neither accidentally became
supported as a side effect of unrelated additions.

**A fresh holdout, 8-12 candidates, mechanically drawn, frozen before
any candidate is inspected** (protocol below).

## Attack category 1: producer/artifact mutations

Against real acquired evidence for each of the 11 implemented
candidates (not a sample -- all 11, since "all known relevant
mutations detected" is a named success criterion):

1. **Rename an emitted key** -- already covered by each candidate's
   own existing stop-condition-8 test (`*_mutating_a_real_emitted_
   path_flips_to_finding`); re-cited here, not re-built.
2. **Delete an emitted key** (shrink `emitted_paths`) -- expectation:
   verdict never gets WORSE (a `Finding` never appears purely because
   evidence went missing; `Pass` stays `Pass` or the SAME `Finding`
   persists if the deleted key wasn't the one causing it). Real
   soundness angle: a broken/starved acquire function must never look
   MORE correct by accident.
3. **Add a genuinely new, unaccepted key** (append rather than
   replace) -- expectation: `Finding`. A different code path from
   "rename" (append vs. in-place replace); tests whether
   `compare_config_contract`'s own iteration-order-dependent "first
   miss" behavior holds regardless of where in the list the bad entry
   lands.
4. **Change nesting shape** -- take a real scalar leaf and wrap it in a
   synthetic nested object (or the reverse: flatten a real nested
   object into a bare scalar), re-run `flatten_structured_value`
   directly, confirm the resulting path(s) are NOT silently treated as
   equivalent to the original real leaf.
5. **Format-valid, contract-invalid near-miss substitution** -- a
   string that LOOKS like a real accepted path but differs in exactly
   the way this project's own boundary-checking discipline (the
   `-zone`/`-zones` lesson, K4c) exists to catch: a prefix, a
   dash-vs-underscore swap, a trailing-segment addition. Expectation:
   `Finding`, never silently accepted as "close enough."

## Attack category 2: binding attacks

Real scenario analysis (not necessarily new Rust tests for all of
these -- some are structural code-reading questions about whether the
CURRENT `B`-proof logic could be fooled, answered honestly even where
the honest answer is "yes, and here's the real gap"):

1. Artifact A is really generated, but the real process argv/env
   actually references artifact B (a different real derivation).
2. Two plausible artifact paths exist; which one does the current code
   actually prove is bound?
3. A wrapper script or env var points at a DIFFERENT real file than
   the one this project's own extraction read.
4. A real default path exists on the consumer side, but is actually
   overridden by a flag/env the current code doesn't check.
5. An ambiguous symlink/reference chain (`environment.etc`-style) with
   more than one real hop.

**A specific, real, already-suspected gap named before investigating**:
several `acquire_*` functions (`unpackerr`, `i2pd`, and others using
`DirectArgv`) check only that `ExecStart` CONTAINS the right FLAG NAME
(`--config=`, `--conf=`), never that the flag's own VALUE is the exact
same store path the `content`/`emitted_paths` evidence was extracted
from. For `StructuredValue`-sourced evidence (read directly from
`cfg.settings`, not from `readFile`-ing the rendered artifact), B's
real proof is closer to "the module's own internal wiring is trusted"
than "independently confirmed at the byte level." This round names it,
measures how many of the 11 implemented candidates share the exact
same shape, and reports it as a real, disclosed soundness finding --
fixing it is explicitly out of scope for this round.

## Attack category 3: consumer-contract attacks

Real scenario analysis against a curated subset (not all 11 -- pick
2-3 structurally different real cases: a `StructuredValue`-typed field
consumer like `unpackerr`, a partial-schema-shaped one like `akkoma`,
one hand-rolled-text one like `unbound`):

1. A wrong pinned consumer version (does the code even notice, or
   silently keep using stale D evidence).
2. A schema from a NEIGHBORING real version (does a real upstream
   rename/addition between two real tags get silently absorbed as "no
   change" by this project's own bounded extractors).
3. A genuinely partial schema (does the code ever claim completeness
   it doesn't have).
4. A freeform-map decode (real for `vault`, already excluded; check
   whether any of the 11 IMPLEMENTED candidates has an unnoticed
   freeform sub-region of its own).
5. **`akkoma` as the running adversarial example, exactly as named**:
   `description.exs` documents the admin-UI-settable surface, NOT the
   whole runtime config -- already demonstrated as a real executable
   `Finding` in C-E1.2b (`:instance.upload_dir`, `Pleroma.Repo`,
   `:joken`, `:tzdata`). This round's own job is confirming that
   result is STABLE (re-run the real test against frozen `984b366`,
   confirm it still reports the same real `Finding`) and asking
   whether any OTHER implemented candidate has an analogous
   "big, structured, but still not the whole contract" risk.
6. Collision after normalization -- covered by attack category 4
   below, not duplicated here.

## Attack category 4: normalization attacks -- a dedicated, permanent
adversarial suite

The `unbound`/`i2pd`/`akkoma` triple is already a real, checked
invariant test
(`normalization_is_per_consumer_never_a_default_unbound_vs_i2pd_vs_akkoma`).
This round's own job: (a) confirm no candidate's own D-extractor
applies a GENERIC normalization default without a real, cited
consumer-specific reason (audit all 11 `extract_*` functions' own doc
comments for this -- every one should name why bare-vs-qualified is
correct FOR THAT CONSUMER specifically, not "because it seemed
convenient"), (b) construct at least one additional synthetic
adversarial case per real extraction SHAPE (section-stripping text,
fully-qualified structured, fully-qualified INI) proving the collision
risk is caught, not just the 3 already-shipped real ones.

## Negative controls

Re-run `libinput`/`nohang`'s own real A-link evaluation (the exact real
`nix eval` that produced C-E1.1's own `NO_PRODUCER_PROOF` verdict for
each) against the current frozen `984b366` tree. Expectation: identical
real result -- still no real, standalone producer artifact for either.
If either flips to looking supported, that is this round's single
highest-priority finding (would mean some later locator/extractor
addition started guessing evidence it doesn't really have).

## Fresh holdout: population and draw

**Population**: real NixOS service modules under
`PhysShell/nixpkgs`'s `nixos/modules/services/` at the same pinned tree
`68740713a1d5904edf9ba92a998a522b1b6ce080` this entire project already
uses, that (a) generate a config artifact via a real `pkgs.formats.*`
call or an equivalent hand-rolled serializer (the same "generated
multi-key config" shape E1 originally flagged), (b) have a real
NixOS-test module-level test (matching every prior round's own
population requirement), (c) were NEVER touched by E1 (the original
40), C-E1.1 (the 14), or C-E1.2a/b (the 11 implemented + `vault` +
`libinput`/`nohang`).

**Draw**: a real, mechanical `gh api search/code` sweep for
`pkgs.formats.(toml|yaml|json|ini|elixirConf).generate` call sites
under `nixos/modules/services/` at the pinned tree, filtered against
the exclusion list above, then a seeded shuffle (seed = the same
`int(freeze_sha, 16)` convention E1 itself used, with `freeze_sha` =
`984b366`'s own full 40-char SHA) taking the first 10 candidates that
pass the population filter. The seed and the full candidate list
BEFORE the draw are recorded in the results file, not just the final
10 -- so the draw is independently reproducible and cannot be quietly
re-rolled if the first 10 look inconvenient.

## Per-holdout-candidate investigation

Same A/B/C/D structure as C-E1.1's own protocol, since these are
genuinely new candidates: does a real Nix producer generate the exact
artifact (A), is it really bound to the consumer (B), is the exact
pinned consumer reachable (C), are accepted keys really extractable in
a bounded way (D) -- fit = A&&B&&C&&D. Verdict: `SUPPORTED` /
`NO_PRODUCER_PROOF` / `NO_ARTIFACT_BINDING` / `CONSUMER_UNRESOLVED` /
`CONTRACT_UNEXTRACTABLE`, matching C-E1.1's own reason-code vocabulary
exactly (no new vocabulary invented for this round). **No
implementation for any holdout candidate in this round** -- this is
support/fit research only, the same "measure honestly, decide after"
discipline C-E1.1 itself followed before C-E1.2a ever started.

## Metrics (recorded, not just narrated)

Per attack category and overall: `false PASS`, `false FINDING`, `true
PASS`, `true FINDING`, `INCONCLUSIVE`, `TOOL_ERROR`. Plus: mutation
detection rate (detected / total relevant mutations run), negative-
control preservation (both still unsupported: yes/no), fresh-holdout
support rate (SUPPORTED / 10).

## Success criteria, fixed before any attack is run

- `false PASS` = 0
- `false FINDING` = 0
- `TOOL_ERROR` = 0
- All known-relevant mutations (category 1, types 1/3/4/5 -- type 2 is
  a non-worsening check, not a detection check) are detected.
- `libinput`/`nohang` both stay unsupported.
- The `unbound`/`i2pd`/`akkoma` normalization triple's own behavior is
  unchanged.
- The fresh holdout's own investigation surfaces zero app-specific
  semantic branching temptation (a candidate that would ONLY fit with
  an `if app == "X"` is recorded honestly as its own real reason code,
  not forced).

**Coverage is explicitly secondary.** A fresh holdout landing at, say,
5/10 `SUPPORTED` with zero false PASS/FINDING is a GOOD result for this
round's own purpose. A binding-attack or consumer-contract-attack
category that surfaces a real, disclosed gap (matching the
`ExecStart`-flag-value gap already named above) is also a legitimate,
useful result -- not a failure of the round, as long as it's reported
honestly and not quietly patched around mid-audit.

## Decision, fixed before any attack is run

```
if false PASS == 0 AND false FINDING == 0 AND TOOL_ERROR == 0
   AND all category-1 mutations detected
   AND libinput/nohang preserved
   AND normalization triple preserved:
      -> GeneratedConfigArtifact v1 is declared FROZEN
      -> CDC treated as a first reusable subsystem, not an
         experimental pile of adapters
      -> only then consider productization (CLI wiring, an advisory
         GitHub check, a PR-local diff mode, a nixpkgs-review
         companion) -- explicitly NOT part of this round
else:
      -> v1 is NOT frozen
      -> every real gap found (binding-attack gaps, consumer-contract
         gaps, any false PASS/FINDING) becomes its own named,
         separately-scoped follow-up item
      -> the freeze decision is revisited only after those are
         resolved or explicitly accepted as known, documented
         limitations
```
