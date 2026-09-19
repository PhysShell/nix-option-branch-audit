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
- **consumer path** (K2e, added below): `direct` (the Nix-emitted value
  flows into Doctrine's own DSN-parsing/driver code with zero
  intermediary) / `mediated-known` (a named library sits between the
  Nix-emitted value and its actual terminal consumer -- the chain is
  fully traced, and the terminal consumer may or may not be Doctrine at
  all) / `mediated-unknown` (an intermediary clearly exists but its
  effect on the value couldn't be pinned down) / `not_applicable`
  (`doctrine/dbal` isn't even present -- this axis's question doesn't
  arise).

## Raw table (4 independent axes)

| service | producer support | consumer support | provenance location | consumer path |
|---|---|---|---|---|
| kimai | supported (SentinelFlow) | supported (3.10.6 @ `c95589d7`) | fetched_source | **direct** |
| davis | supported (EvaluatedLiteral) | supported (3.10.6 @ `c95589d7`) | fetched_source | **direct** |
| strichliste | supported (SentinelFlow, verified for real) | **supported** (3.10.5 @ `95d84866`; Phase D vendored K2g, real `Pass` verdict) | fetched_source | **mediated-known** (Symfony bundle calls Doctrine's own `DsnParser` -- see K2e below) |
| part-db | supported (SentinelFlow-shaped) | **supported** (4.4.3 @ `61e730f1`, Postgres dialect; Phase D vendored K2g, real `Pass` verdict) | fetched_source | **mediated-known** (same bundle mechanism as strichliste) |
| agorakit | supported (`FlatEnvVars`, K2d, verified for real) | unsupported (Laravel; `doctrine/dbal` 3.9.4 present, confirmed NOT the runtime consumer) | fetched_source | **mediated-known** (Illuminate\Database; doctrine/dbal vestigial) |
| movim | supported (`FlatEnvVars`, K2d, verified for real via the `postgresql` path -- see sub-finding below for the `mariadb` path's own real nixpkgs bug) | unsupported (`doctrine/dbal` 4.4.4 present, confirmed NOT the runtime consumer) | fetched_source | **mediated-known** (Illuminate\Database/Eloquent; doctrine/dbal vestigial) |
| snipe-it | supported (`FlatEnvVars`, K2d, verified for real; **has** a dedicated `DB_SOCKET` key) | unsupported (Laravel; `doctrine/dbal` 3.10.5 present, confirmed NOT the runtime consumer) | fetched_source | **mediated-known** (Illuminate\Database; doctrine/dbal vestigial) |
| flarum | needs_new_evidence_form (generated `config.php` PHP array; one-off in this corpus, NOT addressed by K2d) | unsupported (`doctrine/dbal` 2.13.9 present, confirmed used ONLY for migrations, not the runtime connection; also see provenance location) | nixpkgs_local | **mediated-known** (Illuminate\Database/Capsule; doctrine/dbal real but narrow -- migrations only) |
| baikal | not_applicable (not surveyed for producer shape -- no consumer to compare against anyway) | unsupported (absent) | nixpkgs_local | not_applicable |
| bookstack | not_applicable | unsupported (absent) | fetched_source | not_applicable |
| civicrm | not_applicable | unsupported (absent) | fetched_source | not_applicable |
| engelsystem | not_applicable | unsupported (absent) | fetched_source | not_applicable |
| grocy | not_applicable | unsupported (absent) | fetched_source | not_applicable |
| invoiceplane | not_applicable | unsupported (absent) | fetched_source | not_applicable |
| librenms | not_applicable | unsupported (absent) | fetched_source | not_applicable |
| postfixadmin | not_applicable | unsupported (absent) | nixpkgs_local | not_applicable |

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

**consumer support** (14 census candidates -- updated after K2g vendored
Phase D for both strichliste and part-db; original K2c/K2e-round counts
were 1/13, see git history for that version of this file)
- `supported`: 2 -- strichliste, part-db (both Phase D vendored in K2g, real `Pass` verdicts). (+ kimai, davis as frozen K1 baseline.)
- `unsupported`: 12 -- agorakit/movim/snipe-it/flarum (`doctrine/dbal` present, and as of K2e CONFIRMED not the real runtime consumer -- see the K2e section below, no longer just a suspicion); baikal/bookstack/civicrm/engelsystem/grocy/invoiceplane/librenms/postfixadmin (absent from the dependency closure entirely).
- 2 + 12 = 14. ✓

**provenance location** (14 census candidates)
- `fetched_source`: 11 -- strichliste, part-db, agorakit, movim, snipe-it, bookstack, civicrm, engelsystem, grocy, invoiceplane, librenms. (+ kimai, davis, both `fetched_source`.)
- `nixpkgs_local`: 3 -- flarum, baikal, postfixadmin.
- 11 + 3 = 14. ✓

**consumer path** (K2e, 14 census candidates)
- `direct`: 0 among the 14 (+ kimai, davis, both `direct` -- the K1 baseline).
- `mediated-known`: 6 -- strichliste, part-db, agorakit, movim, snipe-it, flarum. Full chains in the K2e section below.
- `mediated-unknown`: 0.
- `not_applicable`: 8 -- baikal, bookstack, civicrm, engelsystem, grocy, invoiceplane, librenms, postfixadmin (`doctrine/dbal` absent, the question doesn't arise).
- 0 + 6 + 0 + 8 = 14. ✓

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

**Closed by K2g (`86c83e4`)**: 3.10.5 confirmed BYTE-IDENTICAL to the
vendored 3.10.6 fixture (diffed directly, not assumed identical) --
`strichliste_golden_is_pass` now gives strichliste a real, full end-to-
end `Pass` verdict, not just a resolved identity. `part-db` closed in
the same round, with a genuinely different real finding: its Postgres
deployment has no `unix_socket` DSN parameter at all -- `host=` doubles
as a socket-directory path there, confirmed by reading both its Nix
module and the pinned doctrine/dbal 4.4.3 Postgres driver directly.

## Real, recurring sub-findings (not new buckets, but worth keeping)

- **Consumer `composer.lock` sometimes lives in nixpkgs itself, not the
  fetched app source.** `flarum`, `baikal`, and `postfixadmin` all set
  `composerLock = ./composer.lock;` in their `package.nix` (upstream
  doesn't ship a lock file, so nixpkgs maintains its own). `fetch_composer_lock`'s
  `pkgs.<attr>.src + "/composer.lock"` path fails outright for these
  three -- a real "needs only a new locator" situation for K2a's
  *consumer* side specifically, distinct from anything producer-shaped.
  **Closed by K2a.1** (`resolve_composer_lock`, `dce5660`): all three now
  resolve via an explicit `pkgs.<attr>.composerVendor.composerLock`
  relationship, real-verified. `flarum`'s `provenance location` axis is
  now unblocked at the identity level (finds a genuine `doctrine/dbal`
  2.13.9 entry) though its `consumer support` axis stays `unsupported`
  until that version is separately vendored (deliberately not attempted
  in K2a.1); `baikal`/`postfixadmin` resolve too, confirmed to genuinely
  have no `doctrine/dbal` at all -- their `consumer support` stays
  `unsupported` for that reason, not a locator failure anymore.
- **`doctrine/dbal` presence in `composer.lock` is necessary, not
  sufficient, evidence that it's the actual runtime DB consumer --
  CONFIRMED, not just flagged, by K2e (below).** `agorakit`/`movim`/
  `snipe-it`/`flarum` are Laravel-family apps; `Illuminate\Database`'s
  own connectors, not Doctrine's DSN parser, are the real consumer of
  their `DB_*`/`config.php` values, verified by reading each app's own
  `config/database.php` (or equivalent) and tracing where `doctrine/dbal`
  itself is actually required from -- in every one of the four, it turns
  out to be present for a reason entirely disconnected from the primary
  database connection (three are outright vestigial pre-Laravel-11
  leftovers; flarum's is real but narrow, migrations-only). Real
  trust-boundary lesson for whenever "just check `composer.lock` for the
  package name" gets reused again: presence isn't wiring, and this round
  is the proof, not just the suspicion.
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

## K2e: consumer reachability census

Motivated directly by the sub-finding above, and done BEFORE any new
`ProducerEvidence` variant, consumer adapter, or checker code -- the
question this round answers: for every one of the 8 corpus apps where
`doctrine/dbal` is present at all (kimai, davis, strichliste, part-db,
agorakit, movim, snipe-it, flarum), what is the REAL chain from the
Nix-emitted external key to whatever actually consumes it? Not "is
`doctrine/dbal` present" (already known from the table above) -- who,
concretely, reads the value, and does the accepted-keys contract this
project would need to check even belong to Doctrine at all.

Each app's real, pinned source was read directly (exact GitHub tag +
commit cited per app below) -- no guessing from framework reputation.

### `direct` (2): kimai, davis

Already proven by K1's own frozen golden proof, not re-verified here --
the Nix-rendered `DATABASE_URL`/socket value flows straight into
Doctrine's own `PDO-MySQL-Driver.php` (`isset($params['unix_socket'])`),
with zero framework or bundle in between at all.

### `mediated-known` (6)

**`strichliste` (tag `v2.1.0`) and `part-db` (tag `v2.13.1`) -- same
mechanism, terminal library is Doctrine's own, contract unchanged:**

```
Nix DATABASE_URL (full DSN incl. unix_socket=...)
  -> config/packages/doctrine.yaml: doctrine.dbal.url: '%env(resolve:DATABASE_URL)%'
     (Symfony's plain env-var resolver -- no key transformation here)
  -> doctrine/doctrine-bundle's ConnectionFactory::createConnection()
     (src/ConnectionFactory.php) EAGERLY calls parseDatabaseUrl()
     itself, one stack frame before DriverManager::getConnection()
     would have
  -> that method instantiates and calls Doctrine\DBAL\Tools\DsnParser
     -- THE SAME CLASS this project already vendored/verified
     elsewhere, not a Symfony-authored reimplementation
  -> merged params array, unix_socket key intact and unrenamed
  -> Doctrine\DBAL driver code (identical accepted-keys contract as
     the direct kimai/davis case)
```
There genuinely IS an intermediary (`doctrine/doctrine-bundle`), so
this is honestly `mediated-known`, not `direct` -- but the terminal
consumer and its accepted-keys contract are Doctrine's own, unchanged.
Checking against Doctrine's `isset($params[...])` list is still the
right thing to do for these two apps.

`part-db` additionally has a SECOND, independent consumer of the
already-parsed params: `src/Command/BackupCommand.php`/
`src/Services/System/BackupManager.php` call `$connection->getParams()`
and re-check `isset($params['host'|'port'|'dbname'|'unix_socket'|'user'|'password'])`
themselves to build `pg_dump`/`mysqldump` shell commands -- same
key-name assumption asserted a second time, in application code this
time, downstream of the connection object rather than upstream of it.
Not a new bucket, just a real detail worth keeping given part-db's
already-known Postgres/MySQL-driver mismatch.

**`agorakit` (tag `v1.11`), `movim` (tag `v0.35`), `snipe-it` (tag
`v8.7.2`) -- same Laravel shape, terminal library is `Illuminate\Database`,
NOT Doctrine, `doctrine/dbal` confirmed vestigial:**

```
Nix DB_HOST / DB_PORT / DB_DATABASE / DB_USERNAME / DB_PASSWORD
  (+ DB_SOCKET for snipe-it; + DB_DRIVER for movim)
  -> config/database.php's connection array, via env('DB_HOST') etc.
     (snipe-it: 'unix_socket' => env('DB_SOCKET', '') -- direct 1:1
     rename, no normalization logic)
  -> Illuminate\Database\Capsule\Manager / Connectors\{MySql,Postgres}Connector
     (movim additionally routes through its own Bootstrap.php and a
     base Eloquent Model class, same terminal library)
```
`doctrine/dbal` is a DIRECT dependency in all three `composer.json`s
(not transitive), but confirmed to have NO real consumer of these
values: Laravel 11 deleted every DBAL bridge method from
`Illuminate\Database` (`getDoctrineConnection()`,
`getDoctrineSchemaManager()`, the `change()`/`renameColumn()` DBAL
bridge) -- confirmed directly in `Illuminate\Database\Schema\Grammars\MySqlGrammar`
v11.30.0, which now generates raw `ALTER TABLE` SQL natively. GitHub
code search of each app's own repo for `Doctrine\DBAL` returned zero
hits. All three are running post-Laravel-11 (`^11.30`/`^12`) without
having dropped the now-pointless `doctrine/dbal` requirement from their
own `composer.json` -- vestigial, not wired to anything.

**`flarum` (tag `v1.8.1`) -- same terminal library, but `doctrine/dbal`
is real, just narrow:**

```
Nix-generated config.php's 'database' array
  -> Foundation/Site.php::loadConfig() -> Config wrapper
  -> Foundation/Application.php::config() -> Database/DatabaseServiceProvider
  -> Illuminate\Database\Capsule\Manager::addConnection()
  -> Illuminate\Database\Connectors\* (reads driver/host/port/database/
     username/password directly, unchanged key names)
```
Unlike agorakit/movim/snipe-it, flarum's `doctrine/dbal` requirement
(`framework/core/composer.json`: `^2.7`, DIRECT) is real and load-bearing
-- but only to unlock Illuminate's OPTIONAL schema `renameColumn()`/
`dropColumn()`/`change()` operations during `php flarum migrate`, which
internally builds a Doctrine wrapper around the *same* Illuminate PDO
connection purely for schema introspection. It never touches the
primary request-time connection built from `config.php`'s values.

### `mediated-unknown` (0), `not_applicable` (8, + kimai/davis excluded from this axis's denominator)

No case in this corpus left the chain genuinely unresolved -- every one
of the 8 `doctrine/dbal`-present apps traced to a definite terminal
consumer. The 8 `doctrine/dbal`-absent apps (baikal, bookstack,
civicrm, engelsystem, grocy, invoiceplane, librenms, postfixadmin) are
`not_applicable`: this axis's question presupposes `doctrine/dbal`
being present at all, and wasn't investigated further (a real consumer
almost certainly exists for THEIR db config too, just not one this
project has any reason to compare against Doctrine's contract).

### The actionable signal this census exists to produce

**3 of the 6 `mediated-known` apps (agorakit, snipe-it, flarum) share
the IDENTICAL chain shape**: Nix-emitted `DB_*` keys ->
`config/database.php`'s Laravel connection array (1:1 `env()` mapping,
including `DB_SOCKET` -> `unix_socket` for snipe-it) -> `Illuminate\Database`.
`movim` is a 4th real instance of the same terminal library (Eloquent/
Capsule), reached via a slightly different bootstrap path but the same
env-var-to-connection-array shape. That's **4 of 6** `mediated-known`
cases converging on one framework, one config file shape, one
normalized-key mapping -- a real, repeated pattern, not a guess dressed
up as one.

The other 2 (`strichliste`, `part-db`) are a genuinely different shape
entirely: mediated through a Symfony bundle that ultimately calls
Doctrine's OWN parser, unchanged contract -- these do NOT need a new
adapter at all, only K1's existing model vendored against each app's own
pinned `doctrine/dbal` version (3.10.5 for strichliste, 4.4.3 for
part-db -- NOT the same version, corrected here; the original K2c note
above assumed both were 3.10.5, which was never independently confirmed
for part-db at the time). **Closed by K2g, see below.**

**K2f (`9e03eca`) acted on exactly this signal**: `ConsumerRoute`/
`ConsumerContractEvidence` + `acquire_illuminate_consumer_contract`,
real-verified for `agorakit`/`snipe-it` (MySQL) and `movim` (Postgres --
a genuinely different connector shape, with NO unix-socket concept at
all, a real finding in its own right). `flarum` deliberately excluded
from closure, per the design review -- its multi-consumer shape
(primary path Illuminate, optional migration-only path Doctrine) is a
real, distinct future corpus case, not folded into v1 for a round
number. See `README.md`'s own K2f section for the full writeup.

**K2g (`86c83e4`) closed the strichliste/part-db side.** Both now have
real, full end-to-end `Pass` verdicts, not just resolved identities --
`strichliste`'s pinned 3.10.5 driver confirmed byte-identical to the
existing 3.10.6 fixture (no duplicate vendored); `part-db`'s pinned
4.4.3 Postgres driver is a genuinely new fixture with no `unix_socket`
key at all -- `host=` doubles as the socket path there. See
`README.md`'s own K2g section for the full writeup.

### K2e stop condition

- [x] Every app with `doctrine/dbal` present got a real chain, not a
      guess -- all 8 (kimai, davis already known from K1; strichliste,
      part-db, agorakit, movim, snipe-it, flarum newly traced this
      round, each from real pinned source at an exact tag, cited above).
- [x] Classification is 4-valued and honest about mediation existing
      even when the terminal contract doesn't change -- strichliste/
      part-db are `mediated-known`, not silently folded into `direct`,
      even though the practical accepted-keys contract is identical to
      the direct case.
- [x] `mediated-unknown` and `no relevant consumer` are real, checked
      buckets, not omitted for looking empty -- both are 0, stated
      explicitly, not implied by absence from the table.
- [x] The actual point (a mechanical next-PR decision) is answered, not
      deferred further: 4 of 6 `mediated-known` apps converge on ONE
      chain shape (`DB_*` env -> `config/database.php` -> `Illuminate\Database`)
      -- a new consumer-adapter class for that shape is now corpus-
      justified, not a guess. The other 2 need zero new code, only
      Phase D vendoring already identified in K2c.
- [x] No `src/cdc.rs` change in this round -- confirmed, this is a
      census-only round, same discipline as K2c itself.

## K2c stop condition

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
