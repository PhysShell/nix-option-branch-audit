# C-E1.1 batch 2: kavita, nebula-lighthouse-service, transmission, i2pd

All evidence below is real: fetched from the exact pinned upstream
source at the exact real tag/commit nixpkgs pins, or read directly from
the real vendored `fixtures/e1-holdout-audit/<name>/module.nix`. No
`src/` files touched, nothing committed.

**Headline: 4/4 SUPPORTED.** All four candidates in this batch prove
the full A->B->C->D chain, across three real languages (C#/.NET,
Python, C++ x2 with two structurally different internal mechanisms)
and three formats (JSON, YAML, INI). The evidence chain SHAPE is
identical across all four despite the format/language variety --
direct support for the protocol's own "format is a locator, not the
abstraction" thesis.

---

## kavita

- **A_producer**: proved. `fixtures/e1-holdout-audit/kavita/module.nix:10-13` -- `settingsFormat = pkgs.formats.json {}; appsettings = settingsFormat.generate "appsettings.json" ({ TokenKey = "@TOKEN@"; } // cfg.settings);`. Real generated JSON derivation.
- **B_binding**: proved. `module.nix:88-96` -- `preStart` installs `appsettings` to `${cfg.dataDir}/config/appsettings.json` (then a `replace-secret` substitution for the token placeholder), with `WorkingDirectory = cfg.dataDir;`. Confirmed against REAL upstream: `Kavita.Common/Configuration.cs:20` (tag `v0.9.0.2`, `repos/Kareadita/Kavita`) -- `private static readonly string AppSettingsFilename = Path.Join("config", GetAppSettingFilename());` -- Kavita's own real source hardcodes exactly the `config/<filename>` relative-path convention the Nix module targets.
- **C_consumer**: proved. Same file, `GetJwtToken`/`GetPort`/etc. (lines ~91-180) -- `var json = File.ReadAllText(filePath); var jsonObj = JsonSerializer.Deserialize<AppSettings>(json);` -- a real `System.Text.Json` call, confirmed at the exact pinned tag.
- **D_contract**: proved. `Kavita.Common/Configuration.cs:375-391` -- `private sealed class AppSettings { public string TokenKey; public int Port; public string IpAddresses; public string BaseUrl; public long Cache; public bool AllowIFraming; public OpenIdConnectSettings OpenIdConnectSettings; }` -- a real, fully typed C# class with explicit fields, the cleanest possible bounded contract.
- **fit**: yes
- **reason_code**: SUPPORTED
- **notes**: `OsInfo.IsDocker` short-circuits `GetPort`/others to hardcoded defaults when running under Docker -- irrelevant here since the Nix/systemd deployment is never Docker, but worth disclosing as a real conditional in the consumer's own logic.

## nebula-lighthouse-service

- **A_producer**: proved. `fixtures/e1-holdout-audit/nebula-lighthouse-service/module.nix:12,48-49` -- `settingsFormat = pkgs.formats.yaml {}; environment.etc."nebula-lighthouse-service/config.yaml".source = settingsFormat.generate "nebula-lighthouse-service-config.yaml" cfg.settings;`. Real generated YAML via `environment.etc`, which NixOS resolves to the real path `/etc/nebula-lighthouse-service/config.yaml`.
- **B_binding**: proved -- and notably an IMPLICIT binding, not a CLI flag: `ExecStart = "${pkgs.nebula-lighthouse-service}/bin/nebula-lighthouse-service";` takes ZERO arguments. Confirmed against REAL upstream (`repos/manuels/nebula-lighthouse-service`, tag `v2.0.2`, package pinned via `pkgs/by-name/ne/nebula-lighthouse-service/package.nix`): `nebula_lighthouse_service/webservice.py:40` -- `CONFIG_PATH = Path('/etc/nebula-lighthouse-service/config.yaml')`, a hardcoded default in the real source that EXACTLY matches the Nix module's own `environment.etc` target path. A real, mechanically-verified binding despite having no CLI flag or env var at all.
- **C_consumer**: proved. `nebula_lighthouse_service/file_config.py` (same tag) -- `contents = path.read_text(); cfg = yaml.safe_load(contents)` -- a real `PyYAML` parser call.
- **D_contract**: proved. Same file -- `get_config(path, name, default)` used via four explicitly named wrapper functions: `get_min_port` (`'min-port'`), `get_max_port` (`'max-port'`), `get_webserver_port` (`'webserver.port'`), `get_webserver_ip` (`'webserver.ip'`). The underlying value is an untyped dict (`yaml.safe_load`'s own return type), but every real access is a bounded, explicitly named key lookup -- matches the protocol's own "bounded set of explicit parser field accesses" branch of D, not the unbounded-freeform-passthrough disqualifier.
- **fit**: yes
- **reason_code**: SUPPORTED
- **notes**: the STRONGEST example in this batch that B must be checked independently of A -- if `ExecStart` had taken even one CLI argument that didn't obviously reference the config, this would have needed real upstream confirmation before trusting it at all; the only reason B is provable here is the hardcoded default path being visible in real source.

## transmission

- **A_producer**: proved. `fixtures/e1-holdout-audit/transmission/module.nix:37-38` -- `settingsFormat = pkgs.formats.json {}; settingsFile = settingsFormat.generate "settings.json" cfg.settings;`. Real generated JSON.
- **B_binding**: proved. `module.nix:398-409` -- `ExecStartPre` merges `settingsFile` with `cfg.credentialsFile` via `jq --slurp add`, installs the result to `${cfg.home}/${settingsDir}/settings.json`; `ExecStart = "... transmission-daemon -f -g ${cfg.home}/${settingsDir} ...";` -- the real `-g`/`--config-dir` flag, pointing at the exact directory the merged settings file was just installed into.
- **C_consumer**: proved. Real pinned version `4.1.3` (`repos/transmission/transmission`, `pkgs/by-name/tr/transmission_4/package.nix`). `daemon/daemon.cc:95` -- `{ 'g', "config-dir", ... }` (confirms the real CLI flag); `daemon/daemon.cc:401-411` -- `load_settings(config_dir)` calls `tr_sessionLoadSettings(config_dir, &app_defaults)`, transmission's own internal session-settings loader, which reads `settings.json` from that directory via `tr_variant_serde::json()` (confirmed elsewhere in the same file, e.g. line 1072's `tr_variant_serde::json().to_string(settings_)`).
- **D_contract**: proved. `load_settings`'s own `app_defaults_map` uses `TR_KEY_watch_dir`, `TR_KEY_rpc_enabled`, `TR_KEY_start_paused`, `TR_KEY_pidfile`, etc. -- transmission's own internal `TR_KEY_*` quark-constant registry, a large but genuinely bounded, explicitly enumerable set of named keys (not a freeform map).
- **fit**: yes
- **reason_code**: SUPPORTED
- **notes**: the settings surface is large (transmission has dozens of real settings keys), but "large" and "unbounded" are different things -- every key is a named `TR_KEY_*` constant, not an arbitrary passthrough.

## i2pd

- **A_producer**: proved. `fixtures/e1-holdout-audit/i2pd/module.nix:397-425` -- `ini = pkgs.formats.iniWithGlobalSection {...}; i2pdConfig = gen "id" cfg.settings;` -- a real generated INI-family config (a customized `pkgs.formats.ini` variant with dotted-key flattening, `unwrapPrefixes`).
- **B_binding**: proved. `module.nix:530-541` -- `ExecStartPre` copies `i2pdConfig.conf` to `%T/conf` (via `loadCredentialsScript`); `ExecStart = [... "--datadir=%S/i2pd" "--conf=%T/conf" "--tunconf=%T/tunconf"];` -- a real `--conf=` flag pointing at that exact copied file. The module even self-verifies this at BUILD time (`module.nix:477-511`, a real `pkgs.runCommand` check that runs the actual `i2pd` binary with `--conf="$conf"` and inspects its own startup log) -- real, if indirect, corroborating evidence.
- **C_consumer**: proved. Real pinned version `2.61.0` (`repos/PurpleI2P/i2pd`, `pkgs/by-name/i2/i2pd/package.nix`). `libi2pd/Config.cpp:47` -- `("conf", value<std::string>()->default_value(""), "Path to main i2pd config file ...")`; `Config.cpp:486-501` -- `ParseConfig(const std::string& path)` -- `std::ifstream config(path, ...); store(boost::program_options::parse_config_file(config, m_OptionsDesc), m_Options);` -- a real, well-known C++ library (`boost::program_options`) call.
- **D_contract**: proved. `Config.cpp:50-113` (and more beyond what was read) -- dozens of explicit `.add_options()("key", value<T>()->default_value(...), "description")` registrations, including `"ipv4"`, `"ipv6"` (matching the Nix module's own `ipv4 = false; ipv6 = false;` settings verbatim) and dotted keys like `"http.enabled"`/`"limits.coresize"` matching the module's own `unwrapPrefixes` flattening convention exactly. A real, bounded, comprehensively-registered schema, despite Nix's own `cfg.settings` type being a FREEFORM attrset with no static NixOS-side schema at all.
- **fit**: yes
- **reason_code**: SUPPORTED
- **notes**: the most structurally interesting case in this batch -- Nix's own side is completely freeform/unschemaed (`cfg.settings` accepts arbitrary attrs), yet the REAL CONSUMER still has a fully bounded contract via `boost::program_options`. Proves D must be assessed from the CONSUMER's own source, never inferred from whether the Nix-side type happens to be freeform.
