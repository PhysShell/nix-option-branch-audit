# S2: prevalence (A) + stress (B) cohorts — population and draw

Drawn mechanically per `protocol.md`, frozen here **before any of the
50 PRs below is individually inspected**. Nothing past this point
(adjudication) may change cohort membership.

## Population funnel

```
S1's own already-screened survivor pool, never drawn                132
  (real path-eligible, non-excluded PRs from S1's own 196-candidate
  population that S1's own 30-PR draw never touched)
+ fresh top-up (commits since S1's own fetch, same path-filtered
  commits API, screened through the identical eligibility/exclusion
  pipeline)                                                            2
= combined S2 population                                             134
  (explicit dedup check against S1's own drawn 30: zero overlap)

S2-A draw: seed = int("4ba459b", 16) = 79316379,
  random.Random(seed).shuffle, first 30                               30
remaining pool (S2-A's own 30 removed)                                104

S2-B stress filter (mechanical, on real `gh pr diff` added/removed
  lines only, OR across all 6 pre-registered categories):
  stress-eligible                                                      86
  not eligible (matched none of the 6 categories)                      18

S2-B draw: seed = int("4ba459b", 16) + 1 = 79316380 (a distinct seed,
  so B's draw order doesn't correlate with A's), first 20 of the
  stress-eligible pool                                                20
```

**A real, disclosed observation about the stress filter itself**:
`nixos_tests_touched` was the single most common matching category
(70/104), not `package_version_source_dep` (31/104) as might have been
expected going in — the filter's own selectivity comes mostly from the
non-version categories, not from version bumps dominating. Reported
honestly since the protocol committed to not tightening the criteria
after seeing this.

```
category hit counts among the 86 stress-eligible candidates
(a PR can match more than one):
  nixos_tests_touched:          70
  environment:                  35
  package_version_source_dep:   31
  option_declaration:           30
  execstart_script_cmdline:     23
  generated_config:             20
```

## S2-A: prevalence cohort (30 PRs)

| PR | state | merged | base SHA | head SHA | title |
|---|---|---|---|---|---|
| [#295514](https://github.com/NixOS/nixpkgs/pull/295514) | merged | 2026-08-21 | `d92dcf1` | `9914054` | nixos/fcitx: fix usage of booleans for settings |
| [#452303](https://github.com/NixOS/nixpkgs/pull/452303) | merged | 2026-09-15 | `e318417` | `1a792f5` | nixos/xrdp: actually use cfg.package parameter |
| [#469112](https://github.com/NixOS/nixpkgs/pull/469112) | merged | 2026-08-30 | `35a35c0` | `336c64e` | nixos/hddtemp: fix start script to work with strictShellChecks enabled |
| [#479381](https://github.com/NixOS/nixpkgs/pull/479381) | merged | 2026-08-30 | `ce09e3c` | `9e8e1c5` | nixos/cage: add option to restart service if changed |
| [#485251](https://github.com/NixOS/nixpkgs/pull/485251) | merged | 2026-09-15 | `86285cf` | `90bdf7c` | nixos/prometheus/mqtt-exporter: add `package` option |
| [#503263](https://github.com/NixOS/nixpkgs/pull/503263) | merged | 2026-08-28 | `282d278` | `6c5919b` | nixos/calibre-web: add split book directory to config |
| [#528118](https://github.com/NixOS/nixpkgs/pull/528118) | merged | 2026-08-29 | `9aa632c` | `4cf9b0a` | kmscon: 10.0.0 -> 10.0.2 |
| [#545231](https://github.com/NixOS/nixpkgs/pull/545231) | merged | 2026-09-10 | `63dd521` | `bef1d9e` | dms-greeter: init at 1.6.0 |
| [#552737](https://github.com/NixOS/nixpkgs/pull/552737) | merged | 2026-08-26 | `dc6900f` | `d85d744` | trailbase: init at 0.32.2 |
| [#552774](https://github.com/NixOS/nixpkgs/pull/552774) | merged | 2026-09-08 | `fb5f539` | `de829fc` | nixos/qemu-vm: 9p -> virtiofs |
| [#554703](https://github.com/NixOS/nixpkgs/pull/554703) | merged | 2026-09-13 | `9808f22` | `ec0c2e2` | wordpress: add new release, drop unmaintained version, update packages |
| [#555377](https://github.com/NixOS/nixpkgs/pull/555377) | merged | 2026-08-25 | `7cb31a3` | `89f5cbb` | cloudflare-warp: 2026.3.846.0 -> 2026.7.1343.0 |
| [#555625](https://github.com/NixOS/nixpkgs/pull/555625) | merged | 2026-08-24 | `3f7e52b` | `295e967` | nixosTests.keymap: migrate to runTest |
| [#556253](https://github.com/NixOS/nixpkgs/pull/556253) | merged | 2026-08-26 | `103cb3a` | `ec361a6` | nixos/pdns-recursor: add api.enable to start the built-in webserver |
| [#556989](https://github.com/NixOS/nixpkgs/pull/556989) | merged | 2026-08-30 | `d68abec` | `e4ac98e` | kubernetes: 1.36.3 -> 1.37.0 |
| [#557327](https://github.com/NixOS/nixpkgs/pull/557327) | merged | 2026-08-29 | `997ddc6` | `08f1d98` | nixos/tests/nextcloud: replace activation script with a tmpfs mount |
| [#558091](https://github.com/NixOS/nixpkgs/pull/558091) | merged | 2026-08-31 | `06b8d70` | `f2e36ba` | nixos/tests/systemd-timesyncd-nscd-dnssec: bind tinydns to loopback |
| [#558149](https://github.com/NixOS/nixpkgs/pull/558149) | merged | 2026-09-01 | `642d7dd` | `d43115b` | nixos/nixosTests.firewall-nftables: fix race condition |
| [#558283](https://github.com/NixOS/nixpkgs/pull/558283) | merged | 2026-09-04 | `597647d` | `9895b81` | Draupnir: include missed file, fix tests flow |
| [#559009](https://github.com/NixOS/nixpkgs/pull/559009) | merged | 2026-09-07 | `3d0763f` | `59928df` | nixos/tests/gitea-actions-runner: init |
| [#559344](https://github.com/NixOS/nixpkgs/pull/559344) | merged | 2026-09-03 | `65d8124` | `a973cc5` | nixos/home-assistant: allow access to cec devices for hdmi_cec component |
| [#559798](https://github.com/NixOS/nixpkgs/pull/559798) | merged | 2026-09-05 | `63dd521` | `617e3f9` | nixos/radicle-ci-broker: add setting logLevel |
| [#559860](https://github.com/NixOS/nixpkgs/pull/559860) | merged | 2026-09-05 | `fb9278a` | `83fb527` | pdfding: 1.13.0 -> 1.14.0 |
| [#560892](https://github.com/NixOS/nixpkgs/pull/560892) | merged | 2026-09-10 | `525d3b4` | `4fd3acb` | strongswan: 6.0.7 -> 6.1.0 |
| [#560968](https://github.com/NixOS/nixpkgs/pull/560968) | merged | 2026-09-13 | `d2bb933` | `b9ecb1d` | nixos/tests/freshrss: fix assertion |
| [#561051](https://github.com/NixOS/nixpkgs/pull/561051) | merged | 2026-09-08 | `b85b405` | `4e7d83c` | nixos/tests/portunus: fix flakiness |
| [#562104](https://github.com/NixOS/nixpkgs/pull/562104) | merged | 2026-09-11 | `9ed84be` | `a10d1e3` | nixosTests.hickory-dns: add basic zone query test |
| [#562703](https://github.com/NixOS/nixpkgs/pull/562703) | merged | 2026-09-18 | `e4d65c2` | `3135971` | nixos/cosmic: don't disable geoclue2's demo agent |
| [#563778](https://github.com/NixOS/nixpkgs/pull/563778) | merged | 2026-09-16 | `3d30332` | `2f1677d` | pghero: drop |
| [#564257](https://github.com/NixOS/nixpkgs/pull/564257) | merged | 2026-09-19 | `7aaf716` | `d6880e9` | xreader: 4.6.5 -> 4.6.7, xepub: init at 1.0.1 |

## S2-B: stress cohort (20 PRs)

| PR | merged | base SHA | head SHA | title | matched categories |
|---|---|---|---|---|---|
| [#532540](https://github.com/NixOS/nixpkgs/pull/532540) | 2026-09-10 | `387a70f` | `2360064` | cliproxyapi: init at 7.2.146 | option_declaration, execstart_script_cmdline, environment, package_version_source_dep, nixos_tests_touched |
| [#534100](https://github.com/NixOS/nixpkgs/pull/534100) | 2026-09-16 | `7a659f9` | `a40518f` | lomiri.lomiri: 0.5.0 -> 0.6.1 | environment, package_version_source_dep, nixos_tests_touched |
| [#547038](https://github.com/NixOS/nixpkgs/pull/547038) | 2026-08-28 | `2700fe2` | `e03c04e` | nixos/nginx: add missing types from the referenced compression configs | generated_config, nixos_tests_touched |
| [#549553](https://github.com/NixOS/nixpkgs/pull/549553) | 2026-08-21 | `89b717b` | `85be869` | nixos/nginx: add locations.\<name\>.useGrpcErrorPages option | option_declaration, nixos_tests_touched |
| [#549618](https://github.com/NixOS/nixpkgs/pull/549618) | 2026-09-10 | `0a32ba7` | `c522e75` | nixos/immich: add database.package option | option_declaration |
| [#552038](https://github.com/NixOS/nixpkgs/pull/552038) | 2026-08-25 | `ab01518` | `88ca9d7` | nixos/prometheus-exporters/snowflake: init | option_declaration, execstart_script_cmdline, environment, generated_config, nixos_tests_touched |
| [#552640](https://github.com/NixOS/nixpkgs/pull/552640) | 2026-09-16 | `5ccceb7` | `12685fe` | nixos/rancher: make preloaded images detect derivation changes | environment, nixos_tests_touched |
| [#553474](https://github.com/NixOS/nixpkgs/pull/553474) | 2026-08-22 | `1f29b50` | `5945396` | nixos/tests: filter out nspawn tests on darwin | nixos_tests_touched |
| [#554495](https://github.com/NixOS/nixpkgs/pull/554495) | 2026-09-10 | `b1cf5c8` | `ed67a1e` | hyphanet: rename from freenet | option_declaration, execstart_script_cmdline, nixos_tests_touched |
| [#554949](https://github.com/NixOS/nixpkgs/pull/554949) | 2026-08-26 | `0bea847` | `55a9ca2` | szurubooru: bump ffmpeg_4-full -> ffmpeg-full | package_version_source_dep |
| [#555612](https://github.com/NixOS/nixpkgs/pull/555612) | 2026-09-12 | `3f7e52b` | `0ccb58b` | redis, valkey: migrate to runTest, simplify | nixos_tests_touched |
| [#555635](https://github.com/NixOS/nixpkgs/pull/555635) | 2026-08-24 | `cc29db4` | `1b36c1b` | nixosTests.thelounge: migrate to runTest | nixos_tests_touched |
| [#556119](https://github.com/NixOS/nixpkgs/pull/556119) | 2026-08-27 | `b0b5526` | `dfa4aeb` | nixos/tests/gitea: lots of clean up, reduce resource usage a lot | environment, generated_config, nixos_tests_touched |
| [#556413](https://github.com/NixOS/nixpkgs/pull/556413) | 2026-09-07 | `cb5baef` | `ab5f845` | nixos/test-driver: cleanup junit.xml creation | environment, nixos_tests_touched |
| [#558400](https://github.com/NixOS/nixpkgs/pull/558400) | 2026-09-02 | `52004e3` | `80a5f13` | sing-box: 1.13.21 -> 1.14.0 | package_version_source_dep, nixos_tests_touched |
| [#559239](https://github.com/NixOS/nixpkgs/pull/559239) | 2026-09-06 | `c1436b4` | `2b11bfe` | nixos/selfoss: add support for configuring nginx, add nixos test | option_declaration, nixos_tests_touched |
| [#559393](https://github.com/NixOS/nixpkgs/pull/559393) | 2026-09-03 | `d145581` | `4eb87f4` | go-neb: drop | option_declaration, execstart_script_cmdline, environment, generated_config, package_version_source_dep, nixos_tests_touched |
| [#560664](https://github.com/NixOS/nixpkgs/pull/560664) | 2026-09-10 | `ab2c1b9` | `7edb309` | python3Packages.exllamav3: 1.4.3 -> 1.4.8; tabbyapi: 0-unstable-2026-08-08 -> 0-unstable-2026-09-07 | option_declaration, environment, package_version_source_dep |
| [#561130](https://github.com/NixOS/nixpkgs/pull/561130) | 2026-09-09 | `925fad0` | `b4bb64d` | jellyfin{,-web}: 10.11.11 -> 12.0 | package_version_source_dep, nixos_tests_touched |
| [#561424](https://github.com/NixOS/nixpkgs/pull/561424) | 2026-09-08 | `63ceb04` | `c34b750` | modules/image/repart: fix Format=empty | nixos_tests_touched |

Adjudication (manifest construction, real `oba audit-diff` runs against
the frozen `v0.4.1` binary, manual review) is the next step, in
`results.md`, not this file.
