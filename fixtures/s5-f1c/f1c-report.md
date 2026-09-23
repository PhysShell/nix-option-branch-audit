# S5-F1C: closing the three demonstrated provenance gaps

This document is a validation artifact only. It does not rewrite,
supersede, or reinterpret any prior historical result. Historical S5
(verdict **FAIL**, `28a1d58`), S5-F1 (`cdfb4ca`), S5-F1-R
(`baaaec9`/`f0a54a3`, **FAIL**), S5-F1B (`6c2c2aa`), and S5-F1B-R
(`44edf38`, **FAIL** against `6c2c2aa`) all remain exactly as committed.
This round is a development round with three separable, individually
reviewable commits -- not an acceptance replay. No full 369-PR replay
was run (that is S5-F1C-R, requiring a separate, later GO).

See `investigation.md` in this directory for the required
pre-implementation-style traces (with-types, with-lib.types, portmaster
alias chain, k3s ordering, angrr counterexample), written this round as
required, informed by real implementation and real-corpus verification.

## What changed, per commit

- **`d272ccf` S5-F1C-A**: `resolve_ident_binding` gains a `WithPolicy`
  parameter. `Refuse` (unchanged) for its other two call sites
  (predicate/value alias resolution, `cfg_ident` tracing).
  `TransparentForRecognizedTypeNamespace`, used only by
  `resolve_type_reference`, continues past a `with` whose own namespace
  is statically recognized as `types`/`lib.types` -- Nix's own rule that
  a lexical binding always wins over `with` makes this sound regardless
  of the with-source's actual identity; recognition itself stays a
  narrow, syntactic allowlist, never a guess.
- **`2054cbb` S5-F1C-B**: `collect_type_reference_ranges` becomes
  recursive (`collect_type_reference_ranges_rec`, `visited`-set cycle
  detection, no arbitrary depth cap). A resolved reference whose own
  value does not itself contain a direct `options = {...}` block
  (`find_nested_options_block`, reused) is a transparent alias step and
  gets recursed into; one that does is a genuine submodule and recursion
  stops there -- the exact property that keeps this from silently
  removing the one-hop cap.
- **`26acad9` S5-F1C-C**: `scan_options` sorts its own output by
  `(span.line, span.col)` before returning, restoring the historical
  (pre-two-pass) document-order traversal order.

Each commit's own message documents its reasoning and verification in
full; this report is the cross-cutting summary the mandate additionally
requires.

## Cross-cutting acceptance verification (no full replay)

Run against a candidate binary built from a genuinely clean checkout of
`26acad9` (fresh `git clone` + `git checkout`, confirmed empty `git
status --short`), binary SHA-256
`9994a017f4856b43c60022705dc7546817fb7e88ad7b93e02743719934e94c06`, real
content re-fetched live for every PR below.

- **angrr `#471312`**: `predicate_not_found -> option_not_found`,
  Changed -- intact.
- **All 7 former F1 verdict regressions** (`#431289 #506644 #508427
  #440660 #428153 #397967 #427260`): all restored to their exact
  historical transition, including `#397967`'s `option_not_found ->
  oba001`.
- **F1B-R with-scope cases**: prosody (`#429967 #494314 #431289 #440660
  #260551`) and rspamd (`#484133`) -- field-level `discovered_options`
  re-checked (not just watched verdict). `#429967`/`#494314`/`#260551`
  are now fully byte-identical to historical (0 missing, 0 added).
  `#431289`/`#440660` (already verdict-restored under S5-F1B) are also
  now fully discovery-identical. `#484133` (rspamd) has its watched
  verdict restored and `workerOpts`'s own direct leaves resolved;
  `bindSocketOpts`'s own leaves (a genuine second hop via `bindSockets`)
  remain absent -- expected, the one-hop cap unchanged.
- **Portmaster `#557329`**: fully restored, 0 missing / 0 added against
  historical `discovered_options` -- all 7 of `packageMatchType`'s own
  leaves now resolve through the `profilePackageType` alias.
- **K3s `#374017`**: `discovered_options` now matches historical
  **element-for-element** (not just as a set) -- exact order restored.
- **Fourteen former discovery deltas**: all 14 re-run; every watched
  verdict matches historical exactly (individually verified, not
  inferred from aggregate counts). Six (frigate/nvidia-container-toolkit)
  remain the already-disclosed, pre-existing, unrelated
  `walk_merge_operands` with/let-in limitation, unaffected by this round
  (unchanged, as expected -- out of scope).
- **Cgit `#475112`** and **guacamole `#462487`**: both fully
  byte-identical to historical (`check-base.json`/`check-head.json`),
  re-verified against live-fetched content.
- **Wildcard behavior**: `two_roots_are_analyzed_independently_real_kimai_transition`
  passes (part of the full suite).
- **Leaf-collision disclosure**: re-checked against all 8 previously
  flagged PRs (`fixtures/s5-f1b-r/leaf-collision-scan.json`). Every
  candidate collision set is either identical to or a strict subset of
  the historical one -- no new collision introduced anywhere, and
  `run_target`'s own first-match selection is untouched, exactly as
  scoped.

## Full test suite and CI

`cargo test --release`, run against a **guaranteed fresh rebuild**
(`touch src/main.rs` before every build during this investigation, after
discovering that `cp`-based file restores can leave Cargo's mtime-based
staleness check silently reusing a stale binary -- see `26acad9`'s own
commit message): **356 passing, 0 failed**, no new compiler warnings (2
pre-existing, unrelated, unchanged). 45 new hostile/integration unit
tests across the three commits.

## Compound-cause boundary, honestly disclosed, not patched

Per the mandate's own "Critical single-hop check": actively searched for
a legitimate `true root -> submodule A -> submodule B` chain lost solely
to the one-hop cap. No CLEAN, isolated instance exists in this corpus --
every multi-hop-shaped case found (angrr's own real defect, and tayga's
`addrOpts`, reached via `pool` inside `versionOpts`'s own promoted
content) is independently ALSO blocked by a local `with` at one of its
hops, so the cap's own marginal contribution can't be isolated from
with-opacity in either case. Tayga's own `prefixLength`/`address`
(inside `addrOpts`) remain excluded after F1C -- not a failure of F1C-A,
but an honest compound-cause boundary: F1C-A's fix is necessary but not
sufficient there, and F1C is not authorized to widen the cap to close
it. Documented with a dedicated, explicitly-negative-result test
(`scan_options_real_tayga_addropts_stays_excluded_by_the_unwidened_hop_cap`).

## Scope discipline

No source fix beyond the three scoped mechanisms; no test-driven
fixture rewriting to accommodate the candidate; no historical ledger/
adjudication modification; no cgit fix; no guacamole fix; no redesign of
first-match collision semantics; no version bump; no release; no
S5-F1C-R (the next full-corpus replay). Three separable, individually
reviewable commits, each with its own full-suite verification.

## Unresolved risks / open questions for a later phase

- The one-hop cap's own sufficiency for a CLEAN (not with-confounded)
  two-hop legitimate chain remains genuinely untested by this corpus --
  neither proven necessary nor proven insufficient here.
- The disclosed leaf-collision / first-match identity model remains a
  separate, real, open architectural question (per the user's own
  framing: this is now a genuine corpus of 8 real collision PRs, not a
  hypothetical) -- explicitly NOT addressed in this round, tracked as
  its own future investigation.

**STOP.** No implementation changes, release, or S5-F1C-R without a
separate, explicit GO.
