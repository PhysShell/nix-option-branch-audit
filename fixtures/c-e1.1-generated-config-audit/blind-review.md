# C-E1.1 blind second-pass review

Independent, blind. Did not read batch1-4.md or any prior C-E1.1 git history.
Candidates: libinput, nohang, akkoma, privoxy, mobilizon.
Tree used for A/B nix eval: PhysShell/nixpkgs @ 68740713a1d5904edf9ba92a998a522b1b6ce080.

## libinput

A_producer: no
Evidence: `services.libinput` module.nix's only `environment.etc` entry
(`X11/xorg.conf.d/40-libinput.conf`) is a `source =` symlink to a file that
ships INSIDE the `xf86-input-libinput` package itself, not content rendered
from `cfg.mouse`/`cfg.touchpad` options. Confirmed via real nix eval
(instantiated `services.libinput.enable + services.xserver.enable` on the
pinned tree):
```
environment.etc."X11/xorg.conf.d/40-libinput.conf".source =
  /nix/store/1gm3zfxy7mwbylc94nqh3fm1ki0bkkv8-xf86-input-libinput-1.5.0/share/X11/xorg.conf.d/40-libinput.conf
```
That path is identical regardless of any option value — no `writeText`/
`writeTextFile`/`format.generate` call anywhere in this module renders it.
The actual option-driven text (`Option "AccelProfile" ...` etc., confirmed
present via the same eval on `services.xserver.inputClassSections`) is
handed off as a list of strings to `services.xserver` (a different,
non-vendored module) to fold into its own generated xorg.conf — this
module never itself writes that artifact to a store path or `/etc`.

B_binding: no (no artifact of this module's own to bind)
C_consumer: not pursued (chain already broken at A)
D_contract: not pursued (chain already broken at A)

fit: no
reason_code: NO_PRODUCER_PROOF
notes: This module doesn't generate the artifact it superficially looks
like it should generate — the module source "mentions" the etc file, but
the content is package-static, and the true user-option content is
produced by a module outside this audit's vendored scope.

## nohang

A_producer: no
Evidence: real nix eval of `services.nohang.enable = true` (default
`configPath = "desktop"`) gives:
```
ExecStart = "/nix/store/.../nohang-0.3.0/bin/nohang --monitor --config /nix/store/.../nohang-0.3.0/etc/nohang/nohang-desktop.conf"
```
That config path is *inside the nohang package derivation itself*
(a file shipped by upstream's own build, installed via the package's
Makefile), not something the NixOS module generates from `cfg` options.
The module's only other choice is `configPath = "basic"` (same
package-static mechanism) or an arbitrary user-supplied `types.path`
(external file, again not module-generated). There is no
`writeText`/`format.generate` anywhere in this module.

B_binding: proved as a mechanism (ExecStart's `--config` argument is
mechanically the exact same store path as whichever option value was
selected — no ambiguity there), but moot: there is no module-generated
artifact to bind in the first place.

C_consumer: (secondary, chain already broken at A, but checked anyway for
completeness) real primary-source check of nohang v0.3.0
(`src/nohang`, github.com/hakavlad/nohang tag v0.3.0): argv parsed at
lines ~3099-3114, `config = a[aaa+1]` (the `--config` value) →
`config = os.path.abspath(config)` (line 3192) → `open(config)` (line
3235) → line-by-line `key=value` parse into `config_dict = dict()`
(lines 3237-3266). Real, traceable path — would be PROVED if A held.

D_contract: (secondary) consumption is via ~30 distinct literal
`config_dict['post_zombie_delay']`-style accesses (grepped, lines
3557-3795) — a bounded, explicit set of field accesses, not scattered
arbitrary use, despite the parser itself calling itself "stupid conf
parsing" and building an unbounded dict. Would likely be PROVED if
reached.

fit: no
reason_code: NO_PRODUCER_PROOF
notes: Ironically C/D look favorable in isolation, but A is the hard
blocker: nothing in this module renders config content from options.

## akkoma

A_producer: proved
Evidence: `format = pkgs.formats.elixirConf {...}`; `configFile =
format.generate "config.exs" (replaceSec (... cfg.config))`. Real nix
eval (NixOS instance with `services.akkoma.enable=true` + minimal
required `:instance` fields) on the pinned tree, building the actual
derivation, gives real rendered content at
`/nix/store/68cpgggqq1rd5qdwybiqp3vd8wj5xqd6-config.exs`:
```
import Config
config :pleroma, :instance, description: "test instance", email: "admin@example.com", name: "test", ...
config :pleroma, Pleroma.Repo, adapter: Ecto.Adapters.Postgres, database: "akkoma", socket_dir: "/run/postgresql", username: "akkoma"
config :pleroma, Pleroma.Web.Endpoint, http: [ip: {:local, "/run/akkoma/socket"}, port: 0], secret_key_base: "<sha256-of-secret-path>", url: [host: "test.example.com", port: 443, scheme: "https"]
...
```
(secret-valued fields hold sha256 hashes of the `_secret` file path, per
`replaceSec`, later substituted for real at deploy time — see B).

B_binding: proved, with a documented content-transform step.
`systemd.services.akkoma-config` (`bindsTo`'d by `akkoma.service`, both
sharing `RuntimeDirectory = "akkoma"`) runs `configScript`, whose real
text (from module source) is `cat ${configFile} >"$tmp"` then
`replace-secret <hash> <secretfile> "$tmp"` per secret, then
`mv -f "$tmp" config.exs` into that shared runtime dir. The main
`akkoma.service` launches via `envWrapper`, whose real built wrapper
script (built and read directly: `/nix/store/89ir5yj32jqnzrcx4wl3191wwyfxr7wa-akkoma-env/bin/pleroma`) sets:
```
AKKOMA_CONFIG_PATH="${RUNTIME_DIRECTORY%%:*}/config.exs"
```
i.e. exactly the file akkoma-config.service wrote. So the consumed file
starts as a byte-identical copy of A's own configFile derivation, with
only the explicit `_secret`-marked placeholder values swapped for real
secret contents — not "a filename that looks similar," a real, traced
content pipeline with the same schema/keys throughout.

C_consumer: proved (independent primary-source research, real fetch of
akkoma tag v3.19.0 via github.com/external-mirrors/akkoma, cross-checked
against GitHub API homepage metadata + internal `mix.exs` version
string). `mix.exs` registers
`config_providers: [{Pleroma.Config.ReleaseRuntimeProvider, []}]` (a real
Elixir `Config.Provider` behaviour hook run at release boot).
`lib/pleroma/config/release_runtime_provider.ex:17-18` reads
`System.get_env("AKKOMA_CONFIG_PATH")` first (falling back to
`PLEROMA_CONFIG_PATH`, then two hardcoded paths), and line ~39 calls
`Config.Reader.read!(config_path)` — Elixir's real stdlib `.exs` config
evaluator.

D_contract: proved. Nearly all runtime access goes through
`Pleroma.Config.get/get!/fetch` (`lib/pleroma/config.ex`), itself a thin
wrapper on `Application.get_env(:pleroma, key, default)` used at ~507
call sites with literal/enumerable key paths (e.g. `[:instance,
:max_toot_chars]`, `Pleroma.Repo`); raw unwrapped `Application.get_env`
calls outside that module: only 4 in the whole tree. There is also an
explicit, machine-readable schema, `config/description.exs`
(3,564 lines), compiled into `Pleroma.Docs.JSON` and used by the admin
API to whitelist settable keys — a real declared contract, not a
formless map (no NimbleOptions-style validator was found on the
config-file *load* path itself; noting that as unverified rather than
claiming it).

fit: yes
reason_code: SUPPORTED
notes: The only soft spot is that B is a copy+secret-substitution
pipeline rather than a single identical store path — I judge it PROVED
because the pipeline is fully traced and non-secret content is
byte-identical, but flagging this explicitly since it's the shape most
likely to divide reviewers.

## privoxy

A_producer: proved
Evidence: `configFile = pkgs.writeText "privoxy.conf" (concatStrings (["confdir ${pkgs.privoxy}/etc\n"] ++ mapAttrsToList serialise cfg.settings))`.
Real nix eval of `services.privoxy.enable = true` on the pinned tree
confirms `cfg.settings` resolves to a real attrset (`listen-address =
"127.0.0.1:8118"`, `actionsfile`, `filterfile`, `user-manual`,
`temporary-directory`, etc.) that gets serialized by `serialise`.

B_binding: proved, cleanest of the five.
Real nix eval gives:
```
ExecStart = "/nix/store/k4f2h0jzc378icq8igv0fim956xqd5wv-privoxy-4.2.0/bin/privoxy --no-daemon /nix/store/jr52j88ak43r2fmspizq4dqrb8bqj41q-privoxy.conf"
```
The exact same `writeText` store path is interpolated directly into
`ExecStart` — no copy step, no substitution, no indirection.

C_consumer: proved (independent primary-source research, real fetch of
the official 4.2.0-stable tarball from privoxy.org's own SourceForge
mirror, version-verified via `configure.in`/`ChangeLog`). `main()` is in
`jcc.c:5541` (not `main.c` — corrected). Positional trailing argv is
assigned to global `configfile` at `jcc.c:5713`, matching invocation
`privoxy --no-daemon <configfile>`. Non-test path: `listen_loop()`
(called from `main()`) → `config = load_config();` (`jcc.c:6223`) →
`load_config()` at `loadcfg.c:675` → `fopen(configfile, "r")`
(`loadcfg.c:786`) → per-line parse loop (`loadcfg.c:794`-`1989`).

D_contract: proved. Each config line's directive token is hashed
(`hash_string()`, `miscutil.c:254`) and dispatched via `switch
(directive_hash)` (`loadcfg.c:847`) against 71 compile-time `#define
hash_*` constants (`loadcfg.c:130-200`, e.g. `hash_listen_address`,
`hash_actionsfile` — sic, `hash_actions_file`/`hash_filterfile`/
`hash_confdir`), each routed to its own small, type-specific handler.
Unrecognized directives hit an explicit `default:` (`loadcfg.c:1970`)
that logs a warning and discards the line — never absorbed into any
free-form structure. Fully bounded and mechanically enumerable straight
from source.

fit: yes
reason_code: SUPPORTED
notes: Strongest/cleanest SUPPORTED case of the five — direct writeText,
direct ExecStart interpolation, C-source directive dispatch table with
an explicit reject-unknown branch.

## mobilizon

A_producer: proved
Evidence: `settingsFormat = pkgs.formats.elixirConf {...}`; `configFile =
settingsFormat.generate "mobilizon-config.exs" cfg.settings`. Real nix
eval + build on the pinned tree (NixOS instance with
`services.mobilizon.enable=true` + minimal required `:instance`/
`Endpoint.url.host`), rendered content at
`/nix/store/6vvaqwiz39b19hw5fd01sgxy0qmx28ak-mobilizon-config.exs`:
```
import Config
config :mobilizon, :instance, demo: false, email_from: ..., hostname: "mobilizon.example.com", name: "test", registrations_open: false
config :mobilizon, Mobilizon.Storage.Repo, adapter: Ecto.Adapters.Postgres, database: "mobilizon_prod", pool_size: 10, socket_dir: "/run/postgresql", username: "mobilizon"
config :mobilizon, Mobilizon.Web.Auth.Guardian, secret_key: System.get_env("MOBILIZON_AUTH_SECRET", nil)
config :mobilizon, Mobilizon.Web.Endpoint, has_reverse_proxy: true, http: [...], secret_key_base: System.get_env("MOBILIZON_INSTANCE_SECRET", nil), server: true, url: [host: "mobilizon.example.com"]
config :tzdata, :data_dir, "/var/lib/mobilizon/tzdata/"
```
(secrets deliberately left as `System.get_env(...)` calls in the
rendered file itself, not baked-in placeholders — a cleaner secret
pattern than akkoma's.)

B_binding: proved, cleanest possible form — no copy/substitution step at
all. Real nix eval: `ExecStart =
"/nix/store/pbbd7b48kdkzfxc2ibd1468srzlzl2hp-mobilizon-launchers-5.2.4/bin/mobilizon start"`.
Module source shows `launchers` is built via `makeWrapper $src/bin/mobilizon
$out/bin/mobilizon --set MOBILIZON_CONFIG_PATH "${configFile}"` — the
env var is set directly to A's own exact store path, confirmed by
reconstructing and tracing the derivation's build context (drvPath
resolved from the real eval's string context).

C_consumer: proved (independent primary-source research, real fetch of
the exact pinned fork+tag nixpkgs uses, framagit.org/kaihuri/mobilizon
tag 5.2.4, cross-checked against framasoft/mobilizon upstream at the
same tag — byte-identical for the relevant file). `mix.exs:29-36`
registers `config_providers: [{Mobilizon.ConfigProvider,
"/etc/mobilizon/config.exs"}]`. `lib/config_provider.ex` (full 36-line
file read): `load/2` does
`config_path = System.get_env("MOBILIZON_CONFIG_PATH") || path`, then
`if File.exists?(config_path)`: `Config.Reader.read!(config_path)` →
`Config.Reader.merge(config, runtime_config)` — real stdlib functions.

D_contract: proved. `config/config.exs` declares 47 top-level `config
:mobilizon, <key-or-module>, ...` blocks (grepped/counted). Consumption
is exactly two narrow surfaces: raw `Application.get_env(:mobilizon,
...)` (47 call sites, all literal/`__MODULE__`/statically-scoped keys —
individually checked, none built from unbounded runtime input) and
`Mobilizon.Config.get/get!/put` (`lib/mobilizon/config.ex:396-437`, a
thin wrapper, 65 call sites, same literal-key discipline). Zero
`Application.fetch_env` calls found; no evidence of freeform/dynamic key
use anywhere in `lib/`.

fit: yes
reason_code: SUPPORTED
notes: Second-cleanest SUPPORTED case — direct env-var-to-store-path
binding with zero content transform, and the most explicitly-confirmed
`Config.Provider`-precedence logic of the three Elixir candidates
(`MOBILIZON_CONFIG_PATH` env var literally overrides the compiled-in
default path, read straight from source).

## Summary

| candidate  | A | B | C | D | fit | reason_code |
|---|---|---|---|---|---|---|
| libinput   | no | no | - | - | no | NO_PRODUCER_PROOF |
| nohang     | no | (moot) | (would be proved) | (would be proved) | no | NO_PRODUCER_PROOF |
| akkoma     | proved | proved | proved | proved | yes | SUPPORTED |
| privoxy    | proved | proved | proved | proved | yes | SUPPORTED |
| mobilizon  | proved | proved | proved | proved | yes | SUPPORTED |

3/5 SUPPORTED. Both non-fits fail at A, not B/C/D — in both cases the
module.nix looks superficially like a config-file generator (it
references a config path prominently) but on real nix eval the actual
path in play is either a package-shipped static file or (nohang) an
externally user-supplied path, never something rendered from Nix module
options via `writeText`/`format.generate`. This is exactly the class of
false positive the protocol's A criterion ("must yield either the exact
real store path pattern or the exact real rendered content ... not read
off the module source by eye") is designed to catch, and it only
surfaced here because I actually ran nix eval instead of trusting the
module's superficial shape.
