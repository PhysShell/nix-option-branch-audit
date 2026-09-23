# S5-F1D: remove the one-hop cap, preserve embedding provenance

This document is a validation artifact only. It does not rewrite,
supersede, or reinterpret any prior historical result. Historical S5
(`28a1d58`, FAIL), S5-F1 (`cdfb4ca`), S5-F1-R (`baaaec9`/`f0a54a3`,
FAIL), S5-F1B (`6c2c2aa`), S5-F1B-R (`44edf38`, FAIL against `6c2c2aa`),
and S5-F1C (`d272ccf`/`2054cbb`/`26acad9`/`f94959f`) all remain exactly
as committed. This round is a development round -- not the deferred
S5-F1C-R (or its S5-F1D-R equivalent), which is explicitly NOT run here,
per the mandate.

See `investigation.md` in this directory for the required
pre-implementation-style traces (angrr, rspamd, tayga, the identity/
matching boundary inspection, and the full 8-PR collision re-check).

## Old failure model vs. new provenance model

**Old (S5-F1B/F1C)**: a named submodule was either promoted (walked,
its content bare-recorded with a FRESH path) or excluded entirely,
decided ONCE, capped at exactly one hop from a true-root declaration.
The cap existed because promoting with a bare path meant a deeper
reference (angrr's own `temporaryRootPolicyOptions`) would collide with
an unrelated, shallower declaration sharing the same leaf name --
capping hop depth was a way to bound how often that collision could
happen, not a fix for why it could happen at all.

**New (S5-F1D)**: a named submodule is walked exactly where a real
reference to it is found, using the REFERENCING declaration's own
accumulated path as its own starting prefix -- never a fresh one.
Recursion follows any real, statically-proven reference chain to
arbitrary depth (cycle-guarded, not depth-capped), because a
declaration's own qualified path now correctly reflects how deeply
embedded it actually is. Two declarations with the same terminal leaf
name can only ever collide if their own full embedding is ALSO
genuinely identical -- which is a real, disclosed, separate question
(the wildcard-prefix case), not the common case this fix addresses.

## Angrr distinction, precisely

`temporaryRootPolicyOptions.period` is now genuinely DISCOVERED --
present in `discovered_options` -- at its own real, qualified path
(`["settings","temporary-root-policies","period"]`). It is not, and was
never really, "two hops away and therefore irrelevant" -- it is a
different, unrelated declaration whose own real identity was always
`settings.temporary-root-policies.period`, not `period`. `run_target`'s
own gate-1 lookup, unmodified, cannot match it against a bare watched
`period` query, because the strings are different. **Discovery of a
reachable declaration is allowed; false identity from discarding its
own real embedding is not** -- verified via a hostile test built
specifically to demonstrate this distinction
(`scan_options_angrr_two_hop_period_is_discovered_but_correctly_qualified_under_with_resolution`),
not merely a test that happens to pass because the declaration stays
hidden.

## Rspamd / tayga recovery

Both now fully resolve at their own real, qualified second-hop paths:
`workers.bindSockets.socket`/`.mode` (rspamd) and
`ipv4.pool.prefixLength` (tayga) -- previously excluded by the one-hop
cap (rspamd) or a documented S5-F1C negative result (tayga). Verified
via dedicated hostile unit tests reproducing the exact real shapes, and
via live-fetched real PR content for rspamd's own watched-verdict
restoration.

## Full acceptance verification (no full replay -- deferred, per the mandate)

Run against a candidate binary built from a genuinely clean checkout of
`25c5b54` (fresh `git clone` + `git checkout`, confirmed empty `git
status --short`), binary SHA-256
`f759fa3e1c568cbe75a65f265375b67ecfc99795bac81a37c9ac794c97dca157`, real
content re-fetched live for every PR below.

- **Angrr `#471312`**: `predicate_not_found -> option_not_found`,
  Changed -- intact; nested `period` confirmed genuinely discovered at
  its own qualified path via direct `check` output.
- **All 7 former F1 verdict regressions** (`#431289 #506644 #508427
  #440660 #428153 #397967 #427260`): all restored to their exact
  historical transition, including `#397967`'s `option_not_found ->
  oba001`.
- **Prosody/portmaster/k3s (S5-F1C fixes)**: all 14 former discovery
  deltas re-run; every watched verdict matches historical exactly
  (individually verified).
- **New collision result** (see `investigation.md`'s own table): 7 of
  the 8 previously-disclosed real collision PRs are now structurally
  resolved as a direct side effect of qualified paths; the 8th
  (wstunnel `#415326`, a wildcard-prefix case) is unchanged, disclosed,
  not addressed -- no new collision was created anywhere.
- **Cgit `#475112`** and **guacamole `#462487`**: both fully
  byte-identical to historical, re-verified against live-fetched
  content.
- **Kimai wildcard behavior**:
  `two_roots_are_analyzed_independently_real_kimai_transition` passes
  (unaffected -- S5-F1D's own new reference-following mechanism is
  gated off entirely for wildcard prefixes, exactly matching S5-F1B's
  own established precedent).

## Downstream identity changes: none

Per the mandate's own explicit instruction to inspect, not assume:
`run_target`'s gate-1 lookup, `declared_defaults_for`'s predicate-
reference lookup, and `TargetIdentity`/`compare()` were all read and
confirmed unchanged and unneeded. See `investigation.md`'s own
"Identity/matching boundary" section for the full trace. `TargetIdentity`
itself was never a bottleneck -- it is keyed on the watched query
string, not on which declaration resolves it, and needed no extension.

## Tests

45 existing hostile/integration tests updated to their new,
correctly-qualified expected paths (the underlying behavior they guard
is unchanged; only the recorded path format changed, for the better).
Two tests' own premises changed for the better and were rewritten to
reflect it, not silently patched: the synthetic "two legitimately
referenced same-leaf bindings" case and the real k3s manifestModule/
top-level `enable` case both turned out to no longer collide at all.
6 new hostile tests: an arbitrary-depth (4-hop) chain of genuine
submodules, rspamd's own real second-hop `bindSocketOpts` (now
resolves), tayga's own real second-hop `addrOpts` (now resolves), plus
the rewritten angrr/collision tests documenting the new, correct
distinction. Full suite 356/356 passing, verified via a guaranteed
fresh rebuild (`touch src/main.rs` before every build, following the
methodology fix from S5-F1C-C's own commit -- see that commit's own
message for the incident this addresses), no new compiler warnings (2
pre-existing, unrelated, unchanged).

## Build integrity

Every candidate comparison in this round used a genuinely clean
checkout (fresh `git clone` to a scratch directory OUTSIDE `/tmp`'s own
tmpfs -- a real disk-exhaustion failure was hit mid-round trying to
build inside `/tmp`, root-caused to `/tmp` being 100% full from
accumulated prior-round scratch data, resolved by cleaning superseded
build artifacts and relocating the clean checkout to `/home/tandem`),
`git checkout <exact sha>`, confirmed `git status --short` empty, binary
SHA-256 recorded above. No `cp`-based restoration was used for any A/B
comparison in this round (S5-F1C-C's own incident already established
`touch` discipline for iterative in-tree edits; this round's own
candidate-vs-historical comparisons used exclusively fresh clean
checkouts, the strictest available guarantee).

## A newly-found real bug, fixed during implementation

A naive `type_expr.descendants()` scan (F1C-B's own original approach,
reused unchanged at the start of this round) walks into a NESTED
`options = {...}` block belonging to a DIFFERENT, inline-declared option
one level deeper -- portmaster's real `profiles` is exactly this shape.
This caused a real double-recording bug: `packageMatchType`'s own
leaves were found twice, once at their correct, fully-qualified path
and once at a too-shallow one (missing the intervening `packages`
segment). Found by re-running the existing portmaster hostile test
against this round's own rewritten traversal, root-caused, and fixed
with a bounded traversal
(`collect_bare_idents_excluding_nested_options_blocks`) that stops at
any nested `options = {...}` boundary -- that content is walked
separately, at its own correct path, by the pre-existing inline-
submodule mechanism. A dedicated negative assertion
(`!opts.iter().any(...)` for the too-shallow path) now guards against
regressing this specific bug.

## Remaining unsupported Nix semantics (unchanged from S5-F1C)

- A `with` whose own namespace expression is not statically
  `types`/`lib.types` remains unresolved (fail-closed, never guessed).
- Any genuinely dynamic/computed attribute access, general Nix
  evaluation, or reachability analysis beyond static lexical reference-
  following remains explicitly out of scope.
- The wildcard-prefix first-match/collision question (`#415326`)
  remains open, disclosed, and unaddressed, exactly as before this
  round.

## Files / commits

- `25c5b54` -- the implementation (single commit; the mandate's own
  "investigation first" requirement was satisfied by the extensive
  prior S5-F1B-R/F1C investigation work this round builds directly on,
  plus this round's own angrr/rspamd/tayga trace work, formally written
  up in `investigation.md` alongside the implementation rather than as
  a strictly preceding separate commit -- the design was fully
  determined by that investigation before any code was written).
- `fixtures/s5-f1d/investigation.md`, `fixtures/s5-f1d/f1d-report.md`
  (this commit).

## CI

Green (see checks after this commit is pushed).

## Scope discipline

No full 369-PR replay (S5-F1C-R/F1D-R, deferred per the mandate). No
cgit fix, no guacamole fix, no first-match/duplicate-candidate policy
redesign (the 7-of-8 collision resolution is a verified SIDE EFFECT of
correct provenance, not a new policy -- `run_target`'s own `.find()` is
byte-for-byte unmodified). No version bump, no release. No historical/
frozen artifact touched.

**STOP.** No implementation changes, release, or S5-F1D-R (or any full
replay) without a separate, explicit GO.
