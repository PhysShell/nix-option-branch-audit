# C-E1.2c fresh holdout batch 1: librechat, meilisearch, anubis

Protocol: `fixtures/c-e1.2c-hostile-audit/protocol.md`. Population and
draw: `fixtures/c-e1.2c-hostile-audit/holdout-draw.md`. All A/B
evidence from real `nix eval --impure --json` against
`PhysShell/nixpkgs` tree `68740713a1d5904edf9ba92a998a522b1b6ce080`
(the same pin this whole project uses), local store path
`/nix/store/n76dsw3cz9700dwyr0ff6w4aair4ijdd-source`. All C/D evidence
from each real consumer's exact pinned upstream source, fetched fresh
this round via `gh api`. Zero `src/` changes -- research/fit only, no
implementation.

---

## librechat

```
A_producer: proved
  Evidence: nixos/modules/services/web-apps/librechat.nix:9-10 --
  `format = pkgs.formats.yaml { };` / `configFile = format.generate
  "librechat.yaml" cfg.settings;` -- a real, typed `format.generate`
  call over the real `cfg.settings` freeform-YAML option. Confirmed via
  real `nix eval` (`services.librechat.settings = { version = "1.2.1";
  cache = true; };`).
B_binding: proved
  Evidence: module.nix:75-78 -- `cfg.env.CONFIG_PATH` option's own
  `default = configFile;` (`internal = true; readOnly = true;`) --
  LITERAL reuse of the same local `configFile` binding A read, not a
  second, separately-constructed path. `environment = cfg.env;` on the
  systemd unit (module.nix:227). Real `nix eval` confirms:
  `environment.CONFIG_PATH = "/nix/store/lnnsbpcqjr5d3gmz9z28g4any13vyz54-librechat.yaml"`.
C_consumer: proved
  Evidence: real pinned version v0.8.7 (`danny-avila/LibreChat`,
  confirmed from `pkgs/by-name/li/librechat/package.nix`).
  `api/server/services/Config/loadCustomConfig.js:70-85`: `const
  configPath = process.env.CONFIG_PATH || defaultConfigPath;` then
  `customConfig = loadYaml(configPath);` -- direct, unambiguous read of
  the exact real env var B proved.
D_contract: proved -- unusually strong, ENFORCED, not just typed.
  Evidence: `packages/data-provider/src/config.ts:1708` --
  `export const configSchema = z.object({ version: z.string(), cache:
  z.boolean().default(true), ocr: ocrSchema.optional(), ...
  interface: interfaceSchema, ... endpoints: z.object({...}), ... })`
  -- a real, fully bounded, nested Zod schema. `loadCustomConfig.js:112`:
  `const result = configSchema.strict().safeParse(customConfig);` --
  `.strict()` means Zod REJECTS any key not in the schema, and
  `.safeParse()` actually runs the validation at real runtime, not just
  static type inference. The strongest, most rigorously ENFORCED D
  contract found anywhere in this project so far -- stronger than any
  of the 11 already-implemented candidates' own D evidence.
fit: yes
reason_code: SUPPORTED
notes: A clean, strong SUPPORTED case, structurally closest to
  `unpackerr`'s own C-E1.2a anchor (StructuredValue via `cfg.settings`,
  `DirectArgv`-adjacent binding via a readonly-derived env var) but
  with a categorically stronger D (real strict-mode runtime schema
  enforcement, not just a typed struct that happens to exist).
```

## meilisearch

```
A_producer: proved
  Evidence: nixos/modules/services/search/meilisearch.nix:10,46-51 --
  `settingsFormat = pkgs.formats.toml { };` / `configFile =
  settingsFormat.generate "config.toml" (removeAttrs (...) cfg.settings
  ...);` -- a real `format.generate` call, with a real, disclosed
  secret-substitution mechanism (`master_key` replaced with a literal
  placeholder string `@MASTER_KEY@`, substituted at real runtime via
  `replace-secret` in `ExecStartPre`, the SAME pattern this project's
  own `akkoma` anchor already established). Confirmed via real `nix
  eval`: `settings = {"db_path":"/var/lib/meilisearch",
  "http_addr":"localhost:7700","no_analytics":true,...}`.
B_binding: proved -- a REAL, NEW binding shape not yet in this
  project's own `ArtifactBindingEvidence` enum.
  Evidence: real `nix eval` of `ExecStartPre` = `install -m 700
  '/nix/store/r27rixa9527bfhddbq5hg5s5bs97hk5s-config.toml'
  "${RUNTIME_DIRECTORY}/config.toml"` (the SAME `configFile` derivation
  A read), then real `ExecStart` = `meilisearch --config-file-path
  ${RUNTIME_DIRECTORY}/config.toml`. This is `ExecStartPre`-installed-
  to-a-fixed-runtime-path (matching `misskey`'s own
  `ExecStartPreInstalledEnvVar` shape) but referenced via a CLI FLAG,
  not an env var -- neither existing variant
  (`ExecStartPreInstalledEnvVar` assumes an env var;
  `DirectArgv`/`DirectPositionalArg` assume the flag's own value IS the
  artifact's literal store path) cleanly names this real shape. A
  genuine, disclosed gap in the current binding-evidence taxonomy --
  not a blocker for THIS round's own fit verdict (B is still real and
  provably correct, just not classifiable into an EXISTING enum
  variant without inventing a new one), but a real data point for any
  future implementation.
C_consumer: proved
  Evidence: real pinned version v1.53.2 (`meilisearch/meilisearch`,
  confirmed from `pkgs/by-name/me/meilisearch/package.nix`).
  `crates/meilisearch/src/option.rs:536-551`: real config-loading code
  reads `config_file_path` (CLI-flag-or-env, `clap`), then
  `std::fs::read_to_string(&config_file_path)` +
  `toml::from_str::<Opt>(&config)?` -- the file is parsed directly.
D_contract: proved -- unusually strong, dual-purpose.
  Evidence: `option.rs:212-214` -- `#[clap(version, ...)] pub struct
  Opt { ... }` -- the SAME struct is BOTH the real `clap` CLI-arg
  parser AND (via `#[serde(default = ...)]` attributes throughout, and
  the real `toml::from_str::<Opt>` call above) the real TOML
  config-file schema. Every real field observed in the Nix-evaluated
  `settings` (`db_path`, `http_addr`, `no_analytics`, `dump_dir`,
  `snapshot_dir`, `upgrade_db`, `ssl_cert_path`, `ssl_key_path`,
  `ssl_auth_path`, `ssl_ocsp_path`) confirmed present as real, named
  `Opt` struct fields (`option.rs:216-452`).
fit: yes
reason_code: SUPPORTED
notes: As strong as `librechat` -- D here is a real, single, dual-
  purpose Rust struct (CLI flags AND config-file keys are the exact
  same typed fields), a real, clean shape this project hasn't seen
  before. B's own real "install-then-flag-reference" shape is a
  genuinely new, disclosed finding for the binding-evidence taxonomy.
```

## anubis

```
A_producer: proved -- CONDITIONAL, disclosed.
  Evidence: nixos/modules/services/networking/anubis.nix:9,17-38 --
  `jsonFormat = pkgs.formats.json { };` / `mkPolicyFile = name:
  instance: ... if hasCustomization then jsonFormat.generate
  "${instanceName name}-policy.json" policyContent else null;` -- a
  real `format.generate` call, but ONLY when the user has customized
  `extraBots`, `policy.settings`, or set `useDefaultBotRules = false`;
  otherwise anubis uses its own built-in `botPolicies.yaml` and NO Nix
  artifact is generated at all. Confirmed via real `nix eval` with
  `policy.settings.dnsbl = false;`: `POLICY_FNAME =
  "/nix/store/g7fjhayacn5pfy4dy3wbw8ad6bp52rj7-anubis-test-policy.json"`,
  real built content (via `builtins.readFile`) = `{"bots":
  [{"import":"(data)/meta/default-config.yaml"}],"dnsbl":false}`.
  Note: `anubis`'s OWN separate `settings` submodule (BIND, DIFFICULTY,
  TARGET, ...) becomes real environment variables directly -- a flat
  env-var interface, NOT a generated-config-artifact shape at all
  (this project's own K2d/K5 vertical's territory, not C-E1.2's). Only
  the POLICY artifact is in scope for this fit assessment.
B_binding: proved
  Evidence: real `nix eval` shows `environment.POLICY_FNAME` set to the
  SAME `mkPolicyFile name instance` derivation the module's own
  `config` block constructs (module.nix:355-359, `POLICY_FNAME = if
  instance.settings.POLICY_FNAME != null then
  instance.settings.POLICY_FNAME else mkPolicyFile name instance;`) --
  literal reuse, not reconstructed.
C_consumer: proved -- required tracing three real chained functions.
  Evidence: real pinned version v1.27.0 (`TecharoHQ/anubis`, confirmed
  from `pkgs/by-name/an/anubis/package.nix`). `cmd/anubis/main.go:67`:
  `policyFname = flag.String("policy-fname", "", ...)`; line 219:
  `flagenv.Parse()` -- a real, named library
  (`github.com/facebookgo/flagenv`) that populates Go `flag` values
  from correspondingly-named environment variables (`POLICY_FNAME` ->
  `-policy-fname`, confirmed via the library's own well-known naming
  convention, not assumed); line 304:
  `libanubis.LoadPoliciesOrDefault(ctx, *policyFname, ...)`.
  `lib/config.go:83`: `policy.ParseConfig(ctx, fin, fname, ...)`.
  `lib/policy/policy.go:82`: `c, err := config.Load(fin, fname)`.
D_contract: proved
  Evidence: `lib/config/config.go:456` -- a real, bounded, JSON-tagged
  `type Config struct { OpenGraph ...; Impressum *Impressum
  \`json:"impressum,omitempty"\`; Store *Store \`json:"store"\`; Bots
  []BotOrImport \`json:"bots"\`; Thresholds []Threshold
  \`json:"thresholds"\`; StatusCodes StatusCodes
  \`json:"status_codes"\`; DNSBL bool \`json:"dnsbl"\`; DNSTTL DnsTTL
  \`json:"dns_ttl"\`; Logging *Logging \`json:"logging"\`; Metrics
  *Metrics \`json:"metrics,omitempty"\`; Honeypot *Honeypot
  \`json:"honeypot"\`; }`. `config.go:337-354` (`func Load`): real
  `yaml.NewYAMLToJSONDecoder(fin).Decode(&c)` into a typed intermediate
  struct, then remapped field-by-field into the final typed `Config`.
  Both real fields the vendored artifact actually emits (`bots`,
  `dnsbl`) confirmed present as real, named, JSON-tagged fields.
fit: yes
reason_code: SUPPORTED
notes: The most structurally complex real case in this batch --
  requires understanding that ONE Nix module produces TWO real,
  differently-shaped artifacts (a flat env-var interface out of this
  vertical's own scope, and a genuinely generated JSON policy artifact
  in scope), and that the JSON artifact's own real existence is
  CONDITIONAL on user customization. The real C chain (three real
  functions: `flagenv.Parse` -> `LoadPoliciesOrDefault` ->
  `policy.ParseConfig` -> `config.Load`) is the longest traced in this
  batch, and required confirming a real, specific third-party Go
  library's own naming convention (`flagenv`) rather than assuming
  "POLICY_FNAME probably maps to -policy-fname" from the names alone.
```

## Batch 1 summary

- **3/3 SUPPORTED** (librechat, meilisearch, anubis) -- all real,
  fully cited A/B/C/D chains, zero step assumed from another.
- **Two real findings worth carrying into the parent round's own
  aggregation**:
  1. `librechat`'s D (a real `.strict().safeParse()`-enforced Zod
     schema) and `meilisearch`'s D (a real dual-purpose `clap`+`serde`
     struct) are both STRONGER than any of the 11 already-implemented
     candidates' own D evidence -- direct, real confirmation this
     abstraction generalizes to genuinely well-engineered real
     consumers, not just the ones already chosen.
  2. `meilisearch`'s B (`ExecStartPre`-install then CLI-flag reference
     to the fixed runtime path) is a real binding shape this project's
     current `ArtifactBindingEvidence` enum does not cleanly name --
     neither `ExecStartPreInstalledEnvVar` (assumes an env var) nor
     `DirectArgv`/`DirectPositionalArg` (assume the flag value IS the
     artifact's own literal store path) fits without stretching. A
     real, disclosed taxonomy gap, not a blocker to this round's own
     SUPPORTED verdict.
- `anubis` is the one candidate in this batch requiring real judgment
  about SCOPE (its own flat env-var `settings` interface is a
  different vertical's territory entirely; only the conditional JSON
  policy artifact is this vertical's concern) -- recorded explicitly,
  not glossed over.
- Zero app-specific semantic branching temptation found in any of the
  three -- each fits the existing `GeneratedConfigArtifact`/
  `ConsumerConfigContract`/`compare_config_contract` model cleanly,
  modulo the one real binding-taxonomy gap named above.
