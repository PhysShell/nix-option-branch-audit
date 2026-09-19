# C-E1.2b: `GeneratedConfigArtifact` transfer census -- results

Protocol: `protocol.md` (pre-registered, committed `e025f4a`, before any
candidate was inspected). Frozen at `0fab0af`. Raw per-batch evidence:
`batch1.md`-`batch3.md`. Zero `src/` changes anywhere in this round --
this is a census, not an implementation round.

## Headline: 6/8 PASS, 1 FINDING, 1 INCONCLUSIVE -- zero app-specific
branching anywhere, zero new semantic abstractions needed anywhere

| candidate | A | B | C | D | normalizes into existing model? | `compare_config_contract()` unchanged? | verdict |
|---|---|---|---|---|---|---|---|
| privoxy | proved | proved | proved | proved | yes | yes | **PASS** |
| misskey | proved | proved | proved | proved | yes | yes | **PASS** |
| kavita | proved | proved | proved | proved | yes | yes | **PASS** |
| transmission | proved | proved | proved | proved | yes | yes | **PASS** |
| i2pd | proved | proved | proved | proved | yes | yes | **PASS** |
| spacecookie | proved | proved | proved | proved | yes | yes | **PASS** |
| akkoma | proved | proved | proved | proved | yes, IF fully-qualified | yes | **FINDING** |
| vault | proved | proved | proved | **not boundedly extractable for the fields the real default artifact actually emits** | partial (`opaque_paths` covers the gap) | yes | **INCONCLUSIVE** |

**Zero candidates required `if app == "X"` branching. Zero candidates
required a change to `compare_config_contract()`'s own semantics. Zero
candidates required a genuinely new type-level CONCEPT** (a new field,
a new relationship between evidence and contract) beyond what
`GeneratedConfigArtifactEvidence`/`ConsumerConfigContract` already
express. This is the single most important number this census
produced: the abstraction C-E1.2a shipped generalizes, it does not
collapse into a zoo of special cases.

## Reuse-classification summary

| candidate | classification |
|---|---|
| privoxy | new generic locator (a new `ArtifactBindingEvidence` variant for a bare positional argv; a new flat/no-section rendered-text extractor; a new bounded C `#define ... /* "name" */` D-scanner) |
| misskey | new generic locator (a new `ArtifactBindingEvidence` variant for an `ExecStartPre`-installed runtime-path env var; a new two-hop TypeScript type-literal D-scanner) |
| kavita | new generic locator (existing `ImplicitDefaultPath` TYPE fits unmodified -- only the ACQUIRE logic is new, combining two real evidence sources into one resolved path; a new bounded C# property-declaration D-scanner) |
| transmission | **existing implementation reuse** for A/B/the comparison path itself (`DirectArgv` + `flatten_structured_value`, both used completely as-is); new generic locator only for D (a bounded `TR_KEY_*` enum-constant D-scanner) |
| i2pd | **existing implementation reuse** for A/B/the comparison path itself, and the CLEANEST transfer in the whole census -- `flatten_structured_value` needs zero modification, not even a normalization decision; new generic locator only for D (a bounded `boost::program_options` literal D-scanner) |
| akkoma | new generic locator (a new `description.exs`-scanning D-extractor -- must emit FULLY QUALIFIED `group.key.field` paths, never bare names; zero type changes) |
| vault | new generic locator for A (a new HCL block-syntax rendered-text extractor, genuinely different shape from `unbound`'s own line format); the real gap is D, not a locator problem -- see below |
| spacecookie | new generic locator (a new bounded Aeson-call-shape D-scanner; zero stripping needed anywhere) |

**Zero candidates landed in "new semantic abstraction required." Zero
candidates landed in "app-specific logic required."** "New generic
locator required" is the dominant, expected bucket -- exactly the same
shape every one of C-E1.2a's own four anchors already needed (each
anchor needed its own acquire function and its own D-extractor; that
was never in question). Two of the eight (`transmission`, `i2pd`) are
stronger than that: a clean, zero-new-code reuse of A/B/the comparison
path itself, with only D needing a new per-consumer scanner.

## The two non-PASS results, in detail

### `akkoma` -- FINDING: a real, concrete warning against copying
`unbound`'s own shortcut, not a broken abstraction

The adversarial normalization check (see below) found a real 18-way
collision on the bare field name `:enabled` alone (and 10-way on
`:api_key`) across akkoma's real `config/description.exs` schema.
Bare-keyword stripping -- the specific move `unbound`'s own shipped fix
made -- would be genuinely UNSOUND here. The fix needs **zero type
changes**: `flatten_structured_value` was never going to strip anything
in the first place; it only produces a stripped shape when a
D-extractor is deliberately written to match one (`unbound`'s own
D-extractor only ever returns bare names, which is why `unbound`
needed the stripping step at all). For `akkoma`, the correct design is
simply to write its D-extractor to emit fully-qualified
`group.key.field` paths, matching what A already naturally produces,
unmodified.

### `vault` -- INCONCLUSIVE: a real gap in C-E1.1's own original
citation, found only by re-deriving D against the EXACT real emitted
surface

C-E1.1's own batch3.md cited `command/server/config.go`'s real,
fully-tagged `hcl:"..."` struct fields as D's proof -- real, but this
round found (by computing the real rendered artifact from the real
Nix template + real module defaults, then reading the FULL real
`config.go`, not the snippet C-E1.1 quoted) that **none of those tagged
fields are what a real default-configured `vault` module actually
emits.** The two blocks it does emit (`storage`, `listener`) are each
handled outside the tagged-struct path entirely: `storage`'s body
decodes into a genuinely freeform `map[string]interface{}`/
`map[string]string` with no fixed key set; `listener` is handled by an
entirely separate, unfollowed external package
(`github.com/hashicorp/go-secure-stdlib/configutil`). A real,
disclosed correction to an assumption C-E1.1 never stated explicitly
either way: this round also establishes `content` here is
`ArtifactContent::RenderedText` (a hand-rolled HCL template), not
`StructuredValue`.

This is not an abstraction failure -- the existing `opaque_paths`
concept already has the right vocabulary (mark the freeform blocks'
bodies opaque, the same "can't verify a real path in a bounded way"
move `flatten_structured_value` already makes for a JSON array). But
honestly reported: once built this way, the real comparison for
`vault`'s own default config would be close to vacuous (almost nothing
survives into `emitted_paths`). Recorded `INCONCLUSIVE`, not forced to
`PASS` by loosening the model, per the protocol's own explicit rule
against padding the count.

## Adversarial normalization check: the census's own second-most
important result

Ran wherever a candidate's D-extraction reads a sectioned/nested real
schema. Real, concrete outcomes on BOTH sides of the question, not a
single confirmatory pattern:

- **`i2pd` is a genuine positive control, and the single most valuable
  result in the whole census**: the bare key `"enabled"` really
  collides across 7 real sections (`bob`/`http`/`httpproxy`/`i2cp`/
  `i2pcontrol`/`sam`/`socksproxy`), toggling 7 unrelated real
  subsystems -- but i2pd's own real `boost::program_options`
  registrations are ALREADY fully dotted (`"http.enabled"`,
  `"httpproxy.enabled"`, ...) in the consumer's own real source, so
  `flatten_structured_value` needs ZERO modification and produces the
  exactly correct fully-qualified paths by construction. **The correct
  answer here is the literal opposite of `unbound`'s own shipped fix**
  (keep the dots, don't strip) -- direct, real proof the normalization
  decision is per-consumer semantic work, never a reusable default in
  either direction.
- **`akkoma`** independently confirms the same lesson from the other
  side: a real 18-way collision exists, and the correct fix is (again)
  to NOT strip -- emit fully qualified paths.
- **`privoxy`, `misskey`, `kavita`, `transmission`, `spacecookie`**: the
  check doesn't even arise for structurally different, individually
  disclosed reasons -- `privoxy`'s real format has no section concept
  in the consumer's own grammar at all (one flat, global dispatch, not
  merely "no observed collision in a small excerpt"); the rest are
  JSON/YAML-shaped, and `flatten_structured_value` never discards a
  path segment in the first place, so there is nothing to strip on
  those paths regardless of nesting depth.
- **A same-session sanity recheck on already-shipped code** (not part
  of the 8-candidate population, prompted by this same concern):
  fetched `unbound`'s own real yacc grammar
  (`util/configparser.y`/`util/configlexer.lex` @ the exact already-
  vendored commit) and confirmed the `server:` and `remote-control:`
  clauses share ZERO tokens -- every real directive keyword maps to
  exactly one token used in exactly one clause's own production list
  (`VAR_SERVER_KEY_FILE`/`VAR_SERVER_CERT_FILE`, despite their
  confusing "server_"-prefixed token names, are used ONLY inside the
  `remote-control:` clause's own grammar, never `server:`'s). Real,
  independent confirmation the already-shipped `unbound` fix is sound.

**Net result: the normalization decision is real, per-consumer,
semantic work every time -- never safely defaulted either way, and
never mechanically inferable from format alone.** Two real INI/Elixir-
shaped candidates (`i2pd`, `akkoma`) needed the opposite move from
`unbound`'s own text-format case, and got it right only because each
was checked against its own real consumer parsing semantics, not
assumed from a superficial format resemblance.

## Cross-cutting pattern: a repeating new `ArtifactBindingEvidence`
shape, found independently in two different batches

`privoxy` (batch 1) and `spacecookie` (batch 3, independently
re-confirmed: "the same real bare-positional-argument mechanism")
both bind via a bare positional CLI argument with no flag name --
`DirectArgv { flag, argv }`'s existing shape assumes a NAMED flag and
does not fit either without being forced. This is the one place this
census's own decision rule's "several candidates independently require
the SAME repeating new [addition]" language directly applies: one new,
general `ArtifactBindingEvidence` variant (e.g.
`DirectPositionalArg { argv: String }`) serves both real candidates,
not two one-off cases.

## Decision, against the rule fixed before any candidate was inspected

```
if most of the 8 transfer via existing model + reuse/new-generic-locator only:
   -> ship the minimal generic additions found necessary (if any) as a
      small follow-up PR, and close C-E1.2b
```

**6/8 PASS, 0/8 required app-specific branching, 0/8 required a new
semantic abstraction. This is a clean hit on the first branch of the
decision rule.** The two non-PASS results (`akkoma` FINDING, `vault`
INCONCLUSIVE) are both real, both honestly reported, and both resolve
with EXISTING vocabulary (full qualification instead of stripping;
`opaque_paths` for an unbounded block) -- neither forces a new
abstraction or an app-specific branch either.

**The minimal generic additions this census actually found necessary,
aggregated across all 8** (nothing here is app-specific; every item
below is a general, reusable shape, matching the same "new generic
locator" pattern every C-E1.2a anchor already needed):

1. `ArtifactBindingEvidence::DirectPositionalArg { argv: String }` --
   needed by 2 real candidates (`privoxy`, `spacecookie`).
2. A new `ArtifactBindingEvidence` variant for an `ExecStartPre`-
   installed runtime-path env var (`misskey`) -- a real, general NixOS
   secret-substitution idiom, not misskey-specific.
3. Two new `RenderedText` extractor SHAPES, neither reusable with the
   other or with `unbound`'s own: a flat, no-section, space-separated
   line format (`privoxy`), and a real HCL block-syntax format
   (`vault`, and only useful there for the `opaque_paths`-marked
   blocks, given vault's own D gap).
4. Seven new per-consumer bounded D-extractors (one each for privoxy,
   misskey, kavita, transmission, i2pd, akkoma, spacecookie) --
   expected, normal, the same shape every existing extractor in
   `src/cdc.rs` already is.
5. `kavita`'s `ImplicitDefaultPath` needs no new variant at all -- only
   a new ACQUIRE-time computation (concatenate `WorkingDirectory` +
   a real hardcoded relative suffix) before constructing the existing
   type.

**Not authorized as part of this round**: actually building any of the
above. This census's own protocol scoped it to research/fit-assessment
only. The decision rule's own recommendation ("ship... as a small
follow-up PR") is a conclusion for the user to act on, not something
this round executes itself.

## What comes after (not started here)

C-E1.2c (soundness/false-positive audit: mutations, `libinput`/`nohang`
as negative controls, ambiguity/failure modes, ideally a fresh holdout)
remains a later, separate, not-yet-authorized round, unaffected by
this one either way.
