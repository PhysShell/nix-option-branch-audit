# S6-R1 P1: target construction (protocol steps 1-6), all 15 PRs

**Parent**: `fixtures/s6-r1/P1-sample.md` (`c46b23a`). Applies
`fixtures/s6-r0/target-construction-protocol.md` verbatim, re-read
before starting this round. All 15 PRs processed, steps 1-6 only --
`oba` was not run until every PR below had reached a frozen stopping
point.

**Base/head SHA convention** (explicit, consistent for all 15 PRs):
`base` = `base.sha` (the commit the PR branched from), `head` =
`merge_commit_sha` (the real merge commit, i.e. nixpkgs `master` after
the PR landed) -- from `gh api repos/NixOS/nixpkgs/pulls/<n>`.

## Per-PR outcome (frozen sample order)

| # | PR | Stop point | Targets |
|---|---|---|---|
| 1 | [#565943](https://github.com/NixOS/nixpkgs/pull/565943) | step 3: diff touches only `makeWrapperArgs` inside a package override, no `options={...}` declaration | 0 |
| 2 | [#566007](https://github.com/NixOS/nixpkgs/pull/566007) | step 3: diff touches only `config`/systemd settings (zfs device access), no option declaration | 0 |
| 3 | [#569867](https://github.com/NixOS/nixpkgs/pull/569867) | step 3: diff touches a `system.checks` derivation body, no option declaration (this PR is a staging-branch merge; only this one file passed the relevance filter) | 0 |
| 4 | [#508090](https://github.com/NixOS/nixpkgs/pull/508090) | step 6: 2 targets frozen | `SKIP_GPU`, `GPU_COLLECTOR` |
| 5 | [#568429](https://github.com/NixOS/nixpkgs/pull/568429) | step 6: 4 targets frozen | `settings`, `settings.server.port`, `settings.oauth.auth-dir`, `openFirewall` |
| 6 | [#563823](https://github.com/NixOS/nixpkgs/pull/563823) | step 6: 1 target frozen | `aclPolicies` |
| 7 | [#566696](https://github.com/NixOS/nixpkgs/pull/566696) | step 3: diff is a version bump touching only package lists / `mkDefault` config sites, no option declaration | 0 |
| 8 | [#443747](https://github.com/NixOS/nixpkgs/pull/443747) | step 6: 10 targets frozen (brand-new module -- every declaration in the new file is "touched" mechanically) | `enable`,`package`,`domain`,`path`,`filters`,`rootDir`,`enableDefaultRootIndex`,`enableUserDirs`,`listenStreams`,`extraArgs` |
| 9 | [#556752](https://github.com/NixOS/nixpkgs/pull/556752) | step 3: diff touches only default *values* inside an existing `siteSettings`-style config attrset, not an `options={...}` declaration | 0 |
| 10 | [#564688](https://github.com/NixOS/nixpkgs/pull/564688) | step 3: treewide `lib`/CI-helper refactor across 7 module files; zero option declarations touched in any of them | 0 |
| 11 | [#568782](https://github.com/NixOS/nixpkgs/pull/568782) | step 5: `package` option declaration found and touched (step 3 passed), but no wired test -- `nixos/tests/all-tests.nix` at head has no `dnscache` entry, and no test file was changed by this PR | 0 |
| 12 | [#567915](https://github.com/NixOS/nixpkgs/pull/567915) | step 2: only `nixos/tests/music-assistant.nix` changed, no `nixos/modules/**` file at all | 0 |
| 13 | [#568245](https://github.com/NixOS/nixpkgs/pull/568245) | step 2: only `nixos/tests/syncthing/*` files changed, no `nixos/modules/**` file at all | 0 |
| 14 | [#561242](https://github.com/NixOS/nixpkgs/pull/561242) | step 2: only `nixos/tests/all-tests.nix` and `nixos/tests/nginx-otel.nix` changed, no `nixos/modules/**` file at all | 0 |
| 15 | [#568048](https://github.com/NixOS/nixpkgs/pull/568048) | step 6: 11 targets frozen (brand-new module, same reasoning as #443747) | `enable`,`package`,`user`,`group`,`home`,`modelsDir`,`host`,`port`,`settings`,`loadModels`,`openFirewall` |

**`derived_pr_count` = 5** (#508090, #568429, #563823, #443747,
#568048). **`derived_target_count` = 28**
(`pilot_accounting.apply_target_cap` on the frozen per-PR target
lists, in frozen sample order: `accepted=28, cut=0` -- well under
`MAX_TARGETS_TOTAL=30`, no budget-cap exclusions).

## Note on the two "init" PRs' target multiplicity (#443747, #568048)

Both PRs add a brand-new module file. Per the protocol's own mechanical
step 3 ("construct ONE target per touched declaration... do not pick
'the most interesting one'"), every single `mkOption`/`mkEnableOption`/
`mkPackageOption` call in a wholly new file counts as "touched" (every
line is newly added), producing 10 and 11 targets respectively from
one PR each. This is not a curation choice -- the protocol explicitly
forbids judgment calls at this step and anticipates exactly this
possibility via the pilot-wide budget cap. See "Phase B" fallout below:
both PRs' targets turned out to be entirely `INPUT_OR_HARNESS_FAILURE`
for an unrelated reason (the base root has no prior version of a
brand-new file to compare against).

## Frozen target records

All 28, with base/head SHA, in `targets/s6-r1-p1.toml`. `cfg_ident`
is `"cfg"` for every target (every one of the 5 modules binds
`cfg = config.services.<name>;` directly).

| PR | base SHA | head SHA (merge commit) |
|---|---|---|
| #508090 | `d4e00cc559c325edc7f8ec7947dd81ef2761c3c4` | `461071d7ab456653ceba20f6f2675cee0020ff14` |
| #568429 | `6adce641b596aac2c69f7217d061019642834c4d` | `e5ebd085e2692963fe3ac47624e0bfa30aa0300e` |
| #563823 | `3d35b67b0b8051a9080c09ffdd9fa4f41e845d15` | `6c6973cdf55fbe86579846439ec5ddd9a1198e46` |
| #443747 | `1acfd66170c28f3271d806d82348f0bd53ff4685` (module absent in base) | `82c6719c63bf9544fd336d8febb0cca9ff638ff8` |
| #568048 | `803c63ce3bb749922969cc7edc346e48897d8029` (module absent in base) | `e97af004f41f80ea370d11f010af15d653fa6931` |

**`oba` was not run on any target until this entire table was
frozen.**
