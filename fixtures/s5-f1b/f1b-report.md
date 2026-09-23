# S5-F1B: provenance-aware replacement for F1's over-broad exclusion

This document is a validation artifact only. It does not rewrite,
supersede, or reinterpret any prior historical result. Historical S5
(verdict **FAIL**, `28a1d58`), S5-F1 (`cdfb4ca`), and S5-F1-R
(`baaaec9`/`f0a54a3`, verdict **FAIL** against `cdfb4ca`) all remain
exactly as committed. This round replaces `cdfb4ca`'s own exclusion
predicate with a provenance-aware one, per S5-F1-R's own finding: F1-R
disproved the METHOD (syntactic named-binding exclusion), not the
underlying HYPOTHESIS (an unrelated named submodule must not impersonate
a removed option via same-leaf-name matching).

See `investigation.md` in this directory for the required pre-implementation
trace (angrr + one prosody + one drupal + one fedimintd regression).

## What changed

`scan_options` (`src/main.rs`) is restructured into two passes:

- **Pass 1** walks every true-root declaration exactly as before F1 ever
  existed, collecting real `OptionDecl`s and, for every real `mkOption`
  found, resolving any bare identifiers in its own `type =` field (via
  the pre-existing, scope-aware `resolve_ident_binding` -- no new
  resolution mechanism) into a `referenced: HashSet<TextRange>`. A named
  `let`/`rec`-bound `options = {...}` candidate is deferred, not decided,
  here.
- **Pass 2** promotes a deferred candidate if and only if its own
  enclosing named binding's value is present in `referenced` -- i.e. it
  is referenced, by name, from a true-root declaration's own `type =`
  field, exactly one hop away. A promoted candidate's own declarations
  are walked and recorded, but its own `type =` references are collected
  into a throwaway set, never fed back into `referenced` -- promotion
  does not cascade. See `investigation.md`'s "Why one hop, not a
  fixpoint" section for why this matters (an earlier, unbounded-fixpoint
  version of this fix passed every test written for it but had a real,
  demonstrated two-hop reintroduction path for angrr's own defect,
  caught only by tracing angrr's actual module, not by any test).

New helpers: `resolve_type_reference`, `collect_type_reference_ranges`,
`enclosing_named_binding_value`. All reuse existing machinery
(`resolve_ident_binding`, `mk_option_field`) rather than introducing a
second resolution path.

## Verification performed this round

### Safety property A -- angrr stays fixed

Re-run against the real, committed `fixtures/synthetic/f1-angrr-period-collision/`
fixture: `changed:1, predicate_not_found->option_not_found`, matching
F1's own intended fix exactly.

### Safety property B -- all 7 real F1-R regressions restored

Real content re-fetched fresh (via `gh api`) at each PR's exact frozen
`base_sha`/`head_sha`, reusing each PR's original `targets.toml`
verbatim, run against a freshly rebuilt candidate binary:

| PR | historical transition | candidate (this round) |
|---|---|---|
| `#431289` (prosody `moderation`) | `option_not_found->predicate_not_found` | same |
| `#506644` (drupal `webRoot`) | `option_not_found->predicate_not_found` | same |
| `#508427` (drupal `configRoot`) | `option_not_found->predicate_not_found` | same |
| `#440660` (prosody `http_external_url`) | `option_not_found->predicate_not_found` | same |
| `#428153` (drupal `configSyncDir`) | `option_not_found->predicate_not_found` | same |
| `#397967` (fedimintd `api_ws.openFirewall`) | `option_not_found->oba001` | same |
| `#427260` (drupal `privateFilesDir`) | `option_not_found->predicate_not_found` | same |

All 7 exact matches, including `#397967`, the already-adjudicated real
S5 finding F1 would otherwise have hidden.

### Safety property C -- all 14 cosmetic-only deltas re-verified

Real content re-fetched fresh for all 14 PRs F1-R classified as
`unexpected_semantic_delta` with `verdict_delta: false`
(`429967, 494314, 484133, 557329, 374017, 260551, 432528, 438285,
423934, 463443, 401840, 398993, 480839, 433539`). **All 14 watched
verdicts match their historical S5 transition exactly** (including the
two named explicitly in `classification-notes.json` as already-adjudicated
S5 findings in their own right: `#260551` prosody `checkConfig`
`option_not_found->oba001`, and `#432528` tayga `wkpfStrict`
`option_not_found->pass`).

Six of the fourteen (`438285`, `423934`, `401840`, `398993`, `480839`,
`433539`) still show a `discovered_options`-count delta from historical
(0 now vs. historically 3, for the underlying frigate.nix /
nvidia-container-toolkit `default.nix` files) -- traced this round to a
**pre-existing, unrelated gap**: both files' own true-root `options =`
value is wrapped in a form `walk_merge_operands` does not unwrap
(`options.services.frigate = with types; { ... };` for frigate.nix;
`options = let mountType = {...}; in { ... };` for
nvidia-container-toolkit/default.nix -- `walk_merge_operands` only
unwraps a literal attrset or a `//`-merge, not `with` or `let...in`).
Since the true root is invisible to Pass 1 for these two files, `referenced`
stays empty and nothing gets promoted -- the historical "3" was always
disconnected noise (a named submodule's own bare-recorded leaves,
unrelated to `option_prefix`, that happened to survive Pass-1's own
pre-F1 unconditional walk) that could never have matched `run_target`'s
own exact-path gate-1 lookup regardless (confirmed: the recorded paths
share only a coincidental leading segment with `option_prefix`'s own
tail, never the full prefix). Zero functional impact, in either the
historical or the candidate binary. This `with`/`let...in`-unwrapping
gap is unrelated to named-binding provenance and out of scope for
S5-F1B; not fixed here.

### Hostile controls -- full suite result

`cargo test --release`: **334 passed, 0 failed** (220 + 22 + 22 + 15 + 1
+ 54 across the workspace's test binaries).

Two of S5-F1's own original unit tests needed redesigning, not just a
fixture tweak: `scan_options_excludes_ambiguous_same_named_leaves_in_unrelated_named_let_bindings`
and `scan_options_named_let_binding_exclusion_is_order_independent` had
each used two named-binding helpers (`helperA`/`helperB`) BOTH
legitimately referenced by real, prefix-anchored options -- under F1's
own (now-disproven) semantics that looked like "two unrelated bindings,"
but under the correct provenance model both are legitimate and correctly
promoted. Renamed to
`scan_options_excludes_unrelated_but_includes_legitimately_referenced_named_let_bindings`
and `scan_options_named_let_binding_provenance_is_order_independent`,
redesigned so exactly one helper is genuinely unrelated (no real
referrer) and one is legitimately referenced (a differently-named leaf,
to avoid conflating with the collision case below).

New tests added this round:

- `scan_options_two_legitimately_referenced_named_bindings_sharing_a_leaf_name_is_a_disclosed_limitation`
  -- a genuinely new edge case surfaced by this fix, not demonstrated
  anywhere in the real S5 corpus: two DIFFERENT named submodules, each
  legitimately referenced by its own real option, happen to declare a
  leaf with the same name. `scan_options` records all of them; it does
  not itself deduplicate or flag the collision -- `run_target`'s own
  gate-1 `.find()` (unmodified, pre-existing) picks whichever is first
  in vector order, exactly its behavior for any other kind of duplicate
  path. Disclosed, not resolved -- a real but separate architectural
  question (should `scan_options` or `run_target` surface an explicit
  ambiguous/inconclusive signal here?) from S5-F1B's own narrow mandate.
  This is the honest answer to hostile control (e); it is not claimed to
  be "handled" beyond disclosure.
- `scan_options_rec_bound_submodule_leaf_is_also_excluded_from_concrete_prefix_match`'s
  own doc comment updated: it still passes, but for a different reason
  than before -- not because the provenance logic treats `rec` specially,
  but because `resolve_ident_binding` already, independently, refuses to
  resolve any name bound inside a `rec {...}` (pre-existing, conservative,
  fail-closed). A real nixpkgs module using `rec {}` for this exact shape
  would still be over-excluded -- not demonstrated in the real corpus
  (prosody/drupal/fedimintd all use `let ... in`), disclosed here rather
  than silently assumed away.

Also fixed: the integration test `tests/diff_cli.rs`'s
`synthetic_generic_collision_removed_field_plus_unrelated_survivor_is_not_unchanged`
(hostile control (b), generic-names collision) had the exact same latent
issue -- its own `unrelatedHelperOptions` was, in fact, legitimately
referenced by a real `profiles` option (`attrsOf (submodule
unrelatedHelperOptions)`), the same shape as prosody's `mucOpts`.
Redesigned so `profiles` uses an inline, differently-shaped anonymous
submodule instead, leaving `unrelatedHelperOptions` genuinely orphaned
(never referenced anywhere) -- the real analog of angrr's defect.

### Build warnings

`cargo build --release`: 2 warnings, both pre-existing and unrelated
(`lower_value_expr` dead code, `ChangeKind` unused variants) -- identical
to the pre-F1B baseline. No new warnings.

### Clippy -- could not be run in this environment (disclosed, not silently skipped)

`cargo-clippy` is only available from the `1.94.1` toolchain in this
environment (the project's pinned `nightly-2026-08-21` toolchain has no
clippy component installed). Invoking `1.94.1`'s `cargo-clippy` against
this project -- with or without a shared `CARGO_TARGET_DIR` -- fails to
even COMPILE the bin target (205 raw `rustc` errors, none touching any
code this fix modified, e.g. type mismatches around `record_bucket` at
`src/main.rs:5845`, far from `scan_options`), a pre-existing toolchain/
dependency-resolution incompatibility unrelated to S5-F1B. This is
disclosed rather than silently claimed as "0 new clippy warnings"; it
was not fixed here (out of scope). Compiler warnings (above) are the
verification actually performed.

## Anchor checks

- **`#471312` (angrr) stays fixed** -- confirmed above.
- **cgit `#475112` and guacamole `#462487`** -- untouched; this round
  changes only `scan_options`'s named-binding provenance logic and the
  test suite, nothing in the code paths those two defects live in.
- **No historical/frozen artifact modified** -- `fixtures/s5-live-pr-shadow/`,
  `fixtures/s5-f1-r/` untouched; all verification this round ran from
  `/tmp` scratch fetches, never writing into those directories.

## Scope discipline

Per the mandate: this round is the implementation fix, its own targeted/
redesigned tests, and full-suite verification -- not the next full
369-PR corpus replay (S5-F1B-R), which requires a separate, later GO.
`#397967` (fedimintd) is flagged here, per the user's own note, as the
PR that should become a permanent regression canary once S5-F1B-R runs.
