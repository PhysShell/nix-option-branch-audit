# C-E1.1: GeneratedConfigArtifact qualification audit -- results

Full protocol: `protocol.md` (pre-registered, committed `a09e6ea`,
before any candidate was inspected). Raw per-batch evidence:
`batch1.md`-`batch4.md`. All 14 candidates are E1's own already-named
set (no new sampling). Zero `src/` changes anywhere in this round.

## Headline: 12/14 SUPPORTED (86%), after one correction from blind review

| candidate | A | B | C | D | fit | reason_code |
|---|---|---|---|---|---|---|
| libinput | no | -- | -- | -- | no | `NO_PRODUCER_PROOF` |
| privoxy | proved | proved | proved | proved | **yes** | `SUPPORTED` |
| nohang | no | -- | -- | -- | no | `NO_PRODUCER_PROOF` |
| misskey | proved | proved | proved | proved | **yes** | `SUPPORTED` |
| kavita | proved | proved | proved | proved | **yes** | `SUPPORTED` |
| nebula-lighthouse-service | proved | proved | proved | proved | **yes** | `SUPPORTED` |
| transmission | proved | proved | proved | proved | **yes** | `SUPPORTED` |
| i2pd | proved | proved | proved | proved | **yes** | `SUPPORTED` |
| akkoma | proved | proved | proved | **proved** (corrected, see below) | **yes** | `SUPPORTED` |
| vault | proved | proved | proved | proved | **yes** | `SUPPORTED` |
| spacecookie | proved | proved | proved | proved | **yes** | `SUPPORTED` |
| unpackerr | proved | proved | proved | proved | **yes** | `SUPPORTED` |
| unbound | proved | proved | proved | proved | **yes** | `SUPPORTED` |
| mobilizon | proved | proved | proved | proved (borderline) | **yes** | `SUPPORTED` |

This clears the decision rule's own `>= 5/14` bar by a wide margin (12,
not a marginal 5 or 6) -- E1's original 35%-of-holdout structural guess
turns out, on real mechanical verification, to UNDERSTATE the real
signal, not overstate it (E1 never checked B/C/D at all; those three
additional links only removed 2 of 14 in the end, not more).

## The 2 real failures, each for a genuinely different reason

- **`libinput` -- `NO_PRODUCER_PROOF`**: looked like generated config at
  a glance; on real `nix eval`, the module's own per-option Xorg text
  becomes a LIST FRAGMENT fed into a *different*, unvendored module's
  (`xserver`'s) own eventual artifact assembly -- never a standalone
  artifact this module itself produces. "Renders option values into a
  string" and "generates an artifact" are different claims, and this
  audit's whole point was catching exactly that conflation. Independently
  re-confirmed by the blind reviewer from scratch, identical conclusion.
- **`nohang` -- `NO_PRODUCER_PROOF`**: a DIFFERENT real reason, not the
  same gap as `libinput` (deliberately not conflated in the aggregate).
  Zero Nix-rendered content exists anywhere -- `configPath` either
  selects a static file bundled inside the upstream package itself, or
  passes an arbitrary user-supplied path straight through. Independently
  re-confirmed by the blind reviewer, who also went further and found
  C/D would likely have been `proved` had A held (~30 bounded literal
  key accesses) -- a real, disclosed irony (this is exactly the
  "C/D fine, D is the bottleneck" shape the decision rule was watching
  for, except here the chain never even reaches that point).

## `akkoma`: corrected from `CONTRACT_UNEXTRACTABLE` to `SUPPORTED`
after blind review found real evidence the first pass missed

The first pass's D verdict for `akkoma` rested on `Config.Reader`
itself having no enforced runtime schema, and characterized
`Application.get_env`/`Pleroma.Config.get` call sites as "scattered
arbitrarily" -- real observations, but incomplete. The blind reviewer,
working from scratch with no access to the first pass's findings,
additionally found `config/description.exs` -- a real, ~105KB,
structured, machine-readable schema file (`%{key: "...", label: "...",
type: :string, description: "..."}` entries for every real accepted
config key), compiled into `Pleroma.Docs.JSON` and used by the admin
API to whitelist settable keys -- plus a call-site audit showing the
access pattern is genuinely disciplined in practice (~507 sites through
the `Pleroma.Config.get`/`get!` wrapper, only 4 raw `Application.
get_env` calls in the whole tree).

**Independently re-verified before accepting the correction**, not
taken on trust: `gh api repos/external-mirrors/akkoma/contents/config?
ref=v3.19.0` confirms `description.exs` really exists (105,482 bytes);
its real content is exactly the structured `%{key:, label:, type:,
description:}` schema format claimed; `gh api search/code` confirms
`lib/pleroma/docs/json.ex` really references it. The blind reviewer's
additional evidence is real, not fabricated -- the first pass's own
investigation was genuinely less thorough here, not wrong about what it
looked at, just incomplete about what else was there to find.

This is exactly what a working blind-review process is FOR: not
"two people guess and average," but a real chance for a second,
independent pass to surface evidence the first pass's own research
depth happened to miss. Recorded as a correction, not smoothed into the
original table silently.

## Cross-cutting findings that matter more than the raw count

1. **The A->B->C->D evidence chain held identically across genuinely
   different languages, formats, AND binding mechanisms**: C#/.NET+JSON
   (kavita), Python+YAML (nebula-lighthouse-service), C+JSON/`jq`-merge
   (transmission), C++/`boost::program_options`+INI (i2pd), PHP-free
   Go+TOML (unpackerr), C+hand-rolled-grammar (unbound), Elixir+`Config.
   Reader` (mobilizon, akkoma), Go+HCL (vault), Haskell+JSON
   (spacecookie), TypeScript+YAML-compiled-to-JSON (misskey), bespoke
   line format (privoxy). Direct, real confirmation of the protocol's
   own pre-committed thesis: format is a locator detail feeding one
   evidence model, never its own abstraction variant.
2. **Binding (B) also varies in concrete mechanism without breaking the
   model** -- a real, useful finding E1 itself never had, since E1 never
   checked B at all: direct `ExecStart` flag (privoxy, transmission,
   unpackerr, vault, i2pd), a bare positional argument (spacecookie), a
   fully IMPLICIT hardcoded-default-path convention with zero CLI
   arguments at all (`nebula-lighthouse-service` -- confirmed only by
   reading the real consumer source, since nothing in the Nix module
   alone proves it), an `environment.etc` activation-time symlink
   (unbound), and a `makeWrapper`-injected env var (mobilizon). A future
   `GeneratedConfigArtifact.binding` field needs to represent at least
   these distinct real shapes, not assume "a substring of `ExecStart`"
   covers every case.
3. **C often required real, non-trivial multi-hop tracing, not a single
   obvious call site** -- `misskey` needed two chained npm scripts with
   a real risk (build-time-only compilation) explicitly ruled out before
   trusting the result; `mobilizon`'s real config-loading module
   (`lib/config_provider.ex`) was only discoverable via `mix.exs`'s own
   `config_providers` release option, not from the Nix module or an
   obvious file path. C is genuinely per-app research, never a generic
   shortcut ("check the docs" would have missed both).
4. **D must be assessed from the CONSUMER's own source, never inferred
   from whether the Nix side happens to be freeform or typed** --
   `i2pd`'s Nix-side `cfg.settings` is a completely freeform attrset
   with zero static schema, yet the REAL consumer (`boost::
   program_options`) has a fully bounded, explicitly-registered
   contract. The inverse also held: `akkoma`'s real failure is entirely
   on the consumer side despite a real, well-defined Nix-side option
   group structure.

## Blind independent review

A second reviewer, given ONLY `protocol.md` and the candidate names --
explicitly instructed not to read `batch1.md`-`batch4.md` or any prior
judgment, and to derive C/D evidence from real upstream source
independently rather than trust any citation -- re-investigated 5 of the
14 candidates from scratch: the three real failures (`libinput`,
`nohang`, `akkoma` -- highest stakes for getting wrong, since a false
negative here would wrongly kill a good vertical and a false positive
would wrongly validate a bad one) plus two successes of different
complexity (`privoxy`, the cleanest SUPPORTED case; `mobilizon`, the
most borderline SUPPORTED case on D).

Full blind-review evidence: `blind-review.md` (the reviewer's own
durable record, written without ever reading `batch1.md`-`batch4.md`).

### Per-link comparison

| candidate | link | pass 1 | blind pass | agree? |
|---|---|---|---|---|
| libinput | A | no | no | yes |
| nohang | A | no | no | yes |
| akkoma | A | proved | proved | yes |
| akkoma | B | proved | proved | yes |
| akkoma | C | proved | proved | yes |
| akkoma | D | no | proved | **no** (corrected to `proved`, see above) |
| privoxy | A | proved | proved | yes |
| privoxy | B | proved | proved | yes |
| privoxy | C | proved | proved | yes |
| privoxy | D | proved | proved | yes |
| mobilizon | A | proved | proved | yes |
| mobilizon | B | proved | proved | yes |
| mobilizon | C | proved | proved | yes |
| mobilizon | D | proved (borderline) | proved | yes |

**13/14 assessed links agree (93%), against E1's own 50% on its first,
looser attempt at this kind of judgment.** The one disagreement
resolved on independent verification in the blind reviewer's favor (see
above) -- not a genuine two-people-read-the-same-evidence-differently
split the way E1's `existing_abstraction`/`requires_new_reusable_adapter`
boundary was. Once the criterion is this concrete (four separately
cited, separately checkable links instead of one vague "does this fit"
judgment), independent reviewers converge. **This IS the qualification
result the audit exists to produce**: the rubric is now operational, not
just a concept two people interpret differently.

## Final decision, against the rule fixed before any candidate was inspected

```
if >= 5 of 14 show A && B && C && D
   AND inter-rater agreement >= 80% (per-link):
      -> build C-E1.2 / GeneratedConfigArtifact vertical
```

**12/14 (86%) show the full chain. Inter-rater agreement is 93%,
comfortably above 80%. Both conditions met, decisively -- this is a
clear GO, not a marginal call.**

Secondary observations, also true and worth carrying into C-E1.2's own
design:

- **A && B && C hold almost universally; D is where the real
  (occasional) friction is** -- `akkoma`'s corrected result actually
  strengthens this: D failing there was closer to "first-pass research
  depth," not "the abstraction can't reach it." No candidate failed at
  B or C specifically once A held -- the two real failures are both
  clean `NO_PRODUCER_PROOF` cases, not partial chains.
- **Support is spread across a genuinely wide format/language/binding
  matrix with the identical A->B->C->D evidence SHAPE throughout**:
  JSON, YAML, TOML, INI, HCL, a bespoke line format, and Elixir's
  `Config` DSL; C#, Python, C++ (twice, two different internal
  mechanisms), Go (twice), Haskell, C (twice, two different
  mechanisms), TypeScript, Elixir (twice). Direct empirical confirmation
  this is one reusable evidence model with format/binding as locator
  details, exactly the design constraint fixed before any evidence was
  looked at -- not five near-identical enum variants waiting to happen.

**C-E1.2 (the real `GeneratedConfigArtifact` CDC vertical) is now
qualified to be built** -- on a real corpus, with the confidence that
this is a genuine reusable abstraction boundary, not a pattern that
merely looked appealing from a distance.
