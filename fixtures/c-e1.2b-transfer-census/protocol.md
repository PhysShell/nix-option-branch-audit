# C-E1.2b: `GeneratedConfigArtifact` transfer census -- protocol

Pre-registered BEFORE any of this round's research was performed, per
this project's own standing discipline (E1/C-E1.1's own protocols).
Committed as its own commit, separate from any research/results commit.

## What this round is, and is not

C-E1.2a built the FIRST real, working implementation of the
`GeneratedConfigArtifact` vertical C-E1.1 qualified, and proved it end
to end on 4 deliberately heterogeneous real anchors. The question this
round answers is different and narrower: **does the code C-E1.2a
already shipped TRANSFER onto the rest of the qualified real corpus
without `src/cdc.rs` turning into a zoo of one-off cases?**

This is explicitly a **census, not an implementation round**:

- **Frozen at commit `0fab0af`** (C-E1.2a's own closing commit,
  confirmed green on all 5 CI workflows). Zero changes to
  `src/cdc.rs`, `src/main.rs`, or `fixtures/integrity-lock.toml`'s own
  schema anywhere in this round.
- Output is a structured evidence record per candidate plus a decision,
  not a pull request.
- If the census itself concludes new generic code is warranted, that
  becomes a SEPARATE, later implementation round -- not folded into
  this one.

## Population

The 8 of C-E1.1's 12 `SUPPORTED` candidates that are **not** one of
C-E1.2a's own 4 real anchors:

`privoxy`, `misskey`, `kavita`, `transmission`, `i2pd`, `akkoma`,
`vault`, `spacecookie`

Excluded, and why:

- `unpackerr`, `unbound`, `mobilizon`, `nebula-lighthouse-service` --
  already real-proven, real code exists for these four; re-running
  them here would inflate the transfer rate without testing anything
  new.
- `libinput`, `nohang` -- C-E1.1's own 2 real `NO_PRODUCER_PROOF`
  negative controls. Reserved for C-E1.2c (soundness/false-positive
  audit), not touched here, matching the standing instruction from
  C-E1.2a's own protocol.

No new sampling. Every one of these 8 already has real, cited A/B/C/D
evidence from C-E1.1 (`fixtures/c-e1.1-generated-config-audit/batch{1,2,3}.md`)
-- this round re-verifies rather than blindly trusts that evidence
(same "never trust a citation" discipline C-E1.1's own blind review
established), but the NEW work is the fit-against-the-EXISTING-code
question, not re-discovering A/B/C/D from a blank page.

## Per-candidate questions

For each of the 8, answer all six, from real re-verified evidence:

1. **can acquire A?** -- the real Nix producer still really generates
   the exact artifact (re-confirm against the current tip of
   `PhysShell/nixpkgs`, not just trust C-E1.1's own citation at its
   older pin, since real modules can and do change).
2. **can prove B?** -- the exact artifact is really bound to the exact
   consumer process.
3. **can resolve C?** -- the exact pinned consumer is really
   reachable.
4. **can extract D?** -- the accepted keys are really extractable in a
   bounded way from real consumer source.
5. **can normalize into the EXISTING `GeneratedConfigArtifactEvidence`
   / `ConsumerConfigContract` dotted-leaf-path model, as it is
   ALREADY WRITTEN in `src/cdc.rs` at `0fab0af`, with zero type
   changes?** (`ArtifactContent::{StructuredValue,RenderedText}`,
   `ConfigFormat::{Toml,Yaml,ElixirConf,HandRolled}` -- a NEW format
   value here, e.g. Ini/Json/BespokeLine, is fine, it's descriptive
   metadata only, never branched on; a NEW variant of the enum itself
   still counts as "no type changes" in the sense that matters, since
   nothing about `compare_config_contract`'s own logic depends on
   which format tag is present -- flag it separately if a NEW format
   variant is needed, but don't count it against this question unless
   it forces `content`/`emitted_paths`/`opaque_paths` themselves to
   change shape). `ArtifactBindingEvidence::{DirectArgv,
   EnvironmentEtcSymlink,WrapperScriptEnvVar,ImplicitDefaultPath}` --
   likewise, a genuinely NEW binding shape not covered by these 4 is a
   real, disclosed finding (see the reuse axis below), not silently
   forced into an existing variant that doesn't really describe it.
6. **can use `compare_config_contract()` UNCHANGED** -- no semantic
   modification to that function itself, for any reason, for this
   candidate.

**Verdict per candidate**: `PASS` / `FINDING` / `INCONCLUSIVE(reason)`.

- `PASS` -- transfers cleanly: existing types unchanged in shape,
  `compare_config_contract()` unchanged, at most a new per-consumer
  extractor function (a "new generic locator", expected and normal --
  every anchor in C-E1.2a already needed its own).
- `FINDING` -- a genuine semantic problem the transfer would introduce
  if done the obvious way (the adversarial normalization check below
  is the main source of these, but not the only possible one).
- `INCONCLUSIVE(reason)` -- blocked at A/B/C/D on real evidence
  grounds, same fail-closed vocabulary this whole project already
  uses; state the reason plainly, don't guess.

## Reuse-classification axis (separate from the verdict)

For every candidate, independent of PASS/FINDING/INCONCLUSIVE, record
which of these four best describes what building it for real would
require:

- **existing implementation reuse** -- an already-existing extractor
  (`flatten_structured_value`, `extract_unbound_style_paths`) and an
  already-existing `ArtifactBindingEvidence` variant both apply as-is.
- **new generic locator required** -- a new per-consumer bounded
  extractor function (expected, one per real consumer, matching every
  anchor in C-E1.2a already), and/or a genuinely new but still GENERAL
  `ArtifactBindingEvidence`/`ConfigFormat` variant (e.g. a different
  real way to bind an artifact to a process, not specific to this one
  app's own quirks).
- **new semantic abstraction required** -- the existing type system
  itself would need a new CONCEPT (a new field, a new kind of
  relationship between evidence and contract) to represent this
  candidate honestly, not just a new locator.
- **app-specific logic required** -- would need `if app == "X"`
  anywhere in comparison/contract semantics, or a change to
  `compare_config_contract()`'s own meaning. **Rejected outright** --
  a candidate landing here fails this round's fit question regardless
  of whether A-D all individually hold.

## Adversarial normalization check (the specific thing to try to break)

C-E1.2a's own `unbound` fix normalized `emitted_paths`/`opaque_paths`
down to bare keyword names (stripped the real section prefix
`extract_unbound_style_paths` otherwise produces), because its
D-extractor (`extract_unbound_lexer_keywords`) only returns bare
keywords. That fix is real and currently shipped, but the risk it
carries generically is real too: **"we compared equivalent
representations" can silently become "we discarded context so
everything matched."**

For every one of the 8 candidates whose real D-extraction would need
to read a SECTIONED or NESTED consumer schema (an INI-style file with
`[section]` headers, a nested struct/map, a multi-block DSL), explicitly
check, from the REAL consumer source:

- **(a)** does the SAME bare key name appear, in the real consumer's
  own accepted contract, in two or more different real
  sections/contexts with DIFFERENT accepted semantics? If yes: bare-
  keyword normalization is UNSOUND for this candidate -- record as
  `FINDING`, and the correct evidence stays section/context-qualified
  no matter how inconvenient that makes the comparison.
- **(b)** or is the real contract genuinely flat/bare-keyword at the
  semantic level (no real cross-section collision is possible even in
  principle, because the consumer's own real parser treats it that
  way)? If so, bare normalization is sound, matching `unbound`'s own
  proven case -- but the justification must come from the CONSUMER's
  own real parsing semantics, never merely from "the vendored excerpt
  happens not to have a repeat."

As a same-session sanity check prompted by this same concern (not
itself part of the 8-candidate population, and not blocking this
round's own decision either way): a real recheck of whether `unbound`'s
own actual grammar (`util/configparser.y`) ever defines the SAME
directive name under two different real clauses with different
accepted meaning -- due diligence on already-shipped code, reported
honestly regardless of outcome.

## Decision rule, fixed before any candidate is investigated

- If most of the 8 transfer via **existing model + reuse/new-generic-
  locator only** -> ship the minimal generic additions found necessary
  (if any) as a small follow-up PR, and close C-E1.2b.
- If several candidates independently require the SAME repeating new
  abstraction -> qualify that abstraction separately (its own mini
  A-D-style argument), do not fold it into this census's own verdict.
- If each failure breaks in a genuinely different way (no repeating
  pattern) -> report honestly, do NOT force 12/12 by loosening the
  model to fit. A lower real transfer number with an honest reason
  beats a padded one.
- **Any candidate requiring app-specific semantic branching is
  rejected outright**, regardless of how small the branch would look.

## What comes after (not started here)

C-E1.2c (soundness/false-positive audit: mutations, `libinput`/`nohang`
as negative controls, ambiguity/failure modes, and ideally one small
fresh holdout that never touched C-E1.1) is explicitly a LATER, separate
round. Not authorized as part of this one.
