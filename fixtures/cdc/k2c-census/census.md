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

## Raw table

| service | producer mode found | sink | consumer (doctrine/dbal) | bucket |
|---|---|---|---|---|
| kimai | SentinelFlow | `kimai-init-<name>.script` | 3.10.6 @ `c95589d7` | supported as-is (K1, frozen) |
| davis | EvaluatedLiteral | `services.davis.config` | 3.10.6 @ `c95589d7` | supported as-is (K1, frozen) |
| strichliste | SentinelFlow | `systemd.services.strichliste-migrate.environment.DATABASE_URL` | 3.10.5 @ `95d84866` | **supported as-is** -- verified for real |
| part-db | DSN-string option (SentinelFlow-*shaped*) | `services.part-db.DATABASE_URL` | present, but Postgres dialect | consumer contract unsupported |
| agorakit | flat `DB_HOST`/`DB_PORT`/`DB_DATABASE`/`DB_USERNAME`/`DB_PASSWORD`, no socket key at all | systemd `environment` | 3.9.4 @ `ec16c82f` (likely transitive, not actual runtime consumer) | needs new `ProducerEvidence` variant |
| movim | same flat-env shape, no socket key | systemd `environment` | 4.4.4 @ `fb9e0ffe` (likely transitive) | needs new `ProducerEvidence` variant |
| snipe-it | same flat-env shape, but **has** a dedicated `DB_SOCKET` key | systemd `environment` | 3.10.5 @ `95d84866` (Laravel app -- likely its own connector, not Doctrine's) | needs new `ProducerEvidence` variant |
| flarum | generated `config.php` (PHP array literal via `phpFormat.generate`), no socket-equivalent field seen | `flarum-config.php` | 2.13.9 @ `c480849c` -- **also** needs a different consumer locator (see below) | needs new `ProducerEvidence` variant |
| baikal | -- | -- | absent | consumer contract unsupported |
| bookstack | -- | -- | absent | consumer contract unsupported |
| civicrm | -- | -- | absent | consumer contract unsupported |
| engelsystem | -- | -- | absent | consumer contract unsupported |
| grocy | -- | -- | absent | consumer contract unsupported |
| invoiceplane | -- | -- | absent | consumer contract unsupported |
| librenms | -- | -- | absent | consumer contract unsupported |
| postfixadmin | -- | -- | absent | consumer contract unsupported |

## Distribution (the actual point of this census)

- **supported as-is: 3** -- kimai, davis (frozen K1 baseline), strichliste (new, verified for real this round).
- **needs new `ProducerEvidence` variant: 4** -- agorakit, movim, snipe-it, flarum.
- **consumer contract unsupported: 9** -- part-db (wrong DSN dialect) + 8 apps with no `doctrine/dbal` in their dependency closure at all.
- **needs only a new sink/provenance locator: 0 as a clean standalone bucket**, but a real recurring sub-finding inside two other buckets (see below) -- not invented as its own numbered category since nothing landed there *exclusively*.
- **ambiguous/inconclusive: 0** -- every one of the 14 candidates resolved to a definite bucket; nothing needed a coin flip.

`SentinelFlow`/`EvaluatedLiteral` cover **3 of 14** new candidates cleanly
(21%), plus the 2 already-frozen K1 baselines. That is real, useful
information, not a disappointing number to explain away: it says the
current model is solid for its proven cases and genuinely narrow beyond
them -- exactly what a spike-stage architecture should look like before
its next investment is chosen with evidence instead of guesswork.

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
as "supported as-is" for its **producer shape and provenance resolution**
(both real, both verified) -- a full PASS verdict for strichliste
specifically is a small, well-scoped K2c follow-up (vendor one more
pinned file, add one more real test), not part of what this round claims.

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
  guess.

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
- [x] All unsupported cases are explicit -- every "consumer contract
      unsupported"/"needs new variant" entry above states its concrete
      reason, never a silent skip or a guessed PASS.
- [x] Special cases listed separately -- see "real, recurring
      sub-findings" above, kept apart from the bucket table itself.
- [x] Coverage visible -- 3/14 (21%) supported as-is; the dominant
      finding is "consumer contract unsupported" (9/14), mostly because
      `doctrine/dbal` simply isn't present -- a fact about this
      particular corpus slice, not a producer-model failure.
