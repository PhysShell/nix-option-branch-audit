# S5-F3-R: full frozen-corpus acceptance replay of `05d4a90`

This document is a validation artifact only. It does not rewrite,
supersede, or reinterpret any prior historical result. Historical S5,
S5-F1D-R (`58b28b2`, PASS), S5-F2-R (`c04c5af`, PASS), and S5-F3
(`05d4a90`/`54fa3a8`) all remain exactly as committed. This round is
validation only -- no implementation changes were made anywhere in
this round.

See `fixtures/s5-f3/investigation.md` and `fixtures/s5-f3/f3-report.md`
for the S5-F3 root-cause trace and implementation report this round
accepts or rejects. See `f3-r-summary.json` for the full
machine-readable summary this document narrates.

## Final verdict

**PASS**, with one prominently disclosed, non-blocking finding about
`destination_confirmed` for the guacamole anchor specifically -- read
"The guacamole destination-confirmation finding" section below before
treating this PASS as unconditional.

`05d4a90` satisfies the frozen S5 acceptance corpus. Its effect on the
full 180-PR applicable corpus is **5 changed PRs**, not merely
guacamole -- the implementation intentionally supports a structural
class broader than one PR, and this round independently, individually
verified all 5 against real, live-fetched source, not merely trusted
that "same helper name" implies correctness.

## Candidate provenance

Built from a genuinely clean checkout: `/home/tandem/f3r-clean-checkout/repo`,
`git checkout 05d4a900f9f10cebfa42981eea1c88d554946dbb`, confirmed
`git status --short` empty and `HEAD` matches. Rustc
`1.100.0-nightly (8925ea358 2026-08-20)`. Binary SHA-256
`30dc21d1b229e08c438e9d2d24e43fa26160b7675c146e75f70e4f333f140558`. No
`cp`-based restoration.

**Explicitly verified this is the implementation commit, not the
documentation commit**: `git diff 05d4a90 54fa3a8 -- src/` is empty.

## Coverage: 369/369, derived and verified

369 total ledger records, 180 applicable / 189 non-applicable,
independently re-derived and confirmed byte-identical to
`fixtures/s5-f2-r/applicable-manifest.json`'s own set (re-verified
directly by the reviewing session, not only the executing fork's own
claim). All 180 evaluated. **0 `window_evaluation_incomplete`, 0 fetch
failures.**

## Comparison methodology: F2-R is the baseline, not raw historical

Per this round's own mandate, comparison is against F2-R's own
already-committed replay output (`fixtures/s5-f2-r/replay/**`), never
raw historical v0.4.5 -- avoiding re-litigating F1D/F2's own already-
accepted changes.

## Comparison result: 5 changed PRs, not just guacamole

**175 identical, 5 changed**: `#392716` (cyrus-imap), `#410856`
(vmalert), `#462487` (guacamole), `#490920` (traefik), `#510900`
(sshd, a DIFFERENT PR from the `#509507` banner-removal control).
Independently re-run by the reviewing session itself against the
already-written replay artifacts -- byte-identical both times.

This is expected, not a red flag: S5-F3's own implementation report
already stated explicitly that the fix "widens witness behavior for a
class structurally broader than guacamole" -- any watched option whose
complete path exactly matches a statically-literal, unambiguous
`mkRenamedOptionModule`/`mkRenamedOptionModuleWith` edge's own
`from_path`, anywhere in the corpus. The mandate's own item 25
explicitly forbids assuming these are correct "merely because they
share a helper name" -- every one of the 4 non-guacamole cases below
was individually traced by the reviewing session against real,
live-fetched source, not accepted on the executing fork's report
alone.

## Discovery invariance: proven, not assumed

F3 never touches `scan_options`. Verified mechanically across all 360
`check-{base,head}.json` files: **360/360 identical, 0 changed** --
same invariant F2-R itself established, still holding.

## The guacamole anchor, from the live replay itself

Head-side watched option `services.guacamole-server.logbackXml`:

- **F2-R baseline**: `OptionNotFound`.
- **F3-R candidate**: `OptionRelocated`, `to =
  ["services","guacamole-client","logbackXml"]`, `helper_form =
  "mkRenamedOptionModule"`, `migration_source_file =
  "nixos/modules/services/web-apps/guacamole-server.nix"`,
  `migration_span.line = 12` -- all independently re-verified by the
  reviewing session against the real, live-fetched head source.
- **`audit-diff`'s own structured output** (from this round's own
  replay, `raw.json` for `#462487`):
  ```json
  "oba_verdict_transitions": {"oba001->option_relocated": 1},
  "notable": [{"bucket": "finding_became_inconclusive",
    "code": "OBA-RELOCATED", "transition_origin": "verdict_changed", ...}]
  ```
  This satisfies the mandate's own item 7 directly: a machine consumer
  can distinguish this from plain `"...->option_not_found"` by reading
  the `oba_verdict_transitions` map key or the `code` field alone --
  never by parsing prose. The `finding_became_inconclusive` bucket's
  own COUNT is unaffected (per F3's own deliberate, disclosed design),
  but the per-result structured evidence is unambiguous.

## The guacamole destination-confirmation finding (read this before trusting an unconditional PASS)

**`destination_confirmed = false` for guacamole in THIS frozen
replay** -- not `true`, contrary to the mandate's own item 6 literal
phrasing ("Verify directly from replay output: ... destination_confirmed
= true ... actual destination declaration found cross-file"). This is
disclosed prominently here, not buried in an appendix.

**Root cause, independently verified**: the historical, frozen
`targets.toml` for PR #462487
(`fixtures/s5-live-pr-shadow/adjudication/s5b-batch1/462487/targets.toml`,
read directly, unmodified) lists ONLY `guacamole-server.nix` and its
own test file -- it was captured before cross-file destination
confirmation existed as a concept, so it never had reason to also list
`guacamole-client.nix`. `run-replay.py`'s own pre-existing, unmodified
fetch logic (`paths_needed = {module, test}`, established three rounds
ago at S5-F1-R, reused verbatim this round per its own explicit
"no implementation modifications" constraint) therefore never fetches
the destination file into this PR's own replay root at all. Confirmed
directly by the reviewing session: `fixtures/s5-f3-r/sources/B/462487/{base,head}/`
contains only `guacamole-server.nix`, nothing else.

**Is this a false claim?** No. `locate_migration_destination` never
asserts `true` when the destination file genuinely isn't present in
the given root -- this is a truthful `unconfirmed` state, explicitly
sanctioned by the mandate's own item 16 ("F3 is allowed to represent an
unconfirmed destination if the rename edge itself is proven... But
`destination_confirmed=true` must never be asserted without a real
exact-path declaration"). The relocation classification itself --
`from_path`, `to_path`, `helper_form`, source span, the entire primary
claim item 6 asks for -- is fully, correctly demonstrated. Only the
separately-gated, additional `destination_confirmed` sub-claim is
affected.

**Is this an F3 implementation defect?** No -- it is a limitation of
the REPLAY METHODOLOGY's own pre-existing, unmodified per-PR
sparse-fetch convention (unrelated to, and unchanged by, this round),
not of the migration-detection mechanism itself.

**Independent proof the cross-file mechanism genuinely works**:

1. S5-F3's own committed real fixture
   (`fixtures/synthetic/f3-guacamole-logbackxml-relocation/`), built
   deliberately to include BOTH files, shows `destination_confirmed:
   true` for the exact same rename edge.
2. A dedicated hostile control (`cross_file_destination_is_confirmed`
   in `tests/f3_migration_hostile.rs`) proves the mechanism
   synthetically, independent of the real fixture.
3. **PR #510900** (below) independently proves destination confirmation
   succeeds WITHIN this exact frozen replay's own sparse-root
   constraints, for a same-file case -- the mechanism is not merely
   theoretical.

**Coordinator's own recommendation**: does not block PASS. The
completion gate's own item 26 wording is "destination confirmation
claims are truthful" (satisfied in all 5 cases, without exception) --
not "destination confirmation must always succeed." Flagged here with
full prominence specifically so this judgment can be overridden if
seen differently.

## Four additional relocations, individually traced against real source

Per the mandate's own item 25 ("no 'same helper name, therefore
probably correct' classification"), every one of these was
independently verified by the reviewing session directly against
live-fetched source -- not accepted from the executing fork's own
report alone.

- **`#510900` (sshd, `moduliFile`)**: `services.openssh.moduliFile ->
  services.openssh.settings.ModuliFile`. Real edge confirmed at
  `sshd.nix:235`; real `ModuliFile = lib.mkOption {...};` confirmed at
  `sshd.nix:736`, SAME file. F2-R baseline: `OptionNotFound`. F3-R:
  `OptionRelocated`, `destination_confirmed: true` -- the ONE case in
  the entire corpus where destination confirmation actually succeeds
  within the replay's own sparse root.
- **`#410856` (vmalert, `enable`)**: `services.vmalert.enable ->
  services.vmalert.instances."".enable` (the empty string is a real,
  literal Nix list element -- the module's own "default instance"
  placeholder convention, not a wildcard). Real edge confirmed at
  `vmalert.nix:42`. F2-R baseline: base `PASS`, head `OptionNotFound`.
  F3-R: head `OptionRelocated`, `destination_confirmed: false` --
  traced to `locate_migration_destination`'s own parent/leaf split
  computing a parent (`[...,"instances",""]`) that is never itself a
  literal true-root declaration (the real leaf lives inside an
  `attrsOf(submodule)` instance, addressed at runtime, not by a literal
  source attrpath) -- an honest, safe `false`.
- **`#490920` (traefik, `staticConfigFile`)**: `services.traefik.staticConfigFile
  -> services.traefik.static.file` -- a PRE-EXISTING rename, already
  present at BASE (unrelated to this PR's own changes, which instead
  RE-introduce a direct `staticConfigFile` declaration at HEAD). Real
  edge and a real `static.file = mkOption {...};` declaration (same
  file, nested two levels inside a flat-dotted `options.services.traefik
  = {...}` block) both confirmed directly. F2-R baseline: base
  `OptionNotFound`. F3-R: **base** `OptionRelocated` (this is the one
  case where the delta is on the BASE side, not head -- expected:
  `analyze()`'s own post-processing pass, and gate-1's own fix, apply
  uniformly to whichever root `run_target` is called against),
  `destination_confirmed: false` -- traced to the same
  `locate_migration_destination`/nested-declaration boundary as
  vmalert, a different concrete manifestation.
- **`#392716` (cyrus-imap, `sslServerCert`)**: `services.cyrus-imap.sslServerCert
  -> services.cyrus-imap.imapdSettings.tls_server_cert`. Real edge
  confirmed at `cyrus-imap.nix:76-78`. F2-R baseline: base
  `PredicateNotFound`, head `OptionNotFound`. F3-R: head
  `OptionRelocated`, `destination_confirmed: false` -- traced to
  `imapdSettings` being a freeform-typed settings submodule with no
  literal `mkOption` declaration for `tls_server_cert` anywhere -- a
  correct, no-caveat `false` (there is genuinely nothing to find).

All 5 have exact, verified `from_path` matches to their own target's
complete watched path; none show any sign of unsound classification.

## Migration-edge manifest: the actual blast radius

Built via a throwaway diagnostic binary (clean `05d4a90` checkout + one
strictly-additive, env-gated `eprintln!` reusing the real
`scan_migrations` output verbatim, never used for any accepted replay
result): **584 total edges** across the corpus -- `mkRenamedOptionModule`:
434, `mkRemovedOptionModule`: 150, **`mkRenamedOptionModuleWith`: 0**.
`matches_watched_path=true`: **16** (5 `Renamed` -> all 5 became
`OptionRelocated`; 11 `Removed` -> all 11 confirmed byte-identical to
F2-R, see below) -- a self-consistent cross-check, independently
reproduced by the reviewing session directly from the manifest file.
`ambiguity_count>1`: **0** anywhere in the corpus.

## Ambiguous-rename scan (mandate item 11)

**0** `(pr, side, from_path)` groups with more than one edge anywhere
in the entire 180-PR corpus. `ambiguous_rename_affecting_watched_target_not_observed:
true`. No case anywhere required, or exercised, the fail-closed
ambiguity-handling path -- it remains a real, tested, but so far
unexercised-in-corpus safety property (proven via S5-F3's own
synthetic hostile control instead).

## Dynamic/unresolved migration-path scan (mandate item 12)

A text-level regex scan across all 180 PRs' base+head module source
found 633 raw helper-name occurrences; 49 have no corresponding
manifest edge. The reviewing session hand-checked every distinct
pattern among these 49: the large majority are `inherit (lib) ...
mkRemovedOptionModule ...;` import-list lines (a false positive of the
naive line-regex itself, confirmed by checking the real call sites
elsewhere in the same files DO have matching manifest edges). **3
genuinely real, corpus-sourced dynamic-path cases, correctly rejected**:
wstunnel `#415326` (`lib.map (option: lib.mkRemovedOptionModule [
option ] "...") [...]`, `option` a lambda-bound identifier), syncthing
`#422094`/`#353770` (`map (o: mkRenamedOptionModule [...o] [...o])
[...]`, same shape), and k3s `#374017` (`mkRemovedOptionModule ([...]
++ config) instruction`, a computed list via `++`). None of these 4 PRs
produced any manifest edge, and none appear in the 5-PR changed list --
**confirmed: no guessed/computed path ever became `OptionRelocated`
anywhere in the frozen corpus.** These are valuable, real, additional
fail-closed positive controls beyond S5-F3's own synthetic hostile
tests, independently spot-verified by the reviewing session against the
real source for all 4.

## Hard-removal controls -- all correct, corpus-wide

`#509507` (sshd `banner`) and `#466806` (gollum `local-time`): both
byte-identical to F2-R baseline. Beyond these two mandatory controls,
the migration-edge manifest identifies **11 total** real watched
targets across the corpus with a matching `Removed` edge (including
angrr `#471312`, already an F1D anchor) -- **all 11 confirmed
byte-identical to F2-R**, independently verified by the reviewing
session. Zero hard removals reclassified anywhere in the entire frozen
corpus, not merely the two named controls.

## Plain-disappearance status

`208 -> 203` `OptionNotFound` verdicts (208 minus the 5 reclassified =
203, confirmed via `fatal-error-tabulation.json`'s own verdict-kind
counts) -- F3 does not turn absence itself into migration evidence
anywhere; only the 5 cases with a real, statically-proven, unambiguous
`Renamed` edge change.

## Cross-file relocation

Guacamole IS structurally a cross-file relocation (source in
`guacamole-server.nix`, real destination in `guacamole-client.nix`),
but -- per "The guacamole destination-confirmation finding" above --
the replay's own sparse root never contains the destination file, so a
SUCCESSFUL cross-file confirmation is not actually exercised anywhere
within this specific frozen replay. The mechanism's own cross-file
capability is proven via the committed implementation-round fixture and
its dedicated hostile control instead -- both deliberately built to
include the destination file, unlike the historical `targets.toml`.

## `mkRenamedOptionModuleWith` corpus coverage

**`mkRenamedOptionModuleWith_not_exercised_by_frozen_watched_targets`**
-- 0 edges of this helper form anywhere in the frozen corpus, confirmed
directly from the migration-edge manifest. Support for this form
remains test-backed (S5-F3's own synthetic hostile control,
`mk_renamed_option_module_with_is_supported`) rather than corpus-backed
-- exactly the outcome the mandate's own item 18 anticipated as
acceptable.

## F1D/F2 anchors -- all byte-identical to the F2-R baseline

Angrr `#471312`, the 7 former F1 regressions (`#431289 #506644 #508427
#440660 #428153 #397967 #427260`), rspamd `#484133`, tayga `#432528`,
k3s `#374017`, cgit `#475112` (PASS with the real instance-key witness,
unaffected) -- all byte-identical across `raw.json`/`check-base.json`/
`check-head.json`. Kimai wildcard: unaffected (unrelated to migration
scanning entirely). Collision corpus: same state as F2-R -- a
full-corpus rescan confirms only the pre-existing wstunnel `#415326`
wildcard case remains.

## F2-specific open caveats and other disclosed limitations -- confirmed untouched

Cross-instance `.find()`-ordering behavior, the `listOf` instance-key
caveat, the `imports=`-only reachability gap, the frigate/nvidia
`walk_merge_operands` limitation, and the wstunnel wildcard collision
are all confirmed untouched, unaffected, not fixed in this round. The
flat-dotted `scan_options`-reuse limitation is confirmed untouched too
-- and is directly RESPONSIBLE for 2 of this round's own
`destination_confirmed=false` cases (vmalert, traefik), which is itself
evidence the limitation is real and load-bearing, not merely a
disclosed hypothetical.

## Fatal/error behavior

| kind | F2-R baseline | F3-R candidate |
|---|---|---|
| OptionNotFound | 208 | **203** |
| OptionRelocated | -- | **5** |
| PredicateNotFound | 108 | 108 |
| TestConfigUnresolved | 21 | 21 |
| PASS | 8 | 8 |
| OBA001 | 16 | 16 |
| TestValueUnresolved | 2 | 2 |
| DefaultUnresolved | 1 | 1 |

Fully self-consistent with the 5 reclassifications; no other class
appears on only one side. **0 parse errors, both sides, all 180 PRs.**

## Reproducibility

Applicable manifest independently re-derived, confirmed byte-identical
to F2-R's own. `compare-replay.py` and `scan-leaf-collisions.py` each
re-run a second time, INDEPENDENTLY, by the reviewing session itself
(not merely re-trusted from the executing fork's own claim) --
byte-identical output both times.

## Full repository test suite / CI

Not re-run in this round -- S5-F3-R is a frozen-corpus replay of an
already-accepted, already-CI-green implementation commit (`05d4a90`,
384/384 at acceptance time, CI green on push). No implementation
change occurred in this round to require re-running it.

## Files added this round

All under `fixtures/s5-f3-r/`: `candidate-provenance.json`,
`applicable-manifest.json`, `derive-applicable-manifest.py`,
`run-replay.py`, `replay/` (180 PR directories), `sources/` (persisted
fetched module/test source for every changed/inspected PR),
`compare-replay.py`, `comparison-report.jsonl`,
`discovery-invariance-check.{py,json}`, `build-migration-manifest.py`,
`migration-edge-manifest.json`, `build-option-relocated-enumeration.py`,
`option-relocated-enumeration.json`, `build-ambiguous-rename-scan.py`,
`ambiguous-rename-scan.json`, `build-dynamic-migration-calls.py`,
`dynamic-migration-calls.json`, `build-removed-edge-watched-matches.py`,
`removed-edge-watched-matches.json`, `anchors-check.{py,json}`,
`scan-leaf-collisions.py`, `leaf-collision-scan.json`,
`fatal-error-tabulation.{py,json}`, `classification-notes.json`,
`classify-deltas.py`, `delta-classification.json`, this file, and
`f3-r-summary.json`. `fixtures/s5-f1d-r/`, `fixtures/s5-f2-r/`, and
every other historical fixture directory untouched.

## Unresolved limitations (explicitly stated, not silently dropped)

- Transitive rename-chain support (`A -> B -> C`) is not implemented --
  direct edges only, per S5-F3's own explicit boundary; not exercised
  by anything found in this corpus.
- `mkChangedOptionModule`/`mkMergedOptionModule` remain unsupported.
- Wildcard collision resolution, `imports=` traversal, and
  cross-instance witness ambiguity resolution all remain unsolved, as
  before.
- Cross-file destination confirmation is proven sound only via the
  implementation-round fixture/hostile control and via one same-file
  corpus case (`#510900`) -- not via a genuine cross-file SUCCESS
  within this specific frozen replay, for the disclosed
  `targets.toml`-sparseness reason above.
- The flat-dotted `scan_options`-reuse boundary (nested/placeholder
  destinations invisible to `locate_migration_destination`) is real
  and, as of this round, demonstrably load-bearing (2 of 5 corpus
  cases) -- not fixed here, per this round's own explicit scope.

## Scope discipline

No implementation changes anywhere in this round (the one diagnostic
binary used to build the migration-edge manifest was a throwaway,
never used to produce any accepted result, never committed). No
release, version bump, wildcard work, `imports=` work, F2-caveat work,
or other implementation change.

**STOP.** No further implementation work, release, or additional full
replay without a separate, explicit GO.
