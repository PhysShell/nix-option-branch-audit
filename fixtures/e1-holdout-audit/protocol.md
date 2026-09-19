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

## Amendment (after batch 1+2 reported, before batch 3+4 were read) --
user-requested safeguards against E1 quietly becoming a self-assessment

Batches 1 and 2 (candidates 1-20) had already reported back when this
amendment was written; batches 3 and 4 had not. Applied retroactively
to backfill 1+2's classifications against the rubric below during
final aggregation (the raw evidence they collected doesn't change, only
the classification scheme applied to it) -- not a re-investigation, and
not something batch 3/4's already-running agents were interrupted or
re-briefed for.

1. **Exact nixpkgs revision, already pinned, restated for emphasis**:
   the population was built from the single pinned tree
   `68740713a1d5904edf9ba92a998a522b1b6ce080` (see "Population and draw
   method" above), fetched once before the draw and never re-fetched --
   not "current master" drifting under repeated reads. Every candidate
   in this audit was fetched `?ref=68740713a1d5904edf9ba92a998a522b1b6ce080`
   explicitly, never a bare `master`/`HEAD` ref.

2. **E1-OBA and E1-CDC are two independent measurements, never blended
   into one "coverage" number.** E1-OBA is an objective, executed
   result (the real frozen binary's real verdict). E1-CDC is an expert
   structural classification against a fixed rubric (below) -- an
   adapter-fit HYPOTHESIS, not executable support, and must never be
   reported alongside E1-OBA's numbers as if they were the same kind of
   evidence.

3. **CDC fit rubric, frozen before aggregation, six categories --
   `requires new reusable adapter` does NOT count as supported/covered
   in any metric**:
   - `existing_abstraction_exact` -- the candidate's real
     producer/consumer shape matches an existing `ProducerEvidence`/
     `ConsumerRoute`/`CliContract`/`EnvContract` variant with zero new
     Rust needed beyond a new per-app call site (the same kind of
     "wire it up" work K2f's Illuminate adapter already does per app).
   - `existing_abstraction_new_locator` -- the SHAPE matches an
     existing abstraction, but reaching it needs a new, still-generic
     locator/extractor (e.g. a new bounded-scan pattern like
     `extract_go_flagset_literal_names`, reusable across apps of that
     same family, not this one app specifically).
   - `requires_new_reusable_adapter` -- no existing abstraction's shape
     fits at all; a genuinely new `ProducerEvidence`-style variant or
     equivalent would be needed, but one that could still serve MULTIPLE
     future apps of the same newly-observed shape (e.g. the "generated
     multi-key settings file" pattern batch 1/2 already saw recur).
   - `requires_app_specific_logic` -- even a new reusable abstraction
     wouldn't cleanly cover it; this candidate's own idiosyncrasy would
     need bespoke, non-reusable handling.
   - `not_applicable` -- no producer/consumer boundary this project's
     abstractions could ever target (e.g. ringboard: picks a local
     binary, no external contract at all).
   - `cannot_determine` -- the investigating agent couldn't reach a
     confident classification from the real source alone (recorded
     honestly, not forced into one of the other five).

4. **Unified per-candidate evidence template, structured fields as the
   primary result -- free prose is supporting detail, never the only
   record of a classification**:
   ```
   candidate:
   module_path:
   test_path:
   nixpkgs_sha: 68740713a1d5904edf9ba92a998a522b1b6ce080
   oba_targets: [ { option_prefix, watch } ... ]
   oba_verdict: PASS | FINDING | INCONCLUSIVE | TOOL_ERROR
   oba_inconclusive_reason: unsupported_predicate | unresolved_config
     | manifest_construction_ambiguity | path_layout_issue | other | n/a
   cdc_producer_shape:
   cdc_consumer_shape:
   cdc_closest_existing_abstraction:
   cdc_fit_classification: existing_abstraction_exact |
     existing_abstraction_new_locator | requires_new_reusable_adapter |
     requires_app_specific_logic | not_applicable | cannot_determine
   cdc_evidence: <file:line citations, not a vibe>
   special_case_required: yes | no
   finding_manually_verified: yes | no | n/a
   notes:
   ```
   Batches 1/2's existing per-candidate write-ups already carry every
   fact this schema needs (module/test paths, real verdicts, real
   source citations) -- backfilling them into these exact fields during
   aggregation is a reformatting pass, not new investigation. Batches
   3/4 (still running under their original free-prose brief) get the
   same backfill treatment once they report.

5. **Double-independent-review subset, after all four batches close**:
   a second, independent pass re-classifies 8-10 of the 40 candidates'
   `cdc_fit_classification` from the same real evidence, blind to the
   first pass's answer, specifically to measure inter-rater consistency
   on the rubric in (3) -- not to re-run the whole corpus. If the two
   passes disagree often, specifically between
   `existing_abstraction_exact`/`_new_locator` and
   `requires_new_reusable_adapter`, that means the rubric itself isn't
   operational enough yet, and gets reported as exactly that finding,
   not smoothed over.

6. **Population caveat, stated in the final report, not just here**:
   the population is every real NixOS service module with a MATCHING
   top-level `nixos/tests/X.nix` -- i.e. already test-covered at the
   module level. This is necessary for OBA (no test.nix, no branch
   evidence to check at all) but means the holdout is more
   test-friendly than nixpkgs as a whole. The honest conclusion is
   "generalizes within NixOS service modules that already have a
   matching module-level test," never "generalizes across 40 random
   NixOS services" or "across nixpkgs."

## Recording schema, per candidate

Superseded by the unified evidence template in the amendment above,
which is now the authoritative per-candidate schema. Kept below only as
the original schema batches 1 and 2 were briefed under (why their raw
write-ups look different before backfilling):

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

**E1-OBA (objective, from the real executed binary):**
- **coverage** -- of applicable boundaries (excludes `not_applicable`),
  how many reached `PASS`/`FINDING` rather than `INCONCLUSIVE`.
- **precision** -- of any `FINDING`s, how many survive manual
  verification against the real module/test source.
- **inconclusive rate, broken down by reason** -- `unsupported_predicate`
  / `unresolved_config` / `manifest_construction_ambiguity` /
  `path_layout_issue` / `other`, reported as separate counts, never one
  blended percentage. A 35% inconclusive rate concentrated in one reason
  is a concrete next H-stage; the same 35% spread across a dozen
  distinct reasons is a different diagnosis entirely, and the rate alone
  can't tell those apart.

**E1-CDC (expert structural classification, explicitly not executable
evidence):**
- **special-case pressure** -- the headline CDC metric:
  `(requires_new_reusable_adapter + requires_app_specific_logic) /
  (all candidates except not_applicable and cannot_determine)`. This is
  the direct, measured answer to "how much of this is universal vs. how
  much would need hardcoding per app."
- **adapter reuse** -- `existing_abstraction_exact +
  existing_abstraction_new_locator`, over the same denominator. Reported
  separately from special-case pressure, never netted against it into a
  single "coverage" figure, and never combined with E1-OBA's own
  coverage number.
- **inter-rater consistency** -- from the double-review subset (5
  above), reported as agreement/disagreement counts on the rubric, not
  folded into either metric above.

## What E1 is explicitly not

Not a new adapter round. Not fixing anything found `inconclusive`
mid-census. Not swapping out an unflattering candidate. Not stopping
early because the numbers already look good or bad -- the batch above
is investigated in full (modulo legitimate, pre-defined skips) before
any conclusion is drawn.
