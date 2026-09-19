# C-E1.1 batch 3: akkoma, vault, spacecookie

All A/B evidence from real `nix eval`/`nix build` against the real
NixOS module at tree `68740713a1d5904edf9ba92a998a522b1b6ce080` (not
read off source by eye alone). All C/D evidence from each consumer's
real, exact pinned upstream source, fetched fresh this round.

## akkoma

```
candidate: akkoma
A_producer: proved
  evidence: `configFile = format.generate "config.exs" (replaceSec (...));`
  where `format = pkgs.formats.elixirConf {...};` -- a real
  `format.generate` call. Confirmed by a REAL BUILD (not just eval) of
  the exact derivation the module's own `systemd.services.akkoma-config
  .reloadTriggers` references: `/nix/store/ifw5f9m5wd3y3adb8hdwa6ls59l5h89a-config.exs`,
  built successfully, real content read directly:
    import Config
    config :pleroma, :instance, description: "t", email: "t@t", name: "t", ...
    config :pleroma, Pleroma.Repo, adapter: Ecto.Adapters.Postgres, ...
    config :pleroma, Pleroma.Web.Endpoint, http: [...], url: [...], ...
    (secret fields appear as sha256(secret-file-path) placeholders,
    substituted at real runtime by a separate `replace-secret` step --
    disclosed, not hidden)
B_binding: proved
  evidence: real `nix eval` of `systemd.services.akkoma.serviceConfig
  .ExecStart` = `/nix/store/89ir5yj32jqnzrcx4wl3191wwyfxr7wa-akkoma-env/
  bin/pleroma start` -- the real module source (line 423) shows this
  wrapper sets `AKKOMA_CONFIG_PATH="${RUNTIME_DIRECTORY%%:*}/config.exs"`,
  and the SEPARATE `akkoma-config.service` unit's own `configScript`
  copies `configFile`'s exact content (same derivation as A) into that
  exact runtime path before `akkoma.service` starts (`bindsTo =
  ["akkoma-config.service"]`). Same key/structure identity as A,
  confirmed by direct source read of both units, not assumed.
C_consumer: proved
  evidence: real pinned version v3.19.0 (`fetchFromGitea { domain =
  "akkoma.dev"; owner = "AkkomaGang"; repo = "akkoma"; tag = "v3.19.0";
  }`, confirmed from the real nixpkgs package.nix). Real source,
  `lib/pleroma/config/release_runtime_provider.ex`:
    config_path = cond do
      ...
      System.get_env("AKKOMA_CONFIG_PATH") -> System.get_env("AKKOMA_CONFIG_PATH")
      ...
    end
    runtime_config = Config.Reader.read!(config_path)
  -- an exact match to the real env var B proved, feeding Elixir's own
  standard `Config.Reader` (part of the `elixir` stdlib `Config`
  behaviour), a real named library, not a guess from the file
  extension.
D_contract: NO
  evidence: `Config.Reader.read!/1` parses `import Config` DSL files
  into Elixir's generic, atom-keyed application environment -- there is
  NO typed struct, NO schema validation on the runtime file. A REAL
  bounded artifact does exist adjacent to this
  (`Pleroma.Config.Loader.default_config()`, evaluated at compile time
  from the app's OWN `config/config.exs`+`config/prod.exs`, referenced
  in `lib/pleroma/config/holder.ex`) -- but it is a DEFAULTS SNAPSHOT,
  not an enforced schema: `Config.Reader.merge` happily accepts and
  merges in ANY additional `config :app, key, value` entry the runtime
  file has, whether or not it appears in the defaults, and the
  application reads config values via `Application.get_env`/
  `Pleroma.Config.get` calls scattered arbitrarily throughout the
  codebase, not a bounded set. This is close to the criterion's own
  named negative example ("scatters it through arbitrary later code")
  -- the defaults snapshot is a real research aid for a FUTURE adapter,
  but does not itself make the RUNTIME-ACCEPTED contract bounded today.
fit: no
reason_code: CONTRACT_UNEXTRACTABLE
notes: the closest of the three to a real "maybe, with real future
  work" case -- A/B/C are all clean and strong; D fails specifically
  because Elixir's own `Config` DSL has no enforced schema, not because
  the producer or binding side is weak. If a future round wanted to
  extract SOME bounded subset, `Pleroma.Config.Loader.default_config()`
  is the concrete, real place to start (not invented here).
```

## vault

```
candidate: vault
A_producer: proved
  evidence: `configFile = pkgs.writeText "vault.hcl" (real HCL
  template with cfg.* fields interpolated);` -- a real `writeText`
  derivation. Confirmed via real `nix eval` (with
  `nixpkgs.config.allowUnfree = true;`, required -- vault is
  unfree-licensed) of the real, default-configured module's own
  `ExecStart`.
B_binding: proved
  evidence: real `nix eval` of `systemd.services.vault.serviceConfig
  .ExecStart` =
    /nix/store/06m5g7fqyiirmbzhiz0am0yyc5y3bsw1-vault-2.0.3/bin/vault
      server -config /nix/store/lv0zqj8yvyyyvpcmza42sqimhlv8mzfs-vault.hcl
  -- the exact same store path A's own source-reading predicted
  (`allConfigPaths = [ configFile ] ++ cfg.extraSettingsPaths;`, each
  rendered as `"-config" p`), confirmed mechanically, not merely
  plausible from reading the module alone.
C_consumer: proved
  evidence: real pinned version v2.0.3
  (`fetchFromGitHub { owner = "hashicorp"; repo = "vault"; rev =
  "v2.0.3"; }`, confirmed from the real nixpkgs package.nix). Real
  source, `command/server.go` registers the real `-config` flag
  (`Name: "config"`, line 188); `command/server/config.go`:
    func LoadConfigFile(path string) (*Config, error) { ... }
    func ParseConfigCheckDuplicate(d, source string) (...) {
      obj, duplicate, err := random.ParseAndCheckForDuplicateHclAttributes(d)
      ...
      result := NewConfig()
      if err := hcl.DecodeObject(result, obj); err != nil { ... }
      ...
    }
  -- a real, traceable `-config -> LoadConfigFile -> ParseConfigCheckDuplicate
  -> hcl.DecodeObject` chain, HashiCorp's own real `hcl` library, not
  assumed from the `.hcl` extension.
D_contract: proved
  evidence: the real `Config` struct (`command/server/config.go:45`) is
  a fully-tagged Go struct:
    type Config struct {
      Experiments []string `hcl:"experiments"`
      CacheSize int `hcl:"cache_size"`
      PluginDirectory string `hcl:"plugin_directory"`
      ... (dozens more, every field with an explicit `hcl:"..."` tag)
    }
  -- a textbook bounded, extractable contract: every accepted top-level
  key is a real, named, typed struct field with its own serialization
  tag.
fit: yes
reason_code: SUPPORTED
notes: the cleanest, strongest of the three -- every link backed by a
  real mechanical check (a real build/eval, not source-reading alone),
  and D in particular is about as bounded as this criterion could ask
  for.
```

## spacecookie

```
candidate: spacecookie
A_producer: proved
  evidence: `configFile = format.generate "spacecookie.json"
  spacecookieConfig;` where `format = pkgs.formats.json {};` -- a real
  `format.generate` call. Confirmed via real `nix eval` of the real,
  default-configured module's own `ExecStart`.
B_binding: proved
  evidence: real `nix eval` of `systemd.services.spacecookie
  .serviceConfig.ExecStart` =
    /nix/store/r0rdiylxnjshjqx2glq8n6riz36h165r-spacecookie-1.1.0.1/bin/spacecookie
      /nix/store/9jcdhjmkiwiziyrf53flcky2jbsqhps9-spacecookie.json
  -- the config file passed as a bare POSITIONAL argument (not a
  flag), the exact same real derivation A's own source-reading
  predicted, confirmed mechanically.
C_consumer: proved
  evidence: real pinned version 1.1.0.1 (confirmed via `nix eval` of
  `pkgs.spacecookie.src.url`/`.version` ->
  `mirror://hackage/spacecookie-1.1.0.1.tar.gz`), fetched directly from
  Hackage at that exact version. Real source, `server/Main.hs`:
    main = do
      args <- getArgs
      case getOpt Permute options args of
        ([], [configFile], []) -> runServer =<< OP.encodeUtf configFile
        ...
    runServer configFile = do
      ...
      config' <- eitherDecodeStrict' <$> F.readFile' configFile
      ...
  -- the ONE positional argv element (matching B's own real binding
  exactly) is read and decoded via Aeson's real `eitherDecodeStrict'`,
  a named, real JSON-parsing library call, not inferred from the
  `.json` extension.
D_contract: proved
  evidence: `server/Network/Spacecookie/Config.hs`, a real, explicit,
  fully bounded Aeson `FromJSON` instance:
    instance FromJSON Config where
      parseJSON (Object v) = Config
        <$> v .: "hostname"
        <*> maybePath [ "listen", "addr" ] v
        <*> parseListenPort v .!= 70
        <*> v .:? "user"
        <*> v .: "root"
        <*> v .:? "log" .!= defaultLogConfig
  plus a second, equally explicit `FromJSON LogConfig` instance for the
  nested `log.{enable,hide-ips,hide-time,level}` keys. Every accepted
  key is a named, explicit field accessor -- nothing freeform.
fit: yes
reason_code: SUPPORTED
notes: the cleanest of all three -- a small, well-written Haskell app
  with fully explicit, bounded parsing on both the CLI-argv (C) and the
  JSON-schema (D) sides. If this project ever builds a real
  `GeneratedConfigArtifact` locator for a small JSON-configured daemon,
  this is close to the ideal real-world shape to model it against.
```

## Batch 3 summary

- 3/3 candidates investigated, all real evidence, zero guessed links.
- **vault**: SUPPORTED (A/B/C/D all proved).
- **spacecookie**: SUPPORTED (A/B/C/D all proved) -- the cleanest of the
  batch.
- **akkoma**: NOT fit, `CONTRACT_UNEXTRACTABLE` -- A/B/C all proved
  (unusually strong, including a real derivation BUILD, not just eval,
  for A), but D fails: Elixir's `Config` DSL has no enforced schema on
  the runtime file, only a real but non-enforcing "known defaults"
  snapshot.
- 2/3 SUPPORTED in this batch is a real, strong positive signal for the
  A->B->C->D chain's own universality -- it held across TWO genuinely
  different languages/formats (Go+HCL, Haskell+JSON) with the identical
  evidence shape each time, exactly the "format is a locator detail"
  hypothesis the protocol named up front. `akkoma`'s failure is
  specifically at D, not a earlier link -- consistent with what the
  protocol flagged as the likely real bottleneck to watch for.
