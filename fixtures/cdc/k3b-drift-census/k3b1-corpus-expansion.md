# K3b.1: bounded search for a real nixpkgs consumer straddling
# doctrine/dbal's `default_dbname` removal

Not "explore Doctrine some more" -- one specific, falsifiable hunt: a
real nixpkgs PHP consumer app whose own real package bump crosses
`doctrine/dbal`'s Postgres driver's `default_dbname` removal (K3b's own
finding: present through the entire 3.x line, including the CURRENT
3.10.6 release; absent from every 4.x release starting at
`4.0.0-beta1`, published 2022-10-22).

**Bounded, per the explicit instruction**: real search effort spent on
~20 candidate consumers below (the existing 16-app K2c corpus + 11 new
candidates from a targeted module search, 5 of which turned out
inapplicable and are recorded as such, not silently dropped), two
concrete real search strategies, then STOP regardless of outcome --
not open-ended Composer archaeology.

## Step 1: does `default_dbname` even matter for our current corpus?

Checked release-by-release, not assumed: `default_dbname` is present
(`grep -c` confirms 3 real occurrences: the `isset` check, the
`Deprecation::trigger` call, the DSN append) in doctrine/dbal's CURRENT
`3.10.6` tag -- the exact version `kimai`/`davis` already pin. It is
absent (0 occurrences) from `4.0.0-beta1` (2022-10-22) onward, including
`part-db`'s own oldest real pin, `4.2.1` (confirmed via `grep` on the
already-vendored file in K3b). So the removal is real and durable
(doctrine/dbal maintains 3.x and 4.x as parallel long-lived lines --
3.10.6 itself was published 2026-07-21, LATER than many 4.x releases),
not a since-reverted anomaly.

## Step 2: search for a Postgres-dialect Doctrine consumer beyond `part-db`

**Strategy 1** -- re-check every doctrine/dbal-having app already known
from K2c/K2e/K2f/K2g's own corpus (16 apps) for a Postgres deployment
using Doctrine's OWN driver (not Illuminate's, not vestigial):

| app | dialect(s) supported | doctrine/dbal consumer? |
|---|---|---|
| kimai, davis, strichliste, snipe-it | mysql only | N/A (dialect mismatch) |
| agorakit, flarum | mysql only | N/A (dialect mismatch; also vestigial) |
| part-db | pgsql | yes -- but never pinned 3.x (nixpkgs entry Dec 2024, well after 4.0.0 shipped Feb 2024) |
| movim | pgsql | consumer is Illuminate, not Doctrine (K2e) |
| baikal, bookstack, civicrm, engelsystem, grocy, invoiceplane, librenms, postfixadmin | -- | doctrine/dbal absent (K2c) |

Zero new candidates from the already-known corpus.

**Strategy 2** -- search NixOS modules combining `phpfpm` and
`postgresql` (a real, targeted query, not a blind grep), for apps
outside the 16-app corpus: `firefly-iii`, `moodle`, `limesurvey`,
`speedtest-tracker`, `zabbix`, `freescout`, `pixelfed`, `dolibarr`,
`mediawiki`, `tt-rss`, `freshrss` (11 real candidates). Checked each
for `doctrine/dbal` in its real, resolved `composer.lock`:

| app | result |
|---|---|
| moodle, limesurvey, zabbix, dolibarr, mediawiki | **not applicable** -- confirmed real, not assumed: neither `pkgs.<attr>.src + "/composer.lock"` nor `pkgs.<attr>.composerVendor.composerLock` resolves for any of these (`composerVendor` is `null`) -- these apps don't use `buildComposerProject2`'s Composer-vendoring mechanism at all, so there's no doctrine/dbal version to track via this project's established provenance pipeline for them |
| firefly-iii, speedtest-tracker, pixelfed, tt-rss, freshrss | `doctrine/dbal` genuinely absent from `composer.lock` (real, confirmed empty result, not an eval failure) |
| freescout | `doctrine/dbal` 2.12.1 present -- but this is the OLD major-2 line (predates the 3.x/4.x split this search is about entirely), and freescout is Laravel-family (`DB_CONNECTION`/`DB_HOST`/`DB_SOCKET`, same `FlatEnvVars` shape as agorakit/snipe-it/movim) -- almost certainly vestigial like every other Laravel-family doctrine/dbal dependency this project has found (K2e), not independently confirmed further since it's not relevant to the 3.x/4.x Postgres boundary regardless |

A second, narrower search (`symfony` + `postgresql` co-occurring in a
NixOS module -- targeting the ONE mechanism, Symfony's DoctrineBundle,
already confirmed in K2e to reach Doctrine's own driver code
unmediated) returned only `davis`/`strichliste`, both already known and
both MySQL-dialect.

## Result: NOT FOUND, within the bounded search

No real nixpkgs consumer was found whose own real package bump
straddles doctrine/dbal's Postgres `default_dbname` removal. The one
real Doctrine-PostgreSQL consumer in the entire corpus surveyed so far
(`part-db`) entered nixpkgs too recently (Dec 2024) to have ever pinned
a pre-4.0 version. No other Symfony/Doctrine-shaped Postgres consumer
turned up in ~20 real candidates checked across two targeted search
strategies.

## Decision, per the rule fixed before this round started

> Не нашли ни одного straddling bump, всё, прекращаем ковырять
> Doctrine.

**The PHP/Doctrine/Illuminate differential-drift corpus is closed for
now, not abandoned** -- if a new Postgres-dialect Doctrine consumer is
ever packaged in nixpkgs with a real pre-2024 history, or if `part-db`
itself is ever repackaged from an older fork/version, this specific,
already-proven-falsifiable pair becomes available again. Not
re-litigated with a synthetic before/after pair in the meantime.

**Next, per the same pre-fixed decision tree**: pivot to a different
interface family for K3/K4-style differential validation. CLI flags
(`--old-flag` → `--new-flag`, checkable via `--help`/argument-parser
source/package-version diffs, with the Nix producer side usually
directly visible in a module's own `ExecStart`/args construction) were
named as the preferred next family over env vars -- process argv is
typically a direct producer↔consumer boundary, unlike the framework-
mediation this project's own K2e/K2f/K2g work found repeatedly sitting
between an env var and its actual consumer. Not started this round --
recorded as the recommended next direction, not decided in advance
without confirmation.
