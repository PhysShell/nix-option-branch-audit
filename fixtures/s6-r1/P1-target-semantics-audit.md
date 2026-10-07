# S6-R1 P1: target-semantics audit -- `TARGET_INVALID_DEFECT_RETRACTED`

**Parent**: `fixtures/s6-r1/P1-defect-boundary.md` (`b16cd41`, corrected
`f7674e7`). This document resolves the question that correction left
open: was `#568429`/`#508090`'s own target construction valid per
`oba`'s real public contract? Does NOT change `oba`, fixtures, or
taxonomy. Does NOT expand the real-world corpus beyond these same two
PRs. Does NOT run GO-B or P2.

## The contract, found in the public documentation BEFORE opening `src/main.rs`

`README.md`'s own "P2: the nested `options = { services.X = {...}; };`
idiom fix" section states the general rule directly, independent of
current behavior:

> "...checking, at each recursive step, whether the path accumulated
> SO FAR exactly equals `option_prefix`... The moment it matches,
> everything under that point is walked with a FRESH path, exactly the
> same 'reset to relative' treatment..."

And the S3-F2 entry: "`option_prefix_relative_path` normalizes every
recorded declaration **at its own record site**" -- i.e. every
discovered option's own `path` field is stored RELATIVE to
`option_prefix`, not as an absolute path and not pre-combined with
`watch` by the caller.

**Consequence, derivable from this text alone, before looking at any
PR**: `option_prefix` must name the point where a real `options={...}`
root exists (the true module/root namespace, or -- per GAP-4 -- a
point `find_nested_options_block` itself recurses into); `watch` must
then be the FULL remaining relative path from that root down to the
leaf, as one dotted string (or equivalent multi-segment form) when
multiple hops are involved -- never a partial path that itself stops
at an interior `mkOption`-wrapped container's own name, expecting
`watch` to supply only the last hop. `h2-case15-xandikos` (`targets/
golden.toml`) is consistent with this: it deliberately watches the
WRONG, bare, pre-fix-collision path (`option_prefix=["services",
"xandikos"]`, `watch=["enable"]`) to assert it now correctly returns
`OptionNotFound` -- a negative check of exactly the málformed-target
shape, not a positive example of the correct one.

This was determined entirely from `README.md`'s own documentation and
existing golden tests, before opening `src/main.rs`'s actual
`scan_options`/`walk_options_block` implementation.

## The two variants, run for real

**Current P1 targets** (`targets/s6-r1-p1.toml`, already run in
`fixtures/s6-r1/P1-adjudication-and-accounting.md`): `option_prefix`
ends in the submodule-wrapped container's own name (`"settings"`/
`"environment"`), `watch` = bare final leaf only.

**Alternative targets** (this audit,
`fixtures/s6-r1/P1-target-semantics-evidence/alt-targets-{568429,508090}.toml`):
`option_prefix` stops at the true module root (`["services",
"cliproxyapi"]` / `["services","beszel","agent"]`), `watch` = the FULL
relative dotted path spanning the container and the leaf
(`"settings.server.port"`, `"settings.oauth.auth-dir"`,
`"environment.SKIP_GPU"`, `"environment.GPU_COLLECTOR"`).

Both variants run on the same sha256-verified `oba` `v0.5.0` binary
already verified in `fixtures/s6-r1/P1-defect-confirmation-evidence/`,
against the same real base/head nixpkgs source already fetched for
these two PRs (no corpus expansion, no new PRs).

## Result: the alternative targets succeed on the real head source

Raw output: `fixtures/s6-r1/P1-target-semantics-evidence/alt-{568429,508090}-{base,head}.json`.

| Target | Base | Head |
|---|---|---|
| `settings.server.port` | `OptionNotFound` (correct -- genuinely absent) | **`PredicateNotFound`** -- declaration found at `['settings','server','port']`, but not gated by its own predicate (matches the first reviewer's own blind-answer prediction) |
| `settings.oauth.auth-dir` | `OptionNotFound` (correct) | **`PredicateNotFound`** -- declaration found at `['settings','oauth','auth-dir']` |
| `environment.SKIP_GPU` | `OptionNotFound` (correct -- genuinely absent) | **`PASS`** -- declaration found at `['environment','SKIP_GPU']`, a real witnessed predicate (`lib.optionals (!cfg.environment.SKIP_GPU) cfg.environment.GPU_COLLECTOR`), evidence from the real test file (`nixos/tests/beszel.nix:101`, `environment.SKIP_GPU = true`) |
| `environment.GPU_COLLECTOR` | `OptionNotFound` (correct) | **`PredicateNotFound`** -- declaration found at `['environment','GPU_COLLECTOR']` |

All 4 base-side runs correctly return `OptionNotFound` (the
declarations are genuinely absent pre-PR) -- confirming the
alternative construction isn't simply "more permissive" in a way that
would also wrongly match base; it tracks ground truth both ways.

**This directly contradicts `OptionNotFound`/`discovered_options: []`
on the exact same real head source under the CURRENT P1 target
construction** (`fixtures/s6-r1/P1-defect-confirmation-evidence/
out-{568429,508090}-head.json`) -- same binary, same source, same PR,
different `option_prefix`/`watch` split.

## Verdict

**`TARGET_INVALID_DEFECT_RETRACTED`**

The current P1 targets for `#568429`/`#508090` used an
`option_prefix`/`watch` split that does not match `oba`'s own
documented contract (`option_prefix` ending mid-way through an
interior `mkOption`-wrapped container's own name, rather than at a
true walkable root). Per that same documented contract, applied
correctly, `oba` v0.5.0 DOES discover all 4 declarations in the real
head source, and reaches sensible, evidence-backed verdicts
(`PredicateNotFound` x3, one real witnessed `PASS`) rather than
`OptionNotFound`. There is no discoverability defect here: the tool
was asked a request its own contract does not define as meaningful,
and `OptionNotFound: discovered_options=[]` for an `option_prefix`
that names no real walkable root is the CORRECT, contractual response
to that malformed request, not a bug.

## What this means for `fixtures/s6-r1/P1-defect-confirmation.md`

**`CONFIRMED_NEW_DEFECT` is retracted.** Both the first reviewer and
the independent second reviewer adjudicated the SAME already-frozen,
incorrectly-constructed target record; neither was asked to (and the
adjudication rubric's own scope never asked them to) re-derive or
re-validate the `option_prefix`/`watch` split itself -- they correctly
judged "does the real tool output match what this exact request
should return," and both reached the same, correct conclusion FOR
THAT REQUEST. Two independent reviewers validating a flawed
instrument's reading does not make the reading correct -- exactly the
"independently measuring a crooked thermometer" risk the user named
before authorizing this audit. This is not a failure of the blind
adjudication methodology; it is a target-construction defect
upstream of it, which blinding was never designed to catch (the
adjudication rubric's evidence packet includes the frozen target
record as a GIVEN, not as something either reviewer is asked to
re-derive).

## Boundary characterization (`fixtures/s6-r1/P1-defect-boundary.md`) stands, reframed

The synthetic matrix, minimal PASS/FAIL pair, and mechanism
explanation in the boundary document are now BETTER explained: they
were never characterizing a defect in `oba`'s discovery mechanism at
all -- they were characterizing exactly this same `option_prefix`
contract requirement, on synthetic fixtures, arriving independently at
the same answer this audit now confirms on the real PRs. The boundary
document's own "GAP-4 mapping: inconclusive" is superseded by this
audit's own more specific finding: this is not a GAP-4 gap of any
kind (narrower, broader, or separate) -- it is a target-construction
contract violation, full stop.

## Relevance to a future GO-B (not acted on here, orthogonal per the user's own instruction)

`target-construction-protocol.md`'s own step 4 wording ("`option_prefix`
is the declaration's own path up to but not including its final
segment; `watch` is that final segment, dotted if nested under further
sub-attributes within the same `options={...}` block") is ambiguous
enough to have produced the invalid split here, and the same
ambiguity may plausibly explain some of the 21/28 `INPUT_OR_HARNESS_FAILURE`
cases from brand-new-module PRs (`#443747`/`#568048`) -- **this is
stated as a hypothesis for GO-B to check, not investigated further in
this document**, which does not touch `target-construction-protocol.md`,
`pilot-accounting.py`, or the brand-new-module cases at all.

## What did not happen

No change to `oba`'s source or binary. No change to `README.md`,
`target-construction-protocol.md`, `pilot-accounting.py`, or
`population-and-sampling.py`. No re-expansion of the real-world corpus
beyond `#568429`/`#508090`. No P2 work. No automatic transition into
GO-B.

**`S6_R1_P1_GOD_TARGET_INVALID_DEFECT_RETRACTED`**

**STOP.** No fix. No `README.md` edit. No P2. No automatic
harness-repair. GO-B (harness-repair) requires its own separate,
later, explicit GO, and per the user's own instruction must NOT use
this document's own boundary/semantics finding to silently adjust
target construction -- any such change belongs to GO-B's own explicit
scope, decided there, not inherited wholesale from here.
