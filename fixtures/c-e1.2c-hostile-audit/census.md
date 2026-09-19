# C-E1.2c: `GeneratedConfigArtifact` hostile soundness audit -- results

Protocol: `protocol.md` (pre-registered, committed `88ef632`, before any
attack was run or holdout candidate inspected). Frozen at `984b366`.
Raw evidence: `holdout-draw.md`, `holdout-batch1.md`-`holdout-batch3.md`,
`attack-categories-2-3.md`, and the new `#[ignore]`/offline test
functions in `src/cdc.rs`'s own `mod ce12_tests` (search for
`// C-E1.2c` / `attack1_`/`attack4_`). Zero production-code
(`acquire_*`/`extract_*`/`compare_config_contract`/
`flatten_structured_value`) changes anywhere in this round.

## Headline: every pre-registered success criterion met

| criterion | result |
|---|---|
| false PASS | **0** |
| false FINDING | **0** |
| TOOL_ERROR | **0** |
| all category-1 mutations detected | **yes** (mutation types 1/3/4/5; type 2 is a non-worsening check, also holds) |
| `libinput`/`nohang` stay unsupported | **yes**, byte-identical to C-E1.1's own original findings |
| `unbound`/`i2pd`/`akkoma` normalization triple preserved | **yes**, plus 2 new synthetic adversarial cases pass |
| fresh holdout: zero app-specific branching temptation | **yes** |

## Metrics

| | count |
|---|---|
| true PASS (11 implemented candidates' own real baseline, excluding `akkoma`) | 10 |
| true FINDING (`akkoma`'s own real, stable, disclosed gap) | 1 |
| INCONCLUSIVE (`vault`, deliberately unimplemented; `libinput`/`nohang`, negative controls) | 3 |
| TOOL_ERROR | 0 |
| false PASS | 0 |
| false FINDING | 0 |
| mutation detection rate | **100%** (every relevant category-1 mutation across all 11 real anchors: delete/append/near-miss) |
| negative-control preservation | **2/2** (`libinput`, `nohang`) |
| fresh-holdout support rate | **10/10 SUPPORTED** (coverage is explicitly secondary per the protocol; reported honestly, not suppressed for looking "too clean" -- see the holdout evidence-quality note below) |

## Attack category 1: producer/artifact mutations -- clean

Three real `#[ignore]` tests, each run against all 11 implemented real
anchors at once (not a sample):

- **Delete a real emitted path** (33+ real deletions checked across 11
  anchors' own real evidence): never turned a real clean `Pass` into a
  `Finding`. Confirms no candidate's own comparison is fragile to
  evidence shrinking.
- **Append a genuinely new unaccepted path**: always produced a
  `Finding`, across all 11, confirming `compare_config_contract`'s own
  "first miss" logic holds regardless of WHERE in the real emitted list
  the bad entry lands (a different code path from the existing
  per-candidate stop-condition-8 in-place-rename tests).
- **Near-miss substitution** (5 representative real cases, one per
  extraction shape): `transmission` dash-vs-underscore (`peer-port` vs
  `peer_port`, the K4c lesson itself), `i2pd` dotted trailing-char
  (`http.enabled` vs `http.enable`), `unbound` bare trailing-char
  (`port` vs `ports`), `akkoma` fully-qualified trailing-char
  (`:pleroma.:instance.name` vs `...names`), `unpackerr` case
  sensitivity (`debug` vs `Debug`). **One real test-design bug caught
  live**: the first version of this test assumed the near-miss would
  always be the FIRST reported problem, but `akkoma`'s own real
  baseline already has a genuine `Finding` (`upload_dir`) that sorts
  first -- fixed by isolating `emitted_paths` to just the probed real
  path before injecting the near-miss (confirming a clean `Pass` in
  isolation first), the same isolation technique C-E1.2b's own akkoma
  mutation test already established. Not a soundness bug -- a test
  harness bug, caught and fixed before being counted as a result.
- **Nesting-shape change** (2 offline synthetic tests, both
  directions): a scalar mutated into a nested object produces a
  genuinely different dotted path, never silently continues to look
  like the original; the reverse direction holds too.

## Attack category 2: binding attacks -- one real, stable, disclosed
finding, not a bug to chase

Full writeup: `attack-categories-2-3.md`. Headline: **8 of the 11
implemented candidates' own B-proof is architectural (the right KIND
of binding mechanism is confirmed present), not byte-level (the
specific artifact's own built content is never independently
re-verified against what the binding mechanism actually references)**.
Only `privoxy`/`spacecookie` prove B and read content from the exact
same single Nix expression by construction. This has been true since
`unpackerr`'s own original C-E1.2a anchor (`StructuredValue` reads
`cfg.settings` directly, trusting the module's own internal
consistency) -- re-confirmed here as a real, stable property of the
whole vertical's design, not a new regression. `akkoma`'s own B-check
(`exec_start.contains("akkoma-env")`) is measurably the loosest of all
11. Named honestly; not fixed in this round, per the protocol's own
explicit "production code frozen" rule.

## Attack category 3: consumer-contract attacks -- confirms `akkoma`'s
own real Finding is stable, finds no unnoticed freeform region elsewhere

Re-ran `akkoma`'s real end-to-end test against the frozen `984b366`
tree during this round (via the attack-1 test suite's own fresh
acquisition) -- **byte-identical real result**, same
`:pleroma.:instance.upload_dir` Finding. Checked all 11 implemented
candidates' own D-extraction scope for an unbounded/freeform region
analogous to `vault`'s own excluded shape -- **none found**; every
implemented D-extractor's own real source is a bounded, named,
typed/tagged structure. Wrong-pinned-version and neighboring-version
drift are real, disclosed, PRE-EXISTING scope boundaries of this whole
vertical (not new gaps this round invented) -- `GeneratedConfigArtifact`
answers "does the CURRENT artifact satisfy the CURRENT contract," not
"did the contract drift between versions" (that question is K3b/K4b/K5b's
own separate vertical).

## Attack category 4: normalization -- extended, still holds

The real `unbound`/`i2pd`/`akkoma` triple (already a permanent test
since C-E1.2b) is joined by 2 new synthetic adversarial cases proving
the SAME invariant on shapes this project has never actually
implemented: a synthetic section-stripping text format with a real
cross-section collision (`extract_unbound_style_paths` itself, before
any acquire-time stripping decision, never collapses the two real
section-qualified paths into one bare collision), and a synthetic
structured-JSON case with two branches sharing a bare field name
(`flatten_structured_value` keeps both fully qualified, general, no
per-consumer knowledge needed). A third new test audits this project's
OWN source text: every D-extractor making a bare-vs-qualified choice
must cite its own real consumer-specific justification in its doc
comment -- confirmed present for `unbound` (the grammar recheck),
`akkoma` (the 18-way collision), and `i2pd` (already-dotted source, no
decision to make).

## Negative controls: preserved, byte-identical

Re-ran the exact real `nix eval` C-E1.1 originally used for both:
`libinput`'s `environment.etc."X11/xorg.conf.d/40-libinput.conf".source`
still resolves to a STATIC path inside the pre-built package (not
derived from `cfg` values), with the real per-option content still
only reaching `services.xserver.inputClassSections` (a list fragment
fed to a different, unvendored module). `nohang`'s real `ExecStart`
still resolves its `--config` value to a path INSIDE its own package
derivation (`.../nohang-0.3.0/etc/nohang/nohang-desktop.conf`), not a
separately Nix-rendered artifact. **Neither locator/extractor addition
across C-E1.2a/b accidentally started guessing evidence for either.**

## Fresh holdout: 10/10 SUPPORTED, all real, all independently verified

Population and draw: `holdout-draw.md` (170 real candidates, seeded
shuffle, `librechat`/`meilisearch`/`anubis`/`loki`/`nomad`/`gocron`/
`clatd`/`redmine`/`nats`/`gancio` drawn). Full evidence:
`holdout-batch1.md`-`holdout-batch3.md`.

**A note on the 10/10 result, since the protocol explicitly named
coverage as secondary and a lower rate would have been an equally good
result**: this was NOT assumed clean going in, and wasn't treated as
suspicious after the fact either -- it was checked. Independently
re-read all 3 batch files in full (not sampled) and spot-verified
citation quality (real file paths, real line numbers, real fetched
upstream source, real `nix eval` output pasted directly, not
paraphrased) before accepting the 10/10 result at face value. The
research is NOT uniformly rosy -- it surfaces real complications
throughout, which is itself evidence against rubber-stamping:

- `redmine`'s B is a genuinely new **3-hop symlink chain** (a
  build-time package-level symlink, then two `preStart`-level symlinks)
  with zero CLI/env footprint at `ExecStart` -- none of the 6 currently-
  shipped `ArtifactBindingEvidence` variants represent this shape.
- `redmine`'s D also needs a real normalization step this project has
  never needed before: Rails' own `default:`/`<env>:`-keyed YAML
  wrapper must be flattened before comparing -- a FOURTH real
  normalization shape, distinct from the unbound/i2pd/akkoma triple.
- `meilisearch`'s B (`ExecStartPre`-install then CLI-flag reference to
  the fixed runtime path) doesn't cleanly fit any existing binding
  variant either -- a second real taxonomy gap.
- `gocron`'s B is real but has literal shell-quoting
  (`--config '${path}'`) a future `binPath`-extraction implementation
  would need to strip -- a real detail no currently-implemented
  candidate's own `ExecStart` has.
- `gancio` is this batch's own best illustration of attack category
  2's own concern made concrete: the "obvious" binding evidence (a CLI
  positional arg) is almost certainly NOT what the real consumer (a
  `yargs`-based CLI) actually honors -- the true proof is a `preStart`
  symlink plus the consumer's own real default option value, not the
  argv at all.
- `anubis` required real scope judgment: one Nix module produces TWO
  differently-shaped real artifacts (a flat env-var interface,
  out of this vertical's scope entirely, and a CONDITIONAL JSON policy
  file, in scope) -- only the latter counted, and its own real
  existence depends on user customization.

**Two candidates' own D evidence is stronger than any of the 11
already-implemented candidates'**: `librechat` (a real
`.strict().safeParse()`-enforced Zod schema -- rejects unknown keys at
real runtime, not just typed) and `meilisearch` (a real dual-purpose
`clap`+`serde` Rust struct, the same fields ARE both the CLI parser and
the config schema). `nats` has the strongest A/C evidence found
anywhere in this project: the real module runs `nats-server --config
"${file}" -t` (real build-time validation against the actual consumer
binary) before the system even builds.

**Zero app-specific semantic branching temptation** -- every one of
the 10 fits the existing `GeneratedConfigArtifactEvidence`/
`ConsumerConfigContract`/`compare_config_contract` model, modulo the
real, disclosed binding-taxonomy gaps named above (which are additive
-- new variants, never new comparison semantics).

## Decision, against the rule fixed before any attack was run

```
if false PASS == 0 AND false FINDING == 0 AND TOOL_ERROR == 0
   AND all category-1 mutations detected
   AND libinput/nohang preserved
   AND normalization triple preserved:
      -> GeneratedConfigArtifact v1 is declared FROZEN
```

**Every condition holds. `GeneratedConfigArtifact` v1 is declared
FROZEN.**

Real, disclosed, NOT-yet-fixed items carried forward as named follow-up
work (not blockers to the freeze, per the decision rule's own literal
text -- these are known limitations of a frozen v1, not open defects):

1. **Binding-proof depth** (category 2): 8/11 implemented candidates'
   B-proof is architectural, not byte-level. `gancio`'s own real case
   in the holdout shows this concern is not hypothetical.
2. **3 new real binding shapes found in the holdout, unmodeled by the
   current `ArtifactBindingEvidence` enum**: `redmine`'s 3-hop symlink
   chain, `meilisearch`'s install-then-flag reference, `gocron`'s
   shell-quoted argv.
3. **A 4th real normalization shape** (`redmine`'s env-keyed YAML
   wrapper flattening), beyond the already-shipped unbound/i2pd/akkoma
   triple.
4. **Vault stays `INCONCLUSIVE`**, unrevisited (its own real gap is
   D-side, structurally unrelated to anything this round tested).

None of these were found via a false PASS or false FINDING -- every one
surfaced through honest, disclosed research (the holdout's own real
investigation, or category-2's own direct code audit), exactly the
"soundness first" discipline this round existed to apply.

## What comes after

Per the protocol's own decision rule: since v1 is frozen, productization
(CLI wiring, an advisory GitHub check, a PR-local diff mode, a
nixpkgs-review companion) may now be considered -- not started in this
round, and not automatically the next thing either; a separate decision
for whoever picks up this project's own roadmap next. Any future
attempt to model the 3 new real binding shapes or the redmine-style
normalization would be its own separately-scoped implementation round,
matching this whole project's own "generic additions only when a real
case demands them" discipline throughout.
