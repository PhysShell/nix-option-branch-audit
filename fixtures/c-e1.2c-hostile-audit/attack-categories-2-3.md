# C-E1.2c attack categories 2 & 3: binding attacks and
consumer-contract attacks -- real scenario analysis

Real code-reading analysis against the frozen `984b366` tree, not new
Rust tests (per the protocol's own framing -- these are structural
questions about whether the current logic could be fooled, answered
honestly including where the honest answer is "yes, and here's the
real gap").

## Attack category 2: binding attacks

### The specific, real, already-suspected gap, confirmed

Surveyed every real B-binding check across all 11 implemented
candidates (`grep -n "if !exec_start.contains\|ETC_PATH\|akkoma-env"
src/cdc.rs`). Two real, distinct shapes:

**Shape 1 -- content and the binding-check string come from the SAME
extracted path, read once (`privoxy`, `spacecookie`)**: `binPath` is
computed via the `builtins.substring`-based last-space extraction from
the real `ExecStart`, and `builtins.readFile binPath` produces
`content` directly from THAT SAME path. There is no possible
divergence between "what B proves is bound" and "what A reads as
content" -- they are, by construction, the same Nix evaluation. This is
the strongest real B-proof shape in the whole vertical.

**Shape 2 -- content and the binding-check are two SEPARATE real Nix
expressions (`unpackerr`, `unbound`, `mobilizon`, `misskey`, `kavita`,
`transmission`, `i2pd`, `akkoma` -- 8 of 11)**: the binding check is a
substring/flag-presence test on `ExecStart` (`"--config="`, `"-g "`,
`"--conf="`, `"akkoma-env"`) or a fixed-path string
(`/etc/unbound/unbound.conf`), while `content` is read from a
SEPARATE real Nix expression -- either `cfg.settings`/`cfg.config`
directly (the `StructuredValue` candidates), or a second `readFile`
call at a path extracted independently (`unbound`, `mobilizon`). **B's
real proof for these 8 is architectural, not byte-level**: it confirms
the RIGHT KIND of binding mechanism exists (a `--config=`-shaped flag
really is present; a wrapper script really does export the right env
var name), but never independently confirms the referenced artifact's
own real BUILT content is byte-identical to what `content` holds.

This is a real, stable, disclosed limitation, not a new bug to chase
down mid-audit: for every `StructuredValue`-typed candidate, `content`
is read from the module's OWN Nix-level `cfg.settings`/`cfg.config`
value -- the exact same value the module ITSELF uses internally to
render the real artifact it then binds. Independently re-deriving and
byte-comparing the ACTUAL rendered file (rather than trusting the
module's own internal wiring) was a deliberate design choice from
`unpackerr`'s own original C-E1.2a anchor onward (see
`ArtifactContent::StructuredValue`'s own doc comment: "the real
Nix-evaluated JSON representation of the value that generator
serialized IS the semantic source of truth, so extraction reads THAT
directly rather than re-parsing rendered TOML/YAML/JSON text"). **8 of
11 real candidates share this same trust boundary.** Recorded here as a
named, real finding -- not fixed in this round.

### `akkoma`'s own B-check is the weakest of all 11

`if !exec_start.contains("akkoma-env")` -- checks only that `ExecStart`
references the real wrapper PACKAGE NAME, not any specific flag or
path at all. This is real (the wrapper genuinely has no CLI-visible
config path -- see the module's own doc comment on
`acquire_akkoma_evidence` for why: the real path is set via a
cross-unit `AKKOMA_CONFIG_PATH` env var populated by a SEPARATE
`akkoma-config.service` unit, and structurally proving that
`bindsTo`-mediated cross-unit link would need a second real `nix eval`
this v1 explicitly didn't add) but it is measurably looser than every
other candidate's own check. Named honestly; not fixed here.

### The 5 real scenario questions, answered

1. **Artifact A generated, process reads B (a different derivation)**:
   for the 3 `RenderedText`/`readFile`-sourced candidates (`unbound`,
   `mobilizon`, and `privoxy`/`spacecookie`'s own single-read shape),
   this is structurally hard to fool BY CONSTRUCTION for
   `privoxy`/`spacecookie` (same read). For `unbound`/`mobilizon`, it's
   real but NixOS-platform-trusted (activation-time `environment.etc`
   symlinking; the module's own wrapper-generation consistency), not
   independently re-verified byte-for-byte.
2. **Two plausible artifact paths**: none of the 11 candidates'
   real modules expose two SIMULTANEOUSLY plausible config paths in
   their default configuration (confirmed by re-reading each
   `acquire_*` function's own real `nix eval` output already captured
   during C-E1.2a/b) -- a real, but currently moot, question for this
   corpus. Would become live if a future candidate has e.g. a
   `configFile`-or-`extraConfigFile` override option.
3. **Wrapper/env points at a different real file**: this IS the real,
   confirmed Shape-2 gap above for `mobilizon`/`misskey` -- the
   wrapper/env-var check confirms the RIGHT VAR NAME is set, never
   that its VALUE equals the specific path `content` was derived from.
4. **A real default path exists but is overridden**: `kavita`'s own
   `ImplicitDefaultPath` is COMPUTED (`WorkingDirectory` + a hardcoded
   relative suffix) specifically because C-E1.2b's own research found
   this is a real, two-source-derived path, not a naive single
   hardcoded literal -- already the most defensive of the implicit-path
   candidates. `nebula-lighthouse-service`'s own single hardcoded
   literal (unchanged since C-E1.2a) would NOT notice a real override
   if one existed in a future nixpkgs version -- a real, disclosed,
   unchanged limitation from C-E1.2a's own original design.
5. **Ambiguous symlink/reference chain**: `unbound`'s own
   `environment.etc` mechanism is the one real case of this shape in
   the corpus, and it's single-hop (one real `.source` attribute, no
   further indirection) -- confirmed by re-reading the real module.nix
   directly, not assumed.

## Attack category 3: consumer-contract attacks

Curated subset per the protocol's own instruction (2-3 structurally
different cases): `unpackerr` (`StructuredValue`, fully-tagged Go
struct D), `akkoma` (partial-schema-shaped, already a real executable
`Finding`), `unbound` (hand-rolled text, lexer-derived D).

1. **Wrong pinned consumer version**: none of the 11 candidates' own
   D-extractors CHECK the vendored fixture's own version against the
   real `nix eval`-derived package version at runtime -- there is no
   code path that would notice if `fixtures/cdc/generated-config-
   artifact/unpackerr/apps.go` silently became stale relative to a
   future `CE12_REV` bump. This is real and structurally identical to
   every earlier K1-K5 vendoring convention in this project (the
   `fixtures/integrity-lock.toml` sha256 lock catches a fixture being
   SILENTLY EDITED, but not a fixture becoming STALE relative to a
   newer pinned nixpkgs revision). Not a new gap this round invented --
   a pre-existing, disclosed property of this project's whole vendoring
   discipline, re-confirmed here rather than assumed fixed.
2. **A neighboring-version schema silently absorbing a real
   rename/removal as "no change"**: this is exactly what K3b/K4b/K5b's
   own real historical-drift census work already tests FOR as its own
   separate vertical (not duplicated here) -- `GeneratedConfigArtifact`
   itself has no built-in drift-detection layer; it answers "does the
   CURRENT pinned artifact satisfy the CURRENT pinned contract," not
   "did the contract change between two versions." Named as a real
   scope boundary, not a bug.
3. **A genuinely partial schema claiming completeness it doesn't
   have**: `akkoma` IS this case, already demonstrated as a real
   executable `Finding` in C-E1.2b
   (`real_akkoma_end_to_end_is_a_real_finding_not_a_forced_pass`).
   Re-ran the real test against the frozen `984b366` tree during this
   round (see `mutation-results.md`) -- confirmed STABLE, same real
   `:pleroma.:instance.upload_dir` Finding, byte-identical.
4. **A freeform-map decode elsewhere in the 11 implemented
   candidates**: re-checked each candidate's own real D-extractor
   scope for an unbounded/freeform region analogous to `vault`'s own
   `storage`/`listener` blocks. None found among the 11 -- every
   implemented D-extractor's own real source is a bounded, named,
   typed/tagged structure (Go struct tags, C# properties, TS type
   literals, boost::program_options literals, Aeson instances, a
   fully-typed Elixir schema for the `:pleroma`-scoped subset). This is
   a real, positive confirmation, not assumed: `vault`'s own freeform-
   map shape was specifically why it was excluded from implementation
   in C-E1.2b, and no other candidate shares it.
5. **`akkoma` as the running adversarial example**: covered above (3).
   The SAME real lesson generalizes: "big and structured" is not the
   same claim as "complete" -- `description.exs` is genuinely both (a
   real, ~105KB, well-organized schema) AND genuinely incomplete
   relative to the runtime's actual accepted surface (`:joken`/
   `:tzdata`/`Pleroma.Repo`/`upload_dir`, confirmed zero-coverage).
