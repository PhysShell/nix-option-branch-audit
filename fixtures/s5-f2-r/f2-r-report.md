# S5-F2-R: full frozen-corpus acceptance replay of `c647d1b`

This document is a validation artifact only. It does not rewrite,
supersede, or reinterpret any prior historical result. Historical S5,
S5-F1, S5-F1-R, S5-F1B, S5-F1B-R, S5-F1C, S5-F1D (`25c5b54`), S5-F1D-R
(`58b28b2`, PASS), and S5-F2 (`c647d1b`/`399a2d5`) all remain exactly as
committed. This round is validation only -- no implementation changes
were made anywhere in this round.

See `fixtures/s5-f2/investigation.md` and `fixtures/s5-f2/f2-report.md`
for the S5-F2 root-cause trace and implementation report this round
accepts or rejects. See `fixtures/s5-f2-r/f2-r-summary.json` for the
full machine-readable summary this document narrates.

## Final verdict

**PASS.**

`c647d1b` satisfies the frozen S5 acceptance corpus. Its effect on the
full 180-PR applicable corpus is confined to exactly one PR --
`#475112` (cgit), the sole intended target of S5-F2 -- with zero
collateral change anywhere else: zero discovery deltas, zero new
collisions, zero new fatal/error classes, all F1D anchors and
guacamole byte-identical to the accepted F1D-R baseline.

## Candidate provenance

Built from a genuinely clean checkout: fresh `git clone` to
`/home/tandem/f2r-clean-checkout/repo`, `git checkout
c647d1b33fc1e328cd5f75037770e404a40edcf8`, confirmed `git status
--short` empty and `git rev-parse HEAD` matches exactly.
`CARGO_TARGET_DIR=/home/tandem/f2r-clean-checkout/target rustup run
nightly-2026-08-21-x86_64-unknown-linux-gnu cargo build --release`,
rustc `1.100.0-nightly (8925ea358 2026-08-20)`. Binary SHA-256
`4d16b399e072e48edd5161133a306925646f568a037f07807a7abbe2758a1048`. No
`cp`-based restoration was used anywhere in this round.

**Explicitly verified this is the implementation commit, not the
documentation commit**: `git diff c647d1b 399a2d5 -- src/` is empty --
`399a2d5` (S5-F2's own investigation/report commit) touches only
`fixtures/s5-f2/*.md`, never `src/`. The replayed binary is provably
built from `c647d1b` alone.

## Coverage: 369/369, derived and verified

Re-derived independently from the frozen ledger
(`fixtures/s5-live-pr-shadow/adjudication-ledger.jsonl`): 369 total
`pr_summary` records, 180 applicable / 189 non-applicable. The
re-derived applicable PR set was confirmed, independently, to be
byte-for-byte IDENTICAL to `fixtures/s5-f1d-r/applicable-manifest.json`'s
own PR set (re-verified directly by the reviewing session itself, not
only the executing fork's own claim: `applicable-manifest.json` in
this directory is byte-identical to F1D-R's own file). All 180
applicable PRs evaluated. **0 `window_evaluation_incomplete`, 0 fetch
failures.**

## Comparison methodology: F1D-R is the baseline, not raw historical

Per this round's own mandate, the primary regression baseline is the
accepted F1D candidate (`25c5b54`), using F1D-R's own already-committed
replay artifacts (`fixtures/s5-f1d-r/replay/**`) directly -- NOT the
raw historical v0.4.5 S5 output used by every prior `-R` round. This is
deliberate: F1D's own intentional provenance/representation changes
were already accepted by `58b28b2`; re-litigating them here would be
exactly the kind of scope creep this project's own history has
repeatedly guarded against. `fixtures/s5-f2-r/compare-replay.py` reads
`fixtures/s5-f1d-r/replay/<cohort>/<pr>/{raw.json,check-base.json,check-head.json}`
as the comparison target for every PR, tagging each row
`"baseline": "f1d-r"`.

## Comparison result: the cleanest S5 replay round to date

**179 identical, 1 changed vs. the F1D-R baseline.** Changed PR:
`#475112` (cgit) -- exactly, and only, the PR this round exists to
validate. No other PR in the entire 180-PR applicable corpus shows any
delta of any kind (not `raw.json`, not `check-base.json`, not
`check-head.json`) relative to the accepted F1D baseline. Re-run
independently by the reviewing session itself against the already-
written replay artifacts (not merely re-trusted from the executing
fork's own claim) -- byte-identical result both times.

## Discovery invariance: proven, not assumed

F2 never touches `scan_options` -- only `path_matches_prefix`,
`evaluate_predicate_witness`, and `run_target`'s own new
`allow_instance_key` computation. `discovered_options` should therefore
be provably untouched everywhere. Verified mechanically across all 360
`check-{base,head}.json` files (180 PRs x 2 sides): **360/360
identical, 0 changed.** This is a stronger invariant than F1D-R itself
had available (F1D intentionally changed `discovered_options`'s own
representation) -- F2's change surface is provably confined to witness
matching alone, corpus-wide, not merely argued from reading the diff.

## The cgit anchor, from the live replay itself

Head-side watched predicate `gitHttpBackend.enable` (kind
`Truthy("optionalAttrs")`, source `lib.optionalAttrs
cfg.gitHttpBackend.enable {`, span
`nixos/modules/services/networking/cgit.nix:308`):

- **F1D-R baseline**: `OBA001`, `predicate_attempts[0].witnessed =
  false`, no evidence.
- **F2-R candidate**: `PASS`, `predicate_attempts[0].witnessed = true`,
  evidence `path =
  ["services","cgit","no-git-http-backend.localhost","gitHttpBackend","enable"]`,
  `value_source = "false"`, `instance = "server"`, `span.line = 58` --
  the real, deliberate assignment from `nixos/tests/cgit.nix`.
- **audit-diff's own independent confirmation**: baseline
  `oba_verdict_transitions: {"option_not_found->oba001": 1}` -> candidate
  `{"option_not_found->pass": 1}`.
- **Base side**: `OptionNotFound`, unaffected either way (the base
  commit predates `gitHttpBackend` entirely).

This matches S5-F2's own accepted implementation report and the real
PR #475112 historical adjudication record exactly -- confirmed here
from the frozen full replay itself, not merely the synthetic fixture
or the implementation round's own unit tests.

## Structurally eligible target manifest (the mandate's own "blast radius" requirement)

For every one of the 180 applicable PRs' own targets,
`option_prefix_is_instance_keyed_submodule` was evaluated using a
throwaway diagnostic binary: a clean `c647d1b` checkout plus one
strictly-additive, env-var-gated `eprintln!` line that calls the REAL
function verbatim (reviewed directly by the reviewing session --
`git diff` in the diagnostic checkout shows exactly 6 added lines, no
behavior change to any existing subcommand's actual output). This
diagnostic binary was never used to produce any of this round's actual
replay/comparison/anchor results -- those all come from the pure,
unmodified `c647d1b` binary built in the "Candidate provenance" section
above.

**182 target records scanned, 12 eligible**:

| PR | option_prefix | watch | container form |
|---|---|---|---|
| 431289 | `services.prosody.muc` | `moderation` | `listOf(submodule ...)` (hand-verified) |
| 353770 | `services.syncthing.settings.folders` | `ignorePatterns` | attrsOf-shaped (inferred) |
| 460724 | `services.btrbk.sshAccess` | `extraArgs` | `listOf(submodule ...)` (hand-verified) |
| 506644/508427/428153/427260 | `services.drupal.sites` | (4 distinct leaves) | attrsOf-shaped (inferred) |
| 395982 | `services.openvpn.servers` | `testserver.authUserPass` | `attrsOf(submodule ...)` (hand-verified) |
| 456004 | `services.snapserver.settings` | `stream.source` | attrsOf-shaped settings-format type (hand-verified) |
| 397967 | `services.fedimintd` | `api_ws.openFirewall` | `attrsOf(submodule ...)` (hand-verified) |
| **475112** | **`services.cgit`** | **`gitHttpBackend.enable`** | **`attrsOf(submodule ...)` (hand-verified)** |
| 386406 | `services.frp.instances` | `role` | `attrsOf(submodule ...)` (hand-verified) |

The sole changed PR (475112) falls entirely inside this eligible set,
with a hand-verified `attrsOf(submodule ...)` container -- no changed
PR falls outside it. The mandate's own "immediately suspicious"
condition does not occur.

**A genuinely informative non-match, hand-traced**: PR 386406 (frp) is
structurally eligible (`services.frp.instances` really is
`attrsOf(submodule ...)`) but does NOT change, because the module uses
`lib.mkRenamedOptionModule ["services" "frp" "role"] ["services" "frp"
"instances" "" "role"]` -- a Nix module-system-level rename, resolved
only by real Nix evaluation, never by `oba`'s own purely-syntactic test
walker. The real test file's own literal source writes
`services.frp.role = "server";` (3 segments) directly, which matches
neither the old exact-length rule (needs 4: `services,frp,instances,role`)
nor F2's own new instance-key-tolerant rule (needs 5). Both F1D-R and
F2-R therefore produce the identical (unmatched) result for this PR,
for the identical reason -- confirmed directly by reading the real
module and test source, not merely inferred from "it didn't change."
This is the same class of pre-existing, disclosed, out-of-scope
limitation as the already-known `imports =` reachability gap (angrr) --
`oba` has never resolved any module-system-level indirection, only
literal, directly-written attrpaths. Not a new backlog item; folded
into the existing disclosed "genuinely dynamic/computed attribute
access... remains explicitly out of scope" limitation from S5-F1D.

## Cross-instance ambiguity scan (mandatory, mandate item 9)

Scope: the 12 structurally eligible targets above (a target that
cannot admit an instance key cannot exhibit this ambiguity by
construction) -- not merely the one changed PR.

Method: the executing fork's own text-level scan over live-fetched real
test source for the instance-key-tolerant shape specifically. The
reviewing session independently, directly re-verified (not merely
re-trusted) the 7 most structurally complex or ambiguous cases by its
own `gh api` source reads: cgit #475112, openvpn #395982, fedimintd
#397967, frp #386406, snapserver #456004, btrbk #460724, prosody
#431289.

**Result: `cross_instance_outcome_ambiguity_not_observed_in_frozen_S5`.**
Every eligible target has either zero real assignments to its own
watched suffix, or (cgit only) exactly one real assignment among
multiple concrete instances -- the other instances demonstrably touch a
different, sibling leaf (`checkExportOkFiles`, not
`gitHttpBackend.enable`), confirmed by direct source read. No target
anywhere in the frozen corpus shows two or more concrete instances
assigning genuinely differing values to the same watched predicate. The
architectural limitation itself (documented, not solved, by S5-F2's own
hostile control 2) remains open regardless -- it simply does not
manifest anywhere in this specific 180-PR corpus.

**A new, narrower, disclosed caveat surfaced by this round's own hand
verification, not exercised anywhere in this corpus**:
`option_prefix_is_instance_keyed_submodule` treats `attrsOf` and
`listOf` identically -- it only checks that `attrsOf`/`listOf` appears
anywhere in the type expression, never distinguishing the two, and
never requiring the wrapped element to be a `submodule` specifically.
For an `attrsOf(submodule ...)`, a real Nix instance genuinely IS
addressed by a literal attribute name, so a literal instance-key
segment in a real test assignment is a faithful signal. For a
`listOf(submodule ...)` -- independently confirmed by hand for both
`listOf`-typed eligible targets in this corpus, prosody's `muc`
(`with types; listOf (submodule {...})`) and btrbk's `sshAccess`
(`with types; listOf (submodule {...})`) -- a real Nix list element has
NO literal name at all; list membership is positional. A literal
single-segment "instance key" can therefore never legitimately arise
between `option_prefix` and a watched suffix for a genuinely valid Nix
config against a `listOf`-typed option. If a test file's own literal
source ever happened to contain a syntactically well-formed but
semantically nonsensical attrpath of that shape, `oba`'s own purely-
syntactic walker could not distinguish it from a real one, and the
current gate would accept it as instance-key evidence. **This is not
exercised anywhere in the frozen 180-PR corpus** -- both real
`listOf`-typed eligible targets show zero matching candidates of any
shape -- so it introduces no unsound witness in this replay and does
not block this round's PASS. It is recorded here as its own disclosed
item, distinct from the already-known cross-instance-ambiguity
limitation, because it is a narrower, `listOf`-specific structural gap
the current gate does not encode, worth tracking separately as the
corpus grows rather than silently treating `attrsOf`/`listOf` as
equivalent going forward.

## Value/default semantics of the one new witness

`gitHttpBackend.enable`'s own declared default is `true`; the real test
instance assigns `false` (a genuine flip, not a default-equivalent
no-op); `eval_pred` resolves the predicate's own `default_outcome` as
`true` and the test-side outcome as `false` -- `d != t2`, a real,
demonstrated transition, matching `evaluate_predicate_witness`'s own
existing transition-detection logic exactly (unchanged by F2). This is
not "a matching path exists" being mistaken for "a witness exists" --
the underlying value-transition semantics that decide witnessing are
the same, unmodified logic as every prior predicate; only the
ASSIGNMENT-MATCHING step (whether this real assignment is even
considered a candidate at all) changed.

## Predicate-form distribution

The corpus's own single delta uses `PredicateKind::Truthy("optionalAttrs")`
(an H1 form). No other predicate form appears among this round's own
deltas -- there is only one. Predicate-form-agnosticism of the fix
itself (H1 `Truthy`/`NegTruthy`, H2 `Eq`/`NotEq` all sharing the
identical `allow_instance_key` value at the identical call sites) was
already independently confirmed at the S5-F2 implementation-acceptance
stage via two dedicated synthetic tests
(`negated_predicate_form_on_instance_keyed_submodule_is_also_witnessed`,
`equality_predicate_form_on_instance_keyed_submodule_is_also_witnessed`)
-- not re-derived here, since the frozen corpus itself happens to
exercise only the `Truthy` form for its one real delta.

## F1D anchors -- all byte-identical to the F1D-R baseline

Diffed per-file (`raw.json`, `check-base.json`, `check-head.json`)
against F1D-R's own committed replay output:

- **Angrr `#471312`**: byte-identical.
- **Seven former F1 regressions** (`#431289 #506644 #508427 #440660
  #428153 #397967 #427260`): byte-identical, all seven.
- **Rspamd `#484133`**: byte-identical.
- **Tayga `#432528`**: byte-identical.
- **K3s `#374017`**: byte-identical.
- **Kimai wildcard**: unaffected -- the targeted existing regression
  test still passes; `allow_instance_key`'s own gate requires a
  concrete `option_prefix` resolving to a real `OptionDecl`, which a
  wildcarded prefix never has.

## Guacamole -- deliberately still unfixed, confirmed unaffected

`#462487`: byte-identical to the F1D-R baseline across all three files.
Per this round's own mandate, historical defect status after F2 is now:
angrr fixed/accepted by F1D, cgit candidate-fixed by F2 (this round
confirms it), guacamole deliberately still unfixed. No unrelated
guacamole difference was accidentally classified as a bonus fix --
there is none; the diff is empty.

## Eight collision PRs -- full-corpus rescan

Re-run against the complete F2-R replay output (not a targeted
spot-check): **exactly 1 finding in the entire 180-PR corpus** --
wstunnel `#415326`, `pre_existing_in_f1d_r_baseline: true`, identical
to F1D-R's own disclosed, unaddressed wildcard-prefix state. The seven
previously-resolved collisions (`429967 494314 431289 440660 260551
463443 374017`) remain resolved. No new collision introduced anywhere.
Re-run independently by the reviewing session itself -- byte-identical
result both times.

## Fatal/error behavior

Verdict-kind counts, baseline vs. candidate:

| kind | F1D-R baseline | F2-R candidate |
|---|---|---|
| OptionNotFound | 208 | 208 |
| PredicateNotFound | 108 | 108 |
| TestConfigUnresolved | 21 | 21 |
| PASS | 7 | **8** |
| OBA001 | 17 | **16** |
| TestValueUnresolved | 2 | 2 |
| DefaultUnresolved | 1 | 1 |

The only change anywhere is exactly the cgit transition. No
verdict-kind class appears in one side only. **0 parse errors, both
sides, all 180 PRs.**

## Reproducibility

- Applicable manifest: independently re-derived from the raw ledger,
  confirmed byte-identical to `fixtures/s5-f1d-r/applicable-manifest.json`.
- `compare-replay.py`: re-run independently by the reviewing session
  itself against the already-written replay artifacts -- byte-identical
  output.
- `scan-leaf-collisions.py`: re-run independently by the reviewing
  session itself -- byte-identical output.
- Candidate binary provenance independently verified (clean checkout,
  correct commit, no `cp`-restore).

## Full repository test suite / CI

Not re-run in this round -- S5-F2-R is a frozen-corpus replay of an
already-accepted, already-CI-green implementation commit (`c647d1b`,
367/367 at acceptance time, CI green on push). No implementation change
occurred in this round to require re-running it.

## Files added this round

All under `fixtures/s5-f2-r/`: `candidate-provenance.json`,
`applicable-manifest.json`, `derive-applicable-manifest.py`,
`run-replay.py`, `replay/` (180 PR directories),
`compare-replay.py`, `comparison-report.jsonl`,
`discovery-invariance-check.{py,json}`, `build-eligibility-manifest.py`,
`eligibility-manifest.json`, `build-witness-deltas.py`,
`witness-deltas.json`, `build-cross-instance-candidates.py`,
`cross-instance-candidates.json`, `anchors-check.{py,json}`,
`fatal-error-tabulation.{py,json}`, `scan-leaf-collisions.py`,
`leaf-collision-scan.json`, `classification-notes.json`,
`classify-deltas.py`, `delta-classification.json`, this file, and
`f2-r-summary.json`. `fixtures/s5-f1d-r/` and every other historical
fixture directory untouched.

## Unresolved limitations (explicitly stated, not silently dropped)

- Cross-instance ambiguity within one test node remains
  source-order-dependent (S5-F2's own documented limitation) --
  unexercised in this corpus, architecturally still open.
- **New this round**: `listOf`-typed instance-key tolerance is
  structurally unverified against real Nix list semantics (no literal
  instance key can ever legitimately exist for a `listOf`), though
  unexercised anywhere in the frozen corpus -- see the dedicated
  section above.
- The `imports=`-only reachability gap (angrr's `commonPolicyOptions`)
  and the flat-dotted declaration-scanner limitation (found while
  building S5-F2's own synthetic fixtures) remain open, unaddressed,
  unaffected by this round, per the user's own explicit deferral to a
  separate backlog item.
- The frigate/nvidia-container-toolkit `walk_merge_operands` wrap gap
  remains open, unaddressed, unaffected by this round.
- The wstunnel `#415326` wildcard-prefix collision question remains
  open, unaddressed, unaffected by this round.
- Guacamole `#462487` remains deliberately unfixed.
- `run_target`'s own first-match `.find()` policy under cross-instance
  or wildcard ambiguity remains unsolved in general (documented, not
  redesigned).

## Scope discipline

No implementation changes anywhere in this round (the one diagnostic
binary used to build the eligibility manifest was a throwaway, never
used to produce any accepted result, never committed). No guacamole
fix. No wildcard/`.find()` redesign. No version bump, no release. No
historical/frozen artifact touched or reinterpreted.

**STOP.** No further implementation work, release, or additional full
replay without a separate, explicit GO.
