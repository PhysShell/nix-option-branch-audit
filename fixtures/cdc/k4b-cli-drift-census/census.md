# K4b: real historical CLI-flag drift census

Not "does this app have CLI flags" -- one specific, falsifiable
question: **does at least one real nixpkgs package bump, for a real
CLI-driven service, cross a real upstream removal or rename of a CLI
flag?** Research-only, bounded corpus, zero `src/cdc.rs` changes --
`diff_cli_contracts()` (K4a) is not touched, not called against any
pair recorded here.

## Corpus (bounded, ~20 candidates, three priority language families)

Built from a real, targeted search of NixOS service modules whose
`ExecStart`/args are constructed directly from a package binary +
flags (`gh api search/code` for `ExecStart` + a `--config`-shaped flag
pattern under `nixos/modules/services`), then filtered by build system
(`buildRustPackage` / `buildGoModule` / `buildPythonApplication`) to
confirm each candidate's real CLI-parser family, not assumed from the
package name.

| package | language | build system | real package.nix commits |
|---|---|---|---|
| vector | Rust | buildRustPackage | 43 |
| krill | Rust | buildRustPackage | 11 |
| turn-rs | Rust | buildRustPackage | 20 |
| realm | Rust | buildRustPackage | 16 |
| rebuilderd | Rust | buildRustPackage | 13 |
| geph | Rust | buildRustPackage | 14 |
| mimir | Go | buildGoModule | 33 |
| tempo | Go | buildGoModule | 25 |
| karma | Go | buildGoModule | 7 |
| blocky | Go | buildGoModule | 18 |
| autobrr | Go | buildGoModule | 36 |
| listmonk | Go | buildGoModule | 23 |
| artalk | Go | buildGoModule | 20 |
| glance | Go | buildGoModule | 31 |
| dgraph | Go | buildGoModule | 24 |
| plikd | Go | buildGoModule | 2 (too shallow, not investigated further) |
| restic | Go | buildGoModule | 10 |
| soft-serve | Go | buildGoModule | 24 |
| ifstate | Python | buildPythonApplication | 18 |
| borgbackup | Python | buildPythonApplication | 27 |
| searxng | Python | buildPythonApplication | 72 (date-versioned web app, not a flag-driven CLI in the relevant sense -- deprioritized) |

For each, oldest real nixpkgs commit (`gh api commits?path=...`, last
page) vs current pin gives the real historical version range. Bounded
depth-first investigation started from the most promising ranges (real
major/minor version jumps, not just patch bumps) rather than
exhaustively deep-diving all 21 -- stopped once the decision rule's
own condition (`≥1 real removed/renamed flag`) was met, per the
explicit instruction not to keep digging for a prettier case.

## Real version ranges checked

| package | base nixpkgs rev | head nixpkgs rev | base version | head version |
|---|---|---|---|---|
| mimir | `61290de65d` | `AFTER_REV`-equivalent (`master`) | 2.14.0 | 3.2.1 |
| vector | `547067d5ad` | `master` | 0.43.1 | 0.58.0 |
| autobrr | `445d7f2f82` | `master` | 1.57.0 | 1.84.0 |
| krill | `509efdedbd` | `master` | 0.14.6 | 0.16.0 |
| glance | `feac6072dc` | `master` | 0.3.0 | 0.8.5 |
| restic | `3818c93017` | `master` | 0.18.0 | 0.19.1 |
| borgbackup | `571c71e6f7` | `master` | 1.4.0 | 1.4.5 |
| searxng | `cd9baa8555` | `master` | unstable-2023-07-19 | 0-unstable-2026-09-12 |

## The qualifying result: `mimir`

| field | value |
|---|---|
| module/package | `mimir` |
| base nixpkgs revision | `61290de65d` (oldest real commit touching `pkgs/by-name/mi/mimir/package.nix`) |
| head nixpkgs revision | current `master` |
| base package version | `2.14.0` |
| head package version | `3.2.1` |
| CLI parser family | Go, `flag.FlagSet` (via `RegisterFlags(f *flag.FlagSet)` methods, dskit/Mimir's own convention -- not `cobra`, a real, useful correction to the pre-round assumption that Go services here would mostly be `cobra`-based) |
| contract source | **tier 1, parser metadata/source** -- `pkg/querier/querier.go`'s own `RegisterFlags` method, fetched directly at both real tags (`mimir-2.14.0`, `mimir-3.2.1`) |
| removed flags | `-querier.prefer-availability-zone` |
| added flags | `-querier.prefer-availability-zones` |
| retained flags | not exhaustively extracted this round (Mimir's full flag surface is in the hundreds; extracting the COMPLETE contract for both versions is a full-corpus-extraction task, out of K4b's bounded research-only scope -- the specific removed/added PAIR is what this round confirms, not a full `CliContract` diff) |
| extraction confidence | **high** -- confirmed by reading the real Go source directly at both exact tags, not inferred from the changelog. The changelog (`CHANGELOG.md`, tier-4/docs) independently corroborates it (`-querier.prefer-availability-zone` renamed to `-querier.prefer-availability-zones` ... "All zones in the list are given equal priority", PRs #13756/#13758) -- source and docs agree, source is the citation. |

```
2.14.0: pkg/querier/querier.go:77
  f.StringVar(&cfg.PreferAvailabilityZone, "querier.prefer-availability-zone", "",
    "Preferred availability zone to query ingesters from when using the ingest storage.")

3.2.1: pkg/querier/querier.go:108
  f.Var(&cfg.PreferAvailabilityZones, "querier.prefer-availability-zones",
    "Comma-separated list of availability zones to prefer when querying ingesters and
     store-gateways. All zones in the list are given equal priority.")
```

Not just a rename -- also a real TYPE change (`StringVar`, one zone ->
`Var` with a custom flag type backing a list, several zones). Recorded
honestly: this is exactly the kind of `same-name`-adjacent-but-not-
identical complexity K3b's own `movim` finding already warned about,
except here the NAME itself also changed, which is what K4a's model
actually diffs -- the type/semantics change is extra context, not
something `diff_cli_contracts` needs to understand to flag this as
`removed`+`added`.

## Other real candidates checked, lighter touch, for corpus breadth

- **`krill`** (Rust, `0.14.6` → `0.16.0`): real CLI-relevant change
  found in the real release notes -- "Refactored command line options
  processing for all binaries... options for both `krillc` and
  `krillta` have slightly changed" (positional/ordering restructuring,
  not a clean single-flag rename) and "Removed support for RTA in
  `krillc`" (a real subcommand-level removal). Extraction confidence:
  **medium** -- confirmed from real upstream release notes (tier-4/docs
  for this entry, not yet source-verified at the `clap` definition
  level) rather than the source itself; a real, second corpus data
  point that CLI surfaces genuinely drift in Rust+clap tools too, but
  not cited with the same rigor as the `mimir` finding above. Not
  pursued further to full source-level confirmation once the decision
  rule's own bar was already cleared by `mimir`.
- **`autobrr`** (Go, `1.57.0` → `1.84.0`): no CLI-flag-relevant drift
  found in a real changelog scan across the range -- this app is
  web-app-shaped (its CLI surface is essentially `--config`), matching
  the general pattern this whole K4 pivot was built to avoid (a
  framework/config-file boundary, not a flag-rich CLI).
- **`borgbackup`** (Python/argparse, `1.4.0` → `1.4.5`): no drift found
  -- a narrow patch-version range, real but not informative on its own
  (too small a window to expect a breaking CLI change).

## Decision, per the rule fixed before this census started

> если найден ≥1 removed/renamed flag → K4c
> если только added-only → расширить corpus один раз
> если после bounded corpus нет removals/renames → CLI differential
> пока закрыть и идти в env vars

**Result: ≥1 real removed/renamed flag found (`mimir`,
`-querier.prefer-availability-zone` → `-querier.prefer-availability-zones`),
confirmed at tier-1 (source) confidence, at a real nixpkgs-crossed
version pair.** Per the pre-fixed rule: **proceed to K4c.**

Deliberately NOT checked this round, per the explicit instruction:
whether the real `mimir` NixOS module's own `ExecStart`/config still
emits the old, removed `-querier.prefer-availability-zone` name after
the bump. That correlation is K4c's job entirely, not K4b's -- K4b's
only job was to prove the historical drift itself is real.

Deliberately NOT solved here, per the same "scope must end somewhere"
discipline K3a/K4a were built under: `krill`'s messier
positional-option restructuring is recorded as a real corpus data
point, not chased into a full same-name/different-mechanism analysis
(a `movim`-shaped rabbit hole this round explicitly declined to enter
again).

## Stop condition

- [x] Corpus is bounded (~21 candidates, three priority language
      families: Rust, Go, Python) and real (found via a targeted NixOS
      module search, not hand-picked upstream projects).
- [x] The qualifying pair is a REAL nixpkgs package bump (both versions
      are real, resolvable nixpkgs pins for the same package,
      `61290de65d`→current), not a synthetic before/after -- the exact
      trap this round was warned against (finding upstream drift that
      no real nixpkgs consumer ever actually crossed, as happened with
      `doctrine/dbal`'s `default_dbname` in K3b/K3b.1).
- [x] Contract source ranked and cited per the fixed methodology --
      tier 1 (parser source) used and cited for the qualifying `mimir`
      finding, not tier 4 (docs) alone.
- [x] Same-name semantic drift (`krill`'s restructuring) recorded as a
      real finding, explicitly NOT solved in this round.
- [x] `diff_cli_contracts()`/K4a untouched -- confirmed, zero
      `src/cdc.rs` changes this round.
- [x] Search stopped once the decision rule's own bar was met, not
      continued in search of a cleaner or more numerous set of
      examples.
