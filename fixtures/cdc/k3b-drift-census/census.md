# K3b: real historical contract-drift census

Not "can a differential checker be built" -- that's already close to
obvious after K3a. The actual question: **is there enough real
historical contract change across the four already-qualified consumer
families (Doctrine MySQL, Doctrine PostgreSQL, Illuminate MySQL,
Illuminate PostgreSQL) for differential mode to be useful, not just
intellectually tidy?**

No code changed as part of this census. `diff_contracts()` (K3a) is not
touched, not extended, not called against any pair recorded here --
every comparison below was done by direct diff/`grep` against real
fetched source, exactly the same discipline K2c's own census used
before any K2-series code was written. `fetch_composer_lock`/
`resolve_consumer_identity` (K2a, unmodified) were the resolution
mechanism for every version pin below -- the SAME provenance pipeline
already proven, not a new one built for this round.

## Methodology

For each of the 4 qualified families, the OLDEST real nixpkgs commit for
that app (its own `init at` commit where available, or the oldest commit
`gh api commits?path=...` returns) was compared against the current
`AFTER_REV` pin (`PhysShell/nixpkgs@d81d88f4354b2c9d8a7492c9b72cd3add62a34e0`).
This is a REAL historical bump pair for a REAL, currently-qualified
consumer app -- not a hand-picked pair of upstream library release
tags. The library's own consumer version pin (`doctrine/dbal` or
`illuminate/database`/`laravel/framework`) was resolved at both
commits via `fetch_composer_lock` (K2a), then the exact driver file at
both resolved commits was fetched directly from the library's own
upstream repo and diffed.

**Accepted-key extraction used the SAME rule this project's own real
extractors already use** (`extract_accepted_keys` for Doctrine;
`illuminate_connector_accepts_key`'s "either array-keyed OR
boundary-checked bare-variable form counts" rule for Illuminate) --
not a looser or stricter ad-hoc reading. This mattered for real: see
the `movim` entry's own methodological note below, where a naive
array-keyed-only reading would have produced a false "removed" finding
this project's actual extractor does not produce.

**Outcome categories**: `no_drift` / `added_only` / `removed_only` /
`removed_and_added` / `inconclusive`. An `added_only` result is
recorded as a real, valid outcome, NOT as a "potential bug" -- a
library adding a new optional key Nix never emits is a real contract
change, but not actionable on its own (the explicit methodological trap
named going into this round).

## Raw table

| app | package | base nixpkgs rev | head nixpkgs rev | library | dialect | base version | head version | removed keys | added keys | resolution provenance | extractor status | outcome |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| kimai | kimai | `58d74ee990` (2025-06-07, kimai 2.34.0) | `d81d88f4354b` (AFTER_REV) | doctrine/dbal | mysql | 3.9.4 @ `ec16c82f` | 3.10.6 @ `c95589d7` | none | none | `fetch_composer_lock` (K2a), both commits | ok | **no_drift** |
| part-db | part-db | `10e92c1832` (2024-12-14, init at 1.14.5) | `d81d88f4354b` (AFTER_REV) | doctrine/dbal | pgsql | 4.2.1 @ `dadd3530` | 4.4.3 @ `61e730f1` | none | none | `fetch_composer_lock` (K2a, `pkgs.part-db.src`), both commits | ok | **no_drift** (see family-level note below) |
| agorakit | agorakit | `122b38d664` (2024-09-28, init at 1.9.2) | `d81d88f4354b` (AFTER_REV) | illuminate/database (via laravel/framework) | mysql | v8.83.27 @ `e1afe088` | v11.44.2 @ `f85216c8` | none | **`use_db_after_connecting`** | `fetch_composer_lock` (K2a); driver source via `laravel/framework`'s own monorepo tag (the `illuminate/database` split-repo's ref for this old commit 404'd -- see note) | ok, with a fallback | **added_only** |
| movim | movim | `f63cb5988f` (2024-02-17, init at 0.24) | `d81d88f4354b` (AFTER_REV) | illuminate/database | pgsql | v10.43.0 @ `a98f7b98` | v12.69.2 @ `499f5112` | none (by key name) | none (by key name) | `fetch_composer_lock` (K2a, `illuminate/database` split-repo resolved cleanly for this ref) | ok | **no_drift** (see mechanism-change note below) |

## Family-level note: `doctrine/dbal`'s Postgres driver DOES have real
## historical drift -- just not inside `part-db`'s own window

`part-db`'s own real nixpkgs history never pinned `doctrine/dbal` older
than 4.2.1 (it entered nixpkgs 2024-12-14, well after `doctrine/dbal`
4.0 had already shipped). Reading `doctrine/dbal`'s own git history for
`src/Driver/PDO/PgSQL/Driver.php` directly (not filtered through any
nixpkgs consumer) turns up two real, dated, upstream-confirmed contract
changes, both OLDER than `part-db`'s own oldest real pin:

- **A real REMOVED key**: commit `3aa0d3e407` ("Remove default_dbname
  parameter of wrapper Connection") deleted the
  `elseif (isset($params['default_dbname']))` fallback branch from
  `constructPdoDsn` -- `default_dbname` was a real, deprecated-then-
  removed alias for `dbname` (deprecation PR referenced in the diff:
  doctrine/dbal#5705). Shipped as part of the 4.0 line, before
  `part-db`'s own oldest pin (4.2.1) -- confirmed absent from that pin
  already (`grep default_dbname` on the vendored 4.2.1 file: no hits).
- **A real ADDED key**: commit `b4329d54eb` ("Add gssencmode option to
  connection string for PgSQL & PDO PgSQL driver", merged 2024-03-03,
  PR #6320) added `isset($params['gssencmode'])` to `constructPdoDsn`.
  Also already present in `part-db`'s own oldest real pin (4.2.1) --
  confirmed by grep, not assumed.

**This is exactly the finding this census exists to surface**: real
upstream contract drift genuinely exists for this family, but the
CURRENTLY-qualified consumer (`part-db`) simply wasn't packaged in
nixpkgs early enough to have ever pinned a version on the other side of
either event. Not a reason to fabricate a synthetic before/after pair
for `part-db` -- if K3c is ever pursued for this family, it would need
either an older consumer (a different, not-yet-qualified Postgres-
Doctrine app) or a direct library-level check independent of any single
Nix-packaged consumer.

## Methodological note: `movim`'s Illuminate PostgreSQL driver changed
## MECHANISM without changing key NAMES -- invisible to a name-only diff

Reading `movim`'s old (`v10.43.0`) and current (`v12.69.2`)
`PostgresConnector.php` side by side: `charset` and `application_name`
are accepted in BOTH versions, but the OLD version applies them via a
post-connect SQL statement (`$connection->prepare("set names '...'")`
/ `"set application_name to '...'"`, both gated on plain
`isset($config['charset'])`/`isset($config['application_name'])`
array-keyed checks), while the CURRENT version embeds them directly in
the DSN string itself (`client_encoding='...'`/
`application_name='...'`, both gated on bare-variable
`isset($charset)`/`isset($application_name)` checks AFTER the
function's own `extract($config, EXTR_SKIP)` call).

A naive extractor that only recognized ONE of these two syntactic forms
(array-keyed OR bare-variable, not both) would have wrongly reported
one of `charset`/`application_name` as "removed" here -- a false
positive, not a real contract break (the KEY NAME Illuminate actually
reads from the caller's config array never changed; only the internal
mechanism consuming it did). This project's own real extractor
(`illuminate_connector_accepts_key`, K2f) already checks BOTH forms via
`||` for exactly this reason -- confirmed here to be load-bearing, not
speculative: this is a real case where the alternative, narrower design
would have produced a wrong census entry.

## Decision, per the rule fixed before this census started

> если есть ≥1 реальный removed/renamed contract token: идти в K3c
> если только added-only drift: расширить corpus / family
> если drift почти нет вообще: не насиловать Doctrine/Illuminate,
> искать другой interface family

**Result: zero of the four qualified consumer families show a real
removed or renamed token within their own currently-observable
historical window.** One family (Illuminate MySQL, via `agorakit`)
shows a real `added_only` event. The other three show no name-level
drift at all within their real windows -- though `part-db`'s own family
(`doctrine/dbal` Postgres) is now confirmed to have real drift
*outside* that window, and `movim`'s family has a real mechanism change
invisible to a name-only diff.

**This round's honest conclusion, per the pre-fixed decision rule: NOT
yet K3c.** The corpus as currently qualified does not contain a real
removed/renamed token inside any consumer's own observable window --
proceeding straight to K3c now would mean building producer-correlation
logic with nothing real to correlate against. The two directions the
rule itself points at, in order of how directly they follow from what
was actually found here:

1. **Expand the corpus within the same two libraries** -- specifically
   look for an app whose real nixpkgs history predates
   `doctrine/dbal`'s 4.0 line (to actually straddle the real
   `default_dbname` removal found above), rather than assuming none
   exists.
2. **Look at a different interface family entirely** (CLI flags,
   env-var naming conventions, etc.), per the user's own hypothesis
   that rename/removal is likely more common there than in these two
   specific, comparatively conservative database-driver libraries.

Neither direction is started by this round. `flarum`, H2, and D3 stay
untouched, per the explicit instruction not to touch them until K3b
produced this statistic.

## Stop condition

- [x] Historical pairs are real nixpkgs package bumps for the four
      already-qualified consumer apps, never hand-picked upstream
      release tags standing in for a real bump.
- [x] Exact consumer source resolved through the same provenance
      pipeline already proven in K2a/K2a.1 (`fetch_composer_lock`),
      not a new or looser resolution mechanism.
- [x] Every entry's extractor status is recorded explicitly (`ok`, or
      `ok, with a fallback` for `agorakit`'s split-repo 404) -- nothing
      silently treated as "contract unchanged" without a real
      extraction actually succeeding.
- [x] `added_only` is recorded as a real, non-actionable-on-its-own
      outcome, not conflated with a potential bug.
- [x] The negative result (no removed/renamed token in-window, across
      all four families) is reported as a real, informative finding,
      not reframed as failure or routed around with a synthetic case.
- [x] `diff_contracts()`/K3a untouched -- confirmed, zero `src/cdc.rs`
      changes this round.
