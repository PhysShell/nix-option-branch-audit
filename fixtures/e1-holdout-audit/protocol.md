# E1: frozen-code holdout generalization audit

**Question**: does the model this project already built (OBA's generic
Layer-1 pipeline, and the CDC K1-K5 adapters/abstractions) generalize to
real NixOS service modules that played NO role in building it -- or does
it only work on the corpus it was shaped by?

Not a new feature round. A measurement.

## Freeze

`src/cdc.rs` and `src/main.rs` (all analysis logic) are FROZEN at commit
`c8e42a1` for the duration of E1. No production-code change of any kind
in response to an E1 finding until the census closes and is reported.

**What "production code" means here, made explicit before any
investigation started** (the exact question a benchmark run this way
could otherwise quietly cheat on): `src/cdc.rs`/`src/main.rs` and their
own `#[cfg(test)]` modules are frozen. Writing new INPUT fixtures (a
holdout candidate's own real `module.nix`/`test.nix`, vendored the same
way every prior round vendored source) and a new target manifest
(`targets/e1-holdout.toml`, or per-case manifests) to RUN the existing,
unchanged `oba check` against them is not a production-code change --
it's the audit's own mechanism, the same way `targets/golden.toml`
itself was never considered "production code." If a candidate needs
`src/cdc.rs`/`src/main.rs` to change AT ALL to be analyzable, the
correct recorded outcome is `unsupported` / `special-case-would-be-
needed`, not a quick patch to make it green.

## Exclusion list (every app touched anywhere in K1-K5, deep or shallow)

Compiled from every census/corpus this project's K1-K5 work ever
produced -- `fixtures/cdc/k2c-census/`, `fixtures/cdc/k3b-drift-census/`
(+ `k3b1-corpus-expansion.md`), `fixtures/cdc/k4b-cli-drift-census/`,
`fixtures/cdc/k5b-env-drift-census/`, plus K1's own kimai/davis. Deliberately
includes apps that were only ever *identified* in a candidate list and
never deep-investigated -- any exposure at all disqualifies an app from
being a genuine holdout, even a name seen once in a table.

```
kimai davis strichliste part-db agorakit movim snipe-it flarum baikal
bookstack civicrm engelsystem grocy invoiceplane librenms postfixadmin
moodle limesurvey zabbix dolibarr mediawiki firefly-iii speedtest-tracker
pixelfed tt-rss freshrss freescout vector krill turn-rs realm rebuilderd
geph mimir tempo karma blocky autobrr listmonk artalk glance dgraph
plikd restic soft-serve ifstate borgbackup searxng grafana vaultwarden
lldap hatsu miniflux shiori fider headscale atticd lemmy alerta kener
traccar firefox-syncserver tap warpgate bookorbit cocoon docuseal
glitchtip nextcloud-notify_push outline papra plausible umami windmill
zerobyte zipline linkwarden
```

77 names, 47 of which actually matched an entry in the population below
(the rest never had a same-named top-level `nixos/tests/*.nix` file to
begin with, so they couldn't have collided with the draw regardless).

## Population and draw method (mechanical, pre-registered, fixed before
any candidate was inspected)

1. Real `NixOS/nixpkgs` master tree, `68740713a1d5904edf9ba92a998a522b1b6ce080`
   (fetched via the GitHub trees API, not this project's own
   `PhysShell/nixpkgs` fork -- E1 is about the real, current upstream
   population, not this project's own historically-pinned corpus).
2. Population = every name `X` such that BOTH `nixos/tests/X.nix` AND
   some `nixos/modules/services/**/X.nix` exist -- the same
   module.nix+test.nix-pair shape every K1-K5 fixture already used, so
   OBA's manifest format needs no new shape invented for E1. 636 such
   names.
3. Remove every name in the exclusion list above -- 589 remain.
4. Deterministic seeded shuffle: `random.Random(seed).shuffle(...)`,
   Python's stdlib `Random` (Mersenne Twister), `seed = int("c8e42a1", 16)`
   -- the E1 freeze commit's own short SHA read as a base-16 integer.
   Chosen for being reproducible and tied to the frozen state itself,
   not picked after seeing any output (this document records the exact
   command before the first candidate below was ever opened).
5. Investigate in that exact shuffled order, taking candidates as they
   come. A candidate is skipped only for a documented, structural,
   pre-scan reason (see "Legitimate skips" below), never for looking
   "boring" or "too easy" or "too hard" -- and every skip is logged with
   its reason, not silently passed over.

**First 40 in draw order** (the working batch; more drawn from the same
list if fewer than ~25-30 end up analyzable after legitimate skips):

```
 1 libinput          11 kavita              21 nimdow             31 akkoma
 2 cadvisor          12 nebula-lighthouse-   22 flame              32 vault
 3 send                 service            23 peerflix            33 spacecookie
 4 privoxy           13 nmtrust             24 coturn              34 unpackerr
 5 esphome           14 transmission        25 convos              35 omada
 6 xandikos          15 matterjs-server     26 tor                 36 retroarch
 7 nohang            16 i2pd                27 bees                37 unbound
 8 svnserve          17 ferretdb            28 kthxbye              38 mobilizon
 9 jitsi-meet        18 ringboard           29 authelia            39 pomerium
10 misskey           19 monado              30 pocket-id           40 pufferpanel
```

## Legitimate skips (structural, decided before inspection -- not
post-hoc)

- The `test.nix` file exists but is not actually a NixOS VM test for
  THIS service (a shared/umbrella test file, a test for an unrelated
  option) -- confirmed by opening it, not assumed from the name.
- The `module.nix` is a thin alias/re-export with no `options.` block of
  its own (nothing for `oba check`'s declaration scanner to find at
  all) -- `not_applicable`, not a failure of the tool.
- A hardware/kernel-only module with no branchable config-value
  predicate structure at all (e.g. a udev/kernel-module-only service) --
  `not_applicable`, recorded with the reason, still counted in the
  denominator for the `not_applicable` rate.

None of these are grounds to swap the candidate out of the corpus and
draw another in its place -- the corpus is the drawn list; a legitimate
skip is itself a recorded outcome, not a mulligan.

## What gets run, per candidate

- **OBA (`oba check`)**: the one genuinely generic piece of this
  project -- no per-app code, just a target manifest (`module`, `test`,
  `cfg_ident`, `option_prefix`, `watch`). For each candidate: read the
  real module to find candidate options with real branching
  (`mkIf`/`optional`/conditional derivation, the same shapes H1/H2
  already model), read the real test to see if it assigns anything
  relevant, build a manifest entry, run the real, frozen binary against
  it, record the real verdict.
- **CDC (K1-K5 abstractions)**: has NO generic entry point at all, by
  design -- every K1-K5 "adapter" (`acquire_kimai_evidence`,
  `acquire_illuminate_consumer_contract`, `evaluate_mimir_cli_drift`,
  `evaluate_grafana_env_drift`, ...) is a hand-written Rust function
  for one specific real app or family. Since writing a new adapter is
  exactly the production-code change E1 forbids, CDC's contribution to
  a holdout candidate is assessed STRUCTURALLY, by reading the
  candidate's real consumer-side source/config mechanism and judging
  (by inspection, not by running new code) whether it would map onto an
  EXISTING abstraction shape (`ProducerEvidence::{SentinelFlow,
  EvaluatedLiteral,FlatEnvVars}`, `ConsumerRoute`/`IlluminateDriver`,
  `CliContract`, `EnvContract`) or would need a genuinely new one. This
  is the audit's answer to "adapter reuse" / "special-case pressure"
  below -- recorded as a judgment call, explicitly labeled as such, not
  disguised as a measured result.

## Recording schema, per candidate

```
name, language/build-system, interface mechanism observed
  (generated-config-file / env-vars / CLI-argv / URI-DSN / mixed / n/a),
OBA outcome: supported | inconclusive | not_applicable | finding,
  + the real verdict kind(s) if supported,
CDC structural fit: existing-abstraction (which one) | new-shape-needed
  | not_applicable,
inconclusive reason (if any) -- recorded, NOT fixed during E1,
finding manually verified: yes/no + how,
notes
```

## Metrics (computed once the batch is closed, not tracked live)

- **coverage** -- of applicable boundaries, how many were actually
  analyzable (OBA `supported`/`finding`, not `inconclusive`/`n/a`).
- **precision** -- of any `finding`s, how many survive manual
  verification against the real module/test source.
- **inconclusive rate** -- how often the system honestly declined,
  and why (bucketed by `ResolveFailure`/gate reason).
- **adapter reuse** -- how many candidates' real consumer contract
  would fit an EXISTING CDC abstraction unchanged (structural judgment,
  see above).
- **special-case pressure** -- how many would need a genuinely new,
  per-app `if app == ...`-shaped adapter to go beyond `not_applicable`.

## What E1 is explicitly not

Not a new adapter round. Not fixing anything found `inconclusive`
mid-census. Not swapping out an unflattering candidate. Not stopping
early because the numbers already look good or bad -- the batch above
is investigated in full (modulo legitimate, pre-defined skips) before
any conclusion is drawn.
