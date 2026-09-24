# S5-F3: investigation and root-cause trace (guacamole PR #462487)

Required by the mandate: a full end-to-end trace of the misleading
"FINDING BECAME INCONCLUSIVE" presentation on
`services.guacamole-server.logbackXml`, base
`3db8153243db1455a18da92119581d0580dd70b8`, head
`598f6dd4fc046410bb2719227d5d48e927e7d39b`, historical presentation id
`B:462487:oba:logbackXml:finding_became_inconclusive:1`, identifying
the exact information boundary where rename semantics were previously
lost, classified against the trace the mandate itself specifies.

## Search finding, verified against the exact baseline before writing any code

`grep -n "mkRenamedOptionModule\|mkRemovedOptionModule\|mkChangedOptionModule\|mkMergedOptionModule" src/main.rs`
against the accepted F2-R baseline (`c647d1b`) returns **zero matches**.
Neither helper had any handler anywhere in this codebase. This is the
root cause's own precondition, confirmed empirically, not assumed.

## Real shape, as read from the fetched source at the frozen SHAs (not assumed)

`nixos/modules/services/web-apps/guacamole-server.nix`, base: a real,
directly-declared `logbackXml = lib.mkOption { type = lib.types.nullOr
lib.types.path; default = null; ... };` under `options.services.guacamole-server`,
with a real, never-tested predicate later in the same file:
`environment.etc."guacamole/logback.xml" = lib.mkIf (cfg.logbackXml !=
null) { ... };` (line 65). The real test file
(`nixos/tests/guacamole-server.nix`) never sets `logbackXml` on either
side -- base's own real verdict is `OBA001` ("uncovered option branch"),
confirmed directly against the fetched base source.

`nixos/modules/services/web-apps/guacamole-server.nix`, head:
`logbackXml` is entirely absent from the module's own `options = {...}`
block (only `enable`/`package`/`extraEnvironment`/`host`/`port`
remain). Instead, the file's own `imports = [...]` (added at head)
contains:

```
imports = [
  (lib.mkRenamedOptionModule
    [ "services" "guacamole-server" "logbackXml" ]
    [ "services" "guacamole-client" "logbackXml" ]
  )
  (lib.mkRenamedOptionModule
    [ "services" "guacamole-server" "userMappingXml" ]
    [ "services" "guacamole-client" "userMappingXml" ]
  )
];
```

`nixos/modules/services/web-apps/guacamole-client.nix`, head: a real,
directly-declared `logbackXml = lib.mkOption { type = lib.types.nullOr
lib.types.path; default = null; ... };` under
`options.services.guacamole-client` -- confirmed, the destination
genuinely exists, in a DIFFERENT file from the source declaration.

## Trace, step by step (base -> head -> comparison -> presentation)

1. **Base**: `scan_options` finds `logbackXml` at
   `["logbackXml"]` (relative to `option_prefix =
   ["services","guacamole-server"]`). `scan_resolved_predicates` finds
   `lib.mkIf (cfg.logbackXml != null) {...}` (H2 `Not(Eq(Ref([logbackXml]),
   Null))`). Base's own default (`null`) makes the predicate's own default
   outcome `false`; no test assignment exists to flip it ->
   `Verdict::Oba001` -- a real, correctly-produced finding. Not the bug.
2. **Head, gate-1 lookup**: `options.iter().find(|o| o.path ==
   ["logbackXml"])` -- **THE FAILURE POINT, pre-fix**. Returns `None`:
   `logbackXml` genuinely isn't declared under `guacamole-server`
   anymore. Before this round, this `None` had exactly ONE possible
   consequence: `verdicts.push(Verdict::OptionNotFound { option:
   "logbackXml".to_string() });` -- no other information was ever
   consulted.
3. **`compare()`**: base verdict kind `Oba001`, head verdict kind
   `OptionNotFound` -> `TargetDiff::Changed` with `ChangeKind::VerdictChanged`.
4. **`audit-diff`'s own bucketing**: `oba_kind_class(Oba001) =
   ResultVerdict::Finding`, `oba_kind_class(OptionNotFound) =
   ResultVerdict::Inconclusive` -> `classify_transition_bucket` returns
   `"finding_became_inconclusive"`.
5. **Rendering**: `oba_verdict_to_result` for `Verdict::OptionNotFound`
   (pre-fix) always produced the literal message `"no mkOption
   declaration found for this watched option"` and provenance
   `["Nix option: logbackXml", "declaration: not found"]`, regardless of
   WHY the declaration was absent. `render_github_summary`'s own heading
   match for the `"finding_became_inconclusive"` bucket had exactly one
   fallback arm (`(other, _) => other` maps to the literal string
   `"FINDING BECAME INCONCLUSIVE"`), with no way to distinguish "genuinely
   gone" from "moved, with real provenance" from "deliberately removed" --
   all three were, and had to be, the SAME rendered sentence, because
   `Verdict::OptionNotFound` itself carried no information beyond the
   bare fact of absence.

**Classification against the mandate's own enumerated failure modes**:
not "never extracted" (nothing WAS extracted -- there was no extractor
at all), not "extracted under wrong path", not "fails instance-submodule
matching" (S5-F2's own concern, unrelated), not "value semantics wrong",
not "predicate resolution wrong" (base's own predicate resolution is
correct), not "evidence discarded by aggregation". The real mechanism:
**the underlying data model (`Verdict::OptionNotFound`) had NO
representation for migration provenance at all, so gate-1's own `None`
branch had nowhere to put real, present, source-level evidence even if
it had been extracted** -- confirming the mandate's own suspicion that
this needed a small new representation, not merely a rendering fix.

## Where the missing concept belongs: source scanning AND the comparison model, not rendering alone

Determined, not assumed, by tracing the full pipeline above: rendering
alone cannot fix this, because `AuditResult`/`NotableChange`'s own
`message`/`provenance` are built FROM a `Verdict` that never carried
migration information in the first place -- there was nothing for a
smarter renderer to read. The fix necessarily needs:

1. **Source scanning**: a new extractor (`scan_migrations`) that finds
   and classifies `mkRenamedOptionModule`/`mkRenamedOptionModuleWith`/
   `mkRemovedOptionModule` calls -- entirely absent before this round
   (confirmed above).
2. **`check`/`diff`'s own result model**: a new `Verdict` variant
   (`OptionRelocated`) that CAN carry a destination path and a
   confirmation flag -- `Verdict::OptionNotFound` itself is
   deliberately left untouched (see "Rename-vs-removal semantics"
   below), so this is an ADDITIVE new variant, not a widened existing
   one.
3. **Comparison model**: no change needed to `compare()`/`TargetDiff`
   itself -- `ChangeKind::VerdictChanged` already fires correctly for
   `Oba001 -> OptionRelocated` (a genuinely different `VerdictKind`),
   exactly as it already does for `Oba001 -> OptionNotFound`. Verified,
   not assumed: `compare()` is untouched by this round.
4. **`audit-diff`'s own presentation layer**: `oba_kind_class`,
   `transition_origin_for_oba_change`, `oba_verdict_to_result`, and
   `render_github_summary`'s own heading logic all need one new,
   narrow branch each, to carry the new information through to a
   maintainer-facing rendering and to structured JSON.

## The fix

- `MigrationKind { Renamed, Removed }` / `MigrationEdge { kind,
  from_path, to_path, source_file, span, helper_form }`
  (`src/main.rs:~1180`).
- `extract_literal_string_list` (`:1201`) -- a literal, non-interpolated
  `[ "a" "b" "c" ]` list; fails closed (`None`) the instant any element
  isn't a plain string, reusing the pre-existing, already-reviewed
  `string_text` helper (the same one `attrpath_segments` itself uses)
  verbatim.
- `scan_migrations` (`:1242`) -- recognizes `mkRenamedOptionModule
  oldPathList newPathList` (2-arg curried call), `mkRenamedOptionModuleWith
  { from = [...]; to = [...]; ... }` (1-arg attrset call, reusing the
  pre-existing `find_attrset_field` verbatim; other fields like
  `sinceRelease` are real in current nixpkgs and simply ignored, never
  validated), and `mkRemovedOptionModule oldPathList message` (2-arg
  curried call, message not extracted). Both bare and `lib.`-qualified
  call heads recognized via the pre-existing `call_head_name`. Deliberately
  does NOT recognize `mkChangedOptionModule`/`mkMergedOptionModule`/any
  generic alias -- guacamole's own demonstrated root cause never requires
  them.
- `resolve_unambiguous_rename` (`:1343`) -- matches by the option's
  COMPLETE path (`option_prefix ++ watched_path`, never a bare leaf),
  and fails closed (returns `None`) whenever more than one edge (of
  EITHER kind) shares the same `from_path`, or the sole match is a
  `Removed` edge -- both cases fall through to the unchanged, existing
  `OptionNotFound` path.
- `locate_migration_destination` (`:1370`) -- cross-file destination
  confirmation, bounded to the analysis root via the pre-existing,
  already-reviewed `collect_nix_files` walker (the same one `--census`
  uses); re-runs the pre-existing `scan_options` with `to_path`'s own
  parent as `option_prefix`, checking for a real declaration at
  `to_path`'s own last segment. Matches by the COMPLETE destination
  path, never a leaf-only search. Fails closed on `to_path.len() < 2`,
  a parse error, or more than one file claiming the identical
  destination path.
- `Verdict::OptionRelocated { option, to, destination_confirmed,
  migration_source_file, migration_span, helper_form }` (`:3438`) -- a
  new, additive variant. `VerdictKind::OptionRelocated`,
  `oba_kind_class(..) -> ResultVerdict::Inconclusive`,
  `transition_origin_for_oba_change(OptionRelocated) ->
  AnalysisBecamePossible` (grouped with the other four
  already-analyzable-but-opaque kinds -- see that function's own
  updated doc comment for why).
- `run_target`'s own gate-1 miss (`:~4676`) now computes
  `full_path = option_prefix ++ watched_path` and consults
  `resolve_unambiguous_rename`; `destination_confirmed` is always
  initialized `false` here (`run_target` alone never has the analysis
  root a cross-file search needs).
- `analyze()` (`:~4166`) -- a new POST-processing pass, after the
  existing per-target loop, fills in `destination_confirmed` for every
  `Verdict::OptionRelocated` using `locate_migration_destination(root,
  ...)`. Deliberately placed here, not threaded into `run_target`'s own
  signature: `analyze()` is the one place in the whole call graph that
  already has the real analysis `root`, and every one of
  `check`/`diff`/`audit`/`audit-diff` goes through it uniformly. A test
  calling `run_target` directly (as many existing unit tests do) always
  sees `destination_confirmed: false` -- honest, not a false negative
  claim, just an accurate reflection of what that narrower entry point
  alone can prove.
- `oba_verdict_to_result` (`ObaResultEvidence` gains two new, purely
  additive, `skip_serializing_if`-gated fields: `relocated_to`,
  `destination_confirmed` -- every other verdict's own JSON is
  byte-for-byte unchanged) and `render_github_summary` (a new
  `OBA_RELOCATED_CODE`-gated heading override, "FINDING'S OPTION WAS
  RENAMED/RELOCATED" / "NEWLY INCONCLUSIVE: OPTION WAS
  RENAMED/RELOCATED", replacing the generic fallback text ONLY for a
  `Verdict::OptionRelocated`-backed notable entry -- the bucket itself,
  and its own count, are completely unchanged).

## Rename-vs-removal semantics: `Verdict::OptionNotFound` is deliberately untouched

Per the mandate's own explicit instruction ("do not change existing
correctly-rendered hard-removal behavior merely for symmetry"):
`Verdict::OptionNotFound`'s own shape (`{option}`, two JSON fields
total, unchanged since before this round) is used for BOTH plain
disappearance AND a confirmed hard removal (`mkRemovedOptionModule`).
`scan_migrations` DOES distinguish `Removed` from `Renamed` internally
(satisfying "the scanner should be able to distinguish... it is
acceptable to introduce explicit removal metadata as part of the same
small migration scanner") -- but that internal distinction is used
ONLY to GATE `resolve_unambiguous_rename` (a `Removed` match, or an
ambiguous multi-edge match, both fall through to plain
`OptionNotFound`, never to a fabricated relocation), never to change
`OptionNotFound`'s own rendered shape. Verified directly: sshd's real
`banner` removal (`#509507`) and gollum's real `local-time` removal
(`#466806`) both produce `Verdict::OptionNotFound` with EXACTLY the
same two-field JSON shape as before this round (`tests/f3_guacamole.rs`'s
own hard-removal controls assert the object's own field count, not
merely the verdict tag, specifically to catch an accidental shape
widening).

## Destination-verification semantics

"Rename directive observed" (the `OptionRelocated` verdict itself)
fires whenever `resolve_unambiguous_rename` finds exactly one
unambiguous `Renamed` edge -- REGARDLESS of whether a destination
declaration is ever found. "Destination declaration confirmed"
(`destination_confirmed: true`) is a SEPARATE, strictly additional
claim, computed independently by `analyze()`'s own post-processing pass,
and is NEVER true unless `locate_migration_destination` actually found
a real declaration. For the frozen guacamole case, both are
established: real fixture test
(`real_guacamole_pr462487_head_is_relocated_not_a_bare_option_not_found`)
confirms `destination_confirmed: true` against the real, cross-file
`guacamole-client.nix` declaration.

## Cross-file relocation

Guacamole moves from `guacamole-server.nix` to `guacamole-client.nix`.
`locate_migration_destination` is bounded to the analysis root (the
same boundary every other command already uses), searching by the
COMPLETE destination path via `collect_nix_files` + a targeted
`scan_options` re-run per candidate file -- never a leaf-name-only
search. Confirmed via a dedicated hostile control
(`cross_file_destination_is_confirmed`) reproducing the guacamole shape
synthetically, independent of the real fixture.

A real, disclosed, narrow limitation found while building this control:
`locate_migration_destination` reuses `scan_options` UNCHANGED, so it
inherits `scan_options`'s own existing form-support boundary --
specifically, `scan_options`'s flat-dotted single-entry form
(`options.a.b = {...};`) additionally requires a real, resolvable
`cfg`-alias binding enclosing that declaration (`resolve_cfg_root`'s own
existing, pre-S5-F3 guard), while the generic NESTED form (`options = {
a.b = {...}; };`, real guacamole-client.nix's own shape) has no such
requirement. This is not a new S5-F3 bug -- it is `scan_options`'s own
pre-existing behavior, reused here exactly as-is, not re-implemented.

## Multiple-edge ambiguity: fail-closed, no new `.find()` policy

Confirmed via a dedicated hostile control
(`ambiguous_same_from_path_different_destinations_fails_closed`): two
real `mkRenamedOptionModule` edges sharing the identical `from_path` but
pointing to different destinations produce plain `OptionNotFound`, not
a source-order-first relocation guess. `resolve_unambiguous_rename`'s
own `match matching.as_slice() { [only] if only.kind == Renamed =>
Some(only), _ => None }` structurally cannot pick a "first" element from
a multi-element slice -- there is no `.find()`/`.first()` call anywhere
in this function.

## Predicate-form / F1-F2 orthogonality

This fix touches only gate-1's own miss branch and `analyze()`'s own
post-processing pass -- it never touches `path_matches_prefix`,
`evaluate_predicate_witness`, `scan_options`'s own true-root discovery,
or any F1D/F2 mechanism. Confirmed via targeted re-runs (not a full
replay, per the mandate): `scan_options_angrr_two_hop_period_is_discovered*`,
`scan_options_resolves_an_arbitrary_depth_chain*` (F1D anchors),
`tests/f2_cgit.rs` + `tests/f2_cgit_hostile.rs` (all 11, F2's own cgit
instance-key witness behavior), and
`two_roots_are_analyzed_independently_real_kimai_transition` (kimai
wildcard) -- all pass, unaffected.

## Current nixpkgs helper surface -- recorded, not opportunistically expanded

Confirmed by direct GitHub code search against current nixpkgs:
`mkRenamedOptionModule` and `mkRenamedOptionModuleWith` are both real,
actively used (e.g. `nixos/modules/virtualisation/oci-options.nix`'s
own `mkRenamedOptionModuleWith { sinceRelease = 2411; from = [...]; to
= [...]; };`), `mkRemovedOptionModule` is real and used by both
negative controls. All three are supported by `scan_migrations`.
`mkChangedOptionModule` and `mkMergedOptionModule` are real, different
semantics (a value TRANSFORM, or a MERGE of multiple old options into
one new one, respectively) -- neither is required by guacamole's own
demonstrated root cause, and neither is touched by this round, per its
own explicit scope restriction.
