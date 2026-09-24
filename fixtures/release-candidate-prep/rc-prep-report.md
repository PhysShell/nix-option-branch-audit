# Release Candidate Preparation report

Docs/research-only round. No source changes, no analyzer behavior
change, no full frozen-corpus replay (none was needed — no source
changed since S5-F3-R's own acceptance), no tag, no release, no
version bump performed. S5 remediation itself (closed at `51cd138`) is
not reopened or re-litigated here.

## 1. Current versioning state

- **Current `Cargo.toml` version**: `0.4.5`.
- **Existing tags/releases**: `v0.1.0` through `v0.4.5`, nine releases,
  strict `v<semver>` naming, all published (confirmed via `gh release
  list`).
- **Release mechanism**: `cargo-dist` (`dist-workspace.toml`, pinned
  `cargo-dist-version = "0.33.0"`), GitHub Actions-driven
  (`.github/workflows/release.yml`), single target
  (`x86_64-unknown-linux-musl`), shell installer, GitHub Artifact
  Attestations enabled.
- **Changelog convention**: **none exists yet.** No `CHANGELOG.md` at
  the repo root. Confirmed by directly inspecting a past release body
  (`gh release view v0.4.5`): it contains only `cargo-dist`'s own
  generic install/download boilerplate — no hand-authored "what
  changed" prose in any of the nine releases to date. `cargo-dist`
  itself supports pulling a matching `## [x.y.z]` section from a
  root-level `CHANGELOG.md` automatically, but the mechanism has never
  been exercised in this repository.
- **Documented versioning policy**: none found as an explicit written
  policy document; the tag history itself (`0.1.0` -> `0.4.5`, nine
  releases, all `0.x`) is consistent with ordinary pre-1.0 SemVer
  practice, where the "minor" position is the project's own de-facto
  signal for a release that isn't purely additive.

**Recommended next version: `0.5.0`.** Not bumped in this round — see
section 3 for the compatibility evidence this rests on.

## 2. Public compatibility audit

### F1: `discovered_options[].path` provenance qualification

A named submodule reached by TYPE reference (not written inline) is
now recorded at its own real, fully-embedding-qualified path, instead
of a bare leaf name. Audited this round:

- **Exposure**: present in `check`/`diff`/`audit`/`audit-diff`'s own
  JSON output (`discovered_options`), not internal/debug-only.
- **Documented**: not explicitly called out in `README.md` as its own
  compatibility note (README documents the WITNESS/verdict model in
  detail but doesn't describe `discovered_options`'s own path-shape
  guarantees as a stable contract).
- **Relied upon by fixtures/scripts**: searched — no script or fixture
  in this repository asserts a specific bare (non-qualified)
  `discovered_options` path as a golden value going forward (the
  historical `fixtures/s5-live-pr-shadow/adjudication/**` snapshots ARE
  bare-path golden data, but those are explicitly frozen HISTORICAL
  artifacts this project's own convention already treats as immutable
  comparison baselines, never as "the current correct shape" — S5-F1D-R's
  own provenance-conservation reconciliation exists specifically because
  this distinction was already understood and handled correctly at the
  time).
- **Classification**: internal representation change with real,
  observable JSON impact for any external consumer reading
  `discovered_options[].path` directly and comparing it against a
  previously-observed bare value. Documented as a compatibility note
  in the changelog draft (section 2 below); no fix needed, no
  action beyond disclosure.

### F2: `OBA001` -> `PASS` for structurally valid instance-key evidence

Confirmed: this is a **behavioral correction**, not a schema/shape
change. `Verdict::Pass`'s own JSON shape is completely unchanged; only
WHICH targets receive it changed (some real, previously-false-negative
`OBA001` targets now correctly resolve to `PASS`, or vice versa for a
different fraction where an actual OBA001 remains). No compatibility
note beyond "some outcomes changed, on purpose, for correctness" is
warranted — recorded as such in the changelog draft.

### F3: `Verdict::OptionRelocated` and its own fields

The primary, real compatibility item, audited exhaustively this round:

- **Source-code search for exhaustive `Verdict`/`VerdictKind` matches**:
  every match site in `src/main.rs` is, by Rust's own compiler
  enforcement, exhaustive by construction — a non-exhaustive match
  cannot compile. This project's own internal code already needed, and
  received, updates at every such site when the variant was added
  during S5-F3 (confirmed directly from that implementation's own
  commit history, not merely asserted). **No remaining un-exhausted
  match exists in this repository's own Rust code** — the entire risk
  is external.
- **Exhaustive verdict-name string sets, outside Rust**: searched the
  full repository (`grep` for verdict-name literals across
  `.sh`/`.py`/`.js`/`.yml`/`.yaml`). Found:
  - `action.yml`/`diff/action.yml` (the project's own two published
    GitHub Actions): confirmed, by direct inspection, to do NO JSON
    parsing at all — "the dumbest possible wrapper," passes the raw
    report and exit code straight through. Not a compatibility risk.
  - `.github/workflows/dogfood*.yml`: use targeted `jq -e
    '.summary.notable[0].code == "OBA001"'`-style assertions checking
    for ONE SPECIFIC, still-valid value — not an exhaustive
    enumeration, unaffected by a new tag being added elsewhere.
  - `fixtures/s5-design/*.py` (`gate.py`, `evaluate-rules.py`, etc.):
    use an entirely DIFFERENT, unrelated `verdict` vocabulary (the S5
    adjudication protocol's own `TIER1_FAIL`/`TIER2_INSUFFICIENT`/
    `TIER3_PASS` meta-verdicts) — confirmed not `oba`'s own
    `Verdict` type at all.
  - Every `fixtures/s5-*-r/*.py` replay/comparison script: internal
    research tooling, not a released consumer; several already handle
    `OptionRelocated` explicitly (built during S5-F3-R itself).
  - **No exhaustive external consumer of the closed verdict-name set
    was found anywhere in this repository.**
- **Docs listing all verdict types**: **one real, confirmed gap** —
  `README.md:95`, "Every verdict is one of `OptionNotFound` /
  `PredicateNotFound` / `DefaultUnresolved` / `TestConfigUnresolved` /
  `TestValueUnresolved` (all inconclusive...) / `OBA001` ... / `PASS`"
  — **does not mention `OptionRelocated`**, and is therefore stale
  relative to already-merged, already-accepted behavior. This is the
  one concrete documentation fix this round recommends bundling with
  the version-bump commit (not performed here — see section 8).
- **Fixtures/goldens**: none broken by `OptionRelocated`'s own
  existence — confirmed via the full 384/384 suite passing (section
  4) and via S5-F3-R's own corpus-wide discovery-invariance and
  hard-removal checks finding zero unintended deltas anywhere in the
  frozen 180-PR corpus.

**Exhaustive-consumer risk found**: real, but narrow and
already-demonstrated (this project's own code), not hypothetical or
newly discovered. No OTHER exhaustive consumer exists in this
repository today. External, third-party consumers not visible to this
repository cannot be audited from here — the migration note (section
7 below) is the mitigation for those.

## 3. Release compatibility class

**Minor** (`0.4.5` -> `0.5.0`). Confirming, not merely repeating, the
closeout's own recommendation, with the evidence gathered in section 2:

- A brand-new, additive JSON tag (`OptionRelocated`) that a strict,
  exhaustive consumer can break on is a real, externally observable
  compatibility-relevant change — squarely a minor bump under
  pre-1.0 SemVer practice (where "minor" is this project's own de
  facto signal for "something beyond a pure bug fix changed,
  externally").
- `discovered_options` path re-qualification and F2's own witness
  broadening are both real behavior changes to already-existing
  fields/outcomes, reinforcing rather than contradicting a minor bump
  (neither is field removal/renaming, which would suggest major; both
  are more than an invisible internal fix, which would suggest patch).
- **Not major**: no existing field is removed or renamed, no existing
  command's calling contract (CLI flags, exit codes, `--root`
  semantics) changes, no existing verdict tag's own JSON shape
  changes for any case it already covered before this release.
- **Not patch**: a patch bump conventionally promises "safe to
  upgrade blindly, nothing externally visible changed" — untrue here,
  by this section's own audit.

## 4. Full test suite (clean checkout)

- **Commit**: `51cd138e4139db6967dfc37003e27243f11cae34` (post-S5
  closeout, the current tip of `main` at the time of this round).
- **Checkout**: fresh `git clone` to `/home/tandem/rc-prep-checkout/repo`
  (now removed), `git status --short` confirmed empty before building.
- **Toolchain**: `rustc 1.100.0-nightly (8925ea358 2026-08-20)`,
  `nightly-2026-08-21-x86_64-unknown-linux-gnu`.
- **Command**: `CARGO_TARGET_DIR=/home/tandem/rc-prep-checkout/target
  rustup run nightly-2026-08-21-x86_64-unknown-linux-gnu cargo test
  --release`.
- **Result**: **384 passed, 0 failed, 68 ignored, 0 measured** (summed
  across all 10 test binaries: 9, 54, 5, 242, 22, 22, 2, 15, 12, 1).
- **Warnings**: exactly 2, both pre-existing and unrelated
  (`lower_value_expr` dead code, unused `ChangeKind` variants) — no
  new warnings introduced.
- **No source modification** was made to produce this result.

## 5. Acceptance-artifact sanity check

No new full frozen-corpus replay performed (no source change since
S5-F3-R's own acceptance to justify one). Verified instead, directly:

- `58b28b2`, `c04c5af`, `6051805`, `51cd138`: all four confirmed
  present as real commit objects (`git cat-file -t`) AND confirmed
  ancestors of the current `HEAD` (`git merge-base --is-ancestor`,
  all four `OK`) — not dangling, not on an orphaned branch.
- Every path `fixtures/s5-remediation-closeout/closeout.md` itself
  references (`fixtures/synthetic/f3-guacamole-logbackxml-relocation/`,
  `tests/f3_migration_hostile.rs`, and each round's own `*-report.md`)
  confirmed present on disk.

## 6. Release notes (draft)

See `fixtures/release-candidate-prep/changelog-draft.md` — a complete
`## [0.5.0]` Keep-a-Changelog-style section (Fixed / Added /
Compatibility / Known limitations), written for an end user, linking
to `fixtures/s5-remediation-closeout/closeout.md` for the full
evidence-based limitation register rather than reproducing it inline.

## 7. Machine-consumer migration note (draft)

See `fixtures/release-candidate-prep/MIGRATION-NOTE-v0.5.0-draft.md` —
standalone, explains `OptionRelocated`'s own fields, the exhaustive-
match risk, and explicitly the `destination_confirmed=false` semantics
("known rename, unconfirmed destination" — never "not renamed").

## 8. Release artifact preparation (prepared, not applied)

- `fixtures/release-candidate-prep/version-bump-diff-proposal.md` —
  the exact `Cargo.toml` one-line diff, plus what else a real
  version-bump commit would plausibly need to touch (a new root
  `CHANGELOG.md`, and the `README.md:95` verdict-list staleness found
  in section 2).
- Tag proposal: `v0.5.0` (matches the existing, unbroken `v<semver>`
  naming convention exactly).
- Release title proposal: `v0.5.0` (matches every prior release's own
  title convention).
- Release body: `cargo-dist`'s own existing auto-generation mechanism,
  which would automatically pick up a `## [0.5.0]` `CHANGELOG.md`
  section if one is added before the tag is cut (no new tooling
  needed — an existing, documented `cargo-dist` capability this
  repository has simply never exercised).

**Nothing in section 8 is applied.** No `CHANGELOG.md` was created at
the repo root, `Cargo.toml` is untouched, no tag or release was
created.

## 9. Residual-feature work: none performed, none blocked

Wildcard `.find()` semantics, cross-instance ambiguity, `listOf`
eligibility, `imports =`, `walk_merge_operands`, flat-dotted scanning,
transitive renames, and additional migration-helper semantics were not
touched. None of them was found, in this round's own audit, to block
release compatibility or packaging — every one is a documented,
fail-closed-or-disclosed limitation (per the closeout's own register),
not a release blocker.

## Completion report

- **Current version**: `0.4.5`.
- **Proposed next version**: `0.5.0` (minor).
- **Versioning rationale**: section 3 — a real, additive-but-
  exhaustiveness-breaking JSON schema change (`OptionRelocated`) plus
  two real behavioral changes to existing outcomes, none of them field
  removal/renaming or calling-contract change.
- **Compatibility audit findings**: section 2 — one real, confirmed
  exhaustive-consumer risk class (any strict `Verdict`/`VerdictKind`
  matcher), zero actual exhaustive consumers found anywhere in this
  repository today (the project's own Rust code already handles it,
  enforced by the compiler; both published Actions do no JSON parsing
  at all; CI's own `jq` assertions target specific known values, not
  a closed set), and one real, concrete documentation gap
  (`README.md:95`, doesn't mention `OptionRelocated`).
- **Exhaustive-consumer risks found**: real but narrow, already
  demonstrated internally during S5-F3's own implementation, not newly
  discovered as a surprise; no external consumer risk directly
  auditable from this repository.
- **Test-suite result**: 384/384 passing, 0 failed, 68 ignored, 0 new
  warnings, from a genuinely clean checkout of `51cd138` (section 4).
- **CI result**: not separately re-run this round (no source change to
  trigger it; `51cd138` itself already ran green, per the prior
  round's own confirmation).
- **Release-note draft path**:
  `fixtures/release-candidate-prep/changelog-draft.md`.
- **Machine-consumer migration-note path**:
  `fixtures/release-candidate-prep/MIGRATION-NOTE-v0.5.0-draft.md`.
- **Files proposed for the RC/version-bump commit** (not created/
  changed in this round): `Cargo.toml` (version bump), a new root
  `CHANGELOG.md` (from the draft), a small `README.md:95` fix to
  mention `OptionRelocated`.
- **Tag/release naming proposal**: `v0.5.0`, release title `v0.5.0`,
  matching the existing, unbroken convention exactly.
- **Is the repository ready for a version-bump/release commit?**
  **Yes** — nothing found in this round blocks it. The three prepared
  documents (changelog, migration note, version-diff proposal) are
  ready for review; applying them (creating `CHANGELOG.md`, bumping
  `Cargo.toml`, fixing `README.md:95`) requires a separate, explicit
  GO, per this round's own scope.

**STOP.** No tag created, no GitHub release created, no package
artifacts published, no residual-limitation research experiments
begun, without a separate, explicit GO.
