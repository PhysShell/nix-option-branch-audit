# C-E1.2b batch 2: kavita, transmission, i2pd

Frozen at `0fab0af`/`e025f4a` (protocol commit, zero `src/` changes).
All A/B evidence re-verified via real `nix eval --impure` against
`PhysShell/nixpkgs` tree `68740713a1d5904edf9ba92a998a522b1b6ce080` (the
same pin already used throughout this project -- not re-pinned to a
fresh tip, since the point of re-verifying is "don't trust the
citation blindly," not "test for staleness against time"; two of the
three candidates below turned up real, unrelated staleness anyway).
All C/D evidence re-fetched fresh from each real consumer's exact
pinned upstream tag, read directly, not trusted from
`fixtures/c-e1.1-generated-config-audit/batch2.md`'s own citation.

## kavita

**Re-verified A/B** (`nix eval`, `services.kavita.enable = true;`):
```json
{"execStart":"/nix/store/.../kavita-0.9.0.2/bin/kavita",
 "preStart":"install -m600 /nix/store/.../appsettings.json /var/lib/kavita/config/appsettings.json\n.../replace-secret '@TOKEN@' ...",
 "settings":{"IpAddresses":"0.0.0.0,::","Port":5000},
 "workingDirectory":"/var/lib/kavita"}
```
Matches batch1's own citation exactly -- `preStart` installs the real
generated `appsettings.json` derivation to `config/appsettings.json`
under `WorkingDirectory`; `ExecStart` itself is a BARE binary
invocation with zero arguments referencing the artifact at all.

**Re-verified C/D**: refetched `Kavita.Common/Configuration.cs` at real
tag `v0.9.0.2` directly (`gh api repos/Kareadita/Kavita/contents/...`).
Confirmed byte-consistent with batch1's own citation: `JsonSerializer.
Deserialize<AppSettings>(File.ReadAllText(filePath))` (lines 96-97 and
6 further call sites), `AppSettings` a real, fully-typed sealed class
(`TokenKey`/`Port`/`IpAddresses`/`BaseUrl`/`Cache`/`AllowIFraming`/
`OpenIdConnectSettings`), with `OpenIdConnectSettings` itself a real
NESTED class (`Authority`/`ClientId`/`Secret`/`CustomScopes: List<
string>`).

1. **A**: proved (re-confirmed).
2. **B**: proved (re-confirmed) -- but structurally a DIFFERENT shape
   than any of the 4 shipped anchors: `ExecStart` takes zero args
   (superficially `ImplicitDefaultPath`-shaped like `nebula-
   lighthouse-service`), but the real bound path isn't a single
   hardcoded absolute string -- it's `${WorkingDirectory}/config/
   appsettings.json`, computed from TWO real evidence sources (the
   module's own real `WorkingDirectory` setting + the consumer's own
   real hardcoded `config/<filename>` relative-path convention,
   `Configuration.cs:20`). `ImplicitDefaultPath{path: String}`'s shape
   still fits -- nothing stops storing the already-CONCATENATED real
   path there -- but the ACQUIRE logic for this shape needs to combine
   two real evidence sources instead of reading one hardcoded literal
   directly, a genuinely new (if still generic) locator pattern.
3. **C**: proved (re-confirmed).
4. **D**: proved (re-confirmed) -- a real, fully-typed, NESTED C#
   class. `OpenIdConnectSettings` nests one level; `CustomScopes` is a
   real `List<string>` (opaque under this v1's own array rule, exactly
   like `unpackerr`'s `radarr`).
5. **Normalizes into the existing model?** Yes, cleanly.
   `ArtifactContent::StructuredValue` (JSON, real Nix-evaluated
   `settings` value) + `flatten_structured_value` handles the nested
   `OpenIdConnectSettings` object and the `CustomScopes` array with
   ZERO changes -- this is structurally the SAME shape
   `flatten_structured_value` was already built for (unpackerr's own
   nested-list case).
6. **`compare_config_contract()` unchanged?** Yes.

**Verdict: PASS.**

**Reuse classification: new generic locator required** -- (a) a new
acquire function combining `WorkingDirectory` + the hardcoded relative
suffix into one real resolved path for `ImplicitDefaultPath`, and (b) a
new bounded C#-attribute-free literal scan over `AppSettings`'s own
public property declarations (`public <Type> <Name> { get; set; }`) --
a genuinely new per-consumer extractor, same category as every anchor
in C-E1.2a already needed, not a new TYPE or new semantic concept.

**Adversarial normalization check**: not applicable -- kavita's real
consumer schema is plain nested JSON with no section/clause concept at
all (a C# class hierarchy, not an INI-style multi-block format); no
bare-keyword stripping is needed or tempting here in the first place.

## transmission

**Re-verified A/B**: the FIRST real staleness this round actually hit.
`services.transmission.package` now REQUIRES an explicit value --
`nix eval` failed outright with a real, dated nixpkgs breaking change:
```
error: `services.transmission.package` previously defaulted to
`pkgs.transmission_3`, which has been removed in favour of
`pkgs.transmission_4`. ... (see NixOS 24.11 release notes)
```
Re-run with `package = pkgs.transmission_4;` set explicitly succeeded:
```json
{"execStart":"/nix/store/.../transmission-4.1.3/bin/transmission-daemon -f -g /var/lib/transmission/.config/transmission-daemon ",
 "execStartPre":["+/nix/store/.../transmission-prestart"],
 "settings":{"download-dir":"...","peer-port":51413,"rpc-bind-address":"127.0.0.1","rpc-port":9091,"watch-dir":"...", ... (flat, 14 keys)}}
```
Real, live confirmation of the exact `-g`/`--config-dir` binding batch2
cited, plus the `transmission-prestart` merge step. Package version
`4.1.3` matches `pkgs.transmission_4.version` exactly (re-confirmed via
a direct `nix eval`).

**Re-verified C/D**: refetched `daemon/daemon.cc` at real tag `4.1.3`
directly. Confirmed `{ 'g', "config-dir", ... }` (line 95) and
`load_settings`/`tr_sessionLoadSettings` (lines 401-411) still real and
unchanged in shape. Followed C one hop further than batch2's own
citation did: `tr_sessionLoadSettings` itself lives in
`libtransmission/session.cc:505`, not `daemon.cc` -- refetched that
file too (real tag `4.1.3`) and read its body directly:
```cpp
if (auto const filename = fmt::format("{:s}/settings.json", config_dir); tr_sys_path_exists(filename))
{
    if (auto file_settings = tr_variant_serde::json().parse_file(filename))
    {
        libtransmission::api_compat::convert_incoming_data(*file_settings);
        settings.merge(*file_settings);
    }
}
```
D's own real bounded registry lives in `libtransmission/quark.h`
(refetched at `4.1.3`): **712 real `TR_KEY_*` enum constants**, a
genuine, large-but-bounded, centrally-declared registry (not scattered
ad hoc string literals) -- e.g. `TR_KEY_peer_port`, `TR_KEY_rpc_enabled`,
`TR_KEY_watch_dir`.

**A genuinely new nuance found, worth disclosing even though it's not
this round's core question**: `quark.h` also declares
`_kebab_APICOMPAT`-suffixed siblings for some real keys (e.g.
`TR_KEY_rpc_enabled_kebab_APICOMPAT`, `TR_KEY_watch_dir_kebab_
APICOMPAT`), and `convert_incoming_data` (called right before the
`.merge()` above) is real, live evidence transmission maintains a
backward-compat alias layer between an older underscore-case naming
convention and its current kebab-case one. A bounded literal scan over
`TR_KEY_<name>,` declarations alone, done naively, would double-count
some real keys as two independent names rather than one canonical name
plus a legacy alias -- a real D-extraction design decision for any
future implementation, not a blocker for this round's own verdict
(the CANONICAL kebab-case key set is still real, bounded, and directly
extractable; the alias layer is an accuracy refinement, not a new
category of problem).

1. **A**: proved (re-confirmed, after finding and working around a
   real, unrelated nixpkgs breaking change).
2. **B**: proved (re-confirmed) -- `DirectArgv`, identical shape to the
   already-shipped `unpackerr` anchor.
3. **C**: proved (re-confirmed, traced one hop further than batch2's
   own citation).
4. **D**: proved (re-confirmed) -- real, large, bounded `TR_KEY_*`
   registry; a real legacy-alias nuance disclosed above.
5. **Normalizes into the existing model?** Yes -- `settings` is a FLAT
   top-level JSON object (no nested objects observed in the real
   evaluated value), `flatten_structured_value` needs no change at
   all.
6. **`compare_config_contract()` unchanged?** Yes.

**Verdict: PASS.**

**Reuse classification: existing implementation reuse** for A/B/the
comparison itself (`DirectArgv` + `flatten_structured_value`, both
used completely as-is); **new generic locator required** only for D (a
new bounded literal scan over `quark.h`'s own `TR_KEY_<name>,`
declarations, with a real, disclosed design choice about the
kebab/underscore alias pair -- a per-consumer extractor decision, not
an app-specific comparison branch).

**Adversarial normalization check**: not applicable -- transmission's
real `settings.json` is a single flat namespace (confirmed directly
from the real evaluated JSON above); there is no section/clause
concept in this format at all, so bare-keyword stripping is neither
needed nor tempting here.

## i2pd

**Re-verified A/B** (`nix eval`; first attempt used the OLD
`services.i2pd.ipv4`/`ipv6` top-level options batch2 didn't mention
using directly and failed with a real error --
`services.i2pd.ipv4' does not exist` -- confirming these must be
set via `settings.ipv4`/`settings.ipv6` instead, matching the module's
own real freeform-attrset design; not itself a regression, just this
round's own first attempt guessing wrong):
```json
{"execStart":"/nix/store/.../i2pd-2.61.0/bin/i2pd '--datadir=%S/i2pd' '--conf=%T/conf' '--tunconf=%T/tunconf'",
 "execStartPre":".../i2pd-load-credentials '%T/conf=/nix/store/.../i2pd.conf' ...",
 "settings":{"bob":{"enabled":false},"http":{"enabled":true},
   "httpproxy":{"enabled":true},"i2cp":{"enabled":false},
   "i2pcontrol":{"enabled":false},"ipv4":true,"ipv6":false,
   "limits":{"coresize":0},"precomputation":{"elgamal":true},
   "sam":{"enabled":false},"socksproxy":{"enabled":true}, ...}}
```
Confirms A/B exactly as batch2 cited: real `--conf=` flag pointing at
the real generated `i2pd.conf` derivation copied into place by
`i2pd-load-credentials`.

**The specific adversarial risk this candidate exists to test**: the
bare key `"enabled"` appears in the real evaluated settings under
**seven different real sections** -- `bob`, `http`, `httpproxy`,
`i2cp`, `i2pcontrol`, `sam`, `socksproxy` -- each toggling a
COMPLETELY DIFFERENT real subsystem. If a D-extractor (or a careless
transfer of `unbound`'s own bare-keyword-stripping fix) treated
`"enabled"` as one bare comparison path, it would be silently comparing
seven semantically unrelated real settings as if they were one --
exactly the failure mode the protocol's adversarial check exists to
catch.

**Re-verified C/D**: refetched `libi2pd/Config.cpp` at real tag
`2.61.0` directly. **The real i2pd source ITSELF already registers
every section-scoped `boost::program_options` key as a fully-dotted,
section-QUALIFIED string literal**, not a bare name:
```cpp
("http.enabled", value<bool>()->default_value(true), "Enable or disable webconsole")
("httpproxy.enabled", value<bool>()->default_value(true), "Enable or disable HTTP Proxy")
```
(17+ more `http.*`/`httpproxy.*` entries read directly, all real,
verbatim-dotted). `ParseConfig`/`parse_config_file`
(`Config.cpp:486-501`) still real, unchanged in shape.

**Answer to the adversarial check: (b), not (a)** -- bare-keyword
normalization is not merely unneeded here, it's actively the WRONG
move: i2pd's real accepted contract is not flat at all (seven distinct
real `X.enabled` keys exist), but it is ALREADY fully qualified in
BOTH real sources at once -- the Nix-side `settings` value nests
naturally (`{"http":{"enabled":true}, "bob":{"enabled":false}, ...}`)
and the consumer's own real registered option strings are the
IDENTICAL dotted form (`"http.enabled"`, `"bob.enabled"` would be, if
present -- `bob.*` isn't in the excerpt read but the convention is
uniform throughout the file for every section that has one). **The
existing `flatten_structured_value`, completely unmodified, already
produces the exact correct fully-qualified paths (`http.enabled`,
`httpproxy.enabled`, ...) with no stripping of any kind** -- this is
the cleanest possible transfer case in this batch, precisely because
NO normalization decision is needed at all; the section/context
information is preserved by construction on both sides.

This is real, useful evidence against always reaching for `unbound`'s
own stripping move: **the correct behavior differs per real consumer,
and must be derived from that consumer's own real parsing semantics
each time — i2pd's honest answer is "keep the dots," `unbound`'s
honest answer was "the dots don't correspond to anything the consumer
itself distinguishes, so keep the bare name instead."** Both are
sound; neither is the default; the difference is unknowable without
looking at the real D-extractor's own actual key strings, which this
round did.

1. **A**: proved (re-confirmed, after locating the real
   `settings.ipv4`/`ipv6` spelling).
2. **B**: proved (re-confirmed).
3. **C**: proved (re-confirmed).
4. **D**: proved (re-confirmed) -- real, already-dotted, section-
   qualified `boost::program_options` registrations.
5. **Normalizes into the existing model?** Yes, and with LESS work
   than any other candidate examined so far -- `flatten_structured_
   value` needs zero modification and zero extra normalization step
   (contrast `unbound`, which needed the section-stripping fix).
6. **`compare_config_contract()` unchanged?** Yes.

**Verdict: PASS.**

**Reuse classification: existing implementation reuse** for A/B and
the entire comparison path (`DirectArgv` + `flatten_structured_value`,
completely unmodified, is not just sufficient but IDEAL here); **new
generic locator required** only for D (a new bounded literal scan over
`Config.cpp`'s own `("KEY.SUBKEY", value<T>()...)` literal
registrations -- straightforward given the keys are already the exact
dotted strings needed, no reconstruction required).

## Batch 2 summary

- 3/3 candidates: **PASS**. Zero `FINDING`s, zero `INCONCLUSIVE`s.
- Two real, unrelated staleness issues hit and worked around during
  A/B re-verification (transmission's `package` default removal;
  i2pd's `ipv4`/`ipv6` having moved under `settings`) -- both are real
  nixpkgs changes since C-E1.1's own original pass, neither affects
  the real conclusion, both are exactly the kind of thing "re-verify,
  don't trust the citation" is supposed to catch.
- **i2pd is the single most valuable result in this batch**: it is
  the adversarial normalization check's own best real positive
  control. Seven real, genuinely name-colliding bare keys
  (`X.enabled` across `bob`/`http`/`httpproxy`/`i2cp`/`i2pcontrol`/
  `sam`/`socksproxy`) exist in the real corpus, and the correct,
  consumer-verified answer is the OPPOSITE of `unbound`'s own shipped
  fix: keep the dots, don't strip them. Confirms the normalization
  decision is real per-consumer semantic work, not a reusable default
  either way.
- Every "new generic locator required" classification in this batch is
  a genuinely per-consumer bounded literal-scan extractor (C#
  property declarations, a Go/C-style enum-constant table, a
  boost::program_options registration table) -- the exact same shape
  every one of C-E1.2a's own four anchors already needed, nothing
  qualifying as a new semantic abstraction or app-specific branch in
  any of the three.
