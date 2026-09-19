# C-E1.2b batch 1: privoxy, misskey

Protocol: `fixtures/c-e1.2b-transfer-census/protocol.md`. Frozen at
`0fab0af`; zero `src/` changes made or needed to produce this batch.
A/B/C/D re-verified from real, fresh evidence (real `nix eval` against
`PhysShell/nixpkgs` tree `68740713a1d5904edf9ba92a998a522b1b6ce080` --
the same pin already used throughout this project's real evidence base,
reused here rather than re-resolving a fresh tip, for direct
comparability with every other real citation in this repo; real
upstream source fetched fresh, not trusted from
`fixtures/c-e1.1-generated-config-audit/batch1.md`'s own citation
without independently re-reading it).

---

## privoxy

### A/B/C/D re-verification

**A -- PROVED, re-confirmed by a real build+read, not just source
reading.** Real `nix eval --impure` against a minimal
`services.privoxy.enable = true; settings = { listen-address = ...;
enable-edit-actions = true; };` config, then `builtins.substring`-sliced
the real `ExecStart` (NOT `builtins.match` -- this project's own
documented string-context-loss lesson, confirmed to still apply here)
to get the real `configFile` store path and `builtins.readFile` its
real content:

```
confdir /nix/store/k4f2h0jzc378icq8igv0fim956xqd5wv-privoxy-4.2.0/etc
actionsfile match-all.action
actionsfile default.action
enable-edit-actions 1
filterfile default.filter
listen-address 127.0.0.1:8118
temporary-directory /tmp
user-manual /nix/store/k4f2h0jzc378icq8igv0fim956xqd5wv-privoxy-4.2.0/share/doc/privoxy/user-manual
```

**A real shape NOT previously noted**: `actionsfile` appears TWICE --
`cfg.settings.actionsfile`'s own real default is a Nix LIST
(`["match-all.action" "default.action"]`), and `serialise`'s own
`isList` branch (module.nix:15-16) emits one line per element. Same
"a real list becomes a repeated key" shape unbound's own
`access-control` already established, but in a format with NO section
structure at all (see below).

**B -- PROVED, re-confirmed.** Real `ExecStart` =
`/nix/store/.../privoxy-4.2.0/bin/privoxy --no-daemon
/nix/store/vj4rnvggibgfx44fjfjdb0yfj7bb7lf8-privoxy.conf` -- the exact
same derivation A read, as a bare POSITIONAL argument (no `--flag=`
name at all, unlike unpackerr's own `DirectArgv`).

**C -- PROVED, re-confirmed against real fetched v4.2.0 source**
(`sourceforge.net/.../privoxy-4.2.0-stable-src.tar.gz`, the exact real
URL `pkgs/by-name/pr/privoxy/package.nix` uses). `loadcfg.c` has
exactly ONE `switch (directive_hash)` block in the whole file (line
846, confirmed via `grep -c '^switch'` style search across the file --
no second dispatch table anywhere), fed by `directive_hash =
hash_string(cmd)` reading whatever directive name appears on each
config line. A single, flat, global dispatch -- not sectioned.

**D -- PROVED, and a real bounded scan pattern.** `loadcfg.c:130-169`+
is a real, exhaustive `#define hash_X <NUMBER>U /* "directive-name" */`
table -- e.g. `#define hash_actions_file 1196306641U /* "actionsfile"
*/`, `#define hash_listen_address 1255650842U /* "listen-address" */`.
The real accepted directive name is LITERALLY present in each line's
own trailing C comment -- a clean, bounded, single-pass regex target
(`#define hash_\S+\s+\d+U\s*/\*\s*"([^"]+)"`), matching this project's
existing "bounded literal scan over one real observed source shape"
discipline exactly (same family as `extract_go_flagset_literal_names`/
`extract_go_toml_tags`), not a new kind of extraction technique.

### Fit against the EXISTING code (`src/cdc.rs` @ `0fab0af`)

- `ArtifactContent::RenderedText(String)` -- fits AS-IS (a hand-rolled
  line-based serializer, same shape as unbound).
- `ConfigFormat::HandRolled` -- fits AS-IS, no new variant needed.
- `ArtifactBindingEvidence` -- **does NOT fit any existing variant.**
  `DirectArgv { flag: String, argv: String }` assumes a NAMED flag
  (unpackerr's `--config=`); privoxy's real binding is a bare
  positional argument with no flag name at all. Forcing `flag =
  String::new()` into the existing variant would be exactly the
  "silently forced into an existing variant that doesn't really
  describe it" the protocol warns against. **A new, still-general
  variant is needed** (e.g. `DirectPositionalArg { argv: String }`) --
  a real, reusable binding SHAPE (any consumer taking its config path
  as a bare positional CLI argument), not privoxy-specific.
- `extract_unbound_style_paths` -- **does NOT correctly parse
  privoxy's own real rendered format.** That function requires (1) a
  non-indented line ending in `:` to open a section, (2) only INDENTED
  lines to become `section.key: value` entries. Privoxy's real content
  has ZERO indentation and ZERO section headers -- every line is
  top-level `key value` (space-separated, no colon at all, e.g.
  `listen-address 127.0.0.1:8118`). Run mentally against
  `extract_unbound_style_paths`: every line fails the `starts_with
  (char::is_whitespace)` check (none are indented), so every line is
  treated as a potential section-header candidate; none end in `:`
  (colon appears mid-value for `listen-address`'s own port syntax, not
  at end-of-line), so `section` never gets set and the function would
  return EMPTY `emitted`/`opaque` for the ENTIRE file. **A genuinely
  new rendered-text extractor is required** for this flat, no-section,
  space-separated shape -- structurally similar in spirit (line-based,
  repeated-key -> opaque) but a different real parser, not a config
  tweak to the existing one.
- `compare_config_contract()` -- unchanged, once both sides are
  extracted at the correct (flat, bare-key) granularity.

### Adversarial normalization check

Not applicable in the risky sense the protocol is watching for:
privoxy's real rendered format has **no sections to begin with** --
there is nothing to strip or discard, since nothing was ever qualified
by a section in the first place. Confirmed from the real consumer
source too: `loadcfg.c` has exactly one flat `switch(hash_string(cmd))`
dispatch, no per-section sub-dispatch anywhere -- the real accepted
contract is genuinely flat at the semantic level, matching case (b) of
the protocol's own check, on the strongest possible grounds (there's
no section concept anywhere in this consumer's own real grammar, not
just "no observed collision in a small excerpt").

### Verdict

**PASS.** Reuse classification: **new generic locator required** --
one new `ArtifactBindingEvidence::DirectPositionalArg`-shaped variant
(general, reusable), one new flat/no-section rendered-text extractor
(A-side, general shape), one new `#define ... /* "name" */`
C-source scanner (D-side, general shape, same family as existing Go
scanners). Zero app-specific branching required anywhere;
`compare_config_contract()` stays untouched.

---

## misskey

### A/B/C/D re-verification

**A -- PROVED, re-confirmed via real `nix eval`**, deliberately with
NO `passwordFile`/`keyFile` options set (avoiding C-E1.1's own already-
disclosed secret-substitution caveat -- a base config with no secrets
means the installed file is byte-identical to the pure Nix-evaluated
content, matching batch1.md's own stated condition for A to cleanly
hold). Real `cfg.settings` evaluates to a genuinely NESTED JSON object:

```json
{
  "db": {"db": "misskey", "host": "/var/run/postgresql", "pass": null, "port": 5432, "user": "misskey", ...},
  "redis": {"host": "localhost", "port": 6379},
  "redisForPubsub": null, "redisForJobQueue": null, "meilisearch": null,
  "port": 3000, "url": "https://misskey.example.org/", "id": "aidx", ...
}
```

**B -- PROVED, but a genuinely NEW real mechanism, re-confirmed via
real `nix eval`**: `systemd.services.misskey.environment.
MISSKEY_CONFIG_YML = "/run/misskey/default.yml"` (a FIXED runtime
path, not a store path) + a real `ExecStartPre` step:
`install -m 700 /nix/store/.../misskey-config.yml
/run/misskey/default.yml`, confirmed to run BEFORE `ExecStart`
(`migrateandstart`) on every real service start. A SECOND real
`ExecStartPre` also installs a JSON rendering of the SAME `cfg.settings`
to `/run/misskey/default.json` -- real, but not the file the consumer
actually reads per C below, disclosed not chased further (out of this
census's own scope).

**C -- PROVED, re-confirmed against real fetched v2026.6.0 source.**
`package.json`: `"start": "cd packages/backend && pnpm compile-config
&& node ./built/entry.js"`, `"migrateandstart": "pnpm migrate && pnpm
start"` -- re-confirmed byte-for-byte matching batch1.md's own
citation. `compile_config.js` -- **one real precision beyond batch1.md's
own citation, worth stating exactly**: `const customYmlPath =
resolve(configDir, process.env.MISSKEY_CONFIG_YML);` looks like a
`.config/`-RELATIVE join at a glance, but since `MISSKEY_CONFIG_YML`'s
real value (`/run/misskey/default.yml`) is itself an ABSOLUTE path,
Node's own `path.resolve(base, path)` semantics make a later absolute
segment override the base entirely -- the real resolved path IS
`/run/misskey/default.yml` exactly, not a `configDir`-nested path. Same
real conclusion batch1.md already reached, stated more precisely here.

**D -- PROVED, real and bounded, but genuinely two-hop.**
`packages/backend/src/config.ts`'s real `type Source = { url?: string;
port?: number; db: {host, port, db?, user?, pass?, ...}; redis:
RedisOptionsSource; redisForPubsub?: RedisOptionsSource; meilisearch?:
{host, port, apiKey, ...}; dbSlaves?: {...}[]; ...}` -- a real, bounded
TypeScript type literal, confirmed by direct read. `redis`/
`redisForPubsub`/`redisForJobQueue`/`redisForTimelines` are all typed
by a SEPARATELY declared `type RedisOptionsSource = {...}` a few lines
above -- correctly extracting the real accepted paths for a CONFIGURED
`redis.*` sub-object (the real corpus case: this round's own re-eval
above shows `redis` really is set, non-null) needs a SECOND bounded
scan of `RedisOptionsSource`'s own declaration, not just the outer
`Source` block. `dbSlaves?: {...}[]` and `allowedPrivateNetworks?:
string[]`/`proxyBypassHosts?: string[]` are real array-typed fields --
consistent with `flatten_structured_value`'s own array-is-opaque rule
(an accepted key whose real value would be an array is simply never
exercised by `compare_config_contract`, since the array-shaped emitted
path never leaves `opaque_paths`).

### Fit against the EXISTING code (`src/cdc.rs` @ `0fab0af`)

- `ArtifactContent::StructuredValue(JsonValue)` + `flatten_structured_
  value` -- fits AS-IS, confirmed directly against the real nested
  `cfg.settings` JSON above (`db.host`, `redis.port`, etc. all come out
  as real dotted leaf paths with zero changes needed; `redisForPubsub`
  being `null` in this base config correctly becomes a bare top-level
  leaf `"redisForPubsub"`, not expanded -- matches the real TS
  optional-field convention exactly, a genuinely lucky/correct
  coincidence worth naming: it means D's own bounded top-level scan
  alone already correctly covers the UNSET case, and only a configured
  `redis`/`redisForPubsub`/etc. sub-object needs the second, nested
  scan).
- `ConfigFormat::Yaml` -- fits AS-IS.
- `ArtifactBindingEvidence` -- **does NOT fit any existing variant.**
  Not `DirectArgv` (no CLI flag at all). Not `EnvironmentEtcSymlink`
  (no `environment.etc` entry -- this is systemd's own `environment`
  field plus an imperative `ExecStartPre install` step, a real, general,
  DIFFERENT mechanism: activation-time Nix-level symlinking vs. a
  per-service-start shell install command). Not `WrapperScriptEnvVar`
  (no `makeWrapper`-generated launcher exists here at all -- the env
  var is set directly on the systemd unit, and the file is staged by a
  separate `ExecStartPre`, not by the wrapper reading/exec'ing
  anything). Not `ImplicitDefaultPath` (the var is set EXPLICITLY by
  the unit, not hardcoded on the consumer's own side). **A new,
  general variant is needed** -- e.g. `ExecStartPreInstalledEnvVar {
  var_name: String, install_path: String }` -- representing "an env
  var pointing at a fixed runtime path a real `ExecStartPre` step
  populates before the main process starts," a common enough real
  NixOS idiom (secret-substitution patterns in particular) to be worth
  a real, reusable variant, not misskey-specific.
- `compare_config_contract()` -- unchanged.

### Adversarial normalization check

**Not applicable, and for a structurally different, stronger reason
than privoxy's**: misskey's D-extraction is over a NESTED type
(`Source`/`RedisOptionsSource`), and the corresponding A-side
extraction (`flatten_structured_value`) ALREADY preserves full,
non-discarded dotted paths (`redis.host` stays distinct from a
hypothetical `db.host`-shaped collision by construction -- nothing is
ever stripped). The section-collision risk this whole adversarial
check exists to catch is specific to TEXT-based bounded extractors that
have to actively THROW AWAY structure to normalize (unbound's/
privoxy's own rendered-text shape) -- misskey's real JSON/YAML path
carries no such risk at all, since `flatten_structured_value` never
discards a path segment in the first place.

### Verdict

**PASS.** Reuse classification: **new generic locator required** --
one new `ArtifactBindingEvidence` variant (`ExecStartPreInstalledEnvVar`
or similarly named, general and reusable), one new two-hop TypeScript
type-literal scanner for D (top-level `Source` fields + one referenced
named type's own fields -- more work than a single-block scan, still a
bounded literal-scan family, not a new semantic abstraction). Zero
app-specific branching required; `compare_config_contract()` stays
untouched.

---

## Batch 1 summary

- **2/2 PASS** (privoxy, misskey).
- **Cross-cutting signal for the parent census to weigh**: BOTH
  candidates in this batch need a genuinely new `ArtifactBindingEvidence`
  variant -- privoxy (bare positional argv) and misskey (ExecStartPre-
  installed runtime-path env var) are two DIFFERENT new shapes, neither
  matching each other or any of the 4 existing anchors. If other
  batches independently surface a bare-positional-argv shape too (the
  parent's own C-E1.1 census already flagged `spacecookie`'s own real
  binding as "a bare POSITIONAL argument (not a flag)" -- worth
  checking directly against whichever batch covers it), that is exactly
  the "several candidates independently require the SAME repeating new
  abstraction" branch of the decision rule, not two isolated one-offs.
- Neither candidate needed a new `ArtifactContent`/`ConfigFormat`
  variant, and neither needed `compare_config_contract()` touched.
  Zero app-specific branching found necessary for either.
- The adversarial normalization check found NO real risk in this
  batch, for two structurally different reasons worth keeping
  separate: privoxy's real format has no sections at all (nothing to
  discard), misskey's real extraction never discards path segments in
  the first place (JSON/YAML structured extraction, not text
  normalization). Neither is the SAME kind of "safe" as unbound's own
  case (which specifically had sections it chose to discard, justified
  by D's own bare granularity) -- worth the parent keeping these as
  three distinct sub-cases, not one bucket, when it writes the final
  census.
