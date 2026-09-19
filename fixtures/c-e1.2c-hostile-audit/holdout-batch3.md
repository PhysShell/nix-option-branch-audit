# C-E1.2c fresh holdout batch 3: clatd, redmine, nats, gancio

Population/draw: `fixtures/c-e1.2c-hostile-audit/holdout-draw.md`. All
four are genuinely fresh -- never touched by E1, C-E1.1, or C-E1.2a/b.
Same A/B/C/D executable-fit criterion as C-E1.1's own protocol
(`fixtures/c-e1.1-generated-config-audit/protocol.md`), same reason-code
vocabulary, no new vocabulary invented. All A/B evidence via real `nix
eval --impure` against `PhysShell/nixpkgs` tree
`68740713a1d5904edf9ba92a998a522b1b6ce080` (the same pin this whole
project uses). All C/D evidence from each consumer's real, exact pinned
upstream source, fetched fresh this round. No implementation --
research only, per the protocol's own explicit instruction.

## clatd

```
candidate: clatd
A_producer: proved
  evidence: nixos/modules/services/networking/clatd.nix:10-12 --
  `settingsFormat = pkgs.formats.keyValue {};` `configFile =
  settingsFormat.generate "clatd.conf" cfg.settings;`. Real `nix eval`
  (`services.clatd.enable=true; settings.plat-prefix="64:ff9b::/96";`)
  confirms the real rendered content: `plat-prefix=64:ff9b::/96\n`.
B_binding: proved
  evidence: real `ExecStart` = `/nix/store/nn0f1v9l3kj50x2l8iw18kznpcx40s0k-clatd-2.1.0/bin/clatd
  -c /nix/store/1v2rciihqfkpd0igc6fjpl7civycmj7y-clatd.conf` -- the
  exact same derivation A read, behind a named `-c` flag (DirectArgv
  shape, matching unpackerr's own already-shipped precedent exactly).
C_consumer: proved
  evidence: real pinned tag v2.1.0 (`fetchFromGitHub { owner =
  "toreanderson"; repo = "clatd"; rev = "v2.1.0"; }`, confirmed from the
  real package.nix). Real source, `clatd` (the main Perl script),
  lines 765-775: `elsif($ARGV[$i] eq "-c") { ... readconf($ARGV[$i+1]);
  ... }` -- an exact match to B's own real `-c` flag.
D_contract: proved, and unusually clean
  evidence: `clatd:35-64` pre-populates a `%CFG` hash with EVERY real
  accepted key as an explicit hash key with its own default value (19
  real keys: `quiet`, `debug`, `script-up`, `script-down`, `clat-dev`,
  `clat-v4-addr`, `clat-v6-addr`, `dns64-servers`, `cmd-ip`,
  `cmd-networkctl`, `cmd-nft`, `cmd-tayga`, `cmd-ufw`, `ctmark`,
  `forwarding-enable`, `plat-dev`, `plat-prefix`,
  `plat-fallback-prefix`, `proxynd-enable`, `route-table`,
  `tayga-conffile`, `tayga-v4-addr`, `v4-conncheck-enable`,
  `v4-conncheck-delay`, `v4-defaultroute-enable`,
  `v4-defaultroute-replace`, `v4-defaultroute-metric`,
  `v4-defaultroute-mtu`, `v4-defaultroute-advmss`). `readconf`
  (`clatd:117-134`) EXPLICITLY validates: `if(!exists($CFG{$1})) {
  w("Unknown key '$1' defined in config file ignored"); }` -- an
  actively enforcing consumer, not merely a documented default list.
  The cleanest, most textbook-bounded D of any candidate in this whole
  holdout batch.
fit: yes
reason_code: SUPPORTED
notes: matches the `unbound`/`clatd` shape (`pkgs.formats.keyValue`,
  key=value rendered text) closely, but with a MEANINGFULLY stronger D
  than unbound's own real lexer-based extraction -- clatd's own real
  consumer code validates unknown keys itself, at runtime, with a
  warning. A strong, unambiguous SUPPORTED.
```

## redmine

```
candidate: redmine
A_producer: proved
  evidence: nixos/modules/services/misc/redmine.nix:10,36-38 --
  `format = pkgs.formats.yaml {};` `configurationYml = format.generate
  "configuration.yml" cfg.settings;`. Real `nix eval`
  (`services.redmine.settings.email_delivery = {...};
  database.type="sqlite3";`) confirms real rendered content including
  the user-set `email_delivery` block AND a module-injected
  `production.{scm_git_command,...}` block (the module's own
  `services.redmine.settings.production = {...}` at redmine.nix:305-317).
B_binding: proved, but via a REAL, genuinely novel THREE-HOP chain --
  the most structurally complex B this whole project has investigated
  so far:
  1. `ExecStart` (real: `.../redmine-7.0.1/share/redmine/bin/bundle
     exec rails server ...`) carries NO reference to the config
     artifact at all -- Rails reads `config/configuration.yml` by pure
     framework convention (`Rails.root`-relative), confirmed directly
     in the real consumer source below (C).
  2. The real Redmine PACKAGE itself (pkgs/by-name/re/redmine/
     package.nix:93-97) build-time-symlinks
     `$out/share/redmine/config -> /run/redmine/config` (a FIXED
     runtime path baked into the derivation, confirmed by reading the
     real `installPhase`).
  3. The real module's own `preStart` script (confirmed via real `nix
     eval` of `config.systemd.services.redmine.preStart`) symlinks
     `/run/redmine/config -> "${cfg.stateDir}/config"`, THEN symlinks
     `"${cfg.stateDir}/config/configuration.yml" ->
     /nix/store/bzpa0r8az86awwhkxq92mb6nnc2l2h89-configuration.yml` --
     the EXACT real derivation A read, confirmed by real nix eval
     output (`ln -fs /nix/store/bzpa0r8az86awwhkxq92mb6nnc2l2h89-configuration.yml
     "/var/lib/redmine/config/configuration.yml"`).
  Each hop is individually real and verifiable, but NONE of the 4
  currently-shipped `ArtifactBindingEvidence` variants
  (`DirectArgv`/`EnvironmentEtcSymlink`/`WrapperScriptEnvVar`/
  `ImplicitDefaultPath`/`DirectPositionalArg`/
  `ExecStartPreInstalledEnvVar`) represent this shape -- "a fixed
  build-time symlink plus a module-level preStart symlink chain, read
  by pure framework convention with zero CLI/env footprint at the
  ExecStart level" is genuinely new. Flagged for any future
  implementation round, not modeled here.
C_consumer: proved
  evidence: real pinned version 7.0.1 (`fetchurl {url =
  "https://www.redmine.org/releases/redmine-7.0.1.tar.gz";}`, confirmed
  from the real package.nix). Real source,
  `lib/redmine/configuration.rb:40-41`: `filename = options[:file] ||
  File.join(Rails.root, 'config', 'configuration.yml')` -- the exact
  real convention-based path B's chain resolves to.
D_contract: proved, but with a REAL normalization wrinkle of its own
  kind, distinct from the unbound/i2pd/akkoma triple --
  `lib/redmine/configuration.rb:100-111`
  (`load_from_yaml`) merges the YAML file's own top-level `default:`
  block, THEN its own top-level `<Rails.env>:` block (`production:`
  for this module's own real deployment) OVER it -- meaning the real
  ACCEPTED keys are the FLATTENED contents of an env-keyed wrapper, not
  the wrapper's own literal nesting. A real bounded scan over
  `Redmine::Configuration['KEY']`/`Redmine::Configuration["KEY"]`
  accessor call sites across the whole real source tree found 33 real,
  distinct accepted keys (some templated, e.g.
  `"scm_#{scm_name}_path_regexp"` in `app/models/repository.rb:500`),
  PLUS `lib/redmine/configuration.rb:26-34`'s own small `@defaults`
  dict (9 keys: `avatar_server_url`, `email_delivery`,
  `max_concurrent_ajax_uploads`, `sudo_mode`,
  `common_mark_enable_hardbreaks`, `thumbnails_generation_timeout`,
  `markdownized_preview_generation_timeout`,
  `markdownized_preview_max_source_size`,
  `markdownized_preview_max_output_size`). A REAL, would-be-necessary
  normalization step for any future implementation: strip the real
  `production:`/`default:` top-level env-key wrapper BEFORE comparing,
  the same "per-consumer, never a generic default" discipline the
  unbound/i2pd/akkoma triple already established, but for a
  structurally different reason (a Rails environment-keyed YAML
  convention, not a bare-vs-qualified collision risk).
fit: yes
reason_code: SUPPORTED
notes: the single most structurally complex real candidate in this
  holdout -- both B (3-hop symlink chain) and D (env-key-wrapper
  stripping) are genuinely NEW shapes this project's currently-shipped
  code has never had to handle. Real, honest SUPPORTED verdict, but
  loudly flagged as "not currently implementable without real new
  work," not a drop-in case like `clatd`/`nats`.
```

## nats

```
candidate: nats
A_producer: proved, with an EXTRA real confidence signal no other
  candidate in this project has --
  evidence: nixos/modules/services/networking/nats.nix:14,30 --
  `format = pkgs.formats.json {};` `unvalidatedConfigFile =
  format.generate "nats.conf" cfg.settings;`. The module ALSO real-
  validates the artifact at BUILD TIME (nats.nix:16-28,32-33): when
  `cfg.validateConfig` (default `true`) is set, `configFile` becomes a
  derivation that runs the REAL `nats-server --config "${file}" -t`
  (the actual consumer binary, in real "test config" mode) BEFORE the
  artifact is ever used -- if the real consumer's own parser rejects
  the generated content, the NixOS build itself fails. No other
  candidate in this whole project (11 implemented + this holdout) has
  this real, build-time cross-check between producer and consumer.
B_binding: proved
  evidence: real `ExecStart = "${pkgs.nats-server}/bin/nats-server -c
  ${configFile}";` -- a real, named `-c` flag (DirectArgv shape,
  identical to unpackerr's/clatd's own precedent), confirmed via real
  `nix eval`.
C_consumer: proved
  evidence: real pinned tag v2.14.7 (`fetchFromGitHub {owner =
  "nats-io"; repo = "nats-server"; rev = "v2.14.7";}`, confirmed via
  the real package.nix; tag dereferenced to real commit
  `8d8b69a8c46a46a150eabb7f312607c4d9c58faf` via `gh api`, an annotated
  tag). Real source, `server/opts.go`: the top-level config loader
  iterates the parsed map and dispatches through
  `o.processConfigFileLine(k, v, ...)`, confirmed by direct read.
D_contract: proved, bounded but LARGE and internally nested
  evidence: `server/opts.go`'s own `processConfigFileLine` (and its own
  nested sub-parsers for `accounts`/`cluster`/`gateway`/`leafnodes`/
  `jetstream`/etc.) is a real, bounded `case "key":`-based dispatch --
  375 real `case "..."` labels total across the whole 6673-line file,
  though NOT all at the same nesting level (top-level keys like
  `listen`/`port`/`server_name`/`jetstream`/`cluster` are one real,
  bounded set; each of those that itself takes a nested block --
  `cluster`, `jetstream`, `accounts`, `authorization` -- has its OWN
  further-nested `case` dispatch, structurally similar to `akkoma`'s
  own multi-level challenge). This module's own minimal real test
  config (`server_name`, `port`, `jetstream.store_dir`) only exercises
  2 real levels of nesting.
fit: yes
reason_code: SUPPORTED
notes: the strongest A/C evidence of any candidate in this project so
  far, specifically because the module's own build already exercises
  the real consumer's own parser against the real artifact -- a
  genuinely different, stronger KIND of B/C proof than "the same store
  path appears in two places," worth naming as a real precedent for
  any future round: a build-time `nats-server -t`-style validation, if
  present, is real, mechanical, additional confidence beyond this
  project's own current A/B evidence model.
```

## gancio

```
candidate: gancio
A_producer: proved
  evidence: nixos/modules/services/web-apps/gancio.nix:9,204 --
  `settingsFormat = pkgs.formats.json {};` `configFile =
  settingsFormat.generate "gancio-config.json" cfg.settings;`. Real
  `nix eval` confirms the real rendered JSON structure
  (`hostname`/`baseurl`/`server.socket`/`db.{dialect,storage,...}`/
  `log_level`/`log_path`).
B_binding: proved, but with a REAL, important subtlety this round's
  own attack-category-2 concern exists to catch --
  the module sets up TWO simultaneous binding mechanisms to the SAME
  real derivation:
  1. `ExecStart = "${getExe cfg.package} start ${configFile}";` -- a
     bare POSITIONAL argument after `start` (superficially a
     `DirectPositionalArg` shape, matching `privoxy`'s/`spacecookie`'s
     own already-shipped precedent).
  2. `preStart` (real, via `nix eval`): `ln -sf ${configFile}
     config.json` (relative to `WorkingDirectory = "/var/lib/gancio"`).
  **Real finding: mechanism 1 is very likely NOT what the real
  consumer actually reads.** The real CLI entrypoint
  (`server/cli.js`, fetched fresh from framagit.org/les/gancio @ v1.28.2)
  uses `yargs`, whose OWN `.command(['start', 'run', '$0'], ...)`
  registration takes no positional config-path parameter at all --
  the config path is instead a NAMED `--config`/`-c` OPTION
  (`.option('config', {alias: 'c', default:
  path.resolve(process.env.cwd, 'config.json'), coerce: ... })`),
  whose own real DEFAULT VALUE resolves to
  `<WorkingDirectory>/config.json` -- exactly the path mechanism 2's
  real symlink populates. The bare positional argument the Nix module
  passes after `start` is, on this real evidence, simply unconsumed by
  yargs' own real command definition. **B still holds** (mechanism 2
  + the real default-value chain genuinely proves the binding), but a
  naive future implementation citing mechanism 1 alone (the more
  "obvious"-looking `DirectPositionalArg` shape) would be citing
  evidence that isn't actually why the real consumer reads the right
  file -- it would get the right answer for the wrong reason, and
  would break silently if the module's own real config path ever
  diverged from the symlinked default. This is exactly the "two
  plausible artifact paths exist; which one does the current code
  actually prove is bound?" scenario attack category 2 named in
  advance.
C_consumer: proved
  evidence: real pinned tag v1.28.2 (`fetchFromGitLab {domain =
  "framagit.org"; owner = "les"; repo = "gancio"; rev = "v1.28.2";}`,
  confirmed via the real package.nix). Real source, `server/config.js`
  (fetched fresh): `config.load()` reads `process.env.config_path ||
  './config.json'` via `fs.readFileSync`+`JSON.parse`; `server/cli.js`
  is the real bridge setting `process.env.config_path` from the
  `--config`/`-c` yargs option's own real default.
D_contract: proved, bounded via a real accessor-call-site scan, not a
  schema
  evidence: `server/config.js`'s own `config.load()` does
  `Object.assign(config, JSON.parse(configContent))` -- a GENUINELY
  FREEFORM merge, no key validation at all (structurally identical to
  `vault`'s own already-excluded real problem). BUT a real, bounded
  literal scan for `config.hostname`/`config.baseurl`/
  `config.server.*`/`config.db.*`/`config.log_level`/`config.log_path`
  accessor call sites across the real full source tree found 71 real,
  distinct real usages (more scattered than `redmine`'s own 33, but
  the same real shape) -- matching this project's own established
  "bounded scan over real accessor call sites, not the merge function
  itself" precedent (K4c, `redmine` above).
fit: yes
reason_code: SUPPORTED
notes: real, but the WEAKEST-margin `SUPPORTED` in this batch --
  D relies on a freeform merge + a real but more scattered (71-site)
  accessor scan, and B's own most-obvious-looking evidence
  (`DirectPositionalArg`) is real but probably not what the actual
  consumer honors, with the TRUE binding proof one layer deeper
  (preStart symlink + a real yargs default-value chain). Recorded as
  SUPPORTED because every link genuinely does hold on real evidence,
  not because it was the easiest story to tell.
```

## Batch 3 summary

- **4/4 SUPPORTED**: `clatd`, `redmine`, `nats`, `gancio`. A
  surprisingly high hit rate for this batch specifically -- NOT
  representative of the fresh holdout's own overall rate (batches 1/2
  cover the other 6 of 10 candidates separately).
- **`clatd`**: the cleanest case in the whole holdout -- a real,
  actively-validating consumer (`readconf` warns and ignores unknown
  keys itself).
- **`nats`**: the strongest A/C evidence in this whole project so far
  -- a real build-time `nats-server -t` validation step cross-checks
  producer against consumer before the system even builds.
- **`redmine`**: the single most structurally complex real B (a
  genuine 3-hop symlink chain: package build-time symlink -> preStart
  symlink -> preStart symlink) and a genuinely new D normalization
  concern (a Rails `default:`/`<env>:` env-keyed YAML wrapper that
  must be stripped, not just bare-vs-qualified stripping). Neither
  shape is representable by anything currently shipped -- named
  explicitly as future-implementation-relevant, not modeled here.
- **`gancio`**: real, but the batch's own best illustration of attack
  category 2's central concern -- the "obvious" binding evidence
  (a CLI positional argument) is very likely NOT what the real
  consumer actually honors; the TRUE proof is a preStart symlink
  combined with the consumer's own real default option value. A
  concrete, real example (not hypothetical) of exactly the kind of
  binding-proof fragility this whole audit round exists to surface.
- Zero candidates in this batch required guessing or forcing a
  verdict -- every fit=yes above is backed by real, independently
  re-checkable citations (real `nix eval` output, real upstream source
  read directly, real commit/tag resolution).
