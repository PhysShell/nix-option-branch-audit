# S5-F3: distinguish rename/relocation from plain disappearance (PR #462487)

This document is a validation artifact only. It does not rewrite,
supersede, or reinterpret any prior historical result. Historical S5,
S5-F1 line (closed by S5-F1D-R, `58b28b2`, PASS) and S5-F2 line (closed
by S5-F2-R, `c04c5af`, PASS) both remain exactly as committed and are
not reopened by this round.

See `investigation.md` in this directory for the full end-to-end trace
and root-cause classification.

## Root cause, in one sentence

Neither `mkRenamedOptionModule` nor `mkRemovedOptionModule` had any
handler anywhere in this codebase (confirmed by grep against the
accepted F2-R baseline before writing any code), so `run_target`'s own
gate-1 miss for `services.guacamole-server.logbackXml` had nowhere to
put the real, present, source-level rename provenance
(`lib.mkRenamedOptionModule [ "services" "guacamole-server"
"logbackXml" ] [ "services" "guacamole-client" "logbackXml" ]`) even in
principle -- `Verdict::OptionNotFound` itself carried no representation
for it, so every absence, for any reason, rendered as the identical
"FINDING BECAME INCONCLUSIVE / no mkOption declaration found /
declaration: not found", regardless of whether the source already
explained exactly where the option went.

## Baseline output vs. new output

- **Accepted F2 baseline (`c647d1b`)**: `check --root head` on the real
  PR #462487 fixture reports `OptionNotFound` for `logbackXml` -- a
  two-field JSON object (`{"verdict":"OptionNotFound","option":"logbackXml"}`),
  confirmed via a genuinely clean checkout (fresh `git clone`, `git
  checkout c647d1b33fc1e328cd5f75037770e404a40edcf8`, `git status
  --short` empty), binary SHA-256
  `f209d27a1adf76f58e06f59e82a7d502b672932d7967daf177f27f6b565f2f0f`.
  `audit-diff` renders the generic "FINDING BECAME INCONCLUSIVE"
  heading.
- **F3 candidate**: same fixture, same `--root head`, reports
  `OptionRelocated` with `to = ["services","guacamole-client","logbackXml"]`,
  `destination_confirmed = true`, `helper_form =
  "mkRenamedOptionModule"`, and the real migration call's own source
  file/span. `audit-diff` renders "FINDING'S OPTION WAS
  RENAMED/RELOCATED", with `destination declaration: confirmed` in the
  detail line -- the `finding_became_inconclusive` bucket's own COUNT is
  unchanged (still exactly 1), only the underlying evidence and heading
  carry the new distinction.
- **Base side**: unaffected either way -- `Oba001` (a real, correct
  finding), confirmed byte-identical before and after.

## Structured migration representation

```
MigrationEdge {
    kind: Renamed | Removed,
    from_path: Vec<String>,   // complete literal option path
    to_path: Option<Vec<String>>,  // Some for Renamed, None for Removed
    source_file: String,
    span: Span,
    helper_form: &'static str,  // "mkRenamedOptionModule" | "mkRenamedOptionModuleWith" | "mkRemovedOptionModule"
}
```

Surfaced in the final `Verdict`/JSON as a new, additive variant:

```
Verdict::OptionRelocated {
    option: String,
    to: Vec<String>,               // complete destination path
    destination_confirmed: bool,   // never true unless actually found
    migration_source_file: String,
    migration_span: Span,
    helper_form: &'static str,
}
```

`identifies the rename/relocation`, `destination full path is present
in structured evidence`, and `exact full-path matching is demonstrated`
are all satisfied directly by this shape and by
`resolve_unambiguous_rename`'s own exact-`from_path` matching (see
hostile controls 1-4, 10-11 below).

## Helper forms supported / deliberately unsupported

- **Supported**: `mkRenamedOptionModule oldPathList newPathList` (the
  real, exact guacamole shape), `mkRenamedOptionModuleWith { from =
  [...]; to = [...]; ... }` (current real nixpkgs form, e.g.
  `nixos/modules/virtualisation/oci-options.nix`; extra fields like
  `sinceRelease` are ignored, not validated), `mkRemovedOptionModule
  oldPathList message` (hard removal -- extracted for INTERNAL
  disambiguation only, never surfaced as its own new verdict, see
  below). Both bare and `lib.`-qualified call heads recognized for all
  three.
- **Deliberately unsupported, per this round's own explicit scope**:
  `mkChangedOptionModule` (a value TRANSFORM, different semantics),
  `mkMergedOptionModule` (a MERGE of several old options into one new
  one, different semantics), any generic module alias, any dynamic/
  computed migration path. None is required by guacamole's own
  demonstrated root cause. Recorded here as the real, current nixpkgs
  helper surface (confirmed via direct GitHub code search), not
  expanded opportunistically.

## Rename-vs-removal semantics

`Verdict::OptionNotFound` (plain disappearance AND confirmed hard
removal alike) is completely UNCHANGED -- same two-field shape, same
message, same provenance, for every case that isn't an unambiguous
`Renamed` match. `scan_migrations` does distinguish `Removed` from
`Renamed` internally (the scanner's own small, explicit
`MigrationKind` enum), but that distinction is used ONLY to prevent a
`Removed` (or ambiguous) match from ever being classified as a
relocation -- never to change `OptionNotFound`'s own rendered output.
Verified directly against both real, previously-adjudicated-as-correct
hard-removal controls: sshd's `banner` (`#509507`) and gollum's
`local-time` (`#466806`) both still produce the exact same two-field
`OptionNotFound` JSON object as before this round.

## Destination-verification semantics

Two separate, independently-computed claims, never conflated:

1. **"rename directive observed"** (`Verdict::OptionRelocated` fires at
   all) -- requires exactly one unambiguous `Renamed` edge whose own
   `from_path` equals the watched option's complete path. Independent
   of whether a destination is ever found.
2. **"destination declaration confirmed"** (`destination_confirmed:
   true`) -- requires `locate_migration_destination` to have actually
   found a real declaration at the destination path, somewhere under
   the analysis root. Computed by `analyze()`'s own post-processing
   pass (the one place with the real root a cross-file search needs);
   `run_target` called directly always reports `false`, honestly.

Never claims claim 2 (or, per the mandate's own explicit non-goal, that
the ORIGINAL finding still holds at the destination, or that the
destination is now test-covered) unless actually established.

## Cross-file behavior

`locate_migration_destination` is bounded to the analysis root via the
pre-existing `collect_nix_files` walker, matches by the COMPLETE
destination path (never a leaf-name-only search), and fails closed
(`None`) on a `to_path` shorter than 2 segments, a parse error, or more
than one file claiming the identical destination path. Confirmed for
the real guacamole case (destination in a different file,
`destination_confirmed: true`) and via a dedicated synthetic hostile
control reproducing the same cross-file shape independently.

## Ambiguous / dynamic behavior

- Two real edges sharing the same `from_path` but different
  destinations: fails closed to plain `OptionNotFound`, never a
  source-order guess (`resolve_unambiguous_rename`'s own match arm
  structurally cannot pick a "first" element from a multi-element
  slice).
- A computed/non-literal migration path (`mkRenamedOptionModule
  computedPath [...]`): `extract_literal_string_list` returns `None`,
  the edge is never extracted, the target falls through to ordinary
  `OptionNotFound`.
- A reverse-direction edge (`new -> old`): never satisfies a query for
  the forward relationship -- matching is purely by `from_path`.

## Real fixture

`fixtures/synthetic/f3-guacamole-logbackxml-relocation/` -- real,
live-fetched `base`/`head` content (both `guacamole-server.nix` and
`guacamole-client.nix`) at the frozen PR #462487 SHAs, `targets.toml`
matching the historical adjudication record exactly. `tests/f3_guacamole.rs`,
5 tests: base-side real `OBA001` finding, head-side `OptionRelocated`
with the exact destination path and `destination_confirmed: true`,
`audit-diff`'s own rendering (bucket count unchanged, new heading,
`OBA-RELOCATED` code, old misleading message string confirmed ABSENT),
plus the two mandatory real hard-removal negative controls
(`fixtures/synthetic/f3-removal-negative-controls/`, sshd `#509507` and
gollum `#466806`, both real, live-fetched at their own frozen head
SHAs) -- both confirmed byte-identical `OptionNotFound` shape.

## Hostile controls

`tests/f3_migration_hostile.rs`, 12 tests, all passing: (1) exact
rename, full path to full path; (2) same leaf, unrelated rename, no
match; (3) same prefix, different leaf, no match; (4) reverse
direction, no match; (5) hard removal, not classified as relocation;
(6) plain disappearance, ordinary `OptionNotFound`; (7) cross-file
destination, confirmed; (8) quoted path component containing a literal
dot, one component; (9) dynamic/computed migration path, unresolved,
never guessed; (10) multiple unrelated migrations in one file, correct
edge selected by exact `from_path`; (11) two edges sharing one
`from_path` with different destinations, fails closed, not a
source-order pick; plus `mkRenamedOptionModuleWith` support (current
real nixpkgs form, extra `sinceRelease`-style field ignored).

## sshd / gollum controls

Both re-verified directly against real, live-fetched source at their
own frozen head SHAs (`#509507` `667a124021ed1d8bbdb1cf66a61499f4fdd7afcd`,
`#466806` `a2ad9d8e6df3095073d839aa3bb5f4b644cd0fec`): both remain
`Verdict::OptionNotFound`, both confirmed to carry EXACTLY the same
two-JSON-field shape as before this round (asserted directly, not just
the verdict tag, specifically to catch an accidental shape widening).

## F1/F2 non-regression results

Targeted re-runs (not a full replay, per the mandate), all passing,
unaffected: `scan_options_angrr_two_hop_period_is_discovered*`,
`scan_options_resolves_an_arbitrary_depth_chain*` (F1D anchors), the
full `tests/f2_cgit.rs` + `tests/f2_cgit_hostile.rs` (11 tests, F2's own
cgit instance-key witness behavior), and
`two_roots_are_analyzed_independently_real_kimai_transition` (kimai
wildcard). This fix touches only gate-1's own miss branch and
`analyze()`'s own new post-processing pass -- it never touches
`path_matches_prefix`, `evaluate_predicate_witness`, or `scan_options`'s
own true-root discovery.

## A test whose premise genuinely, intentionally changed

`tests/diff_cli.rs`'s own `mk_renamed_option_module_relocation_stays_unaffected_by_this_fix`
was written during S5-F1D specifically to confirm THAT round's own
reference-following fix did NOT accidentally add rename detection. That
premise is now, deliberately, no longer true -- rewritten (not silently
patched) to `mk_renamed_option_module_relocation_is_now_detected_by_s5_f3`,
asserting the new, correct, intended `OptionRelocated` behavior,
following the exact precedent S5-F1D itself established for a test
whose own premise improved.

## Full test suite / CI

`cargo test --release`, full suite: 384/384 passing (367 pre-existing +
5 real-fixture + 12 hostile), 0 failed, 68 ignored (pre-existing,
unrelated -- unchanged), 0 new compiler warnings (2 pre-existing,
unrelated: `lower_value_expr` dead code, unused `ChangeKind` variants).
CI: green (see checks after this commit is pushed).

## Build integrity

`touch src/main.rs` before every build. The accepted-baseline comparison
used a genuinely clean checkout (fresh `git clone`, `git checkout
c647d1b`, confirmed `git status --short` empty), not a `cp`-based
restoration.

## Does this change behavior for a class broader than guacamole?

**Yes**, structurally defined: **any target whose watched option's
complete path (`option_prefix ++ watched_path`) exactly equals the
`from_path` of a statically-literal, unambiguous
`mkRenamedOptionModule`/`mkRenamedOptionModuleWith` edge found anywhere
in that target's own module source**. Every such newly-classified
`OptionRelocated` witness is sound for the same reason guacamole's own
is: `from_path` is matched as a COMPLETE literal path (never a leaf or
suffix), ambiguity (multiple edges sharing one `from_path`) is
detected and fails closed rather than guessed, a `Removed` edge is
never conflated with a `Renamed` one, and `destination_confirmed`
itself is never claimed true without an actual, independently-verified
destination declaration. The gate is exercised on real, statically
present source text only -- no dynamic/computed path is ever accepted
(hostile control 9), so there is no path by which this widens
matching beyond what the module's own literal source already,
explicitly states.

## Known unresolved risks / explicitly out of scope

- `locate_migration_destination` inherits `scan_options`'s own
  pre-existing flat-dotted-form limitation (a flat-dotted single-entry
  declaration without a locally resolvable `cfg` alias is invisible to
  it) -- a real, disclosed, pre-existing `scan_options` boundary, not a
  new S5-F3 bug; found and documented while building the cross-file
  hostile control.
- Transitive rename chains (`A -> B -> C`) are NOT resolved -- direct
  edges only, per the mandate's own explicit boundary; not required by
  guacamole (a single, direct edge).
- `mkChangedOptionModule`/`mkMergedOptionModule` remain unsupported,
  deliberately, per this round's own explicit scope.
- The `imports=`-only reachability gap (angrr's `commonPolicyOptions`),
  the frigate/nvidia `walk_merge_operands` wrap gap, the wstunnel
  `#415326` wildcard-prefix collision, F2's own documented
  cross-instance `.find()`-ordering limitation, and F2's own newly-
  disclosed `listOf` instance-key soundness caveat all remain open,
  unaddressed, unaffected by this round.
- No full 369-PR S5-F3-R replay performed this round, per the mandate.

## Files / commits

- Implementation: `src/main.rs`, `tests/f3_guacamole.rs`,
  `tests/f3_migration_hostile.rs`, the updated
  `tests/diff_cli.rs` test, `fixtures/synthetic/f3-guacamole-logbackxml-relocation/`,
  `fixtures/synthetic/f3-removal-negative-controls/` (one commit).
- `fixtures/s5-f3/investigation.md`, `fixtures/s5-f3/f3-report.md`
  (this commit).

## CI

Green (see checks after this commit is pushed).

## Scope discipline

No wildcard/`.find()` redesign. No F2 cross-instance-ambiguity fix. No
F2 `listOf` caveat fix. No `imports =`-based arbitrary module
reachability work. No frigate/nvidia `walk_merge_operands` fix. No
generic Nix evaluation. No unrelated migration helper added merely
because it looked interesting. No full 369-PR replay. No version bump,
no release. No historical/frozen artifact touched or reinterpreted.

**STOP.** No S5-F3-R or any full replay without a separate, explicit
GO.
