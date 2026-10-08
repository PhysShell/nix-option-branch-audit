# GO-F Phase F2.1: target construction (15 PRs, before any `oba` run)

**Parent**: `reports/go-f-f2-corpus.md` (`6444671`). Protocol per
`reports/go-f-prereg.md` F1.5 (GO-D/GO-B-corrected). Written entirely
from real diffs/source (`work/go-f/pr-meta/`, `work/go-f/trees/`)
before any `oba` invocation of any kind.

## Per-PR outcome (frozen sample order)

| PR | Stop point / targets | Note |
|---|---|---|
| #570480 | step 6: 1 target (`nixPath`) | Revert PR; declaration moved `nix.settings.nix-path` -> `nix.nixPath` (both removal and addition appear in the diff). No rename/move at the FILE level (both files unchanged paths) -- `KNOWN-LIMITATION-1` does not apply. |
| #565935 | step 5: no wired test found | `tabbyapi` has no entry anywhere in `nixos/tests/all-tests.nix` at head. 3 real new option declarations exist in the diff (`model.recurrent_checkpoint_interval`, `model.recurrent_checkpoint_interval_pp`, `model.warmup`) but construction correctly stops at step 5, per protocol -- NC2-only non-derivation, not a harness failure. |
| #483650 | step 3: no option declaration touched | Diff only changes a `serviceConfig.BindPaths` VALUE inside `config`, no `options={...}` declaration. |
| #568883 | step 2: no `nixos/modules/**` file | Only `nixos/tests/zoom-us.nix` changed. |
| #510072 | step 2: no `nixos/modules/**` file | New package + new test (`buildstream-plugins-community`), not a NixOS module. |
| #565712 | step 2: no `nixos/modules/**` file | Only `nixos/tests/prometheus-exporters.nix` (maintainers metadata) changed. |
| #519655 | step 6: 5 targets | See below. |
| #569962 | step 3: no option declaration touched | Diff only adds new keys to existing `mkMerge` attrsets inside `config` (environment variables, nginx header list) -- no `options={...}` change. |
| #569876 | step 3: no option declaration touched | Diff is refactoring/formatting + a `buildPackage` cross-compile helper; the one `submodule {...}` line that changed is a pure formatting collapse (same semantics), no declaration added/removed. |
| #564819 | step 6: 2 targets | See below. |
| #569875 | step 6: 1 target | See below. |
| #566204 | step 2: no `nixos/modules/**` file | Only `nixos/tests/lomiri.nix` changed. |
| #565592 | step 2: no `nixos/modules/**` file | Only `nixos/tests/monetdb.nix` changed. |
| #556686 | step 3: no option declaration touched | Diff sets `homeMode` on `users.users.pgbackrest` -- an EXISTING core NixOS option this module's own `config` section merely *sets a value for*, not an `options={...}` declaration this module itself owns. |
| #557977 | step 6: 6 targets | New module (`elk.nix`) + new test (`nixos/tests/web-apps/elk.nix`), both genuinely absent in base -- standard, first-class case per GO-E (not a harness exception). See below. |

**No `BASE_FILE_SELECTION_ERROR`, no rename/move** among the 15 (every
`gh api .../files` entry had `previous_filename: null`) --
`KNOWN-LIMITATION-1` is not exercised by this sample.

## Targets, with full provenance

### #570480 (base `d58dbed4`, head `9a209572`)

| Target | Canonical path | `option_prefix` | `watch` |
|---|---|---|---|
| `pr570480_nixPath` | `nix.nixPath` | `["nix"]` | `["nixPath"]` |

### #519655 (base `575941f5`, head `c43c080f`)

`settings` is `mkOption{type=submodule{freeformType=settingsType; options={analytics.enabled=...; general={auto_update=...; port=...};};};}`
-- `general` is a plain nested attrset (not its own `mkOption`), so
its own leaves take the full relative dotted form per the corrected
contract.

| Target | Canonical path | `option_prefix` | `watch` |
|---|---|---|---|
| `pr519655_settings` | `services.bazarr.settings` | `["services","bazarr"]` | `["settings"]` |
| `pr519655_settings_analytics_enabled` | `...settings.analytics.enabled` | `["services","bazarr","settings"]` | `["analytics.enabled"]` |
| `pr519655_settings_general_auto_update` | `...settings.general.auto_update` | `["services","bazarr","settings"]` | `["general.auto_update"]` |
| `pr519655_settings_general_port` | `...settings.general.port` | `["services","bazarr","settings"]` | `["general.port"]` |
| `pr519655_listenPort_removed` | `services.bazarr.listenPort` (base only -- removed in head) | `["services","bazarr"]` | `["listenPort"]` |

### #564819 (base `ec29f764`, head `3ed8ccb3`)

| Target | Canonical path | `option_prefix` | `watch` |
|---|---|---|---|
| `pr564819_maxConcurrency` | `services.nar-serve.maxConcurrency` | `["services","nar-serve"]` | `["maxConcurrency"]` |
| `pr564819_metricsAddress` | `services.nar-serve.metricsAddress` | `["services","nar-serve"]` | `["metricsAddress"]` |

### #569875 (base `2872e79c`, head `d4c1a15c`)

Only the `description`/`example` prose inside the existing
`declarativePlugins` `mkOption{}` call changed (functional `type`/
`default` unchanged) -- per the protocol's own mechanical, syntactic
rule ("the declaration's own source lines appear in the diff"), this
still counts: the diff touches lines inside the `mkOption{}` call's
own span.

| Target | Canonical path | `option_prefix` | `watch` |
|---|---|---|---|
| `pr569875_declarativePlugins` | `services.grafana.declarativePlugins` | `["services","grafana"]` | `["declarativePlugins"]` |

### #557977 (base: file absent; head `91c77770`)

`settings` is `mkOption{type=submodule{freeformType=...; options={HOST=mkOption{...}; PORT=mkOption{...}};};}`.

| Target | Canonical path | `option_prefix` | `watch` |
|---|---|---|---|
| `pr557977_enable` | `services.elk.enable` | `["services","elk"]` | `["enable"]` |
| `pr557977_package` | `services.elk.package` | `["services","elk"]` | `["package"]` |
| `pr557977_openFirewall` | `services.elk.openFirewall` | `["services","elk"]` | `["openFirewall"]` |
| `pr557977_settings` | `services.elk.settings` | `["services","elk"]` | `["settings"]` |
| `pr557977_settings_HOST` | `...settings.HOST` | `["services","elk","settings"]` | `["HOST"]` |
| `pr557977_settings_PORT` | `...settings.PORT` | `["services","elk","settings"]` | `["PORT"]` |

## Counts after target construction

- `derived_pr_count` = **5** (`#570480`, `#519655`, `#564819`,
  `#569875`, `#557977`).
- `derived_target_count` = **15** (1+5+2+1+6), well under
  `MAX_TARGETS_TOTAL=30` -- no budget-cap cuts.
- `NC2` preview (not yet the frozen computation, see F6): `5/15`,
  below the `>=9` threshold.

## What did not happen

No `oba` invocation of any kind yet. Target manifests are written
(`work/go-f/trees/{570480,519655,557977,564819,569875}/targets.toml`)
but not yet run against the frozen candidate binary.

**`S6_GOF_F21_TARGET_CONSTRUCTION_COMPLETE`**
