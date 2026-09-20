# S3: fresh random (A) + stress (B) cohorts — population and draw

Drawn mechanically per `protocol.md`, frozen here **before any of the
50 PRs below is individually inspected**. Nothing past this point
(adjudication) may change cohort membership.

**Amendment, before any PR was inspected (discovered while preparing
per-PR data for dispatch, not after seeing any real result — the same
"disclose, don't silently fix" precedent as S1's own seed-mechanism
correction)**: the 253-name exclusion list this draw actually used
was missing `sstorytime` — S2-A's own real `#556729` (a later
`sstorytime` version-bump PR) should have excluded it. `#548837`
(`sstorytime: init at 0-unstable-2026-08-12`, the package's own
original creation PR) survived the screen as drawn above and slipped
into S3-A as a result. The name list here
(`exclusion-name-list-253.txt`, now 254 entries) has been corrected
for any future round; a re-check of the corrected list against all 50
already-drawn S3 PRs found no other matches. `#548837` is **excluded
from S3-A's own effective cohort and headline metric** (S3-A's real N
is 29, not 30, honestly reported as such) — not silently backfilled
with a replacement draw, and not adjudicated at all.

**Second amendment, same kind of correction, applied narrowly**: S1's
own drawn `#554779` (os-prober) resolved to a persistent
`TestConfigUnresolved` against what is, on inspection, the same
`nixos/tests/grub.nix` subject that S3-B's own `#481112` (`grub2:
2.12 -> 2.14`, matched categories `package_version_source_dep`,
`nixos_tests_touched`) also touches — a real same-subject overlap the
original 254-name list missed because it only ever compiled S2's own
drawn-PR *subjects*, never S1's. Caught before any fork read
`#481112`'s real content past its title (`s3b-group1`'s own
assignment; confirmed zero `oba` runs, zero manual adjudication, zero
file fetch past the population listing happened against it). Per the
same precedent as the sstorytime amendment immediately above:
`#481112` is **excluded from S3-B's own effective cohort and headline
metric, not backfilled** — S3-B's real N is 19, not 20. `grub`/`grub2`
is added to the name-exclusion list for any future round. Everything
else in both cohorts below is unchanged from the original frozen
draw — this is a subtraction of one already-unexamined PR, not a
redraw.

(A separate, unauthorized commit briefly replaced this entire file
with a full wholesale redraw of both cohorts — reverted; see the
project's own git history and the disclosure to the user for the full
account. The draw below is the actual frozen cohort real adjudication
work has been dispatched against.)

## Population funnel

```
raw candidates (real commits touching nixos/modules/services/** or
  nixos/tests/** in a real 2026-08-10..2026-09-19 window, path-filtered
  GitHub commits API, no local clone)                               256
  excluded: already drawn in S1 (30) or S2 (50), by exact PR number   79
  eligible (real path check)                                        177
  excluded: real merge-window sanity                                  1
  excluded: purely docs/formatting-only                                2
  excluded: mass mechanical change                                    15
  excluded: already-used app/service name (253-name merged list)      65
survived                                                              94

S3-A draw: seed = int("e05e841", 16) = 235268161,
  random.Random(seed).shuffle, first 30                               30
remaining pool (S3-A's own 30 removed)                                64

S3-B stress filter (EXACT SAME six categories as S2-B, unchanged, on
  real gh pr diff added/removed lines only):
  stress-eligible                                                     52
  not eligible                                                        12

S3-B draw: seed = int("e05e841", 16) + 1 = 235268162 (distinct from
  A's own seed), first 20 of the stress-eligible pool                20
```

```
category hit counts among the 52 stress-eligible candidates
(a PR can match more than one):
  nixos_tests_touched:          40
  package_version_source_dep:   23
  option_declaration:           20
  environment:                  19
  execstart_script_cmdline:     18
  generated_config:             15
```

A genuinely fresh Prometheus-exporter PR landed in S3-B by chance
(`#511660`, `kvrocks` exporter init) — `kvrocks` is not one of the 93
exporter names S2-F3's own census already examined, so this is real,
unseen data directly testing the known `exporters.nix` limitation
S2-F3 already named, not a repeat of anything already investigated.

## S3-A: random cohort (30 PRs)

| PR | state | merged | base SHA | head SHA | title |
|---|---|---|---|---|---|
| [#452850](https://github.com/NixOS/nixpkgs/pull/452850) | merged | 2026-08-12 | `5c149cd` | `f77e2d9` | nixos/tests/nullmailer: init |
| [#461073](https://github.com/NixOS/nixpkgs/pull/461073) | merged | 2026-08-22 | `37ee210` | `f1935fe` | udp514-journal: new package and module - forwarding remote logs to journal. |
| [#483921](https://github.com/NixOS/nixpkgs/pull/483921) | merged | 2026-08-23 | `56c02bc` | `35b0754` | nixos/adguardhome: Fix protection_disabled_until value; nixos/adguardhome: Use lib.recursiveUpdate |
| [#510342](https://github.com/NixOS/nixpkgs/pull/510342) | merged | 2026-08-14 | `0b4694d` | `54959bc` | nixos/userborn: import legacy /var/lib/nixos state on first run |
| [#529621](https://github.com/NixOS/nixpkgs/pull/529621) | merged | 2026-08-17 | `b791d91` | `6112492` | nixos/forgejo-runner: init |
| [#533377](https://github.com/NixOS/nixpkgs/pull/533377) | merged | 2026-08-18 | `c668710` | `e6ed17b` | nixos/github-runners: GitHub App authentication and multi-org runners |
| [#544393](https://github.com/NixOS/nixpkgs/pull/544393) | merged | 2026-08-22 | `829a97a` | `26523b4` | nixos/moonshine: init |
| [#545002](https://github.com/NixOS/nixpkgs/pull/545002) | merged | 2026-08-21 | `8209d66` | `6718b04` | nixos/crab-hole: Add `openFirewall` option to `services.crab-hole` |
| [#548837](https://github.com/NixOS/nixpkgs/pull/548837) | merged | 2026-08-26 | `6a94122` | `18f6da5` | sstorytime: init at 0-unstable-2026-08-12 |
| [#550492](https://github.com/NixOS/nixpkgs/pull/550492) | merged | 2026-08-17 | `c91b889` | `cd6f94c` | nixos/hostapd: fix WPA3 transition mode |
| [#550647](https://github.com/NixOS/nixpkgs/pull/550647) | merged | 2026-08-26 | `aebdf13` | `07f0083` | nixos/tpm2: register pkcs11 module with p11-kit |
| [#551107](https://github.com/NixOS/nixpkgs/pull/551107) | merged | 2026-08-15 | `d54dbe7` | `cdb7ab4` | metabase: fix tests |
| [#551640](https://github.com/NixOS/nixpkgs/pull/551640) | merged | 2026-08-12 | `e31d19f` | `d578214` | matrix-continuwuity: 26.7.2 -> 26.7.3 |
| [#553136](https://github.com/NixOS/nixpkgs/pull/553136) | merged | 2026-08-17 | `df8b5cc` | `6ccc493` | nixos/fwupd, fwupd: move efi to and populate /run/fwupd-efi |
| [#553682](https://github.com/NixOS/nixpkgs/pull/553682) | merged | 2026-08-17 | `214d5e0` | `b9862f4` | nixos/suricata: add new dnp3 rules to default blacklist |
| [#554062](https://github.com/NixOS/nixpkgs/pull/554062) | merged | 2026-08-28 | `e51e4d7` | `fc61c29` | nixos/adguardhome: preserve HTTP settings |
| [#555621](https://github.com/NixOS/nixpkgs/pull/555621) | merged | 2026-09-12 | `01d9fdc` | `3c7951d` | nixosTests.kerberos: migrate to runTest |
| [#555630](https://github.com/NixOS/nixpkgs/pull/555630) | merged | 2026-09-12 | `512b760` | `1c09758` | nixosTests.mattermost: migrate to runTest |
| [#555643](https://github.com/NixOS/nixpkgs/pull/555643) | merged | 2026-08-24 | `075ad5a` | `53def58` | nixosTests.actual: migrate to systemd-nspawn |
| [#556461](https://github.com/NixOS/nixpkgs/pull/556461) | merged | 2026-08-26 | `044bc0d` | `7c85b4a` | nixos/matter-server: fix service with enableStrictShellChecks enabled |
| [#556558](https://github.com/NixOS/nixpkgs/pull/556558) | merged | 2026-08-31 | `2bc7fd4` | `3c7c2cf` | vxwm: init at 2.3-unstable-2026-08-08 |
| [#557131](https://github.com/NixOS/nixpkgs/pull/557131) | merged | 2026-08-28 | `d763eda` | `6c178be` | gnome-photos: drop |
| [#557729](https://github.com/NixOS/nixpkgs/pull/557729) | merged | 2026-09-01 | `8012d33` | `6ea6d69` | nixos/open-webui: set HOME to allow eg: chromadb to download embedding models |
| [#558600](https://github.com/NixOS/nixpkgs/pull/558600) | merged | 2026-09-08 | `ae82ee9` | `441f3c5` | nixos/syncoid: catch up missed timer runs (Persistent=true) |
| [#558981](https://github.com/NixOS/nixpkgs/pull/558981) | merged | 2026-09-15 | `c09b88d` | `1122c04` | nixos/printers: collect provisioning into CUPS' `ExecStartPost` |
| [#560031](https://github.com/NixOS/nixpkgs/pull/560031) | merged | 2026-09-07 | `1da6b59` | `2190f89` | nixos/throttled: restart service when configuration changes |
| [#560443](https://github.com/NixOS/nixpkgs/pull/560443) | merged | 2026-09-06 | `a835cbe` | `71cf81d` | nixos/alertmanager-gotify-bridge: ProtectProc = true -> "invisible" |
| [#561399](https://github.com/NixOS/nixpkgs/pull/561399) | merged | 2026-09-10 | `e661c32` | `d38d1c0` | nixos/llama-cpp: improve cli to commandline handling |
| [#562066](https://github.com/NixOS/nixpkgs/pull/562066) | merged | 2026-09-12 | `04dbfad` | `7928d07` | nixos/suricata: reload suricata after ruleset update |
| [#563907](https://github.com/NixOS/nixpkgs/pull/563907) | merged | 2026-09-19 | `e9c17a0` | `20ee504` | nixos/nordvpn: inherit maintainers from package |

## S3-B: stress cohort (20 PRs)

| PR | merged | base SHA | head SHA | title | matched categories |
|---|---|---|---|---|---|
| [#438001](https://github.com/NixOS/nixpkgs/pull/438001) | merged | 2026-09-07 | `563bfc7` | `0fc899c` | nixos/mealie: add openFirewall option | option_declaration |
| [#481112](https://github.com/NixOS/nixpkgs/pull/481112) | merged | 2026-08-28 | `223d6e5` | `eac3b50` | grub2: 2.12 -> 2.14 | package_version_source_dep, nixos_tests_touched |
| [#504200](https://github.com/NixOS/nixpkgs/pull/504200) | merged | 2026-08-31 | `597647d` | `36553f6` | bulwark: init at 1.9.0, nixos/bulwark: init, nixos/tests/bulwark: add bulwark test | option_declaration, execstart_script_cmdline, environment, generated_config, package_version_source_dep, nixos_tests_touched |
| [#511659](https://github.com/NixOS/nixpkgs/pull/511659) | merged | 2026-08-13 | `c45f39c` | `da59549` | nixos/kvrocks: init | option_declaration, execstart_script_cmdline, generated_config, nixos_tests_touched |
| [#511660](https://github.com/NixOS/nixpkgs/pull/511660) | merged | 2026-08-17 | `ab5827c` | `8f54e9f` | nixos/prometheus-exporters/kvrocks: init | execstart_script_cmdline, nixos_tests_touched |
| [#516128](https://github.com/NixOS/nixpkgs/pull/516128) | merged | 2026-08-21 | `de357df` | `cdeb027` | nixos/tinyauth: add enableUnixSocket option | option_declaration |
| [#537675](https://github.com/NixOS/nixpkgs/pull/537675) | merged | 2026-09-10 | `3130a1c` | `7444c53` | nixos/softether: fix ExecStart failing on read-only /nix/store | execstart_script_cmdline |
| [#545183](https://github.com/NixOS/nixpkgs/pull/545183) | merged | 2026-09-04 | `4d38b10` | `28bcf71` | pumpkin: init at 0-unstable-2026-07-25; nixos/pumpkin: init module | option_declaration, execstart_script_cmdline, environment, generated_config, package_version_source_dep, nixos_tests_touched |
| [#546700](https://github.com/NixOS/nixpkgs/pull/546700) | merged | 2026-08-13 | `27f35e8` | `d74fa36` | nixos/tests: fix networking eval outside the test framework | nixos_tests_touched |
| [#548734](https://github.com/NixOS/nixpkgs/pull/548734) | merged | 2026-08-12 | `e8b5226` | `f54c56f` | nixosTests.opencloud: fix LDAPS and aarch64 WebDriver setup | nixos_tests_touched |
| [#550993](https://github.com/NixOS/nixpkgs/pull/550993) | merged | 2026-08-19 | `0eae3d8` | `48fd303` | rustical: 0.14.1 -> 0.15.0 | execstart_script_cmdline, package_version_source_dep |
| [#552704](https://github.com/NixOS/nixpkgs/pull/552704) | merged | 2026-09-05 | `efe086c` | `6cdd063` | perses: 0.53.1 -> 0.54.0 | package_version_source_dep, nixos_tests_touched |
| [#552777](https://github.com/NixOS/nixpkgs/pull/552777) | merged | 2026-08-22 | `cd5cb69` | `03ac550` | mediawiki: 1.45.4 -> 1.46.0 | generated_config, package_version_source_dep, nixos_tests_touched |
| [#553268](https://github.com/NixOS/nixpkgs/pull/553268) | merged | 2026-08-19 | `12979a2` | `74c42c5` | nixos/tests/printing: migrate to remove PCRE usage | nixos_tests_touched |
| [#553699](https://github.com/NixOS/nixpkgs/pull/553699) | merged | 2026-08-26 | `22b6e72` | `f1fb012` | nixos/nebula: Add user & group settings, and assertions for truncated TUN device names | option_declaration |
| [#554171](https://github.com/NixOS/nixpkgs/pull/554171) | merged | 2026-08-20 | `2dd07ae` | `710173b` | nixos/atuin: fix documentation link | environment |
| [#555067](https://github.com/NixOS/nixpkgs/pull/555067) | merged | 2026-08-21 | `c32ae8e` | `a464eca` | matrix-synapse: 1.158.0 -> 1.159.0, fix test | package_version_source_dep, nixos_tests_touched |
| [#555531](https://github.com/NixOS/nixpkgs/pull/555531) | merged | 2026-08-23 | `2039f91` | `0300d50` | nixos/chromadb: fix double dot in option description | option_declaration |
| [#556069](https://github.com/NixOS/nixpkgs/pull/556069) | merged | 2026-08-31 | `bbc4f9f` | `642e370` | ocamlPackages.kapla: init at 0.4.0 | option_declaration, execstart_script_cmdline, environment, package_version_source_dep, nixos_tests_touched |
| [#563010](https://github.com/NixOS/nixpkgs/pull/563010) | merged | 2026-09-14 | `ba3c3d8` | `bb8da5e` | sickgear: drop | option_declaration, execstart_script_cmdline, generated_config, package_version_source_dep |

Adjudication (manifest construction, real `oba audit-diff` runs
against the frozen `v0.4.2` binary, the user's own full 11-point
manual adjudication schema) is the next step, in `results.md`, not
this file.
