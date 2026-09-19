# S1: live nixpkgs PR shadow evaluation -- population and draw

Drawn mechanically per `protocol.md`, frozen here **before any of the
30 PRs below is individually inspected**. Nothing past this point
(adjudication) may change the membership of this list.

## Funnel

```
population (raw candidate PR numbers, from commits touching
  nixos/modules/services/** or nixos/tests/** in the real
  2026-08-21..2026-09-19 window, path-filtered GitHub commits API,
  no local clone)                                            196
  eligible (re-verified: >=1 real changed path under
    nixos/modules/services/ or nixos/tests/)                 196
  excluded: real mergedAt/state outside the observed window     0
  excluded: purely docs/formatting-only                          1
  excluded: mass mechanical change                               12
  excluded: already-used app/service name                        21
survived                                                        162
drawn (seed = int("67bb2e1", 16) = 108770017,
  random.Random(seed).shuffle, first 30)                        30
```

34 candidates were excluded outright (1 docs + 12 mechanical + 21
already-used name), leaving 162 survivors to draw from -- more than 5x
the sample size, so the draw was not forced to reach into a thin tail.

One disclosed false-positive in the name-exclusion step, noted for
honesty rather than silently corrected after the fact: PR #558384
(`repath-studio: fix flaky tests in sandbox`) was excluded for
matching `karma` -- the real match was a vendored patch file named
`fix-karma-sandbox.patch` (the unrelated JavaScript "Karma" test
runner), not this project's own `karma` dashboard candidate. This is
an over-exclusion (the safe direction of error for a "don't leak an
already-tuned name back in" filter) and does not affect the validity
of the 162-candidate survivor pool the draw was made from.

## The 30 drawn PRs

| PR | state | merged | base SHA | head SHA | title |
|---|---|---|---|---|---|
| [#347823](https://github.com/NixOS/nixpkgs/pull/347823) | merged | 2026-08-21 | `d68cbd8` | `87ce79b` | nixos/immich: add doc & test for listen to all interfaces |
| [#492803](https://github.com/NixOS/nixpkgs/pull/492803) | merged | 2026-09-15 | `ae98b8d` | `ff089f6` | nixos/ntfy: remove redundant user creation |
| [#496303](https://github.com/NixOS/nixpkgs/pull/496303) | merged | 2026-09-17 | `93b21a6` | `805b3b4` | opensearch-dashboards: init at 3.8.0 |
| [#519494](https://github.com/NixOS/nixpkgs/pull/519494) | merged | 2026-08-22 | `d838967` | `afbf4a3` | rosec: init at 0.0.28 |
| [#525702](https://github.com/NixOS/nixpkgs/pull/525702) | merged | 2026-08-26 | `1e2f85c` | `7e52e1b` | fleet-{desktop,orbit}: init at 1.55.0 |
| [#527821](https://github.com/NixOS/nixpkgs/pull/527821) | merged | 2026-09-11 | `7a14922` | `cd3f02d` | iocaine: 2.5.1 -> 3.5.1; nixos/iocaine: init module |
| [#539076](https://github.com/NixOS/nixpkgs/pull/539076) | merged | 2026-09-12 | `2f788ab` | `759c638` | lego: 4.35.2 -> 5.4.1 |
| [#543675](https://github.com/NixOS/nixpkgs/pull/543675) | merged | 2026-09-08 | `e9b9cbe` | `ed4375e` | nixos/openvswitch: clean up transient ports on boot |
| [#549506](https://github.com/NixOS/nixpkgs/pull/549506) | merged | 2026-08-26 | `21e4866` | `dd959a0` | kener,nixos/kener: init at 4.1.2, init module |
| [#550960](https://github.com/NixOS/nixpkgs/pull/550960) | merged | 2026-08-27 | `b04b1df` | `13a8f1e` | wivrn: enable building debug information, remove unnecessary default service env vars |
| [#551955](https://github.com/NixOS/nixpkgs/pull/551955) | merged | 2026-08-25 | `823b3ea` | `2c1ed27` | nixos/prometheus-exporters/yace: init |
| [#553349](https://github.com/NixOS/nixpkgs/pull/553349) | merged | 2026-09-09 | `c2258dc` | `7a33d0d` | nixos/cassandra: use structuredAttrs instead of passAsFile |
| [#553770](https://github.com/NixOS/nixpkgs/pull/553770) | merged | 2026-08-24 | `dab930e` | `35e6b55` | nixos/portunus: replace dex' callbackURL with redirectURIs (allows multiple entries) |
| [#554779](https://github.com/NixOS/nixpkgs/pull/554779) | merged | 2026-09-02 | `fa10bdf` | `5bf6dfb` | os-prober: 1.84 -> 1.85 |
| [#555805](https://github.com/NixOS/nixpkgs/pull/555805) | merged | 2026-08-28 | `941ede6` | `d92ae8b` | silverbullet: 2.9.0 -> 2.10.0 |
| [#556710](https://github.com/NixOS/nixpkgs/pull/556710) | merged | 2026-09-07 | `d763eda` | `6077cbc` | nixos/{fail2ban,prometheus-exporter/fail2ban}: use group for socket permissions + fixes |
| [#556729](https://github.com/NixOS/nixpkgs/pull/556729) | merged | 2026-08-26 | `e850c5f` | `6528392` | sstorytime: 0-unstable-2026-08-12 -> 1.0-beta-unstable-2026-08-20 |
| [#557545](https://github.com/NixOS/nixpkgs/pull/557545) | merged | 2026-08-31 | `cc2cf83` | `7d47ca9` | nixos/mediamtx: use correct yaml version |
| [#558121](https://github.com/NixOS/nixpkgs/pull/558121) | merged | 2026-09-01 | `5511771` | `79d0296` | nixos/tests/systemd-networkd-vrf: fix race when checking routing tables |
| [#558854](https://github.com/NixOS/nixpkgs/pull/558854) | merged | 2026-09-07 | `7499398` | `5a1a5ef` | dawarich: 1.14.0 -> 1.14.4 |
| [#559055](https://github.com/NixOS/nixpkgs/pull/559055) | merged | 2026-09-07 | `ab2c1b9` | `5024509` | nixos/opentelemetry-collector: validate configs built from settings |
| [#559588](https://github.com/NixOS/nixpkgs/pull/559588) | merged | 2026-09-16 | `d025c96` | `c3472d0` | nixos/netbird-relay: init module |
| [#559627](https://github.com/NixOS/nixpkgs/pull/559627) | merged | 2026-09-11 | `27fab83` | `6a860c7` | nixos/btrfs: refactor autoScrub services |
| [#560647](https://github.com/NixOS/nixpkgs/pull/560647) | merged | 2026-09-11 | `022cda5` | `12e2ac7` | libp11: 0.4.18 -> 0.4.21 |
| [#561557](https://github.com/NixOS/nixpkgs/pull/561557) | merged | 2026-09-09 | `0e44f01` | `28e11f6` | modules/image: make inclusion without enable a noop |
| [#561669](https://github.com/NixOS/nixpkgs/pull/561669) | merged | 2026-09-12 | `4dffc10` | `fcb300f` | nixos/librespeed: use structuredAttrs instead of passAsFile |
| [#561845](https://github.com/NixOS/nixpkgs/pull/561845) | merged | 2026-09-10 | `3dc54db` | `f7febd5` | nixos/komodo-periphery: fix path issues for docker, git, and system tools |
| [#563958](https://github.com/NixOS/nixpkgs/pull/563958) | merged | 2026-09-18 | `fe8419c` | `79b249f` | nixos/noctalia-greeter: add passwordlessSyncUsers option |
| [#564021](https://github.com/NixOS/nixpkgs/pull/564021) | merged | 2026-09-17 | `b0a239e` | `370bc15` | nixos/tests/freshrss: fix extensions test for 1.30.0 |
| [#564357](https://github.com/NixOS/nixpkgs/pull/564357) | merged | 2026-09-19 | `60b13be` | `f515b4e` | coredns: 1.14.6 -> 1.14.7 |

All 30 landed as MERGED (the open-PR side of the population produced
no survivors after screening in this particular draw window -- an
honest artifact of when this draw happened, not a rule against open
PRs).

Adjudication (per-PR manifest construction, real `oba audit-diff`
runs against the frozen `v0.4.0` binary, and manual review) follows in
`results.md`, not in this file.
