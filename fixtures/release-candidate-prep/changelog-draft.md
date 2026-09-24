# Draft `CHANGELOG.md` entry for the next release

This is a DRAFT, not yet placed at the repository root. `cargo-dist`
(this project's own release tool, `dist-workspace.toml`, version
`0.33.0`) reads a root-level `CHANGELOG.md` in Keep-a-Changelog format
and pulls the matching version section into the GitHub release body
automatically; no such file exists yet, so every past release
(`v0.1.0`-`v0.4.5`) shipped with only `cargo-dist`'s own generic
install/download boilerplate, no hand-authored notes. Creating this
file for real, at the repo root, is itself a decision this round
prepares but does not make -- see the completion report's own
recommendation.

If authorized, the following section would be added as a new
`## [0.5.0]` entry (versioning rationale: see `rc-prep-report.md`
section 3) at the top of a new root `CHANGELOG.md`:

```markdown
## [0.5.0] - <release date>

This release closes the S5 remediation program: all three defects
confirmed by the historical S5 full-corpus audit (`v0.4.5`,
`8f1701a`, FAIL) now have accepted fixes, each independently
validated by its own full 369-PR frozen-corpus regression replay.

### Fixed

- **Nested option identity/provenance.** A named submodule reached by
  reference (not written inline) is now discovered at its own real,
  fully-qualified embedding path, arbitrarily deep, instead of a bare
  leaf name that could collide with an unrelated option of the same
  name elsewhere. Fixes a real false coverage gap in a frozen nixpkgs
  PR (angrr, `#471312`).
- **Witness matching for `attrsOf`/`listOf(submodule ...)` targets.**
  A real, concrete test assignment into a named instance of such a
  target (e.g. `services.cgit."myhost".gitHttpBackend.enable`) is now
  correctly recognized as evidence for that option's own,
  instance-generic predicate — previously invisible, producing a false
  "uncovered option branch" finding on a real nixpkgs PR (cgit,
  `#475112`) whose test genuinely covered the branch.
- **Option rename/relocation is now explicit, not silently
  indistinguishable from an unexplained disappearance.** When a
  watched option's own declaration has genuinely moved (a real,
  statically-literal `mkRenamedOptionModule`/`mkRenamedOptionModuleWith`
  migration edge), the tool now says so explicitly instead of
  reporting a generic, misleading "no declaration found" (guacamole,
  `#462487`).

### Added

- New structured verdict, `OptionRelocated`, carrying the destination
  option path, the migration helper form used, the migration's own
  source file/span, and a `destination_confirmed` flag (see
  *Compatibility* below).
- `audit-diff`'s own rendered summary distinguishes a relocation
  ("FINDING'S OPTION WAS RENAMED/RELOCATED") from a generic
  "FINDING BECAME INCONCLUSIVE" heading, with a machine-readable
  `"OBA-RELOCATED"` result code and an `"oba001->option_relocated"`-shaped
  transition key.

### Compatibility

- **`discovered_options[].path`** may now be a longer, fully embedding-
  qualified path for a named submodule reached by reference (previously
  a bare leaf name in this one case). Internal representation change,
  intentional, does not change any final verdict's own meaning.
- **Some previously-`OBA001` results are now `PASS`** where a real,
  concrete `attrsOf`/`listOf(submodule ...)` instance's own test
  assignment genuinely demonstrates the predicate transition. This is
  an intended correctness fix, not a schema change — the JSON shape of
  `PASS`/`OBA001` themselves is unchanged.
- **New verdict tag `OptionRelocated`.** A JSON/programmatic consumer
  that reads fields by name is unaffected. A consumer that pattern-
  matches or `switch`es over a CLOSED, previously-known set of verdict
  names should be updated to handle it (or a default/fallback arm) —
  see `MIGRATION-NOTE.md` for the exact fields and their meaning.
  `Verdict::OptionNotFound`'s own JSON shape is unchanged for every
  case this new variant doesn't apply to, including real hard removals
  (`mkRemovedOptionModule`).

### Known limitations (tracked, not fixed in this release)

See `fixtures/s5-remediation-closeout/closeout.md` for the full,
evidence-based register. Summary only:

- Declarations reachable only through NixOS's own `imports = [...]`
  mechanism (not a `type =` reference) are not discovered.
- A `with`/`let ... in`-wrapped `options = {...}` block in a known
  shape is not discovered as a true root (frigate/nvidia-container-toolkit).
- A wildcard `option_prefix` (`services.foo.*`) does not get
  provenance-qualified collision resolution; one known real collision
  remains (wstunnel, `#415326`).
- When multiple concrete instances of the same submodule type disagree
  on a watched leaf's own value, source order decides which one is
  used as evidence — not yet outcome-independent.
- `listOf(submodule ...)` targets are currently treated identically to
  `attrsOf(submodule ...)` for instance-key tolerance, though a real
  `listOf` element has no literal instance key — unexercised in
  practice so far.
- Certain flat-dotted/nested declaration shapes are not reached by
  every scanner code path, including destination lookup for a
  relocation.
- Rename chains (`A -> B -> C`) are not followed transitively — direct
  edges only.
- `mkChangedOptionModule`/`mkMergedOptionModule` (different semantics
  from a simple rename) are not recognized.
- A migration path built from a computed/dynamic Nix expression (not a
  literal list) is never guessed at — correctly treated as absent.
```
