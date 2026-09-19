# E1: frozen-code holdout generalization audit -- results

Full protocol: `protocol.md` (pre-registered before any candidate was
inspected, amended once after batches 1-2 reported, before batches 3-4
were read -- see that file's own "Amendment" section for exactly what
changed and when). Frozen binary: commit `c8e42a1`. Population pinned
at `NixOS/nixpkgs` tree `68740713a1d5904edf9ba92a998a522b1b6ce080`.
Raw per-batch write-ups (free-prose, the form batches 1-2 were briefed
under before the amendment): `batch1.md`, `batch2.md`, `batch3.md`,
`batch4.md`.

**Population caveat, stated up front, not buried**: this holdout is
every real NixOS service module with a MATCHING top-level
`nixos/tests/X.nix` file -- i.e. already module-level test-covered.
Necessary for OBA (no test, no branch evidence to check at all) but
means the holdout is more test-friendly than nixpkgs as a whole. The
honest conclusion below is **"generalizes within NixOS service modules
that already have a matching module-level test,"** never "generalizes
across 40 random NixOS services" or "across nixpkgs."

## Headline result

**E1-OBA (objective, from the real, frozen, unmodified binary)**: of 40
holdout candidates -- none of which played any role in building this
project, drawn by a pre-registered mechanical process, zero legitimate
skips taken -- **6 candidate-level PASS (15%), 2 FINDING (5%), 32
INCONCLUSIVE (80%), 0 TOOL_ERROR.** Both findings survive manual
verification against the real source (100% precision on n=2). One of
the 6 PASSes (`xandikos`) is right-answer-wrong-mechanism: a real,
demonstrated false-positive-CAPABLE bug, not a clean pass (see "Bug F /
GAP-4" below) -- so the honestly-clean PASS count is 5, not 6.

The 80% inconclusive rate is NOT diffuse noise. **84% of all
inconclusive candidates (27/32) trace to ONE root cause**: nixpkgs's
own standard `mkEnableOption` helper (and its siblings
`mkPackageOption`, and an `mkEnableOption ... // {...}` override-merge)
is completely invisible to the declaration scanner (`scan_options`).
This is exactly the "one repeating predicate shape" case the audit was
designed to distinguish from "35% for 19 different reasons" -- and it's
the good/actionable case, a clean next fix target, not a diagnosis of
diffuse fragility. A second, independent root cause (nested
`options = { services.X = {...}; };` path-doubling) contributes to
10/32 (31%, overlapping with the first). The remaining causes are true
one-offs -- five distinct, independently-confirmed structural gaps,
each hit exactly once in this 40-candidate draw.

**E1-CDC (expert structural classification against a frozen rubric,
explicitly NOT executable evidence -- an adapter-fit hypothesis)**: of
35 candidates excluding `not_applicable`/`cannot_determine` (4 + 1),
first-pass **special-case pressure = 13/35 = 37%**, **adapter reuse =
22/35 = 63%**. A blind double-review of 10 of the 40 candidates (see
"Double-independent-review subset" below) disagreed with the first
pass on 5/10 (50%) -- every disagreement in the direction of the
second pass being MORE optimistic about existing-abstraction fit,
concentrated specifically on the `existing_abstraction_*` vs.
`requires_new_reusable_adapter` boundary. Re-rating just those 3
affected candidates (of 40) moves the headline to 29%/71%. **The
honest number is "roughly two-thirds adapter reuse, one-third
special-case pressure, ±8 points depending on rater,"** not a precise
37%/63% -- read as a genuine finding about the rubric's own current
sharpness, not noise to average away.

## E1-OBA: verdict counts and root-cause taxonomy

### Candidate-level (40 candidates, best-outcome-wins: FINDING > PASS > INCONCLUSIVE)

| outcome | count | % |
|---|---|---|
| FINDING (manually verified genuine) | 2 | 5% |
| PASS (clean) | 5 | 12.5% |
| PASS (right-answer-wrong-mechanism -- see GAP-4) | 1 | 2.5% |
| INCONCLUSIVE | 32 | 80% |
| TOOL_ERROR | 0 | 0% |

### Watched-path-level (62 real predicate checks across the 40 candidates -- some candidates watch multiple options; independently re-derived from a fresh real run of all four batch manifests against the frozen binary, not hand-tallied from the batch write-ups -- an earlier hand-count of 61 was off by one, caught by this re-run)

| outcome | count | breakdown |
|---|---|---|
| FINDING (`OBA001`) | 2 | — |
| PASS | 8 | — |
| INCONCLUSIVE | 52 | `OptionNotFound` 47, `PredicateNotFound` 4, `TestConfigUnresolved` 1 |

### Root-cause taxonomy for the 32 inconclusive candidates

Canonical names below (batches 1-4 independently named the same two
dominant bugs differently -- "Bug A"/"Bug B" in one batch is "Gap B"/
"Gap A" in another; normalized here, not re-litigated):

| gap | mechanism | candidates hit (of 32 inconclusive) | pre-registered taxonomy bucket |
|---|---|---|---|
| **GAP-1** | `mkEnableOption`/`mkPackageOption`/an `mkEnableOption ... // {...}` merge -- never recognized as a declaration at all (`is_mk_option_call` only matches literal `mkOption`) | **27/32 (84%)** | doesn't fit cleanly -- closest is `other`; see note below |
| GAP-2 | nested `options = { services.X = {...}; };` idiom -- the path accumulator never resets at the option_prefix boundary, so real options are stored at absolute paths `watched_path` can never match | 10/32 (31%, overlaps GAP-1) | `path_layout_issue` (fits cleanly) |
| GAP-3 | `with types;`/`with lib;` wrapping an ENTIRE options block makes it invisible, not just one entry | 1/32 (jitsi-meet) | `other` |
| GAP-4 | nested submodule option blocks walked with a fresh, EMPTY path -- same-named leaves anywhere else in the file silently collide; proven false-positive-capable (see below) | 1/32 counted as inconclusive (misskey, at scale); ALSO produced a flawed PASS (xandikos, not in the 32) | `other` |
| GAP-5 | `cfg` bound inside `config`'s own value (not the module's top-level `let`) -- `resolve_cfg_root` can't reach it, so the WHOLE declaration block is skipped | 1/32 (pomerium) | `other` |
| GAP-6 | empty list/attrset literal (`!= []`/`!= {}`) not representable by `ValueExpr::Literal` (`Null`/`Bool`/`Str` only) -- a predicate-LOWERING gap, not declaration-scanning | 1/32 (bees; also hit by `flame`'s one inconclusive watch inside an otherwise-FINDING candidate) | `unsupported_predicate` (fits cleanly) |
| GAP-7 | a predicate mediated through a plain function call (`filterAttrs`/`mapAttrs'`), never a textual `mkIf`/`optional*` site at all | 1/32 (authelia, secondary to GAP-1) | `unsupported_predicate` |
| (not a bug) `with cfg;` scope | documented, pre-existing `ResolveFailure::UnsupportedScope("with")`, confirmed for the FIRST time on real code (previously only synthetic H2 fixtures) | 1/32 (endlessh, secondary to GAP-1) | `unresolved_config` |
| (not a bug) import opacity | the opacity gate correctly refusing a conclusion when test config lives behind an unresolved `imports = [...]`, confirmed on real code | 1/32 (monado, secondary to GAP-1) | `unresolved_config` |

**A finding about the taxonomy itself, not just the tool**: the
dominant real cause (GAP-1, 84% of inconclusives) doesn't map cleanly
onto ANY of the five pre-registered `oba_inconclusive_reason` buckets
(`unsupported_predicate`/`unresolved_config`/
`manifest_construction_ambiguity`/`path_layout_issue`/`other`) --
it's not a path mismatch (the option is never found AT ALL, not found
at the wrong path) and not an unresolved predicate (gate 2/3 never
even runs). The taxonomy needs a sixth bucket,
`declaration_shape_unrecognized`, to describe this honestly. Recorded
as an audit finding, not silently forced into the nearest-fitting
label.

### GAP-4 in detail: the one real, demonstrated soundness bug

Not a coverage gap -- a real false-positive-CAPABLE bug. Proven two
ways: (1) in the wild, `xandikos`'s reported real `PASS` on `enable` is
backed by the WRONG declaration (`services.xandikos.nginx.enable`, an
unrelated nested submodule option, not `services.xandikos.enable`
itself) -- numerically harmless here only because the two defaults
happen to coincide; (2) an isolated, minimal, vendored reproducer
(`fixtures/e1-holdout-audit/_bisect/`, `targets/e1-bisect.toml`) shows
this conflation producing a demonstrated FALSE `OBA001` finding on a
case that is actually a clean, real, opposite-of-default PASS. Not
fixed during E1 (freeze held); flagged as the single highest-priority
follow-up this audit produced.

### Real, verified true positives (evidence the pipeline works when it reaches its own predicate/alias layer)

- `flame`/`openFirewall`, `convos`/`reverseProxy`: both real `FINDING`
  (OBA001) results, both manually verified genuine against the real
  module/test source -- neither test ever exercises the branch opposite
  its default.
- `peerflix`/`enable`, `kthxbye`/{`openFirewall`,`logJSON`,`maxDuration`},
  `pufferpanel`/`enable`: real, clean `PASS` results on completely
  unfamiliar code -- including `kthxbye`'s `maxDuration`, a genuine
  `Not(Eq(Ref, Literal(Null)))` (`!= null`) compound witnessed correctly,
  and `convos`'s `reverseProxy`, a scalar `if/then/else` VALUE expression
  (not `mkIf`-wrapped) recognized as a real predicate site -- both
  broader real-world predicate recognition than the H1-H2 corpus itself
  had directly confirmed before E1.
- The import-opacity gate (`monado`) and the `with cfg;` scope
  limitation (`endlessh`) both correctly refused a conclusion on real,
  unfamiliar code for the first time -- previously confirmed only by
  synthetic H2 fixtures.

## E1-CDC: structural fit classification (frozen rubric, expert
judgment, not executable evidence)

| classification | count | candidates |
|---|---|---|
| `existing_abstraction_exact` | 15 | cadvisor, xandikos, svnserve, matterjs-server, ferretdb, endlessh, flame, peerflix, coturn, convos, bees, kthxbye, retroarch, pomerium, pufferpanel |
| `existing_abstraction_new_locator` | 7 | send, akkoma, vault, spacecookie, unpackerr, unbound, mobilizon |
| `requires_new_reusable_adapter` | 12 | libinput, privoxy, esphome, nohang, misskey, kavita, nebula-lighthouse-service, nmtrust, transmission, i2pd, authelia, pocket-id |
| `requires_app_specific_logic` | 1 | jitsi-meet |
| `not_applicable` | 4 | ringboard, monado, nimdow, omada |
| `cannot_determine` | 1 | tor |

**special-case pressure = (12 + 1) / (40 - 4 - 1) = 13/35 = 37%**
**adapter reuse = (15 + 7) / 35 = 22/35 = 63%**

Two borderline calls flagged explicitly (backfilled from batch
free-prose, not hard executable results -- exactly what the double-
review subset below checks): `authelia`'s lookup-table-keyed env-var
naming and `pocket-id`'s `systemd-creds`-backed runtime-resolved
credentials were both classified `requires_new_reusable_adapter` on the
judgment that a future app sharing either exact mechanism is plausible,
not certain.

**Recurring "new-shape-needed" pattern, most load-bearing for a future
H-stage**: a generated, multi-key settings FILE (JSON/YAML/TOML/HCL/
INI/Elixir-conf/bash-array), structurally different from every existing
CDC abstraction (`EvaluatedLiteral` checks ONE literal value;
`FlatEnvVars` assumes a flat, discrete env-var set; `CliContract`
extracts flag NAMES, not nested config content) -- appears across BOTH
`requires_new_reusable_adapter` (libinput/privoxy/nohang/misskey/
kavita/nebula-lighthouse-service/transmission/i2pd -- 8 candidates
where no format-specific locator exists yet at all) AND
`existing_abstraction_new_locator` (akkoma/vault/spacecookie/
unpackerr/unbound/mobilizon -- 6 candidates where the SHAPE already
fits `EvaluatedLiteral` but the specific format has no parser). 14 of
40 candidates (35%) are some flavor of "a real config file gets
generated and read back, in a format this project has never actually
parsed" -- the single most common structural pattern in this entire
holdout, dwarfing DSN-in-string (K1's own original case) and CLI-argv
(K4's case) combined.

## Double-independent-review subset

A second, independent pass (blind to this document and to
`batch{1..4}.md` -- given only the real vendored source and the frozen
rubric, explicitly instructed not to read any prior classification)
re-classified 10 of the 40 candidates' `cdc_fit_classification`:
cadvisor, send, privoxy, ferretdb, ringboard, akkoma, vault, authelia,
pocket-id, omada.

**Result: 5/10 disagreement (50%) -- and it is NOT random noise, it is
concentrated exactly on the boundary the user flagged as the real
risk before this review ran: `existing_abstraction_*` vs
`requires_new_reusable_adapter`.**

| candidate | pass 1 (this document, backfilled from batch free-prose) | pass 2 (blind) | agree? |
|---|---|---|---|
| cadvisor | existing_abstraction_exact | existing_abstraction_exact | yes |
| send | existing_abstraction_new_locator | existing_abstraction_exact | **no** |
| privoxy | requires_new_reusable_adapter | existing_abstraction_exact | **no** |
| ferretdb | existing_abstraction_exact | existing_abstraction_exact | yes |
| ringboard | not_applicable | not_applicable | yes |
| akkoma | existing_abstraction_new_locator | existing_abstraction_new_locator | yes |
| vault | existing_abstraction_new_locator | existing_abstraction_new_locator | yes |
| authelia | requires_new_reusable_adapter | existing_abstraction_new_locator | **no** |
| pocket-id | requires_new_reusable_adapter | existing_abstraction_exact | **no** |
| omada | not_applicable | not_applicable | yes |

Every single disagreement runs the same direction: pass 1's three
`requires_new_reusable_adapter` calls in this subset (privoxy,
authelia, pocket-id) were all reclassified more optimistically by pass
2 (two to `exact`, one to `new_locator`), and pass 1's one
`new_locator` call (send) was reclassified all the way to `exact`. Pass
2's own stated reasoning: a flat, line-based, or dotenv-style config
surface counts as `exact` reuse of `FlatEnvVars`/`EvaluatedLiteral`
regardless of the SPECIFIC delivery mechanism (a `writeText` file, a
`keyValue`-formatted `EnvironmentFile`, a unit's own `environment=`),
where pass 1 had treated a different delivery mechanism (e.g.
pocket-id's `systemd-creds`-backed runtime secret indirection, or
authelia's lookup-table-keyed env-var naming) as evidence of a
qualitatively new shape rather than a new call site on an old one.

**Conclusion, stated as the user's own pre-committed test asked for**:
the rubric, as currently worded, is NOT yet operationally sharp at the
`existing_abstraction_*` / `requires_new_reusable_adapter` boundary --
different raters draw that line in different places on real,
non-synthetic evidence, at a rate too high to treat the headline 37%/
63% split as precise. Applying pass 2's more optimistic reading to just
these 3 disagreements (leaving the other 30 candidates' pass-1
classifications untouched) would move the headline from **37%
special-case pressure / 63% adapter reuse to 29% / 71%** -- an 8-point
swing from re-rating 3 of 40 candidates. The honest headline number is
therefore **"roughly two-thirds adapter reuse, one-third special-case
pressure, ±8 points depending on rater,"** not a precise 63%/37% -- and
the rubric itself, specifically the exact/new-reusable-adapter
boundary, is the concrete next thing to sharpen before trusting this
metric at finer resolution than that.

## What this audit does and doesn't license concluding

**Licensed**: within NixOS service modules that already have a
matching module-level test, OBA's declaration-scanning layer is the
real bottleneck (84% of inconclusives trace to one fixable gap), not
the predicate/alias/counterfactual layer H2 built -- which, when
actually reached, correctly classified genuinely novel real predicate
shapes (a bare scalar `if/then/else`, an `Or` compound, a `!= null`
compound) it had never been directly exercised against before. CDC's
existing abstractions cover roughly two-thirds of applicable holdout
cases structurally, with the clearest, most load-bearing gap being
generated multi-key config files -- a concrete, data-driven candidate
for whatever comes after this audit, not a guess.

**Not licensed**: "OBA generalizes to nixpkgs" (the population is
scoped to test-covered modules only, see the caveat above). "CDC
generalizes" in any executable sense (the CDC numbers are a structural
hypothesis pass, not a single line of new adapter code was run against
real evidence). "The tool is 80% broken" (nearly all of that 80% is one
root cause with a clear fix path, confirmed independently four separate
times across four independent investigators).
