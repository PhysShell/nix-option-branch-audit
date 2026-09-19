# E1 holdout audit — batch 2 (candidates 11-20)

Real, unmodified `oba` binary (frozen at `c8e42a1`), run against real,
unmodified NixOS/nixpkgs module.nix/test.nix pairs (tree
`68740713a1d5904edf9ba92a998a522b1b6ce080`), vendored under
`fixtures/e1-holdout-audit/<name>/`, manifest
`targets/e1-holdout-batch2.toml`. Command:
`cargo run --bin oba -- check --root . --targets targets/e1-holdout-batch2.toml --json`.

## Headline finding: `mkEnableOption` is invisible to gate 1, entirely

`scan_options` (src/main.rs) recognizes an option declaration ONLY when
its value expression's call head is literally `mkOption` (bare or
`lib.`-prefixed) — grep for `mkEnableOption` anywhere in `src/main.rs`
returns zero results. `mkEnableOption "..."` is nixpkgs's own canonical,
overwhelmingly most common idiom for a service's `enable` flag. Every
`enable` in this batch, declared this way, comes back `OptionNotFound`
— not because no predicate branches on it (most do, cleanly, via a
top-level `mkIf cfg.enable {...}`), but because gate 1 never sees the
declaration at all.

**Isolated, not inferred**: `monado`'s own module declares
`forceDefaultRuntime`/`defaultRuntime` via longhand `mkOption {...}`
(found, `TestConfigUnresolved` — a real, different, correctly-reached
verdict) right next to `enable`/`highPriority` via `mkEnableOption`
(both `OptionNotFound`) in the exact same `options.services.monado =
{...}` block, 15 lines apart. Confirmed again with a live control:
added `dataDir` (kavita's own `lib.mkOption`-declared option) to
kavita's watch list in the same run — `PredicateNotFound` (found, no
predicate references it, correct), while `enable` in the same file
stayed `OptionNotFound`. Every `OptionNotFound` in this batch traces
cleanly to an `mkEnableOption`-declared option; every option declared
via `mkOption` was found regardless of `lib.` prefix or bareness (via
`inherit (lib) mkOption ...;`).

**Why K1-K5 never caught this**: every prior round's watched options
(`database.socket`, `database.driver`, the various `FlatEnvVars`/
`CliContract`/`EnvContract` targets) are all `mkOption`-declared
config-VALUE options. Nothing in K1-K5 ever watched a service's own
`enable` flag itself — the one option nearly every real NixOS service
declares via `mkEnableOption`. A corpus built entirely around
"interesting config values gated behind `enable`" never had a reason to
watch `enable` itself.

**Consequence for this batch's numbers**: `transmission`/`openRPCPort`
and `matterjs-server`/`openFirewall` were picked specifically because
their real test.nix never touches them (both stay at their `false`
default) — genuine candidate `finding`s. Both come back `OptionNotFound`
instead, for the wrong reason: the tool never gets far enough to ask
"was the branch exercised" at all. Recorded as `inconclusive` below
(gate-1 declaration-shape gap), not `finding` and not `not_applicable`.

Per protocol: not fixed during E1. This is exactly the kind of
"held-together-by-corpus-shape" gap E1 exists to surface, not patch
mid-census.

## Per-candidate results

### kavita
- language/build-system: dotnet (C#), `buildDotnetModule`-family
- interface mechanism: generated config file, JSON (`pkgs.formats.json`,
  `appsettings.json`)
- OBA outcome: **inconclusive** — `enable`: `OptionNotFound`
  (`mkEnableOption` gap, see above). Control: `dataDir`
  (`lib.mkOption`-declared, same file) → `PredicateNotFound` (found,
  correctly no predicate references it).
- CDC structural fit: **new-shape-needed** — nested JSON via
  `settingsFormat.generate`, no existing abstraction handles structured
  JSON config content (`EvaluatedLiteral`/`FlatEnvVars` are both
  flat/string-shaped).
- inconclusive reason: gate-1 declaration-shape gap (`mkEnableOption`
  unrecognized).
- finding manually verified: n/a (no finding reached).
- notes: real predicate is a clean, single top-level `mkIf cfg.enable`
  — would have been a trivial clean PASS (test sets `enable = true`,
  default false) if gate 1 recognized the declaration.

### nebula-lighthouse-service
- language/build-system: Rust (per `pkgs.nebula-lighthouse-service`)
- interface mechanism: generated config file, YAML (`pkgs.formats.yaml`)
- OBA outcome: **inconclusive** — `enable`: `OptionNotFound` (same gap).
- CDC structural fit: **new-shape-needed** — YAML generated config,
  same reasoning as kavita's JSON case.
- inconclusive reason: gate-1 declaration-shape gap.
- finding manually verified: n/a.
- notes: same single top-level `mkIf cfg.enable` shape; would have been
  a clean PASS otherwise (test sets `enable = true`).

### nmtrust
- language/build-system: shell-script consumer (`pkgs.nmtrust`,
  language of the binary itself not determined from the module alone)
- interface mechanism: generated config file, a bespoke bash-array
  `KEY=(...)` format at `/etc/nmtrust/config` — not JSON/YAML/INI/env.
- OBA outcome: **inconclusive** — `enable`: `OptionNotFound` (same gap).
- CDC structural fit: **new-shape-needed** — the most structurally
  novel case in this batch: a hand-rolled bash-array serialization, not
  close to any existing `ProducerEvidence` shape at all.
- inconclusive reason: gate-1 declaration-shape gap.
- finding manually verified: n/a.
- notes: richest module in this batch — real nested `attrsOf(attrsOf
  submodule))` options (`userUnits`) with their own `allowOffline`
  boolean, consumed via `lib.optional unitCfg.allowOffline ...` inside
  a `mapAttrs'`/lambda closure (`unitCfg` is a LAMBDA PARAMETER, not a
  `cfg`-rooted select) — this specific site would hit
  `ResolveFailure::UnsupportedScope("function parameter")` if reached;
  not built as a separate watched-option case (too deep to isolate
  cleanly in this batch), but a real, plausible second inconclusive
  class on this same file, consistent with H2's own documented
  boundary, not run for real here.

### transmission
- language/build-system: C (`transmission-daemon`)
- interface mechanism: generated config file, JSON (`settings.json` in
  `cfg.home`), CLI argv only points at the directory
  (`-g <dir>`), doesn't carry the actual settings.
- OBA outcome: **inconclusive** (both watched options)
  — `openRPCPort`: `OptionNotFound` (mkEnableOption gap — this was
  meant to be a real candidate `finding`: default `false`, real test
  (`fixtures/e1-holdout-audit/transmission/test.nix`) only sets
  `enable = true`, never touches `openPeerPorts`/`openRPCPort`/
  `performanceNetParameters`); `enable`: `OptionNotFound` (same gap).
- CDC structural fit: **new-shape-needed** — JSON settings file, same
  class as kavita's.
- inconclusive reason: gate-1 declaration-shape gap. The REAL,
  interesting question this candidate was chosen for ("does `oba`
  correctly flag a genuinely unexercised `mkIf cfg.openRPCPort` branch
  on unfamiliar code") was never reached.
- finding manually verified: n/a — verified BY HAND instead (reading
  the real module + test directly, not via the tool) that
  `openRPCPort`'s branch (`mkIf cfg.openRPCPort { allowedTCPPorts = [
  cfg.settings.rpc-port ]; }`, a clean direct `cfg`-rooted select,
  default `false` via `mkEnableOption`) is genuinely never exercised
  opposite its default anywhere in the real, current nixpkgs test for
  this service — a real, true positive the tool *would* have needed to
  report as `finding` (OBA001-shaped) had gate 1 recognized the
  declaration. This is the strongest single piece of evidence in this
  batch that the `mkEnableOption` gap has real, not just theoretical,
  cost: a genuine unexercised branch on totally unfamiliar code, missed
  for the wrong reason.
- notes: —

### matterjs-server
- language/build-system: TypeScript/Node (`matterjs-server`,
  Matter.js-based)
- interface mechanism: CLI argv
  (`--storage-path=...--listen-address=...--port=...--production-mode`
  + `cfg.extraArgs`)
- OBA outcome: **inconclusive** (both watched options) —
  `openFirewall`: `OptionNotFound` (declared via `lib.mkEnableOption
  null // { description = ...; }` — the `//`-merge form, same
  underlying gap); `bluetoothSupport`: `OptionNotFound` (bare
  `lib.mkEnableOption "..."`).
- CDC structural fit: **existing-abstraction (`CliContract`)** — clean
  CLI-argv construction via `lib.escapeShellArgs [...]`, the same shape
  K4's `CliContract`/`extract_go_flagset_literal_names`-style extraction
  already models (language differs — Node, not Go — but the CONTRACT
  shape, a flag list built from a package binary + args, is identical).
- inconclusive reason: gate-1 declaration-shape gap.
- finding manually verified: same shape as transmission —
  `openFirewall`'s branch (`networking.firewall = lib.mkIf
  cfg.openFirewall { allowedTCPPorts = [ cfg.port ]; };`, default
  `false`) is genuinely never exercised in the real test (only `enable
  = true` is set). A second real, true-positive-shaped case the tool
  never reached.
- notes: `AmbientCapabilities`/`CapabilityBoundingSet` are gated by
  `lib.optionals cfg.bluetoothSupport [...]` — a real predicate site
  for `bluetoothSupport` too, also never exercised in the real test,
  also unreachable behind the same gap.

### i2pd
- language/build-system: C++ (`i2pd`)
- interface mechanism: generated config file, INI-like
  (`pkgs.formats.ini`-shaped freeform `settings`), similar in spirit to
  grafana's own K5c producer (INI generation), but this project has
  never built anything that PARSES/verifies INI content, only checked
  whether grafana's producer EMITTED certain env vars.
- OBA outcome: **inconclusive** — `enable`: `OptionNotFound` (same gap).
- CDC structural fit: **new-shape-needed** — closest conceptual
  relative is grafana's INI generation, but no existing abstraction
  actually parses/verifies rendered INI *content*.
- inconclusive reason: gate-1 declaration-shape gap.
- finding manually verified: n/a.
- notes: real, separate structural finding independent of the
  `enable` gap — `cfg.settings.meshnets.yggdrasil or false` (an `or`-
  default expression over a FREEFORM settings key, not a formally
  `mkOption`-declared one) would fail at gate 1 for an entirely
  different reason even if reached (freeform keys have no `Declaration`
  at all; separately, `lower_pred`/`lower_value_expr` have no handling
  for Nix's `a.b or c` node shape at all, distinct from the `mkIf`
  wrapper syntax) — not exercised for real this round (moot once gate 1
  already fails upstream on `enable`), but a second, real, novel
  structural pattern worth naming for a future round.

### ferretdb
- language/build-system: Go (`ferretdb`)
- interface mechanism: **env-vars** — `environment = cfg.settings;` on
  the systemd unit, `cfg.settings` a submodule with declared enum/string
  options (`FERRETDB_HANDLER`, `FERRETDB_TELEMETRY`) plus a freeform
  `attrsOf str` fallback, including a real DSN-shaped default:
  `FERRETDB_POSTGRESQL_URL = "postgres://ferretdb@localhost/ferretdb?host=/run/postgresql"`.
- OBA outcome: **inconclusive** — `enable`: `OptionNotFound` (same
  gap).
- CDC structural fit: **existing-abstraction (`ProducerEvidence::FlatEnvVars`)**
  — a clean, near-perfect structural match: this is EXACTLY agorakit/
  movim/snipe-it's own K2d shape (`environment = cfg.settings;` on a
  systemd unit), independently rediscovered on a totally unrelated,
  unfamiliar Go/MongoDB-alternative service. The strongest single
  "adapter reuse" data point in this batch.
- inconclusive reason: gate-1 declaration-shape gap.
- finding manually verified: n/a.
- notes: real test.nix is a SECOND, independently interesting
  structural novelty — `{ runTest, pkgs }: { postgresql = runTest
  {...}; sqlite = runTest {...}; }`, a multi-named-subtest file wrapped
  through a `runTest` helper function, NOT the single top-level `{
  name = ...; nodes.machine = ...; testScript = ...; }` shape every
  prior fixture in this project (K1-K5, and every other candidate in
  this batch) has ever used. `services.ferretdb.enable = true;` sits
  three levels deep inside a function-call argument
  (`postgresql = runTest { ...; nodes.machine = { ...; services.ferretdb.enable = true; }; };`),
  not at a file-top-level `nodes.machine`. Whether
  `scan_test_assignments`'s walker would even find this assignment was
  never actually exercised this round — `enable` already failed at
  gate 1 before the test-side walker's own behavior on this shape could
  be observed. Flagged explicitly as an OPEN QUESTION for a future
  round (not resolved here): does the test-assignment walker require a
  literal top-level `nodes` position, or does it walk into arbitrary
  function-call arguments? This project's own `--census` mode exists
  specifically to answer exactly this kind of "did the walker see this
  shape at all" question and would be the right tool to point at it.

### ringboard
- language/build-system: Rust (`ringboard`)
- interface mechanism: no external consumer contract in the CDC sense
  (a clipboard manager; picks between two local binaries by desktop
  environment, no config file/env-vars/DSN)
- OBA outcome: **inconclusive** — `x11.enable`/`wayland.enable`: both
  `OptionNotFound` (both `lib.mkEnableOption`-declared, same gap).
- CDC structural fit: **not_applicable** — no producer/consumer
  boundary this project's abstractions target at all.
- inconclusive reason: gate-1 declaration-shape gap.
- finding manually verified: n/a.
- notes: real predicate is `lib.mkIf (cfg.x11.enable ||
  cfg.wayland.enable) {...}` — a genuine `Or(...)` compound over two
  nested (non-wildcarded) submodule options, exactly the shape H2's
  `Pred::Or` was built for. Real test sets `x11.enable = true` only
  (`wayland.enable` stays at its `false` default) — would have been a
  clean, real `Or`-compound witness (the FIRST real, non-synthetic `Or`
  case this project has ever seen — K1-K5's only real compound was
  davis's `And`) had gate 1 recognized the declaration. A second
  separate real predicate, `cfg.x11.enable && cfg.wayland.enable`
  (an `And`, used inside a bare `if...then...else` VALUE expression for
  `script`, not wrapped in `mkIf`/`optional`), was also present but not
  investigated further — whether `scan_predicates`/`scan_resolved_predicates`
  even recognize a bare `if...then...else` NOT wrapped in one of the
  known helper calls as a "predicate site" at all is a second open
  question this candidate raises but doesn't resolve.

### monado
- language/build-system: C/C++ (`monado`, OpenXR runtime)
- interface mechanism: writes a fixed-path JSON file
  (`active_runtime.json`) whose content isn't itself a variable
  producer contract in the CDC sense.
- OBA outcome: **mixed** — `enable`/`highPriority`: `OptionNotFound`
  (`mkEnableOption`/`mkEnableOption // mkOption` gap); `forceDefaultRuntime`:
  **inconclusive, `TestConfigUnresolved`** — a REAL, correctly-reached
  different verdict: gate 1 found the declaration (longhand
  `mkOption`), gate 2 found the real predicate (`mkIf
  cfg.forceDefaultRuntime`, default `false`), but the real test never
  sets `services.monado.forceDefaultRuntime` anywhere the walker can
  see — the test's `nodes.machine` imports `./common/openxr.nix` (a
  shared NixOS test helper this project doesn't vendor), and the
  opacity gate correctly refuses to certify a negative conclusion when
  part of the test config lives behind an unresolved import.
- CDC structural fit: **not_applicable**.
- inconclusive reason: `enable`/`highPriority`: gate-1 declaration gap.
  `forceDefaultRuntime`: real import-opacity (`imports =
  [./common/openxr.nix]` in the test's own node config) — correctly
  NOT treated as a false PASS or a false OBA001.
- finding manually verified: n/a.
- notes: the cleanest real confirmation in this batch that the opacity
  gate (K2e/H1-era design) works correctly on real, unfamiliar code —
  a genuine shared-test-helper import, not a synthetic fixture.

### endlessh
- language/build-system: C (`endlessh`, SSH tarpit)
- interface mechanism: **CLI argv** — `ExecStart` built via `with cfg;
  lib.concatStringsSep " " [ "endlessh" "-p ${toString port}" ... ]`.
- OBA outcome: **mixed** — `enable`: `OptionNotFound` (mkEnableOption
  gap); `openFirewall`: **inconclusive, `PredicateNotFound`** — a REAL,
  differently-reached verdict: `openFirewall` IS declared via longhand
  `lib.mkOption` (found at gate 1, confirmed by the different verdict
  kind), but its one real consuming predicate,
  `networking.firewall.allowedTCPPorts = with cfg; lib.optionals
  openFirewall [ port ];`, resolves `openFirewall` as a bare identifier
  reached only through a `with cfg;` scope — `resolve_ident_binding`'s
  documented, deliberate `UnsupportedScope("with")` fires, so neither
  H1 nor H2 ever attaches a predicate referencing this path, and
  `run_target` correctly falls through to `PredicateNotFound` (no
  attempt at all, not a wrongly-confident answer).
- CDC structural fit: **existing-abstraction (`CliContract`)** — clean
  CLI-flag construction, same shape as matterjs-server's.
- inconclusive reason: `enable`: gate-1 declaration gap.
  `openFirewall`: documented `with`-scope limitation, confirmed for the
  first time on real code (every prior confirmation of this exact
  `ResolveFailure::UnsupportedScope("with")` variant was a synthetic
  H2 test fixture, not a real nixpkgs module).
- finding manually verified: n/a — `openFirewall` genuinely IS exercised
  in the real test (`openFirewall = true;` is set), so this was never a
  candidate `finding` regardless; it's a clean confirmation of the
  `with`-scope gap alone, isolated from the separate `enable` gap this
  same file also has.
- notes: —

## Batch 2 totals

- 10/10 candidates had real, analyzable predicate structure (0
  legitimate `not_applicable` skips for lack of any branching at all —
  every single module in this batch had at least one real `mkIf`/
  `optional`-family predicate).
- OBA outcome: 0 `supported`/clean PASS, 0 `finding`, **10/10
  `inconclusive`** — but NOT 10 uniform reasons: 8 candidates hit ONLY
  the `mkEnableOption` gate-1 gap; `monado` and `endlessh` each hit ONE
  real, DIFFERENT, correctly-reached inconclusive class instead/in
  addition (import opacity; `with`-scope) that the `mkEnableOption` gap
  never got in the way of, because those two candidates' relevant
  predicates happened to be declared via longhand `mkOption`.
- 2 candidates (`transmission`, `matterjs-server`) were hand-verified
  (reading real source directly, not via the tool) to contain a
  genuine unexercised branch the tool *should* have reported as
  `finding` — real, true-positive-shaped cases lost entirely to the
  `mkEnableOption` gap.
- CDC structural fit: 1 clean existing-abstraction match on `FlatEnvVars`
  (ferretdb, plus a real DSN inside it), 2 clean existing-abstraction
  matches on `CliContract` (matterjs-server, endlessh), 5 new-shape-
  needed (kavita/JSON, nebula-lighthouse-service/YAML, nmtrust/bash-
  array, transmission/JSON, i2pd/INI-freeform), 2 not_applicable
  (ringboard, monado).
