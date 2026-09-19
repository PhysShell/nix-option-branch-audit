# E1 holdout audit — batch 3 (candidates 21-30)

Real, frozen `oba check` run against `targets/e1-holdout-batch3.toml`:
`summary: {'pass': 4, 'finding': 2, 'inconclusive': 12}` across the 10
candidates' 16 total watched paths. Zero `src/` changes; the binary run
was the exact commit `c8e42a1` frozen state, unmodified.

## Two real, load-bearing structural gaps found this batch (not one-offs)

**Gap A — `mkEnableOption` (and any non-literal-`mkOption` helper, e.g.
`mkPackageOption`) is completely invisible to `scan_options`'s
declaration scanner.** `is_mk_option_call` (src/main.rs) matches the
call head name against the literal string `"mkOption"` only — an
`enable = lib.mkEnableOption "...";` declaration (the single most
common boolean-flag idiom in all of nixpkgs) is neither an `mkOption`
call nor an attrset to recurse into, so `walk_options_block` silently
skips the entry: the option is never added to `OptionDecl` at all, and
any target watching it gets `OptionNotFound`, not `PredicateNotFound`
or any inconclusive-with-reason — indistinguishable, from the report
alone, from a genuinely absent option. Confirmed the negative control
too: `peerflix` declares `enable` via a plain `lib.mkOption { type =
bool; default = false; }` (not the helper) and got a clean real `PASS`
in the same run, everything else about its shape identical to the
failing cases. Hit in THIS batch alone by nimdow, flame, convos, tor,
kthxbye, authelia, pocket-id, coturn (8 of 10) for their `enable`
option specifically. K1-K5's own corpus never exercised this because
kimai/davis/every K1-K5 fixture happens to watch a non-`enable` option
and none of them happen to use `mkEnableOption` for the options they
DO watch.

**Gap B — modules that declare options via the nested `options = {
services.<name> = { ... }; };` form (one multi-segment attrpath per
top-level entry, rather than the flat `options.services.<name> = {
...};` form) get `OptionDecl.path` values with `option_prefix`'s own
segments baked in as a literal prefix, breaking `watch`'s documented
contract ("Option paths (relative to cfg_ident, dot-joined)").**
`scan_options`'s generic `segs == ["options"]` branch calls
`walk_options_block` with an EMPTY starting path; `walk_options_block`
then extends that path by each entry's own attrpath BEFORE deciding
whether to recurse — so `services.coturn = { no-auth = mkOption
{...}; ...}` produces `no-auth`'s final path as `["services","coturn","no-auth"]`,
not `["no-auth"]`. `coturn`'s `no-auth`/`static-auth-secret-file` (both
real, plain `mkOption` declarations, nothing to do with Gap A) got
`OptionNotFound` purely from this — confirmed by checking coturn uses
the nested form while flame/peerflix/convos/kthxbye/bees/pocket-id all
use the flat form and their plain-`mkOption` watched options resolved
correctly. `tor` hits BOTH gaps stacked on its own `enable` (nested
form AND `mkEnableOption`).

Neither gap was worked around in this batch's manifest -- `watch` was
written exactly per the documented contract each time, not adjusted
once the internal cause was understood, per the protocol's own rule.

## Per-candidate results

**nimdow** — Nix/shell, X11 window-manager session launcher, no
external consumer boundary at all.
OBA outcome: inconclusive (`OptionNotFound` on `enable`) — Gap A (its
one option is `mkEnableOption`, plus the value isn't an attrset to
recurse into either way).
CDC structural fit: not_applicable (spawns a bare binary with no
flags/env/config/DSN of any analyzable kind).
Notes: cleanest, smallest possible Gap-A repro in the batch.

**flame** — Node.js, JSON settings file + SQLite seed + a firewall
toggle.
OBA outcome: mixed. `enable` → inconclusive (Gap A). `openFirewall` →
**real FINDING (OBA001)**, manually verified: `fixtures/e1-holdout-audit/flame/test.nix`
never sets `openFirewall`, so `networking.firewall = lib.mkIf
cfg.openFirewall {...}` was genuinely never exercised opposite its
`false` default — survives manual check. `apps` → inconclusive
(`PredicateNotFound`) — a THIRD real gap: `lib.mkIf (cfg.apps != [ ]
|| cfg.categories != [ ])` compares against an empty LIST literal,
which `ValueExpr::Literal(Scalar)` (Null/Bool/Str only) can't
represent at all, so neither H1 nor H2 can lower this predicate.
CDC structural fit: existing-abstraction (`EvaluatedLiteral`-shaped —
a real generated JSON settings file plus a file-loaded secret, same
class K1 already models).

**peerflix** — Node.js torrent streamer, JSON config file.
OBA outcome: **real PASS** on `enable` (plain `mkOption`, not
`mkEnableOption` — the batch's negative control for Gap A). Test node
is named `peerflix`, not `machine` — confirms the walker's node-name
handling is genuinely generic, not hardcoded.
CDC structural fit: existing-abstraction (`EvaluatedLiteral` — JSON
config with `"tmp": "${cfg.downloadDir}"` interpolated in).

**coturn** — C, real generated `turnserver.conf`.
OBA outcome: inconclusive across all 3 watched options
(`enable`/`static-auth-secret-file`/`no-auth`, all `OptionNotFound`) —
Gap B (nested `options = { services.coturn = {...}; };` form), with
`enable` additionally hitting Gap A. The real predicate structure here
is genuinely rich and would otherwise have been a strong case:
`static-auth-secret-file != null` (default null, a real node in the
test sets it to a real path) and `no-auth` (Truthy, never set by
either test node) are both real, IR-representable predicates that
never got the chance to run because gate 1 never found their
declarations at all.
CDC structural fit: existing-abstraction (`EvaluatedLiteral` — a
generated conf file with many `optionalString`-gated key lines, same
shape class as K1's own DSN-in-config-file pattern, just more keys).

**convos** — Perl (Mojolicious), env vars set directly on the systemd
unit.
OBA outcome: `enable` → inconclusive (Gap A). `reverseProxy` → **real
FINDING (OBA001)**, manually verified: test.nix never sets
`reverseProxy`, and the module's `CONVOS_REVERSE_PROXY = if
cfg.reverseProxy then "1" else "0";` is a genuine branch (recognized
as a `Truthy`/`"if"`-kind predicate on a bare scalar `if/then/else`
expression, not just `mkIf`-gated attrsets — broader real predicate
recognition than assumed going into this batch). Survives manual
check.
CDC structural fit: existing-abstraction (`FlatEnvVars`, K2d's own
shape — `environment = { CONVOS_HOME=...; CONVOS_REVERSE_PROXY=...;
MOJO_LISTEN=...; };` is about as textbook a match as this batch has).

**tor** — C, extremely elaborate multi-node directory-authority test
topology (per-node configs built via `map`/helper functions over
`daNames`/`relayNames`/`exitNames` lists, not literal `nodes.<name> =
{...}` text).
OBA outcome: inconclusive (`OptionNotFound` on `enable`) — Gap A + Gap
B stacked (same nested-options-block shape as coturn). Real
`services.tor.enable = true;` text does appear in the file, but only
inside function bodies (`mkDANode`/`mkRelayNode`/`mkExitNode`) later
mapped over name lists to build the actual `nodes` attrset — even had
gates 1/2/3 all passed, this would very likely have surfaced as real,
disclosed test-side opacity (the documented "function call" limitation
on the test-assignment walker), not a clean witness. Not fully traced
to that point since gate 1 already failed; flagged, not fabricated.
CDC structural fit: not independently confirmed this round (module too
large to fully trace in the time available) — likely `EvaluatedLiteral`
(a generated settings format), not verified.

**bees** — C++, per-filesystem `attrsOf submodule`, CLI-argv daemon.
OBA outcome: inconclusive (`PredicateNotFound` on `filesystems`) — the
SAME empty-collection-literal gap as flame's `apps`/`categories`
(`lib.mkIf (cfg.filesystems != { })` compares against an empty
ATTRSET literal this time, not a list — same underlying
`ValueExpr::Literal` limitation, second independent confirmation in
one batch that this is a recurring pattern, not a one-off). Gate 1
itself succeeded here (flat `options.services.beesd = {...}` form,
plain `mkOption`) — this is purely a predicate-lowering failure, a
cleanly isolated repro of the empty-collection-literal gap with no
other gap mixed in.
CDC structural fit: existing-abstraction (`CliContract` — the real
daemon's `ExecStart` is built from a literal flag list
`[fs.spec "verbosity=..." "idxSizeMB=..." "workDir=..."]` plus
`fs.extraOptions`, genuinely CLI-argv-shaped like K4a/K4c's own
family).

**kthxbye** — Go, pure CLI-argv daemon (Alertmanager silence
extender).
OBA outcome: `enable` → inconclusive (Gap A). `openFirewall`,
`logJSON`, `maxDuration` → **all real PASS** (test node `server` sets
all three to real non-default values: `openFirewall=true`,
`logJSON=true`, `maxDuration="15m"`). `maxDuration`'s real IR, from the
tool's own JSON output: `{"Not":{"Eq":[{"Ref":["maxDuration"]},{"Literal":"Null"}]}}`
— a genuine `!= null` compound lowered correctly, unlike the
empty-collection cases above (`null` IS a representable `Scalar`).
The single richest real multi-witness result in this batch.
CDC structural fit: existing-abstraction (`CliContract` — textbook:
`-alertmanager.timeout`/`-extend-by`/`-max-duration`/... flags built
directly from option values, closest real analogue to K4a/K4c's mimir
case in the whole batch).

**authelia** — Go, multi-instance (`instances.<name>`, same shape
class as kimai's own `sites.*`), generated YAML config + a
name-keyed env-var secrets map.
OBA outcome: inconclusive (`OptionNotFound` on `instances.*.enable`)
— pure Gap A (the per-instance submodule's own `options = { enable =
mkEnableOption "..."; ...}` block is found via the generic tree walk
with a naturally-relative path, so this one is NOT also hitting Gap B
— confirmed by structure, not just by the observed OptionNotFound
alone). This module's real conditional structure (which instances
actually get a systemd service) is mediated through
`enabledInstances = lib.filterAttrs (name: instance: instance.enable)
cfg.instances;`, a plain function call feeding `mapAttrs'` — not a
textual `mkIf`/`optional*` site at all, a genuinely different and
harder shape than anything else in this batch; even with Gap A fixed,
this specific mediation pattern is not one `scan_predicates`/
`scan_resolved_predicates` currently look for.
CDC structural fit: **new-shape-needed** (judgment call) — real YAML
settings file (`EvaluatedLiteral`-adjacent) PLUS a real per-secret env
var map where the ENV VAR NAME comes from a lookup table value, not a
flat literal attrset the way `FlatEnvVars` assumes
(`envSecretsMap = { AUTHELIA_..._FILE = "jwtSecretFile"; ... };` then
`lib.mapAttrs (_: v: "%d/${v}") nonNullEnvSecretsMap`) — more indirect
than any existing K1-K5 abstraction.

**pocket-id** — Go, real key=value env-var settings file + a
`systemd-creds`-backed credentials mechanism.
OBA outcome: inconclusive (`OptionNotFound` on `enable`) — pure Gap A
(flat `options.services.pocket-id = {...}` form, so no Gap B). Test
nodes `machineSqlite`/`machinePostgres` (again non-`machine` names,
both set `enable = true`) would very likely have produced a clean real
PASS had Gap A not blocked gate 1 — flagged as a real, disclosed
missed opportunity, not fabricated as a result.
CDC structural fit: **new-shape-needed** (judgment call) — `settings`
alone is textbook `FlatEnvVars` (a real `pkgs.formats.keyValue`
env-file), but `credentials` (an `attrsOf path`, each loaded via
`systemd.LoadCredential` and exported at runtime as `${n}` from
`systemd-creds cat ${n}_FILE`) is resolved entirely at RUNTIME, opaque
to static Nix evaluation in principle, not just in practice — a
structurally new kind of producer this project hasn't modeled at all
(not even `SentinelFlow`, since there's no static rendered value to
search for a sentinel in).

## Summary for this batch

- 10/10 candidates investigated, 0 legitimate skips (every one had at
  least one real, well-formed candidate predicate+test pair worth
  running).
- OBA: 2 real findings (both manually verified genuine), 1 real clean
  PASS + 3 more PASS on kthxbye's extra watches (4 total pass), 12
  inconclusive verdicts across 8 candidates' `enable` watches
  (predominantly Gap A) plus 3 predicate-lowering failures (Gap B ×2
  fields on coturn, empty-collection-literal ×2 independent hits on
  flame/bees).
- Two real, load-bearing, previously-unexercised structural gaps found
  (Gap A: `mkEnableOption` invisible to declaration scanning; Gap B:
  nested-options-block path-prefix mismatch), each independently
  confirmed 2+ times within this one 10-candidate batch — not one-off
  noise.
- One recurring predicate-IR gap independently reconfirmed twice
  (empty list/attrset literal comparison unsupported by
  `ValueExpr::Literal`).
- CDC structural fit: 5 existing-abstraction, 2 new-shape-needed, 2
  not_applicable/not-independently-confirmed.
