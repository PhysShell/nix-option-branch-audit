# S1: live nixpkgs PR shadow evaluation -- protocol

Pre-registered BEFORE any real PR is fetched for adjudication or any
finding is inspected, per this project's own standing discipline.
Committed as its own commit, separate from the population draw and
separate from any results.

## What this round is, and is not

Every prior round (K1-K5, E1/E2, C-E1.1/C-E1.2a/b/c) was evaluated
against fixtures this project built, curated, or mechanically drew
from a FROZEN snapshot of nixpkgs. That answers "does the model fit
the shapes we know about" -- a necessary question, but one where the
tool has started grading its own homework. **This round asks a
different question, against an exam this project did not write**:

> If you run v0.4.0 on a real, live stream of nixpkgs PRs, how often is
> it applicable, how useful are its findings, and how much manual work
> is required to understand the result?

This is explicitly NOT a coverage-maximization exercise, NOT another
chance to add CDC candidates, and NOT a search for the highest possible
precision number. It is a measurement of the CURRENT, FROZEN,
already-released product's real-world cost and value, taken once,
honestly, before any further engineering.

## The freeze

- **Frozen at `v0.4.0`, commit `67bb2e1` (tag object `6177839`)** -- the
  exact same release `audit-diff/action.yml`'s own dogfood already
  installs and verified end-to-end. No exception.
- **Zero `src/` changes for the entire round**, regardless of what a
  real PR's own shadow run surfaces. This includes the CDC registry:
  C-E1.2c's own fresh holdout already named concrete, disclosed gaps
  (a 3-hop symlink binding shape, shell-quoted argv, at least one new
  normalization shape) that are known NOT to be handled by v0.4.0 --
  S1 deliberately does NOT fix any of them mid-round. If a real PR in
  the sample happens to hit one of these gaps, the correct, honest
  outcome is a real `INCONCLUSIVE` (or CDC simply not applying), and
  that IS part of what this round is measuring: the real cost of
  today's frozen product, not a constantly-moving target that would
  make every subsequent PR "somehow" pass.
- **Shadow means shadow.** No GitHub Action is installed on, and no
  comment or check run is ever posted to, any real external PR or
  repository. The entire pipeline is a local, own-CI worker only:
  fetch PR base/head -> `oba audit-diff` -> store the report -> manual
  adjudication, offline, in this repository's own `fixtures/`
  directory. Nothing in this round is visible to nixpkgs maintainers.

## Population

Real, live `NixOS/nixpkgs` pull requests (merged and open), drawn
mechanically, not cherry-picked:

1. Real commits touching `nixos/modules/services/**` or
   `nixos/tests/**` were fetched directly from GitHub's own
   path-filtered commits API (`GET /repos/NixOS/nixpkgs/commits?path=...`,
   no local clone needed -- this VPS has tight main-disk headroom, see
   `[[vps-single-core-and-disk]]`), 3 pages of 100 each per path (600
   raw commits, spanning 2026-08-21 through 2026-09-19).
2. PR numbers were extracted from each commit's own message via
   nixpkgs' own `(#NNNNN)` merge-commit convention, deduplicated: 196
   distinct candidate PR numbers.
3. Real per-PR detail (`title`, `state`, `mergedAt`, `baseRefOid`,
   `headRefOid`, `files`, `changedFiles`) fetched for every one of the
   196 via `gh pr view --repo NixOS/nixpkgs`.

## Eligibility

A candidate is **ELIGIBLE** iff at least one of its real changed file
paths matches `^nixos/modules/services/` or `^nixos/tests/` --
re-verified directly from the real per-PR file list (step 3 above),
not merely assumed from the commit-message-derived population in step
1-2.

This is the operative reading of the original three-part eligibility
sketch ("touches `nixos/modules/services/**` and/or `nixos/tests/**`
and/or package source/version corresponding to such a service"): an
OBA target needs a real `module.nix`+`test.nix` pair under exactly
these two trees to analyze at all, and CDC only ever matches one of
its 11 fixed registry candidate names (all 11 of which are excluded
below anyway, being already-used). A package-only PR that touches
neither tree has, structurally, nothing for either engine to point at
-- so the third clause is folded into the first two rather than
separately approximated (e.g. by name-matching a package attribute
against some module's own internals). This is a real methodological
simplification, disclosed here, not silently smoothed over.

## Exclusion

Applied, in order, to every ELIGIBLE candidate:

1. **Real merge/open sanity.** Drop any candidate whose real `mergedAt`
   (or, for an open PR, creation activity) falls outside the observed
   2026-08-21..2026-09-19 window this population was actually drawn
   from -- catches the ~19 candidates (of 196) whose PR number was far
   below the live ~564000s range, i.e. a backport commit's message
   citing its own original, much older PR number rather than a
   genuinely recent one.
2. **Purely docs/formatting-only.** Dropped if every matched path is a
   `.md`/comment-only change, or the title matches an obvious
   formatting-bot pattern (`nixfmt`, `treewide: format`, `typo`).
3. **Mass mechanical changes.** Dropped if `changedFiles` is large
   enough to signal a repo-wide mechanical edit rather than a targeted
   service change (more than 15 files), or the title matches a known
   mechanical-migration pattern (`treewide:`, `by-name migration`,
   `maintainers:`).
4. **Already-used app/service.** Dropped if any changed path component,
   or the PR's own title, names one of the 113 real package/service
   names this project has already used ANYWHERE in K1-K5, E1/E2,
   C-E1.1, C-E1.2a/b/c (including the fresh holdout), or any P3a/b/c
   dogfood fixture -- compiled directly from this repository's own
   `targets/*.toml`, `fixtures/**`, `src/cdc.rs`'s
   `CDC_CANDIDATE_NAMES`, and every census/holdout report, not from
   memory:

   `agorakit, akkoma, anubis, artalk, atticd, authelia, autobrr,
   baikal, bees, blocky, bookorbit, bookstack, borgbackup, cadvisor,
   civicrm, clatd, cocoon, convos, coturn, davis, dgraph, docuseal,
   endlessh, engelsystem, esphome, ferretdb, fider, firefox-syncserver,
   flame, flarum, gancio, geph, glance, glitchtip, gocron, grafana,
   grocy, hatsu, headscale, i2pd, ifstate, invoiceplane, jitsi-meet,
   karma, kavita, kimai, krill, kthxbye, lemmy, libinput, librechat,
   librenms, linkwarden, listmonk, lldap, loki, matterjs-server,
   meilisearch, mimir, miniflux, misskey, mobilizon, monado, movim,
   nats, nebula-lighthouse-service, nextcloud-notify_push, nimdow,
   nmtrust, nohang, nomad, omada, outline, papra, part-db, peerflix,
   plausible, plikd, pocket-id, pomerium, postfixadmin, privoxy,
   pufferpanel, realm, rebuilderd, redmine, restic, retroarch,
   ringboard, searxng, send, shiori, snipe-it, soft-serve, spacecookie,
   strichliste, svnserve, tap, tempo, tor, transmission, turn-rs,
   umami, unbound, unpackerr, vault, vaultwarden, vector, warpgate,
   windmill, xandikos, zerobyte, zipline`

   (`libinput`/`nohang` are C-E1.1's own real negative controls, not
   CDC candidates -- excluded for the same reason: already-used, not
   fresh.)

## Sample draw

From the eligible, non-excluded pool: a deterministic seeded shuffle,
`random.Random(seed).shuffle(...)` (Python's stdlib Mersenne Twister),
`seed = int("67bb2e1", 16)` -- the exact same mechanism E1's own
holdout draw already used (`seed=int(freeze SHA,16)`, the short-form
abbreviated SHA), applied here to keep the draw reproducible and
auditable rather than "whichever 30 looked interesting." The first **30** of the shuffled pool are the S1 sample,
frozen before any of them is individually inspected -- committed as
its own separate commit (`fixtures/s1-live-pr-shadow/sample.md`), with
the full funnel disclosed (population size -> eligible count ->
excluded count by reason -> drawn 30), matching this project's own
holdout-draw precedent exactly.

If the eligible pool after exclusion is smaller than 30, the real
count is used and disclosed as-is -- never padded back up by loosening
a criterion after the fact.

## Recording schema (per sampled PR)

```
pr: <number>
title: <string>
state: merged | open
base_sha: <baseRefOid>
head_sha: <headRefOid>
applicable_engines:
  oba: yes | no   (a real [[target]] manifest entry could be built --
                    a module.nix declaring the option(s) touched, PLUS
                    a corresponding NixOS test exercising them)
  cdc: yes | no   (in practice expected to be near-always "no" by
                    construction -- every CDC-registry-supported name
                    is itself in the exclusion list above; disclosed as
                    a known, structural limitation of this specific
                    round, not a surprise to explain away later)
runtime_seconds: <real wall-clock time of the oba audit-diff run>
result:
  new_findings: <int>
  resolved_findings: <int>
  new_inconclusives: <int>
  evidence_only_changes: <int>
  persistent_findings: <int>   (present in BOTH base and head -- recorded
                                 for completeness, explicitly OUT of the
                                 headline/precision metrics below)
notable:
  - code: OBA001 | CDC001 | CDC002
    subject: <option path or CDC candidate name>
    manually_valid: yes | no
    actionable_in_this_pr: yes | no
    already_known_or_discussed_in_pr: yes | no
    maintainer_would_reasonably_care: yes | no
    manual_verification_time: "<2min" | "2-10min" | ">10min"
    notes: <free text, the real reasoning, not just the checkbox>
```

Every field is filled from a REAL run against REAL fetched PR trees
(the exact two files/paths named in that PR's own manifest, at
`base_sha` and `head_sha`, via GitHub's raw content -- no full nixpkgs
clone, matching this same disk-conservation reasoning used for the
population draw itself) and a REAL reading of the PR's own diff/
discussion on GitHub, never guessed or extrapolated from a similar
past candidate.

## Metrics

```
correctness precision = correct findings / inspected findings
actionable precision  = useful-in-this-PR findings / inspected findings
```

"Inspected findings" means every entry in `notable` across the whole
sample (bounded per-PR by `NOTABLE_LIMIT`=10, per P3c's own real CLI
behavior -- if a single PR ever produces more than 10 notable entries,
that is itself worth flagging in the writeup, not silently truncated
out of the metric).

**Headline priority, explicitly**: `new_findings`/`resolved_findings`/
`new_inconclusives` -- the transitions `audit-diff` itself attributes
to THIS PR -- are what the round's own headline is built around.
`persistent_findings` (present in base AND head, i.e. pre-existing and
untouched by the PR) are recorded but explicitly EXCLUDED from both
precision denominators: a finding that was already true before the PR
and stays true after it is not something a PR author or reviewer needs
from THIS tool on THIS PR -- grading it as if it were PR-local signal
would overstate actionability. This is the direct product hypothesis
P3b's own diff machinery exists to test.

## Decision rule (pre-registered BEFORE any result is seen)

No target precision percentage is pre-committed -- 30 PRs is too small
a sample for a number like "precision >= 95%" to mean anything stable;
a few point-swings either way are expected noise, not a verdict.
Instead:

- **Any real dangerous false PASS** (a real config-contract mismatch or
  a real uncovered branch that `audit-diff` reported as clean/resolved)
  -> STOP, investigate immediately, do not continue drawing conclusions
  from the rest of the sample until understood.
- **Several repeating false FINDINGs of the same class** -> a real bug,
  fix before any advisory beta, named as a concrete follow-up.
- **Mostly-correct-but-irrelevant findings** (technically true, useless
  in context -- e.g. "this branch is untested" on a PR that only
  touched the option's own description) -> a scoping/product-layer
  problem, not a correctness bug; noted as its own category, not
  conflated with a false finding.
- **Many INCONCLUSIVE results of one specific type** -> a candidate
  for a future generic adapter (not built during S1 -- named as debt
  only).
- **Diffuse INCONCLUSIVE** (many different causes, no single dominant
  one) -> stay fail-closed, live with it, exactly E2's own precedent
  for OBA's residual inconclusive census.

**Ship an advisory beta if**: false-positive rate stays very low, no
dangerous false PASS survives investigation in the audited sample,
findings are regularly PR-relevant (actionable precision meaningfully
tracks correctness precision, not far below it), inconclusive output
stays understandable rather than cryptic, median manual verification
time is small, and real runtime is acceptable inside a PR workflow.

If the results are bad, that is also a complete, useful, real answer:
it would mean this is currently a good research verifier but a poor
review assistant -- cheaper to learn now than after building four more
CDC integrations nobody asked for.

## What does NOT happen during S1

- No GitHub Action, bot, or comment on any real external PR or
  repository -- shadow only.
- No new CDC registry candidates, even for the already-named,
  already-disclosed gaps from C-E1.2c's own fresh holdout.
- No `src/` changes of any kind against the frozen `v0.4.0`/`67bb2e1`.
- No loosening the sample size or the exclusion criteria after seeing
  how few (or many) PRs survive the funnel.
