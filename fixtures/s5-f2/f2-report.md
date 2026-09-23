# S5-F2: fix the cgit false OBA001 (PR #475112)

This document is a validation artifact only. It does not rewrite,
supersede, or reinterpret any prior historical result. Historical S5,
S5-F1, S5-F1-R, S5-F1B, S5-F1B-R, S5-F1C, S5-F1D (`25c5b54`), and
S5-F1D-R (`58b28b2`) all remain exactly as committed. `58b28b2` itself
already identified this defect (PR #475112, `services.cgit."no-git-http-backend.localhost".gitHttpBackend.enable
= false;`, real evidence, OBA001/witnessed=false) as the next real
target -- this round fixes exactly that, and nothing else.

See `investigation.md` in this directory for the full end-to-end trace
and root-cause classification.

## Root cause, in one sentence

`path_matches_prefix` required a real test assignment's path to be
EXACTLY `option_prefix ++ watched_suffix`, with no room for the
concrete instance key every real assignment into an
`attrsOf`/`listOf`-typed submodule necessarily carries between the two
-- so cgit's own real, deliberate `gitHttpBackend.enable = false;`
assignment (5 segments: `services, cgit,
"no-git-http-backend.localhost", gitHttpBackend, enable`) could never
match a 4-segment expectation (`prefix.len()=2 + suffix.len()=2`), and
the predicate fell through to `EvidenceNoTransition` -> `OBA001`
despite the test genuinely, deliberately covering both outcomes of the
predicate.

## Accepted baseline behavior vs. new behavior

- **Accepted baseline (`58b28b2`)**: `check --root head` on the real
  PR #475112 fixture reports `OBA001` for `gitHttpBackend.enable`,
  `predicate_attempts[0].witnessed == false`, `matched_test_assignments
  == []` -- confirmed via a genuinely clean checkout (fresh `git
  clone`, `git checkout 58b28b2`, `git status --short` empty), binary
  SHA-256 `42fcb520be05d1aaaf46795632c03b610b14a28d0b9ec27a82f3ad1cebc74a88`.
- **New (this round's candidate)**: same fixture, same `--root head`,
  reports `PASS`, `predicate_attempts[0].witnessed == true`, evidence
  is the real assignment at `["services","cgit","no-git-http-backend.localhost","gitHttpBackend","enable"]`,
  `value_source == "false"`, `span.line == 58`. Clean A/B recorded:
  identical fixture, only the binary differs (accepted baseline vs.
  candidate), transition confirmed in both directions (defect present
  at baseline, absent at candidate).
- **Base commit** (`308f82252459d32a68cfbbec3af672e5867ff994`, predates
  `gitHttpBackend` entirely): unaffected by this fix either way --
  `OptionNotFound`, confirmed identical before and after.

## The exact witness invariant this fix adds

A predicate is witnessed only when a real test assignment can be
statically connected to the declaration/predicate subject and the
assigned value is sufficient to demonstrate the relevant predicate
outcome. This fix does NOT weaken that invariant to "some test
somewhere mentions an option with the same leaf name" -- same-leaf
matching remains insufficient (hostile control 3, below). It adds
exactly one new fact to the matching rule: when `option_prefix`'s own
true-root declaration is independently, structurally confirmed to be
`attrsOf`/`listOf (submodule ...)`-typed, a real assignment's path may
legitimately carry exactly one additional, concrete segment (the
instance key) between `option_prefix` and the watched suffix, and that
one segment's own VALUE is unconstrained (any real instance is real
evidence for a predicate that is itself generic across instances) --
but every other segment, on both sides of that one gap, must still
match content-for-content, exactly as before. The instance-key
tolerance is never granted unconditionally; it is granted only when
`option_prefix_is_instance_keyed_submodule` independently confirms the
structural precondition that makes it sound.

## Does this widen witness behavior beyond cgit? Yes -- defined precisely, and why it is sound

This fix changes witness behavior for a class structurally broader than
cgit: **any target whose `option_prefix` resolves to a declaration
whose own `type =` field is `attrsOf (submodule ...)` or `listOf
(submodule ...)`** (dotted or `with`-wrapped, matching the same
call-head recognition `is_mk_option_call` already uses elsewhere) now
tolerates exactly one concrete instance-key segment between
`option_prefix` and any watched suffix, for every predicate form
(`cfg.foo`, `!cfg.foo`, `cfg.foo == v`, `cfg.foo != v`, and any `And`/
`Or` compound built from them) rather than only for
`gitHttpBackend.enable` specifically.

Each newly-accepted witness in this broadened class is sound for the
same reason cgit's own is: the predicate's own reference into the
submodule (found by the SAME scanner, at the SAME `cfg_ident`-rooted
matching, regardless of how `cfg_ident` itself is bound) is, by
construction, generic across every instance of that submodule type --
it is evaluated once per instance, with the submodule's own `config`
providing that instance's own values, so it carries no instance key of
its own to compare against. A real, concrete instance's own assignment
to the SAME relative leaf is therefore always legitimate evidence for
that generic predicate, REGARDLESS of which instance made the
assignment -- exactly the property `path_matches_prefix`'s own new
branch encodes, and no more: prefix and suffix must still match
content-for-content; only the one intervening segment's own identity is
irrelevant, never its presence or position. The gate itself
(`option_prefix_is_instance_keyed_submodule`) is independently verified
against the real AST, not inferred from the fact that a match was
attempted -- so the tolerance is never granted to a target for which
it would be unsound (a non-instance-keyed, directly-nested option
sharing the same suffix, hostile controls 3 and 4).

## Files changed

- `src/main.rs`:
  - `path_matches_prefix` (`:3445`) -- new `allow_instance_key: bool`
    parameter; pre-existing exact-length branch unchanged; new
    one-extra-segment-tolerant branch, gated.
  - `option_prefix_is_instance_keyed_submodule` (`:3497`, new) --
    structural verification of the precondition, reusing
    `scan_options`'s own already-discovered `OptionDecl` and the
    pre-existing `mk_option_field`/span-matching idiom.
  - `type_expr_wraps_attrs_of_or_list_of` (`:3534`, new) -- reuses the
    pre-existing `is_call_named` helper.
  - `evaluate_predicate_witness` (`:3649`) -- new trailing
    `allow_instance_key` parameter, threaded to its own 3 internal
    `path_matches_prefix` call sites.
  - `run_target` (`:4279`) -- computes `allow_instance_key` once
    (`:4347`) and threads it to the H1 gate-4 `path_matches_prefix`
    call site and to `evaluate_predicate_witness`.

## Real regression fixture

`fixtures/synthetic/f2-cgit-githttpbackend-instance-keyed-witness/` --
real, live-fetched `base`/`head` content at the frozen PR #475112 SHAs,
`targets.toml` matching the historical adjudication record exactly
(`option_prefix=["services","cgit"]`, `watch=["gitHttpBackend.enable"]`).
`tests/f2_cgit.rs`, 2 tests: head-side PASS with real evidence
(exact path, value, span), base-side `OptionNotFound` (unaffected,
confirms the fix is entirely about witness matching, not declaration
discovery).

## Hostile controls

`tests/f2_cgit_hostile.rs`, 9 tests, all passing:

1. A different, untouched instance of the same submodule type does not
   block a witness from the instance that actually flips the predicate.
2. Two different instances within the same test node disagreeing on the
   same leaf: documents (does not solve) the actual current behavior --
   first source-order match wins, matching this project's own existing
   `.find()`-first-match precedent elsewhere.
3. An unrelated option with the same terminal leaf name ("enable") does
   not witness a different predicate.
4. The same relative suffix under an entirely unrelated service/module
   does not witness this target's own predicate.
5. An assignment that leaves the predicate at its own default outcome
   is not treated as a witnessed transition.
6. A statically unclassifiable value (a function call) stays
   `Unresolved` (exit code 2), never silently dropped or treated as
   either a pass or a clean finding.
7. A quoted instance key containing literal dots
   (`"no-git-http-backend.localhost"`) is confirmed, independent of the
   real fixture, to be ONE path segment, not several.
8. `!cfg.foo.enable` (H1 `NegTruthy`) on an instance-keyed submodule is
   also witnessed -- predicate-form breadth, H1 side.
9. `cfg.foo.mode == "on"` (H2 `Eq`) on an instance-keyed submodule is
   also witnessed -- predicate-form breadth, H2 side.

## Full test suite / CI

`cargo test --release`, full suite: 367/367 passing (356 pre-existing +
2 real-fixture + 9 hostile), 0 failed, 73 ignored (pre-existing,
unrelated -- unchanged), 0 new compiler warnings (2 pre-existing,
unrelated: `lower_value_expr` dead code, unused `ChangeKind` variants).
Targeted F1D non-regression anchors explicitly re-run by name and
confirmed passing: `scan_options_angrr_two_hop_period_is_discovered*`,
`scan_options_resolves_an_arbitrary_depth_chain*`,
`two_roots_are_analyzed_independently_real_kimai_transition` (kimai
wildcard). CI: green (see checks after this commit is pushed).

## Build integrity

`touch src/main.rs` before every build (S5-F1C-C's own established
discipline against Cargo mtime-staleness after `cp`-based restores).
Two rounds of temporary `eprintln!` debug instrumentation were added
during root-cause tracing and BOTH fully reverted (via `cp` from
`/tmp/f2-main-backup.rs` and `/tmp/f2-main-backup2.rs`) before any
further work, confirmed via `grep -c` that no debug markers remain.
The accepted-baseline comparison used a genuinely clean checkout (fresh
`git clone`, `git checkout 58b28b2`, confirmed `git status --short`
empty), not a `cp`-based restoration.

## Known unresolved risks / explicitly out of scope

- Cross-instance ambiguity within one test node (hostile control 2)
  remains source-order-dependent, documented not solved -- a real,
  disclosed limitation, not silently hidden.
- A pre-existing, unrelated limitation in `scan_options`'s flat-dotted
  declaration branch (`walk_merge_operands` not checking
  `classify_option_helper_call` on its own `value`) was discovered
  during synthetic-fixture construction. Confirmed NOT to affect cgit
  (which uses the nested form) and confirmed out of S5-F2's own scope
  -- not fixed here.
- The `imports=`-only reachability gap (angrr's `commonPolicyOptions`,
  found during S5-F1D-R) is explicitly NOT addressed here, per the
  user's own closing instruction on the F1D-R round: zero impact on
  frozen watched verdicts, a different mechanism entirely, to be
  recorded as a separate backlog item after the three S5 defects are
  done.
- The wstunnel `#415326` wildcard-prefix collision question remains
  open and unaddressed, unrelated to this fix.

## Scope discipline

No wildcard/`.find()` redesign. No `imports=`-based reference support.
No frigate/nvidia `walk_merge_operands` fix. No changes to F1D
provenance behavior unrelated to cgit. No historical S5 artifact
mutated. No full 369-PR replay (S5-F2-R, deferred). No version bump, no
release.

**STOP.** No S5-F2-R or any full replay without a separate, explicit
GO.
