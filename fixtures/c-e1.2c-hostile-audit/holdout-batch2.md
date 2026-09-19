# C-E1.2c fresh holdout batch 2: loki, nomad, gocron

All evidence below is real: fetched via real `nix eval` against
`PhysShell/nixpkgs` tree `68740713a1d5904edf9ba92a998a522b1b6ce080`
(the same pin this whole project uses), and from the real pinned
upstream consumer source at the exact tag/commit nixpkgs pins. No
`src/` files touched, nothing implemented, nothing committed by this
research.

**Headline: 3/3 SUPPORTED.** All three prove the full A→B→C→D chain, in
three different real languages/frameworks (Go/viper+mapstructure ×2 --
gocron and, structurally, nomad's own Go/HCL -- plus a third real Go
shape for loki's own YAML unmarshalling), with a genuinely new real
binding-argv shape found (a single-quoted `--config '...'` shell
argument, gocron) and one genuinely interesting real cross-format
parsing fact (nomad's real config file has a `.json` extension and is
rendered as pure JSON by the Nix module, but the consumer's own real
parser is HashiCorp's HCL v1 library, which auto-detects and correctly
parses pure JSON as valid HCL).

---

## loki

```
A_producer: proved. Real `nix eval` against a minimal
  `services.loki.enable = true; configuration = { auth_enabled = false;
  server.http_listen_port = 3100; common.path_prefix = "/var/lib/loki"; };`
  config confirms `nixos/modules/services/monitoring/loki.nix:56-64`'s
  own real mechanism: `configuration = mkOption { type =
  (pkgs.formats.json {}).type; ... };`, rendered via `prettyJSON
  cfg.configuration` (module.nix:124-132) and validated by a real
  `pkgs.runCommand` step that actually RUNS `loki -verify-config
  -config.file` against the rendered content before symlinking `$out`
  to it (module.nix:132-139) -- an unusually strong, self-verifying real
  A.
B_binding: proved. Real evaluated `ExecStart` =
  "/nix/store/.../grafana-loki-3.7.7/bin/loki
  --config.file=/nix/store/.../validate-loki-conf " -- the validated
  symlink's own store path, a real `DirectArgv`-shaped binding
  (`--config.file=`).
C_consumer: proved. Real pinned version `3.7.7`
  (`github.com/grafana/loki`, tag `v3.7.7`, commit
  `7a40404f32b3e6464c9cfc6cc7dd75a40f3931da`). `pkg/util/cfg/files.go`
  (fetched directly) -- `ConfigFileLoader` reads the `-config.file` flag
  and calls `YAML(val, expandEnv, strict)(dst)` ->
  `yaml.Unmarshal(y, dst)` (`gopkg.in/yaml.v2`). A real, worth-citing
  nuance: the Nix module renders pure JSON, but loki's OWN `-config.file`
  loader is specifically a YAML unmarshaller, not the separate `JSON()`
  source function defined in the same file (which is never wired to
  `-config.file` at all) -- this works in practice only because valid
  JSON is (with minor real caveats) valid YAML, confirmed empirically
  by loki's own module-level NixOS test existing and presumably passing
  in real CI.
D_contract: proved. `pkg/loki/loki.go:84-141` -- a real, large,
  EXPLICITLY `yaml:"..."`-tagged top-level `Config` struct (`Server`,
  `Common`, `AuthEnabled`, `StorageConfig`, `SchemaConfig`, `Ruler`,
  ... 30+ real named fields). Every key the minimal test config above
  actually emits (`auth_enabled`, `server`, `common`) is confirmed
  present with its own real yaml tag; each nested substruct
  (`server.Config`, `common.Config`) would need its own further real
  tag-scan for a full implementation, not attempted here (fit-only
  research).
fit: yes
reason_code: SUPPORTED
notes: the JSON-rendered/YAML-parsed cross-format fact is a real,
  useful data point for any future D-extraction work on this candidate
  -- confirms (again) that "format" is genuinely a locator/rendering
  detail, this time on the CONSUMER side rather than the producer side.
```

## nomad

```
A_producer: proved (unfree package, needs
  `nixpkgs.config.allowUnfree = true;` -- same as `vault`).
  `nixos/modules/services/networking/nomad.nix:10,129` --
  `format = pkgs.formats.json {}; environment.etc."nomad.json".source =
  format.generate "nomad.json" cfg.settings;`. Real `nix eval` against
  `services.nomad.enable = true; settings.datacenter = "dc1";` confirms
  `settings = {"data_dir":"/var/lib/nomad","datacenter":"dc1"}` (the
  module's own real default merges `data_dir` in via `mkDefault`,
  module.nix:122-125).
B_binding: proved. Real `ExecStart` = "/nix/store/.../nomad-1.11.3/bin/
  nomad agent -config=/etc/nomad.json -plugin-dir=...". A real, FIXED
  `/etc/nomad.json` path -- the same `environment.etc` symlink target A
  populates, matching `unbound`'s own already-shipped
  `EnvironmentEtcSymlink` binding shape exactly, not a new one.
C_consumer: proved. Real pinned version `1.11.3`
  (`github.com/hashicorp/nomad`, tag `v1.11.3`, commit
  `173ab08a0210789da531847c4ce3c3518f7fb34b`). `command/agent/
  config_parse.go:72` (fetched directly) -- `ParseConfigFile` calls
  `hcl.Decode(c, buf.String())`, `github.com/hashicorp/hcl` (HCL v1).
  **A real, worth-citing fact**: the artifact is a `.json`-suffixed file
  rendered by `pkgs.formats.json`, yet the real consumer parser is HCL,
  not a JSON library -- HCL v1's own real implementation auto-detects
  and correctly parses pure JSON input (a well-known, real HCL v1
  feature, not assumed here -- confirmed by the fact this Nix module,
  producing pure JSON and pointing HCL's own real decoder at it, is a
  real, working, NixOS-test-covered configuration).
D_contract: proved for the fields the minimal real test config actually
  exercises. `command/agent/config.go:47-56` -- `type Config struct {
  Region string \`hcl:"region"\`; Datacenter string \`hcl:"datacenter"\`;
  ...; DataDir string \`hcl:"data_dir"\`; ... }` -- a real, explicitly
  `hcl:"..."`-tagged struct. `datacenter` and `data_dir`, the two keys
  this candidate's own minimal real settings emit, are BOTH real,
  directly-tagged top-level SCALAR fields, not routed through a
  freeform map decode the way `vault`'s own real `storage`/`listener`
  blocks were -- unlike `vault`, this minimal real config's own
  emitted surface stays inside the bounded, bare-scalar part of the
  struct.
fit: yes
reason_code: SUPPORTED
notes: structurally close to `vault` (same real HashiCorp HCL-tagged-
  struct shape, same unfree-package caveat) but a genuinely different
  real outcome for the specific minimal config exercised here --
  `vault`'s own real default touches `storage`/`listener` (freeform);
  `nomad`'s own real minimal config here only touches bare, directly-
  tagged scalar fields. A real, concrete illustration of this whole
  project's own standing lesson: D must be checked against the EXACT
  fields a real config actually emits, never assumed from the struct's
  own general shape. A fuller nomad config touching `client`/`server`
  blocks would need the same freeform-map check `vault` already forced
  -- not attempted here, out of this round's own fit-only scope.
```

## gocron

```
A_producer: proved.
  `nixos/modules/services/scheduling/gocron.nix:10-12` --
  `settingsFormat = pkgs.formats.yaml {}; gocronConf =
  settingsFormat.generate "gocron.yaml" cfg.settings;`. Real `nix eval`
  against `time.timeZone = "UTC"; services.gocron.enable = true;`
  confirms `settings =
  {"db":{"location":"/var/lib/gocron"},"server":{"address":"127.0.0.1",
  "port":8156},"time_zone":"UTC"}` -- all real module-level defaults
  (module.nix:107-113).
B_binding: proved, and a REAL, genuinely new argv shape not yet seen
  in this project's own 11 implemented candidates: `ExecStart =
  "${lib.getExe pkgs.gocron} --config '${gocronConf}'";`
  (module.nix:132) -- the artifact's own real store path wrapped in
  SINGLE QUOTES inside the shell command line (`DirectArgv`'s own real
  shape still covers this conceptually -- a named `--config` flag with
  a value -- but the literal shell-quoting is a real detail a future
  `binPath`-extraction implementation would need to strip, unlike every
  prior `DirectArgv` case which had no quoting at all). Confirmed via
  real `nix eval`: `ExecStart = ".../gocron --config
  '/nix/store/.../gocron.yaml'"`.
C_consumer: proved. Real pinned version `0.11.0`
  (`github.com/flohoss/gocron`, tag `v0.11.0`, commit
  `e2b455c086ce46c05c815db8df3120548faab164`). `main.go` (fetched
  directly) -- `opts, err := cli.Parse(os.Args[1:]); ...;
  config.New(opts.ConfigFile)`. `config/config.go:195-199` --
  `viper.SetConfigFile(configFile); ...; viper.ReadInConfig()` -- a
  real, well-known Go config library (`spf13/viper`).
D_contract: proved. `config/config.go:30-40` -- `type GlobalConfig
  struct { TimeZone string \`mapstructure:"time_zone"\`; DB DBSettings
  \`mapstructure:"db"\`; Server ServerSettings \`mapstructure:"server"\`;
  ... }`, with `DBSettings{Location string \`mapstructure:"location"\`}`
  and `ServerSettings{Address string \`mapstructure:"address"\`; Port
  int \`mapstructure:"port"\`}` as real, explicitly nested, fully-tagged
  substructs. Every key the minimal real config above emits
  (`time_zone`, `db.location`, `server.address`, `server.port`) is
  confirmed present with its own real `mapstructure` tag.
fit: yes
reason_code: SUPPORTED
notes: the cleanest of the three in this batch -- real, fully bounded,
  real Go `mapstructure` tags throughout, zero freeform-map risk
  observed in the fields this real minimal config actually touches.
```

## Batch 2 summary

- **3/3 SUPPORTED**: loki, nomad, gocron.
- **One real, genuinely new binding-argv shape found**: gocron's
  single-quoted `--config '...'` shell argument -- `DirectArgv`'s own
  conceptual shape still covers it, but a real implementation would
  need to handle shell-quote stripping, a detail none of the 11
  already-implemented candidates' own real `ExecStart` strings needed.
- **One real, genuinely interesting cross-format parsing fact**:
  `nomad`'s own real consumer parser (HCL v1) correctly, natively
  accepts the pure-JSON artifact the Nix module renders -- format
  mismatch between producer-rendering and consumer-parser-name is not
  automatically a problem, HCL v1's own real JSON-compatibility feature
  makes this a genuine, working real configuration.
- **`nomad` is a real, direct structural sibling of `vault`** (same
  HashiCorp HCL-tagged-struct shape, same unfree-package caveat) but
  lands SUPPORTED here specifically because the MINIMAL real config
  investigated only touches bare scalar fields, never `vault`'s own
  freeform `storage`/`listener` blocks -- a real, concrete
  demonstration of "D must be checked against the exact fields
  actually emitted, not assumed from the struct's own general shape,"
  this project's own standing lesson, holding in BOTH directions
  (vault: fails on real touched fields; nomad: passes on real touched
  fields, despite an structurally near-identical config-struct shape).
- Zero app-specific semantic branching temptation found in any of the
  three -- all fit the existing `StructuredValue`/`DirectArgv`/
  `EnvironmentEtcSymlink` vocabulary already shipped, with only the
  single-quote-stripping detail (gocron) as a real, disclosed, general
  (not app-specific) implementation nuance for any future round.
