# Changelog

All notable changes to this project are documented here. Format is
loosely based on [Keep a Changelog](https://keepachangelog.com/).

## [0.5.0]

This release closes the S5 remediation program: all three defects
confirmed by the historical S5 full-corpus audit (`v0.4.5`, commit
`8f1701a289ddab8bc23f23a25cc0937863fa0356`, FAIL) now have accepted
fixes, each independently validated by its own full 369-PR
frozen-corpus regression replay. Full evidence, including a residual-
limitations register and a disclosed replay-methodology exception, is
in `fixtures/s5-remediation-closeout/closeout.md`.

### Fixed

- **Provenance-qualified nested option identity/discovery.** A named
  submodule reached by type reference (not written inline) is now
  discovered at its own real, fully-qualified embedding path,
  arbitrarily deep, instead of a bare leaf name that could collide
  with an unrelated option of the same name elsewhere.
- **Instance-key-aware witness matching for `attrsOf`/`listOf(submodule
  ...)` targets.** A real, concrete test assignment into a named
  instance of such a target (e.g.
  `services.cgit."myhost".gitHttpBackend.enable`) is now correctly
  recognized as evidence for that option's own, instance-generic
  predicate.
- **Rename/relocation recognition instead of an unexplained
  disappearance.** When a watched option's own declaration has
  genuinely moved (a real, statically-literal
  `mkRenamedOptionModule`/`mkRenamedOptionModuleWith` migration edge),
  the tool now says so explicitly instead of reporting a generic,
  misleading "no declaration found."

### Added

- New structured verdict, `OptionRelocated`, carrying the destination
  option path, the migration helper form used, the migration's own
  source file/span, and a `destination_confirmed` flag. See
  *Compatibility* below and `MIGRATION-v0.5.0.md`.
- `audit-diff`'s own rendered summary distinguishes a relocation
  ("FINDING'S OPTION WAS RENAMED/RELOCATED") from the generic
  "FINDING BECAME INCONCLUSIVE" heading, with a machine-readable
  `"OBA-RELOCATED"` result code.

### Compatibility

- **`discovered_options[].path`** may now be fully embedding-qualified
  (a longer path) for a named submodule reached by reference, where it
  was previously a bare leaf name in this one case. Internal
  representation change; does not change any final verdict's own
  meaning.
- **Some previous `OBA001` outcomes may correctly become `PASS`**
  where a real, concrete `attrsOf`/`listOf(submodule ...)` instance's
  own test assignment genuinely demonstrates the predicate transition.
  This is an intended correctness fix, not a schema change.
- **`OptionRelocated` is a new verdict tag.** A consumer reading JSON
  fields by name is unaffected. A consumer that exhaustively matches
  or `switch`es over a previously-closed set of verdict names should
  be updated to handle it (or add a default/fallback arm) — see
  `MIGRATION-v0.5.0.md` for the exact fields and their meaning.

### Known limitations

Kept short here on purpose — see
`fixtures/s5-remediation-closeout/closeout.md` for the full,
evidence-based register (each item's own corpus examples, verdict
impact, and disposition). None of the following is newly introduced by
this release; every one predates it and is disclosed, not fixed, here.

- Declarations reachable only through NixOS's own `imports = [...]`
  are not discovered.
- A wildcard `option_prefix` does not get provenance-qualified
  collision resolution.
- Multiple concrete submodule instances disagreeing on a watched
  leaf's own value: source order currently decides which is used as
  evidence.
- `listOf(submodule ...)` targets are treated identically to
  `attrsOf(submodule ...)` for instance-key tolerance, though a real
  `listOf` element has no literal instance key.
- Rename chains (`A -> B -> C`) are not followed transitively — direct
  edges only.
- `mkChangedOptionModule`/`mkMergedOptionModule` are not recognized.
- A migration path built from a computed/dynamic Nix expression is
  never guessed at.
