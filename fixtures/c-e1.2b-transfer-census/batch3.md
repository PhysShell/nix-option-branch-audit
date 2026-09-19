# C-E1.2b batch 3: akkoma, vault, spacecookie

Population per protocol.md, frozen at `0fab0af`. All A/B re-verified via
real `nix eval --impure` against `PhysShell/nixpkgs`
@ `68740713a1d5904edf9ba92a998a522b1b6ce080` (same pin C-E1.1 and
C-E1.2a both already use). All C/D re-verified by fetching and reading
the real pinned consumer source fresh this round, not trusted from
`fixtures/c-e1.1-generated-config-audit/batch3.md`'s own citation.

## akkoma

```
A: proved, zero drift. Real ExecStart = /nix/store/89ir5yj32jqnzrcx4wl3191wwyfxr7wa-akkoma-env/bin/pleroma start
   -- BYTE-IDENTICAL store-path hash to batch3.md's own C-E1.1 citation.
   bindsTo = ["akkoma-config.service"] confirmed.
B: proved, same real mechanism as C-E1.1 found (AKKOMA_CONFIG_PATH wrapper env var + a
   separate akkoma-config.service unit copying the same real config.exs derivation into
   that runtime path before akkoma.service starts).
C: proved. lib/pleroma/config/release_runtime_provider.ex -> System.get_env("AKKOMA_CONFIG_PATH")
   -> Config.Reader.read!(config_path). Unchanged from C-E1.1.
D: proved, re-fetched directly (Gitea API, akkoma.dev -- gh api 404s here, matches
   the protocol's own expectation; used the real Gitea contents/raw endpoints instead):
   config/description.exs @ v3.19.0, 105482 bytes (byte-identical size to C-E1.1's own
   citation). Real structure confirmed by reading the actual file: a flat Elixir list of
   `%{group: :pleroma, key: <Module>, type: ..., children: [%{key: "field", ...}, ...]}`
   records -- i.e. genuinely a THREE-LEVEL real schema (group / module-key / field), not
   a flat field-name list.
```

**Can normalize into the existing model?** Structurally yes --
`ArtifactContent::StructuredValue` + `ConfigFormat::ElixirConf` both
already exist and fit; `flatten_structured_value` already produces
fully-qualified dotted paths (it never strips a prefix) which is
EXACTLY the granularity this schema needs. **Can use
`compare_config_contract()` unchanged?** Yes.

**Adversarial normalization check -- ran, and it caught something real.**
Grepped every field-level `key: "..."` / `key: :atom` entry across the
whole real `description.exs` and counted repeats:

```
18  :enabled
10  :api_key
 3  :url
 3  :local
 3  :headers
 2  :verify / :username / :tls / :secret / :region / :port / ...
```

`:enabled` alone appears under 18 DIFFERENT real module keys (Pleroma.
Upload.Filter.Mogrify, Pleroma.Captcha, several mailer/emoji/frontend
modules, ...) -- each a genuinely different real feature toggle, not
the same field repeated. A bare-keyword extractor modeled on `unbound`'s
own shipped fix (strip to the field name alone, drop the module-key
qualifier) would make `:enabled` (and `:api_key`/`:port`/`:username`/
`:secret`) semantically ambiguous across at least a dozen real,
distinct config values -- **check (a) is a real yes for akkoma.
Bare-keyword normalization is UNSOUND here.**

The good news: this doesn't actually require any new code to avoid,
because `flatten_structured_value` was never going to strip anything in
the first place -- it only STRIPS when a real D-extractor is written to
match a stripped shape (that's what `unbound`'s own fix specifically
did, on purpose, because ITS D-extractor only returns bare names). For
akkoma, the correct move is the opposite of `unbound`'s: write the new
D-extractor to emit FULLY QUALIFIED `group.key.field`-shaped paths
(matching what A already naturally produces unmodified), never
collapse to bare field names. Recorded as `FINDING` per the protocol's
own literal instruction for a real "(a) yes" -- but the finding is a
real, concrete WARNING against copying `unbound`'s own specific
shortcut here, not a broken abstraction; the existing type system
handles the fully-qualified case with zero changes.

```
verdict: FINDING (bare-keyword normalization would be unsound; full
  group.key.field qualification -- already the DEFAULT behavior of the
  existing flatten_structured_value, requiring no stripping at all --
  is the correct and sufficient design)
reuse: new generic locator required (a new description.exs-scanning
  D-extractor emitting fully-qualified paths; zero type changes)
```

## vault

```
A: proved, zero drift. Real ExecStart =
   /nix/store/06m5g7fqyiirmbzhiz0am0yyc5y3bsw1-vault-2.0.3/bin/vault server
     -config /nix/store/lv0zqj8yvyyyvpcmza42sqimhlv8mzfs-vault.hcl
   -- BYTE-IDENTICAL store-path hashes to batch3.md's own C-E1.1 citation.
B: proved, same real `-config <path>` flag mechanism.
C: proved, unchanged from C-E1.1 -- command/server.go's real `-config` flag ->
   LoadConfigFile -> ParseConfigCheckDuplicate -> hcl.DecodeObject.
```

**A real correction to C-E1.1's own original characterization, found by
actually reading the real `command/server/config.go` (hashicorp/vault
@ `v2.0.3`) in full this round, not just the snippet batch3.md quoted:**

The real module's own template (`fixtures/e1-holdout-audit/vault/module.nix`,
read directly) is:

```nix
configFile = pkgs.writeText "vault.hcl" ''
  ${lib.optionalString (!cfg.dev) ''
    listener "tcp" {
      address = "${cfg.address}"
      ${if (cfg.tlsCertFile == null || cfg.tlsKeyFile == null)
        then '' tls_disable = "true" ''
        else '' tls_cert_file = "..."; tls_key_file = "..."; ''}
      ${cfg.listenerExtraConfig}
    }
  ''}
  storage "${cfg.storageBackend}" {
    ${lib.optionalString (cfg.storagePath != null) ''path = "${cfg.storagePath}"''}
    ${lib.optionalString (cfg.storageConfig != null) cfg.storageConfig}
  }
  ${lib.optionalString (cfg.telemetryConfig != "") ''telemetry { ${cfg.telemetryConfig} }''}
  ${cfg.extraConfig}
'';
```

This is a real HAND-ROLLED text template with genuine HCL BLOCK syntax
(`block_type "label" { key = value }`), not a `pkgs.formats.*`
generator -- so **A's real `content` is `ArtifactContent::RenderedText`,
not `StructuredValue`** (C-E1.1's own batch3.md never stated which, and
the phrase "configFile = pkgs.writeText" was there, but this round is
the first to draw the actual consequence for the type system). With
every module option left at its real default (`dev=false`,
`address="127.0.0.1:8200"`, `storageBackend="inmem"`,
`tlsCertFile`/`tlsKeyFile`=null, `storagePath`/`storageConfig`=null,
`telemetryConfig`/`extraConfig`=""), the real rendered content is
(computed directly from the real template + real defaults, both cited
above -- not guessed):

```hcl
listener "tcp" {
  address = "127.0.0.1:8200"
  tls_disable = "true"
}
storage "inmem" {
}
```

**D re-checked against exactly this real emitted surface, not the
struct in the abstract, and it does NOT hold the way C-E1.1's own
citation implied.** Fetched the full real `command/server/config.go`
(1530 lines, not the earlier snippet). The `hcl:"..."`-tagged flat
scalar fields (`Experiments`, `CacheSize`, `PluginDirectory`, ...) are
real and genuinely bounded -- but **none of them are what the real
default config above actually emits.** The two blocks the real default
DOES emit are handled completely differently:

```go
Storage   *Storage `hcl:"-"`   // literally excluded from tag-based decode
HAStorage *Storage `hcl:"-"`
```
```go
func ParseStorage(result *Config, list *ast.ObjectList, name string) error {
    ...
    var config map[string]interface{}
    if err := hcl.DecodeObject(&config, item.Val); err != nil { ... }
    m := make(map[string]string)
    for k, v := range config { ... }   // GENUINELY FREEFORM -- any key accepted
    ...
}
```
```go
type Storage struct {
    Type              string
    RedirectAddr      string
    ClusterAddr       string
    DisableClustering bool
    Config            map[string]string   // freeform, confirmed at the type level too
}
```

`storage`'s inner body decodes into a bare `map[string]interface{}`
with NO fixed key set -- genuinely freeform, confirmed at both the
parse-function level and the `Storage.Config map[string]string` field
level. `listener` is handled even further away: `*configutil.
SharedConfig `hcl:"-"`` embeds an entirely separate real package
(`github.com/hashicorp/go-secure-stdlib/configutil`), not vendored or
followed further this round (out of this census's own bounded scope).

**So: the specific real fields this real default artifact emits
(`storage`'s block type + `listener.address`/`listener.tls_disable`)
are NOT boundedly extractable the way `unpackerr`'s flat
`toml:"..."`-tagged struct or `spacecookie`'s explicit Aeson instance
are** -- they go through a freeform map decode (storage) or an
unfollowed external package (listener). The dozens of real, genuinely
bounded `hcl:"..."` scalar fields exist, but a real default-configured
`vault` module never touches them.

```
verdict: INCONCLUSIVE(D is not boundedly extractable for the specific
  fields the real default-configured artifact actually emits -- the
  bounded hcl:"..." struct C-E1.1 cited is real but unexercised by this
  artifact; the exercised fields go through a freeform map decode
  (storage) or an unfollowed external package (listener))
reuse: new generic locator required for A (a RenderedText extractor
  recognizing real HCL `block_type "label" { ... }` syntax -- a
  genuinely different rendered-text SHAPE from unbound's own
  `section:\n  key: value` line format, not reusable as-is); the
  correct, honest way to represent the storage/listener blocks once
  such an extractor exists is to mark their BODIES opaque (the exact
  same "can't verify a real path in a bounded way -> opaque_paths, not
  a guess" precedent flatten_structured_value already uses for a JSON
  array) rather than attempt to compare an unbounded map -- this is a
  correct REUSE of the existing opaque_paths concept, not a new one,
  but the resulting comparison would then be close to vacuous (little
  to nothing left in emitted_paths for vault's real default config).
```

## spacecookie

```
A: proved, zero drift. Real ExecStart =
   /nix/store/r0rdiylxnjshjqx2glq8n6riz36h165r-spacecookie-1.1.0.1/bin/spacecookie
     /nix/store/9jcdhjmkiwiziyrf53flcky2jbsqhps9-spacecookie.json
   -- BYTE-IDENTICAL store-path hashes to batch3.md's own C-E1.1 citation.
B: proved, same real bare-positional-argument mechanism.
C: proved, unchanged -- real Hackage source @ 1.1.0.1, `server/Main.hs`'s
   `getOpt Permute options args -> ([], [configFile], []) -> runServer` ->
   `eitherDecodeStrict'`.
D: proved, RE-FETCHED FRESH this round (real Hackage tarball
   spacecookie-1.1.0.1.tar.gz, not trusted from the earlier citation) --
   full real server/Network/Spacecookie/Config.hs read in this round:

    instance FromJSON Config where
      parseJSON (Object v) = Config
        <$> v .: "hostname"
        <*> maybePath [ "listen", "addr" ] v
        <*> parseListenPort v .!= 70
        <*> v .:? "user"
        <*> v .: "root"
        <*> v .:? "log" .!= defaultLogConfig

    parseListenPort v = (<|>)
      <$> maybePath [ "listen", "port" ] v
      <*> (v .:? "port")

    instance FromJSON LogConfig where
      parseJSON (Object v) = LogConfig
        <$> v .:? "enable"     .!= ...
        <*> v .:? "hide-ips"   .!= ...
        <*> v .:? "hide-time"  .!= ...
        <*> v .:? "level"      .!= ...
```

Real accepted paths, confirmed directly from this source: `hostname`,
`listen.addr`, `listen.port` OR bare `port` (a genuine either/or --
`parseListenPort` tries both), `user`, `root`, `log.enable`,
`log.hide-ips`, `log.hide-time`, `log.level`.

**Real producer shape, re-checked directly against `fixtures/
e1-holdout-audit/spacecookie/module.nix`:**

```nix
spacecookieConfig = { listen = { inherit (cfg) port; }; } // cfg.settings;
```

-- the module's own top-level `port` option is nested under
`listen.port` in the real emitted JSON (a direct, real match to one of
the two real accepted paths above); `cfg.settings` supplies `hostname`/
`root`/`log.*` directly, matching the real submodule option tree
1-for-1 (confirmed by reading the module's own `options.settings`
submodule, which already declares `hostname`, `root`, and a nested
`log = { enable; hide-ips; hide-time; level; }` block, mirroring the
Haskell schema's own real nesting exactly). `listen.addr` is never
emitted by the real producer (the module's own `address` option
targets the systemd socket unit, not the JSON content) -- a real,
disclosed non-collision (an accepted-but-never-emitted path, not a
problem either way for `compare_config_contract`, which only ever
checks the emitted side).

**Adversarial normalization check: no stripping needed at all.** Every
real emitted path (`listen.port`, `hostname`, `root`, `log.enable`,
`log.hide-ips`, `log.hide-time`, `log.level`) is already exactly
dotted-leaf-path shaped, and `flatten_structured_value` produces
EXACTLY this shape unmodified -- no bare-keyword collapsing anywhere,
so check (a)/(b) doesn't even arise here the way it does for `akkoma`.
Confirmed no cross-nesting collision is possible: `port` only ever
occurs under `listen.` or bare-top-level (both explicitly, genuinely
accepted by the real parser itself, not a coincidence of a small
fixture); `enable`/`hide-ips`/`hide-time`/`level` only ever occur under
`log.`.

```
verdict: PASS
reuse: new generic locator required (a new bounded literal scan over
  the real `.: "X"` / `.:? "X"` / `maybePath [ "X", "Y" ]` Aeson call
  shapes -- a new per-consumer D-extractor, matching every anchor in
  C-E1.2a already needing its own; zero type changes, zero stripping,
  compare_config_contract unchanged)
```

## Batch 3 summary

- **spacecookie**: clean `PASS`. Existing types transfer with zero
  modification; the only new code needed is a new bounded D-extractor
  (expected, normal, matches every C-E1.2a anchor's own precedent).
- **akkoma**: `FINDING` -- not because anything is broken, but because
  the adversarial check found a REAL, concrete reason `unbound`'s own
  bare-keyword-stripping shortcut must NOT be copied here (18-way real
  collision on `:enabled` alone). The fix requires no type changes:
  just don't strip. `flatten_structured_value` already does the right
  thing by default; only a new D-extractor is needed, and it must
  preserve full qualification.
- **vault**: `INCONCLUSIVE` -- a real, concrete gap this round found
  that C-E1.1's own original citation didn't surface: the real default
  artifact's own emitted content (`storage`/`listener` HCL blocks) is
  governed by freeform-map decode + an unfollowed external package, NOT
  the bounded `hcl:"..."` struct fields C-E1.1 cited as D's proof
  (those are real, but genuinely unexercised by the real default
  config). A's own `content` type is also `RenderedText`, not
  `StructuredValue` -- a real correction to an implicit assumption, not
  previously stated explicitly either way.
- 2/3 in this batch show the pattern the whole census is watching for:
  real complications that the EXISTING type system already has the
  right vocabulary for (`opaque_paths` for vault's freeform blocks,
  full-qualification for akkoma) rather than needing anything new or
  app-specific -- but neither is a clean, no-caveats transfer the way
  C-E1.2a's own four anchors were.
