# Proposed version-bump diff (NOT applied in this round)

Current: `Cargo.toml` `version = "0.4.5"`. Latest tag/release: `v0.4.5`
(2026-09-22). Proposed next: `0.5.0` / tag `v0.5.0`.

This round makes no source changes. The diff below is what a SEPARATE,
later, explicitly-authorized version-bump commit would contain, shown
here only for review.

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -1,7 +1,7 @@
 [package]
 name = "oba"
-version = "0.4.5"
+version = "0.5.0"
 edition = "2021"
 description = "Layer-1 spike: option branch activation evidence for NixOS modules"
```

`Cargo.lock`'s own `[[package]] name = "oba" version = "..."` entry
would need the matching update (`cargo build`/`cargo check` regenerates
this automatically; not hand-edited).

## What else a real version-bump/release commit would plausibly touch

- `CHANGELOG.md` (new file at repo root) — see `changelog-draft.md` in
  this same directory for the proposed content. Currently no such file
  exists; every prior release (`v0.1.0`-`v0.4.5`) shipped with only
  `cargo-dist`'s own generic install/download boilerplate as its
  release body, no hand-authored notes at all.
- Possibly `README.md:95` — the verdict-list sentence ("Every verdict
  is one of `OptionNotFound` / `PredicateNotFound` / `DefaultUnresolved`
  / `TestConfigUnresolved` / `TestValueUnresolved` ... / `OBA001` ...
  / `PASS`") does not mention `OptionRelocated` and is now stale
  relative to already-merged, already-accepted behavior. This is a
  real documentation gap found during this round's own compatibility
  audit (see `rc-prep-report.md` section 2) — recorded here as
  something a release-prep commit should fix, not fixed unilaterally
  in this docs/research-only round.

## Tag / release naming proposal

- Tag: `v0.5.0` (matches the existing `v<semver>` convention exactly,
  every prior tag: `v0.1.0` through `v0.4.5`).
- Release title: `v0.5.0` (matches every prior release's own title
  convention — plain tag name, no additional prose in the title
  itself).
- Release body: `cargo-dist`'s own auto-generated install/download
  section (unchanged mechanism) PLUS, if `CHANGELOG.md` is created and
  a `## [0.5.0]` section added before the tag is cut, `cargo-dist`
  will pull that section in automatically (its own documented,
  existing behavior — not a new mechanism this round introduces).

Neither the tag nor the release is created in this round.
