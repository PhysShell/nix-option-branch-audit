# C-E1.1 batch 1: libinput, privoxy, nohang, misskey

Protocol: `fixtures/c-e1.1-generated-config-audit/protocol.md`. All
producer-side (A/B) evidence from the real vendored
`fixtures/e1-holdout-audit/<name>/module.nix` (E1's own files, not
re-fetched) plus real `nix eval` where cited. All consumer-side (C/D)
evidence from REAL upstream source fetched fresh this round, at the
EXACT version/tag nixpkgs pins (confirmed from each package's own real
`package.nix`), never assumed or guessed. Zero `src/` changes.

---

## libinput

```
A_producer: no
  Evidence: fixtures/e1-holdout-audit/libinput/module.nix has NO
  pkgs.writeText/writeTextFile/format.generate call at all for the
  per-option Xorg settings. The one `environment.etc` entry
  (`X11/xorg.conf.d/40-libinput.conf`) is a STATIC file shipped INSIDE
  the `xf86-input-libinput` package itself -- confirmed by a real
  `nix eval --impure --json` against tree
  68740713a1d5904edf9ba92a998a522b1b6ce080 with
  `services.libinput.enable = true; services.xserver.enable = true;`:
  `environment.etc."X11/xorg.conf.d/40-libinput.conf".source` resolves
  to `/nix/store/1gm3zfxy7mwbylc94nqh3fm1ki0bkkv8-xf86-input-libinput-1.5.0/share/X11/xorg.conf.d/40-libinput.conf`
  -- a path INSIDE the already-built package, not derived from
  `cfg.mouse`/`cfg.touchpad`'s own live option values.
  The REAL per-option rendered text (`mkX11ConfigForDevice`, reflecting
  cfg's own values) instead becomes an entry in
  `services.xserver.inputClassSections` -- confirmed by the SAME real
  eval: a list of two real strings (`Option "AccelProfile" "adaptive"`
  etc., correctly reflecting the evaluated cfg values). This is a LIST
  FRAGMENT contributed to a DIFFERENT, downstream module's (xserver's)
  own eventual config assembly -- xserver.nix is not vendored here and
  its own artifact-generation mechanism was not traced. There is no
  single, mechanically-identifiable "the artifact" this module itself
  produces.
B_binding:  inconclusive (moot -- A already fails; would require
  tracing xserver.nix's own unvendored artifact assembly)
C_consumer: not assessed (A fails first)
D_contract: not assessed (A fails first)
fit: no
reason_code: NO_PRODUCER_PROOF
notes: A real, interesting NEGATIVE result -- libinput LOOKS like a
  generated-config case at a glance (E1's own quick structural pass),
  but the real mechanism is "contributes a fragment to ANOTHER
  module's own artifact via a plain Nix list", not "generates its own
  config file". Worth remembering as a real false-positive risk for
  this whole family: "renders option values into a string" is not the
  same claim as "generates an artifact", and conflating the two is
  exactly the loose pattern-matching this audit exists to catch.
```

## privoxy

```
A_producer: proved
  Evidence: fixtures/e1-holdout-audit/privoxy/module.nix:47-55 --
  `configFile = pkgs.writeText "privoxy.conf" (concatStrings ([
  "confdir ${pkgs.privoxy}/etc\n" ] ++ mapAttrsToList serialise
  cfg.settings));` -- a real `pkgs.writeText` derivation, rendering the
  real, typed `cfg.settings` option (a `types.submodule` with a bounded
  `freeformType`) through a bespoke line-based `serialise` function
  (line 14-21: `"${name} ${toString val}\n"`, list values repeated,
  bools as "1"/"0"). A real generated artifact, mechanically confirmed
  from source (a live nix eval of the exact rendered content wasn't
  additionally run -- the `pkgs.writeText` call itself, reading real
  option values, is already unambiguous proof of real generation, same
  bar K1's own DSN-in-config-file evidence used).
B_binding:  proved
  Evidence: module.nix:241 -- `ExecStart = "${pkgs.privoxy}/bin/privoxy
  --no-daemon ${configFile}";` -- the SAME `configFile` derivation
  (Nix string interpolation of the exact same local binding, not a
  second, separately-constructed path) is privoxy's own positional
  config-file argument.
C_consumer: proved
  Evidence: real privoxy 4.2.0 source (the EXACT version
  `pkgs/by-name/pr/privoxy/package.nix` pins), fetched from the real
  SourceForge URL the nixpkgs derivation itself uses
  (`mirror://sourceforge/ijbswa/Sources/4.2.0 (stable)/privoxy-4.2.0-stable-src.tar.gz`).
  `jcc.c:5710-5711`: the last unrecognized command-line argument
  becomes the global `configfile` (`configfile = argv[argc_pos];`).
  `jcc.c:5818/6223/6291`: `load_config()` is called with no argument,
  reading that same global. `loadcfg.c:694-704`: `load_config()`
  really opens and reads `configfile` via `check_file_changed`+`fopen`
  (not a stub).
D_contract: proved
  Evidence: `loadcfg.c:130-169` -- a real, bounded, NAMED
  directive-hash dispatch table (`hash_actions_file` = hash of the
  literal string `"actionsfile"`, `hash_confdir` = hash of
  `"confdir"`, `hash_enable_edit_actions`, `hash_filterfile`,
  `hash_listen_address`, ...) mapping each real config directive NAME
  -- the EXACT same key spellings the Nix module's own `cfg.settings`
  schema uses (`listen-address`, `actionsfile`, `filterfile`,
  `enable-edit-actions`) -- to typed fields on `struct
  configuration_spec *config`. A real, finite, named contract, not a
  freeform blob.
fit: yes
reason_code: SUPPORTED
notes: The cleanest, most straightforward SUPPORTED case in this
  batch -- one real derivation, one real binding, one real parser with
  a real named-directive table matching the exact Nix-side keys.
```

## nohang

```
A_producer: no
  Evidence: fixtures/e1-holdout-audit/nohang/module.nix has NO
  pkgs.writeText/writeTextFile/format.generate call anywhere. The
  `configPath` option (line 29-41) is `types.either (types.enum [
  "basic" "desktop" ]) types.path`, defaulting to `"desktop"`. In
  `config` (line 54-63), the real `ExecStart` resolves to
  `"${cfg.package}/etc/nohang/nohang-desktop.conf"` for the default
  case -- a STATIC file shipped INSIDE the upstream `nohang` PACKAGE
  itself (its own bundled example config), never rendered from any
  Nix-level structured option values by this module. The `custom path`
  branch (an arbitrary `types.path`) is a raw passthrough to a
  user-supplied file, not Nix-generated either.
B_binding:  moot -- there is no Nix-generated artifact for this to
  bind.
C_consumer: not assessed (A fails first)
D_contract: not assessed (A fails first)
fit: no
reason_code: NO_PRODUCER_PROOF
notes: A second real negative result of the SAME general shape as
  libinput -- looked like "generated config" at a glance, but on real
  inspection there is ZERO Nix-rendered config content anywhere in
  this module. `configPath`'s own real options are "use the package's
  own bundled static file" or "point at an arbitrary user file" --
  neither is a Nix-generated artifact. This is a distinct root cause
  from libinput's own (fragment-feeds-another-module) -- worth keeping
  separate in the final aggregation, not conflated as "the same
  reason".
```

## misskey

```
A_producer: proved
  Evidence: fixtures/e1-holdout-audit/misskey/module.nix:10 --
  `settingsFormat = pkgs.formats.yaml { };`, and line 331:
  `${settingsFormat.generate "misskey-config.yml" cfg.settings}` (a
  real, typed `format.generate` call rendering the real `cfg.settings`
  option). One real nuance, disclosed rather than glossed over: the
  generated file is NOT installed byte-identical -- `ExecStartPre`
  (lines 330-344) first `install`s the generated YAML to
  `/run/misskey/default.yml`, THEN (for any secret-bearing option that
  was set) runs `replace-secret` to substitute placeholder strings
  (`@DATABASE_PASSWORD@` etc.) with real secret-file contents at
  service start. For a deployment with no secrets set, the installed
  file IS byte-identical to the pure Nix-evaluated content; with
  secrets, the final content depends on a real but non-Nix-evaluable
  runtime substitution step. A is still proved (the MECHANISM is real
  and the base content is Nix-generated), just with this caveat
  attached.
B_binding:  proved
  Evidence: module.nix:326-328 -- `environment = { MISSKEY_CONFIG_YML
  = "/run/misskey/default.yml"; };` -- the EXACT same real path the
  generated (+ secret-patched) file is installed to in the SAME
  `ExecStartPre` block.
C_consumer: proved -- required tracing through TWO real chained
  scripts, not one obvious call site.
  Evidence: real misskey 2026.6.0 source (the exact tag
  `pkgs/by-name/mi/misskey/package.nix` pins, `misskey-dev/misskey`).
  `package.json:39`: `"migrateandstart": "pnpm migrate && pnpm start"`.
  `package.json:31`: `"start": "cd packages/backend && pnpm
  compile-config && node ./built/entry.js"` -- `compile-config` runs on
  EVERY real service start, not just at nixpkgs's own package-build
  time (a real risk this audit specifically needed to rule out: if
  `compile-config` only ran once during the nixpkgs build, the Nix
  module's own runtime env var would be inert). `packages/backend/
  scripts/compile_config.js:42-48`: reads
  `process.env.MISSKEY_CONFIG_YML`, resolves it relative to `.config/`,
  parses via `yaml.load` (js-yaml), writes the real compiled
  `.config.json`. `packages/backend/src/config.ts:254`:
  `loadConfig()` reads that same compiled file
  (`JSON.parse(fs.readFileSync(compiledConfigFilePath, 'utf-8')) as
  Source`) at real process startup.
D_contract: proved
  Evidence: `packages/backend/src/config.ts:24-77` -- a real, explicit,
  bounded TypeScript `type Source = { url?, port?, socket?, db: {host,
  port, db, user, pass, ...}, redis: RedisOptionsSource, meilisearch:
  {host, port, apiKey, ...}, ... }` -- a real typed schema naming every
  accepted top-level and nested key, not a formless
  `Record<string,unknown>`.
fit: yes
reason_code: SUPPORTED
notes: The most rigorously chased SUPPORTED case in this batch -- C
  required confirming the config-compile step actually re-runs at
  every real service start (not just once at package-build time),
  which took two real source files to establish, not one. Also the
  strongest real-world confirmation in this batch that B genuinely
  needs independent verification: the artifact IS regenerated with a
  runtime secret-substitution step, a real nuance A alone wouldn't
  have surfaced.
```

## Batch 1 summary

- **2/4 SUPPORTED** (privoxy, misskey) -- both with fully real, cited
  A/B/C/D evidence, no step assumed from another.
- **2/4 NO_PRODUCER_PROOF** (libinput, nohang) -- two DIFFERENT real
  reasons, not the same gap: libinput's real per-option config is a
  fragment feeding a DIFFERENT, unvendored module's own artifact
  assembly; nohang has NO Nix-rendered config content at all (static
  package-bundled file or raw user passthrough only). Both looked like
  "generated config" cases from a quick structural read (E1's own
  original classification) -- neither survives real verification.
- Both SUPPORTED cases needed genuinely different, non-trivial C-chain
  tracing (privoxy: a single, direct `argv -> global -> load_config()`
  path; misskey: two real chained npm scripts, with a real risk --
  build-time-only compilation -- that had to be explicitly ruled out,
  not assumed away).
