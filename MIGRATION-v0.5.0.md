# Migration note for programmatic/JSON consumers of `oba` (v0.5.0)

This is a compatibility note for anyone parsing `oba`'s JSON output
(`check`, `diff`, `audit`, `audit-diff`) programmatically, not just
reading its human-readable summary text. It is written to be usable
standalone (e.g. pasted into an integration's own upgrade guide), not
only in the context of this repository.

## What changed that can affect a strict consumer

A new `Verdict` tag exists: **`OptionRelocated`**. It was not present
in any `oba` release before this one.

**If your consumer treats the verdict-name set as closed** — a
`match`/`switch` over previously-known names with no default arm, a
schema with a fixed `enum` of allowed values, a strict deserializer
that rejects unknown tags — it will now fail, or silently mis-handle
this case, the first time it encounters a real option relocation. This
is a demonstrated, not hypothetical, risk: this project's own internal
Rust code needed several `match` sites updated when this variant was
added, caught only because Rust's own compiler enforces match
exhaustiveness at compile time — a dynamically-typed or loosely-typed
consumer gets no equivalent safety net.

**If your consumer reads fields by name and tolerates an unrecognized
tag gracefully** (the normal, recommended posture for a versioned API,
and the posture `schema_version: 1`'s own design has always assumed),
you are not affected and no action is required.

## What `OptionRelocated` means, and what it does not

```json
{
  "verdict": "OptionRelocated",
  "option": "logbackXml",
  "to": ["services", "guacamole-client", "logbackXml"],
  "destination_confirmed": false,
  "migration_source_file": "nixos/modules/services/web-apps/guacamole-server.nix",
  "migration_span": {"file": "...", "line": 12, "col": 6},
  "helper_form": "mkRenamedOptionModule"
}
```

- **`to`**: the destination option's own complete path (never a bare
  leaf name).
- **`helper_form`**: which real migration helper produced this edge —
  currently `"mkRenamedOptionModule"` or `"mkRenamedOptionModuleWith"`.
- **`migration_source_file`** / **`migration_span`**: where, in the
  real source, the rename directive itself was found.
- **`destination_confirmed`**: **read this field carefully.**
  - `true` means `oba` actually located a real declaration at the
    destination path, somewhere under the analysis root.
  - **`false` does NOT mean "not renamed."** It means: a real,
    statically-proven rename directive exists (the option genuinely
    moved), but `oba` was not able to independently confirm a real
    declaration at the destination path within the given analysis
    root. This can happen for reasons that have nothing to do with
    whether the rename is real — most commonly, the destination file
    simply wasn't included in whatever root was analyzed (a narrow
    `--root`, or a historical replay fixture scoped before this
    capability existed). `false` is `oba`'s own honest "I can't
    confirm" — it is never asserted `true` without a real match found.

**What `OptionRelocated` deliberately does NOT claim**: that whatever
finding or predicate applied to the OLD option location still applies,
unchanged, at the new one, or that the new location is itself
test-covered. `OptionRelocated` proves exactly one thing — the option
was relocated, to this specific path, with this specific evidence — and
nothing more. If you need to know whether the DESTINATION option is
itself covered, that requires watching the destination path as its own
target.

## Verdicts unaffected by this change

`OptionNotFound`'s own JSON shape is completely unchanged — including
for a real, deliberate option REMOVAL (`mkRemovedOptionModule`, no
replacement path). `oba` does not currently emit any distinct verdict
for a confirmed removal; it stays `OptionNotFound`, exactly as before
this release, specifically so this change does not alter behavior a
consumer may already depend on for that case.

## Recommended action

1. If your consumer has any exhaustive match/switch over `Verdict`
   tag values, add a case (or a default/fallback arm) for
   `"OptionRelocated"`.
2. If you display verdicts to a human, consider surfacing `to` and
   `destination_confirmed` when the verdict is `OptionRelocated` —
   this is genuinely more actionable information than the "no
   declaration found" message this case used to produce.
3. Do not treat `destination_confirmed: false` as evidence the rename
   itself is fake, incomplete, or unlikely — treat it as "the
   relocation is real; the destination just wasn't independently
   located in this particular run."
