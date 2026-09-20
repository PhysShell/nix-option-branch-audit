# S3: fresh random (A) + stress (B) cohorts — population and draw

Drawn mechanically per `protocol.md`, frozen here **before any of the
50 PRs below is individually inspected**. Nothing past this point
(adjudication) may change cohort membership.

## A disclosed, pre-execution correction

The first version of this draw (visible in this file's own git
history, commit `9132a78`) used a 253-name exclusion list that merged
S1's original 113 names with every real subject from S2's own drawn
50 PRs and the 93 exporter names S2-F3 examined — but never compiled
S1's OWN drawn 30 PRs' real subject names into that list (only their
PR *numbers* were excluded). Two candidates slipped through as a
result: `#548837` (`sstorytime`, already S1's own real `#556729`) and
`#481112` (`grub2`, the same real `grub.nix` module S1's own `#554779`
os-prober PR already touched). Caught before any fork read either
PR's real content beyond its title (no `oba` run, no manual
adjudication, no file fetched past the population listing itself) —
a legitimate pre-execution protocol correction, the same kind S1's own
protocol received before its own investigation began, not a
retroactive edit after seeing results.

Fixed by compiling S1's own 30 drawn PRs' real subject names (a 28-name
list) into the exclusion set (now 281 names total,
`exclusion-name-list-281.txt`, superseding the original 253-name file)
and re-running the ENTIRE screen — 3 more candidates were excluded by
the corrected list (91 survivors instead of 94) — then both cohorts
were redrawn from scratch, from the corrected pool, using the exact
same seeded mechanism. **The table below is the real, final, frozen
draw** — the one actually adjudicated.

## Population funnel (final, corrected)

```
raw candidates (real commits touching nixos/modules/services/** or
  nixos/tests/** in a real 2026-08-10..2026-09-19 window, path-filtered
  GitHub commits API, no local clone)                               256
  excluded: already drawn in S1 (30) or S2 (50), by exact PR number   79
  eligible (real path check)                                        177
  excluded: real merge-window sanity                                  1
  excluded: purely docs/formatting-only                                2
  excluded: mass mechanical change                                    15
  excluded: already-used app/service name (281-name merged list,
    corrected -- see above)                                           68
survived                                                              91

S3-A draw: seed = int("e05e841", 16) = 235268161,
  random.Random(seed).shuffle, first 30                               30
remaining pool (S3-A's own 30 removed)                                61

S3-B stress filter (EXACT SAME six categories as S2-B, unchanged, on
  real gh pr diff added/removed lines only):
  stress-eligible                                                     48
  not eligible                                                        13

S3-B draw: seed = int("e05e841", 16) + 1 = 235268162 (distinct from
  A's own seed), first 20 of the stress-eligible pool                20
```

A genuinely fresh Prometheus-exporter PR remains in the corrected
S3-A cohort (`#511659`, `kvrocks` module init) — `kvrocks` is not one
of the 93 exporter names S2-F3's own census already examined, so this
is real, unseen data directly testing the known `exporters.nix`
limitation without repeating any already-investigated exporter.

## S3-A: random cohort (30 PRs)

| PR | state | merged | base SHA | head SHA | title |
|---|---|---|---|---|---|
| [#452850](https://github.com/NixOS/nixpkgs/pull/452850) | merged | 2026-08-12 | `5c149cd` | `f77e2d9` | nixos/tests/nullmailer: init |
| [#461073](https://github.com/NixOS/nixpkgs/pull/461073) | merged | 2026-08-22 | `37ee210` | `f1935fe` | udp514-journal: new package and module - forwarding remote logs to journal. |
| [#495112](https://github.com/NixOS/nixpkgs/pull/495112) | merged | 2026-08-26 | `eb0b168` | `cd325b4` | kaidan: 0.14.0 -> 0.16.0  |
| [#495731](https://github.com/NixOS/nixpkgs/pull/495731) | merged | 2026-09-03 | `ea75749` | `f3d6ad4` | nixos/qbit-manage: init module |
| [#511659](https://github.com/NixOS/nixpkgs/pull/511659) | merged | 2026-08-13 | `c45f39c` | `da59549` | nixos/kvrocks: init |
| [#512581](https://github.com/NixOS/nixpkgs/pull/512581) | merged | 2026-09-08 | `5a3564b` | `7271b8c` | nixos/logrotate: Copy configFile to /etc, remove option |
| [#532071](https://github.com/NixOS/nixpkgs/pull/532071) | merged | 2026-08-17 | `9e4ba99` | `cdb300c` | homebox: 0.25.0 -> 0.26.2 |
| [#534234](https://github.com/NixOS/nixpkgs/pull/534234) | merged | 2026-09-17 | `b55025c` | `6a66dea` | canaille: 0.2.7 -> 0.3.6 |
| [#545002](https://github.com/NixOS/nixpkgs/pull/545002) | merged | 2026-08-21 | `8209d66` | `6718b04` | nixos/crab-hole: Add `openFirewall` option to `services.crab-hole` |
| [#545183](https://github.com/NixOS/nixpkgs/pull/545183) | merged | 2026-09-04 | `4d38b10` | `28bcf71` | pumpkin: init at 0-unstable-2026-07-25; nixos/pumpkin: init module |
| [#549663](https://github.com/NixOS/nixpkgs/pull/549663) | merged | 2026-08-12 | `a493fe6` | `0c2b525` | nixos/tests/rosenpass: various fixes |
| [#550492](https://github.com/NixOS/nixpkgs/pull/550492) | merged | 2026-08-17 | `c91b889` | `cd6f94c` | nixos/hostapd: fix WPA3 transition mode |
| [#550650](https://github.com/NixOS/nixpkgs/pull/550650) | merged | 2026-09-06 | `b85f329` | `a2ed9f7` | nixos/wireless: add pkcs11 option for smartcard/TPM EAP-TLS |
| [#551640](https://github.com/NixOS/nixpkgs/pull/551640) | merged | 2026-08-12 | `e31d19f` | `d578214` | matrix-continuwuity: 26.7.2 -> 26.7.3 |
| [#553854](https://github.com/NixOS/nixpkgs/pull/553854) | merged | 2026-08-18 | `c4be37b` | `7eb0bf4` | nixos/avahi: fix on 32 bit host platforms |
| [#554045](https://github.com/NixOS/nixpkgs/pull/554045) | merged | 2026-08-19 | `3138554` | `8c24648` | curl-impersonate: adopt, switch to maintained fork and update 1.5.6 -> 2.1.0 |
| [#554366](https://github.com/NixOS/nixpkgs/pull/554366) | merged | 2026-09-04 | `4aa7aa0` | `775afc1` | git-pages.services.default: init |
| [#555151](https://github.com/NixOS/nixpkgs/pull/555151) | merged | 2026-08-21 | `89b717b` | `403a052` | nixos/frigate: use isolated /dev/shm for frame buffers |
| [#555621](https://github.com/NixOS/nixpkgs/pull/555621) | merged | 2026-09-12 | `01d9fdc` | `3c7951d` | nixosTests.kerberos: migrate to runTest |
| [#555707](https://github.com/NixOS/nixpkgs/pull/555707) | merged | 2026-09-11 | `b3431be` | `ca69071` | cloudcompare: 2.13.2 -> 2.13.2-unstable-2026-08-21 |
| [#555801](https://github.com/NixOS/nixpkgs/pull/555801) | merged | 2026-09-01 | `0ad5f09` | `0859607` | nixos/syncthing: escape properly ignorePatterns |
| [#556461](https://github.com/NixOS/nixpkgs/pull/556461) | merged | 2026-08-26 | `044bc0d` | `7c85b4a` | nixos/matter-server: fix service with enableStrictShellChecks enabled |
| [#558154](https://github.com/NixOS/nixpkgs/pull/558154) | merged | 2026-09-01 | `9df1985` | `ffc29d1` | nixos/paperless: fix paperless-manage wrapper |
| [#558600](https://github.com/NixOS/nixpkgs/pull/558600) | merged | 2026-09-08 | `ae82ee9` | `441f3c5` | nixos/syncoid: catch up missed timer runs (Persistent=true) |
| [#558981](https://github.com/NixOS/nixpkgs/pull/558981) | merged | 2026-09-15 | `c09b88d` | `1122c04` | nixos/printers: collect provisioning into CUPS' `ExecStartPost` |
| [#561987](https://github.com/NixOS/nixpkgs/pull/561987) | merged | 2026-09-10 | `408915d` | `2d72ab3` | nixos/coder: add url to Coder configuration reference |
| [#562044](https://github.com/NixOS/nixpkgs/pull/562044) | merged | 2026-09-12 | `a22c3bf` | `23ca81b` | nixos/suricata: add support for --drop-conf rules |
| [#563010](https://github.com/NixOS/nixpkgs/pull/563010) | merged | 2026-09-14 | `ba3c3d8` | `bb8da5e` | sickgear: drop |
| [#563907](https://github.com/NixOS/nixpkgs/pull/563907) | merged | 2026-09-19 | `e9c17a0` | `20ee504` | nixos/nordvpn: inherit maintainers from package |
| [#564742](https://github.com/NixOS/nixpkgs/pull/564742) | merged | 2026-09-19 | `1639aff` | `c525824` | harmonia: 3.2.0 -> 3.3.0; nixos/harmonia: add services.harmonia.gc |

## S3-B: stress cohort (20 PRs)

| PR | merged | base SHA | head SHA | title | matched categories |
|---|---|---|---|---|---|
| [#438001](https://github.com/NixOS/nixpkgs/pull/438001) | merged | 2026-09-07 | `563bfc7` | `0fc899c` | nixos/mealie: add openFirewall option | option_declaration |
| [#510342](https://github.com/NixOS/nixpkgs/pull/510342) | merged | 2026-08-14 | `0b4694d` | `54959bc` | nixos/userborn: import legacy /var/lib/nixos state on first run | option_declaration, execstart_script_cmdline, nixos_tests_touched |
| [#512162](https://github.com/NixOS/nixpkgs/pull/512162) | merged | 2026-08-13 | `63c04e9` | `f5087b9` | {hister,nixos/hister,nixosTests.hister}: init at 0.17.0 | option_declaration, execstart_script_cmdline, environment, generated_config, package_version_source_dep, nixos_tests_touched |
| [#516128](https://github.com/NixOS/nixpkgs/pull/516128) | merged | 2026-08-21 | `de357df` | `cdeb027` | nixos/tinyauth: add enableUnixSocket option | option_declaration |
| [#537675](https://github.com/NixOS/nixpkgs/pull/537675) | merged | 2026-09-10 | `3130a1c` | `7444c53` | nixos/softether: fix ExecStart failing on read-only /nix/store | execstart_script_cmdline |
| [#547607](https://github.com/NixOS/nixpkgs/pull/547607) | merged | 2026-08-11 | `a5dfdaa` | `6308537` | romm: init at 5.1.0; nixos/romm: init | option_declaration, execstart_script_cmdline, environment, generated_config, package_version_source_dep, nixos_tests_touched |
| [#548734](https://github.com/NixOS/nixpkgs/pull/548734) | merged | 2026-08-12 | `e8b5226` | `f54c56f` | nixosTests.opencloud: fix LDAPS and aarch64 WebDriver setup | nixos_tests_touched |
| [#548856](https://github.com/NixOS/nixpkgs/pull/548856) | merged | 2026-08-17 | `4164b51` | `0d5649a` | nixos/dnscrypt-proxy: fix `.settings` type | generated_config, nixos_tests_touched |
| [#550993](https://github.com/NixOS/nixpkgs/pull/550993) | merged | 2026-08-19 | `0eae3d8` | `48fd303` | rustical: 0.14.1 -> 0.15.0 | execstart_script_cmdline, package_version_source_dep |
| [#551186](https://github.com/NixOS/nixpkgs/pull/551186) | merged | 2026-08-12 | `c416877` | `e7c2e80` | llama-swap: 240 -> 249 | package_version_source_dep, nixos_tests_touched |
| [#552777](https://github.com/NixOS/nixpkgs/pull/552777) | merged | 2026-08-22 | `cd5cb69` | `03ac550` | mediawiki: 1.45.4 -> 1.46.0 | generated_config, package_version_source_dep, nixos_tests_touched |
| [#553268](https://github.com/NixOS/nixpkgs/pull/553268) | merged | 2026-08-19 | `12979a2` | `74c42c5` | nixos/tests/printing: migrate to remove PCRE usage | nixos_tests_touched |
| [#553645](https://github.com/NixOS/nixpkgs/pull/553645) | merged | 2026-08-18 | `d3773e0` | `e78efec` | taskwarrior3: 3.4.2 -> 3.5.0 | package_version_source_dep, nixos_tests_touched |
| [#553699](https://github.com/NixOS/nixpkgs/pull/553699) | merged | 2026-08-26 | `22b6e72` | `f1fb012` | nixos/nebula: Add user & group settings, and assertions for truncated TUN device names | option_declaration |
| [#554171](https://github.com/NixOS/nixpkgs/pull/554171) | merged | 2026-08-20 | `2dd07ae` | `710173b` | nixos/atuin: fix documentation link | environment |
| [#555067](https://github.com/NixOS/nixpkgs/pull/555067) | merged | 2026-08-21 | `c32ae8e` | `a464eca` | matrix-synapse: 1.158.0 -> 1.159.0, fix test | package_version_source_dep, nixos_tests_touched |
| [#555212](https://github.com/NixOS/nixpkgs/pull/555212) | merged | 2026-08-22 | `9b725db` | `7b1c10a` | jool: 4.1.14 -> 4.1.15 | package_version_source_dep, nixos_tests_touched |
| [#555615](https://github.com/NixOS/nixpkgs/pull/555615) | merged | 2026-08-24 | `3f7e52b` | `eb7be27` | nixosTests.hitch: migrate to runTest | environment, nixos_tests_touched |
| [#556069](https://github.com/NixOS/nixpkgs/pull/556069) | merged | 2026-08-31 | `bbc4f9f` | `642e370` | ocamlPackages.kapla: init at 0.4.0 | option_declaration, execstart_script_cmdline, environment, package_version_source_dep, nixos_tests_touched |
| [#562127](https://github.com/NixOS/nixpkgs/pull/562127) | merged | 2026-09-14 | `583ed41` | `57ae5ac` | nixos/tests/duplicity: init | environment, nixos_tests_touched |

Adjudication (manifest construction, real `oba audit-diff` runs
against the frozen `v0.4.2` binary, the user's own full 11-point
manual adjudication schema) is the next step, in `results.md`, not
this file.
