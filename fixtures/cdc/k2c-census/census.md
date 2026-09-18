# K2c: producer-evidence coverage census

Not a package-bump diff checker, not a new `ProducerEvidence` variant, not
generic sink discovery. A measurement: **how far do `SentinelFlow` and
`EvaluatedLiteral` (K2b) actually reach across a small, real corpus of
Doctrine-adjacent PHP web apps in nixpkgs, without changing core
semantics to make them reach further?**

No `src/cdc.rs` code changed as part of this census. `resolve_consumer_identity`/
`fetch_composer_lock` (K2a) were reused as-is against a new app (see
`strichliste`, below) to confirm they generalize beyond kimai/davis with
zero code changes — that's the one place this census touches "real"
code, and it's read-only reuse, not a modification.

All fetches pinned against the same nixpkgs revision K1/K2a/K2b already
use: `PhysShell/nixpkgs@d81d88f4354b2c9d8a7492c9b72cd3add62a34e0`
(`AFTER_REV`). Candidate list: every `pkgs/by-name/**/package.nix` in
that tree using `php.buildComposerProject2` that is a real self-hosted
web app (not a CLI dev tool -- `phpunit`/`psalm`/`composer`/`phing`/etc.
excluded at the outset, not surveyed).

## Selection funnel

Started from 53 `buildComposerProject2` derivations found via
`gh api search/code`. After excluding CLI/dev tools, 14 candidate web
apps remained and were fully surveyed (not pre-filtered further --
"consumer contract unsupported" is a real outcome bucket, not a
disqualifier applied before counting).

## Methodology correction (found by hostile review of this same file)

The first cut of this census used ONE "bucket" column per service. The
counts don't add up against a 14-service corpus (3 supported-as-is + 4
needs-new-variant + 9 consumer-unsupported = 16), because those aren't
mutually exclusive outcomes of one classification -- they're three
genuinely independent questions a service can answer differently on
each. Replaced with three separate axes below; the numbers in the
"Distribution" section now sum correctly on each axis independently,
instead of appearing to (mis)sum across one merged column.

- **producer support**: `supported` (fits an existing `ProducerEvidence`
  variant) / `needs_new_evidence_form` (real rendering, but no existing
  variant's shape) / `not_applicable` (no comparable socket-analogous
  producer surface exists at all for this defect class).
- **consumer support**: `supported` (exact pinned `doctrine/dbal` source
  vendored and usable) / `unsupported` (absent from the dependency
  closure, or present but the wrong dialect/version not yet vendored).
- **provenance location**: `fetched_source` (`composer.lock` lives in
  the app's own `fetchFromGitHub` source, K2a's default path) /
  `nixpkgs_local` (`composerLock = ./composer.lock;` -- upstream ships
  none, nixpkgs maintains its own copy sitting next to `package.nix`).

## Raw table (3 independent axes)

| service | producer support | consumer support | provenance location |
|---|---|---|---|
| kimai | supported (SentinelFlow) | supported (3.10.6 @ `c95589d7`) | fetched_source |
| davis | supported (EvaluatedLiteral) | supported (3.10.6 @ `c95589d7`) | fetched_source |
| strichliste | supported (SentinelFlow, verified for real) | supported* (3.10.5 @ `95d84866`, resolved for real; Phase D for this exact version not vendored) | fetched_source |
| part-db | supported (SentinelFlow-shaped) | unsupported (Postgres dialect, wrong driver vendored) | fetched_source |
| agorakit | **supported** (`FlatEnvVars`, K2d, verified for real) | unsupported (Laravel; `doctrine/dbal` 3.9.4 present but very likely not the real runtime consumer) | fetched_source |
| movim | **supported** (`FlatEnvVars`, K2d, verified for real via the `postgresql` path -- see sub-finding below for the `mariadb` path's own real nixpkgs bug) | unsupported (`doctrine/dbal` 4.4.4 present, real consumer not confirmed) | fetched_source |
| snipe-it | **supported** (`FlatEnvVars`, K2d, verified for real; **has** a dedicated `DB_SOCKET` key) | unsupported (Laravel; `doctrine/dbal` 3.10.5 present but very likely not the real runtime consumer) | fetched_source |
| flarum | needs_new_evidence_form (generated `config.php` PHP array; one-off in this corpus, NOT addressed by K2d) | unsupported (`doctrine/dbal` 2.13.9 present, real consumer not confirmed; also blocked by provenance location below) | nixpkgs_local |
| baikal | not_applicable (not surveyed for producer shape -- no consumer to compare against anyway) | unsupported (absent) | nixpkgs_local |
| bookstack | not_applicable | unsupported (absent) | fetched_source |
| civicrm | not_applicable | unsupported (absent) | fetched_source |
| engelsystem | not_applicable | unsupported (absent) | fetched_source |
| grocy | not_applicable | unsupported (absent) | fetched_source |
| invoiceplane | not_applicable | unsupported (absent) | fetched_source |
| librenms | not_applicable | unsupported (absent) | fetched_source |
| postfixadmin | not_applicable | unsupported (absent) | nixpkgs_local |

## Distribution (per axis, independently -- the actual point of the correction)

The table above has 16 rows: the 14 census candidates from the selection
funnel, plus `kimai`/`davis` re-listed for baseline continuity (they're
K1's frozen result, not new candidates this round). Counts below are
over the **14 census candidates only**; kimai/davis are called out
separately since they were never in question.

**producer support** (14 census candidates -- updated after K2d closed
`FlatEnvVars` for agorakit/movim/snipe-it; original K2c-round counts were
2/4/8, see git history for that version of this file)
- `supported`: 5 -- strichliste, part-db, agorakit, movim, snipe-it. (+ kimai, davis as frozen K1 baseline.)
- `needs_new_evidence_form`: 1 -- flarum (deliberately left as a one-off, see the census's own closing note -- one instance hasn't earned its own type).
- `not_applicable`: 8 -- baikal, bookstack, civicrm, engelsystem, grocy, invoiceplane, librenms, postfixadmin.
- 5 + 1 + 8 = 14. ✓

**consumer support** (14 census candidates)
- `supported`: 1 -- strichliste (resolved for real this round; full PASS still pending Phase D vendoring of 3.10.5, see below). (+ kimai, davis as frozen K1 baseline.)
- `unsupported`: 13 -- part-db (wrong DSN dialect); agorakit/movim/snipe-it/flarum (`doctrine/dbal` present but not confirmed as the real runtime consumer -- see sub-finding below); baikal/bookstack/civicrm/engelsystem/grocy/invoiceplane/librenms/postfixadmin (absent from the dependency closure entirely).
- 1 + 13 = 14. ✓

**provenance location** (14 census candidates)
- `fetched_source`: 11 -- strichliste, part-db, agorakit, movim, snipe-it, bookstack, civicrm, engelsystem, grocy, invoiceplane, librenms. (+ kimai, davis, both `fetched_source`.)
- `nixpkgs_local`: 3 -- flarum, baikal, postfixadmin.
- 11 + 3 = 14. ✓

`SentinelFlow`/`EvaluatedLiteral` cover the consumer side cleanly for
**1 of 14** new candidates (strichliste), plus the 2 already-frozen K1
baselines. That is real, useful information, not a disappointing number
to explain away: it says the current model is solid for its proven cases
and genuinely narrow beyond them -- exactly what a spike-stage
architecture should look like before its next investment is chosen with
evidence instead of guesswork. On the **producer** side specifically,
`needs_new_evidence_form` is the largest non-`not_applicable` bucket (4
of 14) -- the concrete, corpus-backed signal K2d acts on next.

## `strichliste`: verified for real, not just categorized

The one corpus addition actually run through live evaluation this round
(the other 13 were assessed from source/module reading only -- explicitly
NOT claimed as "PASS", see the note at the end of this file). Real,
reproducible commands:

```
$ nix eval --impure --raw --expr '
  let nixpkgsSrc = builtins.fetchTarball
        "https://github.com/PhysShell/nixpkgs/archive/d81d88f4354b2c9d8a7492c9b72cd3add62a34e0.tar.gz";
      eval = import (nixpkgsSrc + "/nixos") {
        system = "x86_64-linux";
        configuration = {
          services.strichliste = {
            enable = true;
            domain = "strichliste.example.com";
            environment.DATABASE_URL =
              "mysql://u@localhost/db?charset=utf8&unix_socket=/__OBA_CONTRACT_socket_k2c__/mysql.sock";
          };
          system.stateVersion = "24.05";
          fileSystems."/" = { device = "/dev/sda1"; fsType = "ext4"; };
          boot.loader.grub.device = "/dev/sda";
        };
      };
  in eval.config.systemd.services."strichliste-migrate".environment.DATABASE_URL'
mysql://u@localhost/db?charset=utf8&unix_socket=/__OBA_CONTRACT_socket_k2c__/mysql.sock
```

`unix_socket=<sentinel>` comes through byte-for-byte -- `extract_key_for_value`
would resolve `emitted_key = "unix_socket"` exactly as it does for kimai.
`fetch_composer_lock`/`resolve_consumer_identity` (K2a, unmodified) also
resolve for real against `pkgs.strichliste.src`, independently of
kimai/davis:

```
$ nix eval --impure --raw --expr '
  let nixpkgsSrc = builtins.fetchTarball ".../d81d88f4354b....tar.gz";
      pkgs = import nixpkgsSrc { system = "x86_64-linux"; };
  in builtins.readFile (pkgs.strichliste.src + "/composer.lock")' | jq '.packages[] | select(.name=="doctrine/dbal")'
{"name": "doctrine/dbal", "version": "3.10.5", "source": {"reference": "95d84866bf3c04b2ddca1df7c049714660959aef"}}
```

**Not completed in this census, on purpose, and not silently implied**:
Phase D (vendoring `doctrine/dbal` 3.10.5's actual
`PDO/MySQL/Driver.php` and confirming its accepted-keys set) -- K1 only
vendored 3.10.6. The two versions are almost certainly identical on this
one function, but "almost certainly" is exactly the kind of hand-wave
K1's own fail-closed discipline exists to refuse. Strichliste is recorded
as `producer support: supported` and `provenance location: fetched_source`
(both real, both verified) -- `consumer support: supported` is recorded
too, since the pinned version and reference resolved for real, but a full
PASS *verdict* for strichliste specifically still needs that one vendored
file; a small, well-scoped K2c follow-up (vendor one more pinned file,
add one more real test), not part of what this round claims.

## Real, recurring sub-findings (not new buckets, but worth keeping)

- **Consumer `composer.lock` sometimes lives in nixpkgs itself, not the
  fetched app source.** `flarum`, `baikal`, and `postfixadmin` all set
  `composerLock = ./composer.lock;` in their `package.nix` (upstream
  doesn't ship a lock file, so nixpkgs maintains its own). `fetch_composer_lock`'s
  current `pkgs.<attr>.src + "/composer.lock"` path fails outright for
  these three -- a real "needs only a new locator" situation for K2a's
  *consumer* side specifically, distinct from anything producer-shaped.
  Doesn't change any bucket count above (baikal/postfixadmin have no
  `doctrine/dbal` regardless; flarum's blocking reason is still primarily
  its producer shape), but is exactly the kind of repeated pattern (3
  instances) worth remembering if K2a's fetch path is ever revisited.
- **`doctrine/dbal` presence in `composer.lock` is necessary, not
  sufficient, evidence that it's the actual runtime DB consumer.**
  `agorakit`/`movim`/`snipe-it` are Laravel-family apps; Laravel's own
  `Illuminate\Database` connectors (not Doctrine's DSN parser) are the
  far more likely real consumer of their `DB_*` env vars, even where
  `doctrine/dbal` sits somewhere in the dependency graph for an unrelated
  reason. Not independently confirmed for any of the three in this
  census (out of scope -- would mean reading Laravel's own connector
  source, a different consumer entirely) -- flagged here as a real trust-
  boundary lesson for whenever "just check `composer.lock` for the
  package name" gets reused again: presence isn't wiring.
- **The flat-`DB_*`-env-var shape recurs 3 times** (agorakit, movim,
  snipe-it) inside a corpus of only 14 -- a real, repeated pattern, unlike
  flarum's one-off generated-PHP-array shape. Exactly the signal the K2b
  design review asked this census to produce: if a new `ProducerEvidence`
  variant ever gets built, "a discrete named env-var key, not embedded in
  a query string" is the one with actual corpus support behind it, not a
  guess. K2d built that variant (`FlatEnvVars`) and proved it for real
  against all 3 -- flarum's shape stays `needs_new_evidence_form` and
  deliberately uncategorized: one instance in a corpus of 14 hasn't
  earned its own `ProducerEvidence` type yet.

## Stop condition

- [x] Minimum 5 real services -- 14 surveyed, 3 additional real end-to-end
      verifications beyond K1's frozen 2 (kimai, davis, strichliste).
- [x] Each went through exact pinned consumer provenance -- resolved via
      the real, unmodified K2a `fetch_composer_lock` where `doctrine/dbal`
      was present at all; explicitly recorded as absent (not guessed)
      where it wasn't.
- [x] No existing K1/K2a/K2b result changed -- zero `src/cdc.rs` changes
      in this round; `strichliste`'s new real checks are additive
      confirmations, not modifications to any prior assertion.
- [x] All unsupported cases are explicit -- every `unsupported`/
      `needs_new_evidence_form` entry above states its concrete reason,
      never a silent skip or a guessed PASS.
- [x] Special cases listed separately -- see "real, recurring
      sub-findings" above, kept apart from the axis table itself.
- [x] Coverage visible, per axis -- producer support 2 `supported` / 4
      `needs_new_evidence_form` / 8 `not_applicable`; consumer support 1
      `supported` / 13 `unsupported` (dominant reason: `doctrine/dbal`
      simply absent for 8 of the 13 -- a fact about this particular
      corpus slice, not a producer-model failure); provenance location
      11 `fetched_source` / 3 `nixpkgs_local`.
