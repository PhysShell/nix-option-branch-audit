# E1 holdout audit -- batch 1 (candidates 1-10)

Ten real `NixOS/nixpkgs` service modules, drawn per the pre-registered
order in `protocol.md`, investigated against the frozen binary
(`c8e42a1`, unmodified through this whole batch -- `git log --oneline
-1` was checked before and after; only files under
`fixtures/e1-holdout-audit/` and `targets/e1-holdout-batch1.toml` were
added). Manifest: `targets/e1-holdout-batch1.toml`. Real run:
`cargo run --bin oba -- check --root . --targets
targets/e1-holdout-batch1.toml --json`.

**Headline result: 3 PASS / 0 finding / 7 inconclusive (all
`OptionNotFound`, gate 1) -- and of the 3 PASS, one (`xandikos`) is
proven, by direct inspection of its own declaration span, to be the
right verdict reached through a WRONG mechanism: a real, demonstrated
false-positive-capable bug in the declaration scanner, not a coverage
gap.** This is a materially more serious finding than "the tool doesn't
recognize X yet" -- see "Cross-candidate findings" below, which is the
important part of this batch; the per-candidate table is supporting
detail.

## Per-candidate results

| name | mechanism | OBA outcome | verdict | CDC structural fit | notes |
|---|---|---|---|---|---|
| libinput | generated X11 config-block string (`environment.etc`) | inconclusive | `OptionNotFound` on `enable` | new-shape-needed (X11 config-string templating; no existing abstraction fits) | zero options discovered at all -- see Bug C/mouse-touchpad below |
| cadvisor | CLI argv (systemd `ExecStart` flags) | inconclusive | `OptionNotFound` on `enable` | existing-abstraction: `CliContract` (K4a) | `enable` (mkEnableOption) never discovered; other options ARE discovered but path-doubled (Bug A) |
| send | env vars (`environment`/`environmentFile`) + CLI | inconclusive | `OptionNotFound` on `enable` | existing-abstraction: `EnvContract`-ish, imperfect (an arbitrary user-defined attrset of env vars, not a fixed flat set) | same Bug A + Bug B combo as cadvisor |
| privoxy | generated multi-key config file (`settings = {...}` -> `-c <path>`) | **supported** | **PASS** on `inspectHttps` | new-shape-needed (generated settings-file contract; no existing K1-K5 abstraction models "many keys in one generated file") | genuinely clean pass -- flat-dotted `options.services.privoxy = {...}`, real `mkOption`, real predicate, real test evidence, real IR (`Eq` + the direct-H1 predicate both witnessed) |
| esphome | CLI argv + env (`EnvironmentFile`) | **supported** | **PASS** on `openFirewall` | new-shape-needed (mixed CLI+env, no single existing abstraction covers a *combined* boundary) | genuinely clean pass; real `And(Eq,Not(Eq))` compound IR witnessed across a 2-node test, same shape class as davis's own `mysqlLocal` |
| xandikos | CLI argv (`extraOptions`) | **supported, but see below** | **PASS on `enable` -- via the WRONG declaration** | existing-abstraction: `CliContract` | **false-positive-capable bug, proven -- see "Cross-candidate findings"** |
| nohang | config-file selection (`configPath` enum) | inconclusive | `OptionNotFound` on `enable` | new-shape-needed | only 1 of ~4 real options discovered at all (`configPath`, the one declared via raw `mkOption`); `enable`(mkEnableOption)/`package`(mkPackageOption) invisible |
| svnserve | CLI argv (`svnserve` daemon flags) | inconclusive | `OptionNotFound` on `enable` | existing-abstraction: `CliContract` | textbook Bug A: `enable` IS a real `mkOption`, discovered with path `["services","svnserve","enable"]` instead of `["enable"]` |
| jitsi-meet | multi-consumer, generated config files (prosody/jicofo/jigasi) + CLI | inconclusive | `OptionNotFound` on `jigasi.enable` | new-shape-needed, AND multi-consumer (same complexity class this project already parked for `flarum`) | **zero options discovered at all** -- `with types; { ... }` wrapper makes the whole block invisible (Bug E) |
| misskey | generated settings file (YAML-ish) + CLI | inconclusive | `OptionNotFound` on `database.createLocally` | new-shape-needed (same generated-settings-file family as privoxy) | dozens of options discovered, but scattered across ~5 colliding submodule namespaces (`host`/`port`/`db` each appear 2-4x un-prefixed) -- Bug F at real scale, not just my synthetic reproducer |

## Cross-candidate findings: FIVE-SIX distinct, real gaps in `scan_options`

Not one bug, several independent ones, each confirmed against real
source (file + line), not inferred:

**Bug A -- path-doubling on the generic `options = { services.<name> =
{...}; };` idiom.** When a module writes `options = { services.foo =
{...}; };` (one bare `options` attrpath whose value nests the service
name inside), `scan_options`'s own doc comment says this "nested" form
is walked as if already option_prefix-relative (the shape kimai's
`siteOpts` submodule actually has) -- but for a TOP-LEVEL module using
this idiom, the recursive walker's path accumulation genuinely does
include `services`+`<name>` as real segments, producing e.g.
`["services","svnserve","enable"]` instead of the option_prefix-
relative `["enable"]` `run_target` actually looks up. Confirmed
directly: `svnserve`'s real, correctly-`mkOption`-declared `enable`
shows up in `discovered_options` at exactly that wrong, over-qualified
path (see table). This idiom (`options = { services.X = {...}; }`, not
the flat-dotted `options.services.X = {...}` form K1-K5's own corpus
happened to use for kimai/davis/agorakit/etc.) is at least as common
across real nixpkgs as flat-dotted -- 4 of these 10 holdout candidates
use it (libinput, cadvisor, send, svnserve).

**Bug B -- `mkEnableOption` is never recognized as an option
declaration at all.** `is_mk_option_call` matches only a literal
`mkOption` call by name. `enable = lib.mkEnableOption "description";` --
the single most common way a NixOS module declares its own `enable`
toggle -- is a call to a DIFFERENT helper and is silently skipped by
`walk_options_block` (neither the `is_mk_option_call` branch nor the
`NODE_ATTR_SET` recursion branch matches an `mkEnableOption` call node).
Confirmed on libinput, cadvisor, send, nohang (all real `enable =
mkEnableOption ...;` declarations, all absent from `discovered_options`).
K1-K5's own corpus apparently never needed to watch a bare
`mkEnableOption`-declared toggle directly -- their watched options were
always deeper, `mkOption`-declared config values.

**Bug C -- an `mkEnableOption ... // { default = ...; }` override merge
is invisible too**, a step beyond Bug B: libinput's real `enable = lib.
mkEnableOption "libinput" // { default = config.services.xserver.enable;
... };` is a `//`-merge binary-op node, matching neither
`is_mk_option_call` (not an `mkOption` call) nor `NODE_ATTR_SET` (not a
plain attrset either) -- silently skipped, on top of Bug B.

**Bug D -- `mkPackageOption` is never recognized either.** Same root
cause as Bug B (name-checked against `mkOption` only), affecting a
second extremely common nixpkgs helper. Confirmed: nohang's real
`package = mkPackageOption pkgs "nohang" { };` is absent from
`discovered_options`.

**Bug E -- `with <namespace>; { ... }` around an options block makes
the WHOLE block invisible**, not just individual entries. `scan_options`
requires the attrpath-value's direct child to be `NODE_ATTR_SET`
(`if value.kind() != NODE_ATTR_SET { continue; }`); jitsi-meet's real
`options.services.jitsi-meet = with types; { ... };` has a `NODE_WITH`
wrapper as that child instead, so the entire real, well-formed options
block (dozens of real declared options) contributes ZERO entries to
`discovered_options`. Confirmed: jitsi-meet's `discovered_options` is
empty despite ~30+ real `mkOption` calls in the file.

**Bug F -- the most serious one, false-positive-CAPABLE, not just a
coverage gap: nested submodule `options = {...}` blocks are walked with
a completely fresh, empty path, discarding all containing-submodule
context.** Any `type = lib.types.submodule { options = {...}; };`
anywhere in the file is found independently by `scan_options`'s own
`root.descendants()` walk (this is deliberate and documented -- it's
how kimai's real `siteOpts` submodule gets scanned at all) and walked
starting from an EMPTY path, exactly the same way the module's own
top-level block is. If a nested submodule happens to declare an option
with the SAME leaf name as something in the top-level scope (`enable`,
`host`, `port` are all extremely common names), the two get silently
conflated into one `OptionDecl` entry under that bare name, and
whichever one `root.descendants()` happens to visit is the one that
wins.

Proven two ways:
1. **In the wild**: `xandikos`'s `discovered_options` entry for `enable`
   is at module line 68 -- inside `nginx = mkOption { ...; type =
   types.submodule { options = { enable = mkOption { default = false;
   ...}; ...}; }; };`, i.e. the unrelated `services.xandikos.nginx.enable`
   reverse-proxy toggle, NOT `services.xandikos.enable` itself (the
   real one, declared via `mkEnableOption` at line 17 -- itself also
   invisible per Bug B). The real run's `PASS` verdict for `xandikos`
   on `enable` is backed entirely by this wrong declaration. It happens
   to be numerically harmless here only because `nginx.enable`'s
   default (`false`) coincides with what `services.xandikos.enable`'s
   own real default effectively is too.
2. **Isolated, minimal, real reproducer** (`fixtures/e1-holdout-audit/_bisect/`,
   `targets/e1-bisect.toml`, both vendored in this batch as evidence,
   not counted as one of the 10 holdout draws): a synthetic module with
   a top-level `enable = lib.mkEnableOption "bisect probe";` (real
   default `false`) gated by a real `lib.mkIf cfg.enable {...}`, PLUS
   an unrelated nested `nested.enable` submodule option whose own
   default is deliberately the OPPOSITE, `true`. Real run: `oba check`
   reports `OBA001` (a FINDING -- "never witnessed opposite its
   default") for a case that is actually a clean, real, opposite-of-
   default PASS. `discovered_options` shows the matched `enable` decl
   at line 14 (`default = true`, the nested one), not line 5 (the real
   top-level `mkEnableOption`, which per Bug B wasn't even a candidate
   match). This is a demonstrated FALSE FINDING, produced purely by
   declaration-scope conflation, not a hypothetical.

Also visible at real scale in `misskey`, whose module nests ~5 separate
submodules (`database`/`redis`/`meilisearch`/`settings.{db,redis,...}`)
each contributing their own colliding `host`/`port`/`db`/`user`/`pass`
entries into one flat, unscoped bucket.

## Metrics contribution from this batch (10 of the ~25-30 target total)

- coverage: 3/10 (all three via a real predicate+evidence match; one of
  the three, `xandikos`, is right-answer-wrong-mechanism per Bug F)
- precision of findings: n/a, 0 findings this batch (the bisect
  reproducer is a synthetic diagnostic, not a holdout candidate, and
  IS a real false finding -- not counted in the 10-candidate corpus
  metrics, called out separately as a bug)
- inconclusive rate: 7/10, entirely gate-1 (`OptionNotFound`), zero
  `PredicateNotFound`/`ResolveFailure`-shaped inconclusives this batch
  -- H2's alias/predicate IR itself was never even reached for 7 of 10
  candidates; the declaration scanner is the bottleneck for this batch,
  not the predicate/alias layer.
- adapter reuse (CDC, structural judgment): `CliContract` fits 3-4
  (cadvisor, xandikos, svnserve, partially esphome) cleanly.
- special-case pressure: a recurring pattern, not covered by ANY
  existing CDC abstraction, shows up 4/10 times (privoxy, nohang,
  jitsi-meet, misskey): a generated, multi-key settings FILE (INI/YAML/
  JSON-ish), structurally different from `EvaluatedLiteral` (one DSN
  literal), `FlatEnvVars` (a flat env-var set), or `CliContract` (flag
  names) -- worth naming explicitly as a candidate new abstraction if
  this pattern keeps recurring in later batches.
