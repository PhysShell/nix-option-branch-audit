# E1 holdout audit — batch 4 (candidates 31-40)

All 10 candidates were analyzable (no legitimate skips this batch — a
genuinely diverse real set: a fediverse server, a secrets manager, a
gopher server, an archive-extraction daemon, a network-controller UI, a
desktop session manager, a DNS resolver, a federation platform, an
auth reverse proxy, a game-server panel).

**Headline result: 9 of 10 candidates came back `OptionNotFound` for
EVERY watched option, despite every predicate side (H1/H2) correctly
finding and lowering the real branch condition.** This is not "the
census found nothing" — root-caused precisely, by isolating each case
against the real, frozen, unmodified binary, this batch surfaced THREE
separate, previously-unexercised structural gaps in `scan_options`
(the declaration scanner, gate 1), none of which K1-K5's own corpus
ever exercised. Full mechanism below, confirmed by direct inspection
of `src/main.rs` (read-only — nothing in `src/` was touched).

## Root causes found (not fixed — this is E1, the freeze holds)

**Bug A — `mkEnableOption` is never recognized as an option
declaration at all.** `is_mk_option_call` (`src/main.rs:732-738`) only
matches a call literally headed `mkOption`. `mkEnableOption "..."` —
nixpkgs's own standard helper for a boolean enable-toggle, and the
single most common way real modules declare `enable` — is a different
function call entirely; `walk_options_block` silently skips it (it's
a `NODE_APPLY`, not a `NODE_ATTR_SET`, so it doesn't even recurse), and
the option never appears in `discovered_options`. K1-K5's own corpus
(kimai/davis/mimir/grafana/...) never once watched an
`mkEnableOption`-declared option — every prior watched option was a
hand-written `mkOption {...}` (a DSN socket path, a driver string, an
env var). Confirmed by direct source inspection: EVERY `enable` in this
batch (akkoma, vault, spacecookie, unpackerr, omada, retroarch, unbound,
mobilizon, pomerium — 9 of 10) is `mkEnableOption`, not `mkOption`.
`pufferpanel` is the one candidate that happens to spell `enable`'s
declaration out as a literal `lib.mkOption { type = lib.types.bool;
default = false; ... };` instead — which is why it's the one candidate
in this whole batch that worked.

**Bug B — a plain top-level `options = { services.X = {...}; };` block
loses the option_prefix boundary, so a real option ends up stored at
its full absolute path instead of a prefix-relative one.**
`scan_options` (`src/main.rs:639-684`) has two matched shapes: (1) the
literal `segs == ["options"]` case, which walks the WHOLE tree under
`options = {...}` with NO reset when it descends through nested
attrpaths (`services` -> `vault` -> `dev` all just keep extending one
shared `path` accumulator, `src/main.rs:708`), producing e.g.
`["services","vault","dev"]`; (2) the single-dotted
`options.services.<option_prefix...> = {...}` case (`src/main.rs:671-681`),
which resets `path` to empty AT the option_prefix boundary, producing
the correct, prefix-relative `["dev"]`. `run_target`'s own gate-1
lookup computes `watched_path` from the manifest's dotted `watch`
string ALONE (`watched.split('.')`, never prepended with
`option_prefix`) — so it only ever matches shape (2)'s relative paths.
Any option declared under shape (1) — a plain `options = {...};` block,
by inspection THE single most common real nixpkgs idiom (8 of 10
candidates this batch: akkoma, vault, spacecookie, unpackerr, omada,
unbound, mobilizon all use it at some level) — is real, present,
correctly typed, and STILL reports `OptionNotFound`, because its
discovered path carries the option_prefix twice. Confirmed directly:
isolating `vault`'s `dev` (a plain `mkOption`, not `mkEnableOption`)
shows it correctly appears in `discovered_options` as
`["services","vault","dev"]` — never matchable against
`watched_path=["dev"]`.

**Bug C — `resolve_cfg_root`'s gate-1 safety check can't see a `cfg`
bound inside `config`'s own value, so the WHOLE options block is
skipped, not just one option.** `pomerium` computes `cfg` via
`config = let cfg = config.services.pomerium; cfgFile = ...; in mkIf
cfg.enable {...};` — i.e. `cfg` is scoped to `config`'s OWN expression,
a sibling attribute, never an ancestor of the `options.services.pomerium
= {...}` node. `resolve_cfg_root` walks ancestors from the options
node's own tree position (`src/main.rs:1094-1101`) and can never reach
that binding, so it returns `Err`, the gate-1 safety condition at
`src/main.rs:676-678` fails, and `walk_options_block` is never invoked
at all — confirmed: `discovered_options` is completely EMPTY for
pomerium, not just missing `enable`. This is a real, different failure
mode from Bug A/B (it's not "one option was missed", it's "the whole
declaration block was never scanned").

None of these three bugs overlap with each other's root cause, and
none were reachable by K1-K5's own corpus (which happened to only use
the doubly-nested submodule-reset shape for its watched leaves, or the
single-dotted `options.services.X = {...}` shape combined with an
explicit `mkOption`, never `mkEnableOption`, and never a `cfg` computed
inside `config`'s own value).

## Per-candidate results

### akkoma
- language/build-system: Elixir (Pleroma/Akkoma fork), fediverse server
- interface mechanism: generated Elixir config file (`pkgs.formats.elixirConf`)
- OBA outcome: **inconclusive** — `enable` never discovered at all (Bug A: `mkEnableOption`)
- CDC structural fit: existing-abstraction-shaped (EvaluatedLiteral: a rendered config file) but a genuinely NEW format family (Elixir config syntax, never seen in K1-K5) — judgment call
- inconclusive reason: Bug A
- finding manually verified: n/a (no finding)
- notes: real predicate (`mkIf cfg.enable`) correctly found/lowered by H2; only the declaration side failed.

### vault
- language/build-system: Go, secrets manager
- interface mechanism: generated HCL config file (`pkgs.writeText`)
- OBA outcome: **inconclusive** for both watched options — `enable` (Bug A), `dev` (Bug B, confirmed by direct isolation: correctly discovered at `["services","vault","dev"]`, never matched)
- CDC structural fit: existing-abstraction (EvaluatedLiteral, HCL-rendered config file)
- inconclusive reason: Bug A (`enable`), Bug B (`dev`)
- finding manually verified: n/a
- notes: also has a genuinely unresolved real predicate,
  `lib.optional (config.services.consul.enable && cfg.storageBackend ==
  "consul") "consul.service"` — correctly flagged
  `UnsupportedScope("function parameter")` (the module argument `config`
  isn't a lexical alias `resolve_ident_binding` can trace) rather than
  silently dropped; a real, honest H2 boundary working as designed.

### spacecookie
- language/build-system: Haskell, gopher server
- interface mechanism: generated JSON config file (`pkgs.formats.json`)
- OBA outcome: **inconclusive** for both — `enable` (Bug A), `openFirewall` (Bug B)
- CDC structural fit: existing-abstraction (EvaluatedLiteral, JSON-rendered config file, a format never seen in K1-K5)
- inconclusive reason: Bug A, Bug B
- finding manually verified: n/a
- notes: test.nix assigns both `enable=true` and `openFirewall=true`
  (both real, provable transitions from their `false` defaults) — the
  evidence a working scanner would need is genuinely present in the
  real test; the tool just never got to see it.

### unpackerr
- language/build-system: Go, archive-extraction daemon
- interface mechanism: generated TOML config file (`pkgs.formats.toml`)
- OBA outcome: **inconclusive** for all three — `enable` (Bug A), `group`/`user` (Bug B)
- CDC structural fit: existing-abstraction (EvaluatedLiteral, TOML-rendered config file)
- inconclusive reason: Bug A, Bug B
- finding manually verified: n/a
- notes: real, novel-to-this-project H2 shape present in the source —
  `mkIf (cfg.group == "unpackerr")` / `mkIf (cfg.user == "unpackerr")`,
  a real `Eq` predicate over a STRING option with a non-null default
  ("unpackerr"), different from every prior K1-K5 `Eq` case (all were
  either null-checks or davis's own boolean-And). The real test flips
  `group` away from its default ("users" vs "unpackerr") — exactly the
  kind of transition this predicate shape should be able to witness —
  but never gets the chance, because gate 1 fails first (Bug B).

### omada
- language/build-system: Java, network-controller web UI
- interface mechanism: no generated config file at all — the controller reads its own state from a data directory; Nix only sets `ExecStart`/systemd unit properties. Not config-drift-relevant in the CDC sense.
- OBA outcome: **inconclusive** for both — `enable` (Bug A), `openFirewallWebPorts` (Bug B)
- CDC structural fit: not_applicable — no producer/consumer config boundary of the kind CDC's abstractions model at all
- inconclusive reason: Bug A, Bug B
- finding manually verified: n/a

### retroarch
- language/build-system: C (RetroArch), X11 desktop session
- interface mechanism: CLI argv only (`retroarch -f <extraArgs>`), user-supplied passthrough, no config-drift-relevant contract
- OBA outcome: **inconclusive** — `enable` (Bug A only; this module uses the single-dotted `options.services.xserver.desktopManager.retroarch = {...}` shape, so Bug B does NOT apply here — confirmed by isolation: exactly 1 option discovered, `extraArgs`, the only OTHER option that's a real `mkOption`)
- CDC structural fit: CliContract-shaped in principle (existing abstraction) but the "contract" here is opaque user passthrough, not a real flag set worth diffing
- inconclusive reason: Bug A
- finding manually verified: n/a
- notes: the cleanest isolation of Bug A alone in this batch — proves
  Bug A and Bug B are genuinely independent failure modes, not the same
  bug observed twice.

### unbound
- language/build-system: C, DNS resolver
- interface mechanism: generated config file via a hand-rolled Nix serializer (`toConf`, not a `pkgs.formats.*` generator) — structurally a config-file shape, but the renderer is bespoke module code, not a reused library
- OBA outcome: **inconclusive** for both — `enable` (Bug A), `enableRootTrustAnchor` (Bug B, confirmed by isolation: correctly discovered at `["services","unbound","enableRootTrustAnchor"]`)
- CDC structural fit: existing-abstraction-shaped (EvaluatedLiteral) but the custom serializer is a genuinely new sub-case (no `pkgs.formats.*` involved at all)
- inconclusive reason: Bug A, Bug B
- finding manually verified: n/a
- notes: the most structurally interesting real test in this batch —
  `services.unbound.enableRootTrustAnchor = false;` is set through a
  SHARED, separately-defined module (`common = { lib, pkgs, ... }: {
  config = {...}; };`) imported via `imports = [ common ];` into three
  different nodes, not assigned inline. Whether OBA's test-config walker
  can see an assignment reached through an imported module function
  (rather than inline in the node's own attrset) is exactly the kind of
  question H1's own gate-5 opacity check exists for — never actually
  reached in this run, since gate 1 failed first. Worth a targeted
  follow-up once Bug A/B are addressed (not now — E1 doesn't fix).

### mobilizon
- language/build-system: Elixir/Phoenix, federated events platform
- interface mechanism: generated Elixir/Phoenix nested-settings config (colon-prefixed atom-style keys, e.g. `":mobilizon"."` `:instance"`)
- OBA outcome: **inconclusive** — `enable` (Bug A)
- CDC structural fit: existing-abstraction-shaped (EvaluatedLiteral) but a genuinely novel key-naming convention (colon-atom keys) never seen in K1-K5
- inconclusive reason: Bug A
- finding manually verified: n/a

### pomerium
- language/build-system: Go, authenticating reverse proxy
- interface mechanism: generated YAML config (`pkgs.formats.yaml`) or a user `configFile` override, plus a `secretsFile` (`EnvironmentFile`)
- OBA outcome: **inconclusive** — BOTH watched options, but via Bug C specifically (the whole declaration block is skipped, not just `enable`) — confirmed: `discovered_options` is empty for this module, not merely missing one entry
- CDC structural fit: existing-abstraction, and a close structural match to K5's own real `grafana` case (a YAML/INI-generated config file via a `pkgs.formats.*` helper) — the best "adapter reuse" data point in this batch
- inconclusive reason: Bug C
- finding manually verified: n/a
- notes: `cfg` is computed inside `config`'s own `let`, not at the
  module's top level (`config = let cfg = config.services.pomerium; in
  mkIf cfg.enable {...};`) — a real, different, and arguably even more
  common real-world idiom than the ones K1-K5's corpus used.

### pufferpanel
- language/build-system: Go, game-server management panel
- interface mechanism: **flat env vars** (`environment = cfg.environment;`, directly passed through to the systemd unit's `Environment=`) — a clean, exact structural match for the existing `ProducerEvidence::FlatEnvVars` shape (same as agorakit/movim/snipe-it in K2d)
- OBA outcome: **supported — real PASS.** `enable` (`mkOption`, not `mkEnableOption`; single-dotted `options.services.pufferpanel = {...}` shape) correctly transitions `false -> true`, real evidence from `fixtures/e1-holdout-audit/pufferpanel/test.nix:11`, `witnessed: true`
- CDC structural fit: existing-abstraction (FlatEnvVars) — exact reuse, no new shape needed
- inconclusive reason: n/a
- finding manually verified: n/a (PASS, not a finding — but manually re-confirmed anyway: `services.pufferpanel.enable = true;` really is the only assignment in the real test, and the module's default really is `false`; a clean, unambiguous, correctly-classified real transition on a completely unfamiliar app)
- notes: the ONE candidate in this batch that hits none of Bug A/B/C —
  the control case proving the pipeline still works end-to-end when the
  declaration shape happens to match what K1-K5's corpus already
  exercised.

## Batch 4 summary

- 10/10 analyzable (0 legitimate skips)
- OBA: 1 supported (PASS) / 9 inconclusive / 0 not_applicable(OBA-side; omada's CDC-side was flagged not_applicable, its OBA-side was still `inconclusive`) / 0 findings
- Every inconclusive is attributable to one of 3 precisely identified,
  previously-unexercised `scan_options` gaps (Bug A/B/C) — NOT a vague
  "the tool gave up", and NOT randomness across candidates (the same 3
  causes recur predictably by declaration shape)
- CDC structural fit: 8 existing-abstraction-shaped (mostly
  EvaluatedLiteral/config-file, one exact FlatEnvVars match), 1
  not_applicable (omada), 1 CliContract-shaped-but-low-value (retroarch)
  — 0 candidates in this batch would need a genuinely NEW CDC
  abstraction *category*, though several use config FORMATS (Elixir,
  TOML, JSON, a hand-rolled serializer) this project has never actually
  vendored/parsed before
