# K5b: real historical env-var drift census

Same question as K3b/K4b, applied to the new interface family: does at
least one real nixpkgs package bump, for a real direct-boundary
service, cross a real upstream removal or rename of an environment
variable? Research-only, bounded corpus, zero `src/cdc.rs` changes --
`diff_env_contracts()` (K5a) is not touched, not called against any
pair recorded here.

**Explicit stop condition honored**: this round stops the moment a
real, nixpkgs-crossed consumer drift is confirmed. The real Nix
producer for the qualifying case (`grafana`'s NixOS module) is
deliberately NOT inspected in this round -- that correlation is K5c's
job entirely, not K5b's, per the explicit instruction not to let the
two rounds quietly merge.

## Corpus (bounded, direct-boundary only, Laravel/Symfony excluded)

Built from a real, targeted search of NixOS service modules that set
`environment` directly for a real binary (`gh api search/code` for
`systemd.services`+`environment`+a `DATABASE_URL`-shaped variable name
under `nixos/modules/services`), then filtered to services whose
upstream consumer reads env vars close to directly -- a real
`os.Getenv`/`env::var`/declarative-schema mechanism, not a PHP web
framework's config layer. Laravel/Symfony-family apps (already known
from K2e/K2f to interpose real framework mediation between an env var
and its actual consumer) were deliberately excluded from this corpus
from the start, not filtered out after the fact.

| package | language | consumer/source mechanism | investigated |
|---|---|---|---|
| grafana | Go | declarative INI-schema-derived `GF_<SECTION>_<KEY>` env vars, real `os.Getenv()` call confirmed in `pkg/setting/setting.go` | yes, deep |
| vaultwarden | Rust | direct env-driven config struct | yes, light |
| lldap | Rust | direct env-driven config | yes, light |
| hatsu | Rust | direct env-driven config (ActivityPub bridge) | attempted, repo/release data inconclusive, not pursued further |
| miniflux | Go | direct env-driven config (well-documented `MINIFLUX_*` convention) | attempted, no clean signal found in the checked range, not pursued further |
| shiori | Go | direct env-driven config | identified, not deep-checked (stopped once `grafana` qualified) |
| fider | Go | direct env-driven config | identified, not deep-checked |
| headscale | Go | direct env-driven config | identified, not deep-checked |
| atticd | Rust | direct env-driven config | identified (exact package.nix path not resolved), not deep-checked |
| lemmy | Rust | direct env-driven config | identified (`pkgs/servers/web-apps/lemmy/server.nix`), not deep-checked |
| alerta, kener, traccar, firefox-syncserver, tap, warpgate, bookorbit, cocoon, docuseal, glitchtip, nextcloud-notify_push, outline, papra, plausible, umami, windmill, zerobyte, zipline, linkwarden | mixed | from the same initial search, language/mechanism not independently confirmed this round | identified only, not investigated -- the search stopped once the decision rule's own bar was cleared, matching K4b's own precedent |

~29 candidates identified from the initial search; 3 investigated to
real depth or light-touch confidence before the bounded search stopped
per the explicit instruction ("не насиловать" -- don't keep digging
once a real qualifying case exists).

## The qualifying result: `grafana`

| field | value |
|---|---|
| service/package | `grafana` |
| base nixpkgs revision | `3f22226334` (oldest real commit touching `pkgs/by-name/gr/grafana/package.nix`) |
| head nixpkgs revision | the project's own `AFTER_REV` (`PhysShell/nixpkgs@d81d88f4354b2c9d8a7492c9b72cd3add62a34e0`) |
| base version | `12.3.3` |
| head version | `13.1.4` |
| consumer/source mechanism | declarative INI-schema-derived env vars -- `pkg/setting/setting.go`: `envKey := EnvKey(section.Name(), key.Name()); envValue := os.Getenv(envKey)`. A real, direct `os.Getenv()` call (tier-1-adjacent), reading a name MECHANICALLY DERIVED from the INI config schema (tier 2, "structured generated metadata/schema") rather than a literal string per variable -- confirmed by reading the source, not assumed from the well-known `GF_*` documentation convention alone. |
| removed vars | `GF_AUTH_PASSWORDLESS_ENABLED`, `GF_AUTH_PASSWORDLESS_CODE_EXPIRATION` |
| added vars | none identified in this specific removed section (a pure removal, not a rename -- the `[auth.passwordless]` section has no direct successor in `13.1.4`'s config surface) |
| retained vars | not exhaustively extracted this round (Grafana's full config surface is enormous; extracting the COMPLETE contract for both versions is out of K5b's bounded research-only scope, same disclosed limitation K4b recorded for `mimir`) |
| confidence | **source + schema** -- the removed section (`[auth.passwordless]`, containing `enabled`/`code_expiration`) confirmed absent by diffing the real, fetched `conf/defaults.ini` directly at both exact real tags (`v12.3.3`/`v13.1.4`); independently corroborated by a real, dated upstream commit (`fa90e56303`, "Auth: Remove passwordless (magic link) authentication backend implementation", merged before `v13.1.4` shipped) |
| genuinely nixpkgs-crossed | **yes** -- both `12.3.3` and `13.1.4` are real nixpkgs pins for the same `grafana` package (oldest real commit vs. the project's own `AFTER_REV`) |

```
v12.3.3 conf/defaults.ini:
  [auth.passwordless]
  enabled = false
  code_expiration = 20m

v13.1.4 conf/defaults.ini:
  (section entirely absent -- confirmed via grep, zero "passwordless"
  occurrences anywhere in the file)
```

**A real methodological trap caught and discarded before trusting
anything**: an initial search of Grafana's commit history for
"removed"/"renamed" turned up a MORE recent, cleaner-looking hit first
-- `1b8cf693d2` ("rename config.ini section from marketplace to
plugins_marketplace"). Checked directly against both real pinned
versions before treating it as a finding: `[marketplace]` doesn't
exist in EITHER `12.3.3` or `13.1.4` -- the whole feature (and its
later rename) postdates `13.1.4`'s release entirely, genuinely outside
this nixpkgs-crossed pair's window. Discarded, not reported as a
finding -- exactly the `doctrine/dbal`/`default_dbname` trap from
K3b/K3b.1, caught again here on the first attempt this round, by
verifying against real fetched files rather than trusting a commit
message's own framing.

## Lighter-touch corpus breadth, real negative data points

- **`vaultwarden`** (Rust, `1.33.2` → `1.37.3`): no removed/renamed env
  var found in a real release-notes scan across the range.
- **`lldap`** (Rust, `0.5.1-unstable-2024-10-30` → `0.6.3`): no
  removed/renamed env var found in a real release-notes scan; the only
  env-adjacent change found was an unrelated tool behavior change
  (`lldap_set_password` SSL certificate handling), not a contract
  change.

## Decision, per the rule fixed before this census started

> >=1 real removed/renamed env var → K5c
> only added-only drift → один умеренный corpus expansion
> no useful drift after bounded search → env vertical parked, reassess
> next family

**Result: ≥1 real removed env var found** (`grafana`,
`GF_AUTH_PASSWORDLESS_ENABLED`/`GF_AUTH_PASSWORDLESS_CODE_EXPIRATION`
removed, `12.3.3` → `13.1.4`), confirmed at source+schema confidence,
on a real nixpkgs-crossed pair. **Per the pre-fixed rule: proceed to
K5c.**

**Explicitly NOT done this round, per the explicit instruction**:
whether `grafana`'s real NixOS module still sets either
`GF_AUTH_PASSWORDLESS_ENABLED` or `GF_AUTH_PASSWORDLESS_CODE_EXPIRATION`
after the bump. That producer correlation is entirely K5c's job --
K5b's only job was proving the historical consumer drift itself is
real, and it stops exactly there.

**Explicitly NOT done this round, per K3a's/K4a's/K5a's own "scope
must end somewhere" discipline**: no same-name semantic env drift
(changed default/parser semantics under an unchanged variable name) was
chased, even though the K4b `mimir` round already proved this class of
issue is real and findable. Names only, this round.

## Stop condition

- [x] Corpus is bounded (~29 candidates identified from a real
      targeted search, 3 investigated to real/light depth) and
      genuinely direct-boundary -- Laravel/Symfony excluded from the
      corpus at the search stage, not filtered out after the fact.
- [x] The qualifying pair is a REAL nixpkgs package bump (`12.3.3` and
      `13.1.4` are both real nixpkgs pins for the same `grafana`
      package), not a synthetic before/after -- the exact trap this
      round was warned against, caught and discarded once already
      (the `marketplace`→`plugins_marketplace` rename) before landing
      on the real, verified finding.
- [x] Contract source ranked and cited per the fixed methodology --
      the removed section confirmed by diffing real fetched source at
      both exact tags AND by reading the actual `os.Getenv()` call
      site, not docs alone.
- [x] `diff_env_contracts()`/K5a untouched -- confirmed, zero
      `src/cdc.rs` changes this round.
- [x] Search stopped the moment consumer drift was proven -- the real
      Nix producer for `grafana` was deliberately never inspected this
      round, per the explicit instruction not to let K5b and K5c
      quietly merge.
