# S5-F1D: investigation and traces

Required by the mandate: complete traces for angrr, rspamd, and tayga,
identifying the actual embedding/provenance relationship (not "depth <=
1" as the reason for exclusion), plus verification of where identity
must survive downstream of `scan_options`.

## Angrr `#471312`

- **Chain**: `services.angrr.settings` -> `settingsOptions` ->
  `temporary-root-policies` -> `temporaryRootPolicyOptions` -> `period`.
- **Structural/type edges**: `settings = mkOption { type =
  types.submodule settingsOptions; };` is a real, true-root declaration
  at `option_prefix`'s own boundary (`option_prefix = [services,
  angrr]`) -- its own accumulated path at the moment its `type =` field
  is scanned is `["settings"]`. `settingsOptions` is a genuine submodule
  (has its own direct `options = {...}` block) -- walked at
  `["settings"]`. Its own `temporary-root-policies = mkOption { type =
  with lib.types; attrsOf (submodule temporaryRootPolicyOptions); };`
  is itself a real declaration, discovered DURING that walk, at
  `["settings","temporary-root-policies"]`. `temporaryRootPolicyOptions`
  is ALSO a genuine submodule -- walked at
  `["settings","temporary-root-policies"]`. Its own `period` is recorded
  at `["settings","temporary-root-policies","period"]`.
- **Why the nested `period` is NOT the watched root-level `period`**:
  not "it's two hops away" -- it is that its own real, embedded identity
  (`settings.temporary-root-policies.period`, a per-policy retention
  string) is structurally, provably DIFFERENT from
  `services.angrr.period`'s own real identity (a directly-declared,
  since-removed top-level option). The qualified PATH is exactly this
  embedding, made explicit. `run_target`'s own gate-1 lookup
  (`options.iter().find(|o| o.path == watched_path)`, unmodified) can
  never conflate them, because their paths are literally different
  strings -- not because one of them was hidden from discovery.
- **Where historical scanning discarded context**: `walk_options_block`'s
  own S5-F1/F1B/F1C-era promotion mechanism walked a referenced
  candidate's content with a FRESH, EMPTY path (`&mut Vec::new()`)
  rather than the referencing declaration's own accumulated path -- the
  actual bug, present since named-submodule-by-reference discovery was
  first introduced. S5-F1D fixes this directly: `walk_named_type_references`
  threads `path` (the CALLER's own accumulated position) through, exactly
  paralleling how `find_nested_options_block`'s own pre-existing
  INLINE-submodule mechanism already worked (`nginx = mkOption { type =
  types.submodule { options = {...}; }; };` was always walked at
  `["nginx", ...]`, never bare -- S5-F1D simply extends this same,
  already-correct treatment to submodules reached BY NAME instead of
  written inline).
- **Verified**: real, live-fetched PR #471312 content re-run against a
  clean-checkout `25c5b54` binary --
  `predicate_not_found -> option_not_found`, Changed (intact). Direct
  `check` output confirms `temporaryRootPolicyOptions`'s own `period` IS
  now present in `discovered_options`, at
  `["settings","temporary-root-policies","period"]` -- discovery is
  allowed; it simply never matches the bare watched query.

## Rspamd (`workers -> workerOpts -> bindSockets -> bindSocketOpts`)

- **Structural/type edges**: `workers = mkOption { type = with types;
  attrsOf (submodule workerOpts); };` is a true-root declaration
  (`option_prefix = [services, rspamd]`), one recognized `with`-wrapped
  hop (S5-F1C-A) to `workerOpts`, a genuine submodule -- walked at
  `["workers"]`. `workerOpts`'s own `bindSockets = mkOption { type =
  types.listOf (types.either types.str (types.submodule
  bindSocketOpts)); };` is a real declaration discovered during that
  walk, at `["workers","bindSockets"]`. `bindSocketOpts` is ALSO a
  genuine submodule (its own `socket`/`mode`/`owner`/`rawEntry` fields)
  -- walked at `["workers","bindSockets"]`.
- **What distinguishes these leaves from unrelated root-level leaves**:
  their own qualified path (`["workers","bindSockets","socket"]`, etc.)
  -- structurally impossible to collide with any true-root leaf's own
  (necessarily shorter or differently-prefixed) path.
- **Verified**: real, live-fetched PR #484133 (rspamd) re-run --
  watched verdict restored (`option_not_found -> predicate_not_found`,
  matching historical). `workerOpts`'s own direct leaves (`count`,
  `includes`) confirmed present at their own qualified path. A dedicated
  hostile unit test (`bindSockets`/`bindSocketOpts`, previously excluded
  by the one-hop cap) confirms the second hop now resolves too, at its
  own further-qualified path.

## Tayga (`ipv4 -> versionOpts -> pool -> addrOpts`)

- **Structural/type edges**: `ipv4 = mkOption { type =
  types.submodule (versionOpts 4); };` (true root) -> `versionOpts`
  (one hop, dotted) -> walked at `["ipv4"]`. Its own `pool = mkOption {
  type = with types; nullOr (submodule (addrOpts v)); };` is discovered
  during that walk, at `["ipv4","pool"]` -- a recognized `with`-wrapped,
  function-application-form reference (`addrOpts v`, not a bare
  `submodule addrOpts`) to `addrOpts`, ALSO a genuine submodule -- walked
  at `["ipv4","pool"]`. Its own `prefixLength` is recorded at
  `["ipv4","pool","prefixLength"]`.
- **Verified**: a dedicated hostile unit test reproducing this exact
  real shape confirms `prefixLength` is now discovered at its own
  qualified path, and never matches a bare watched query for
  `prefixLength`. (No live PR re-fetch was needed beyond the unit test
  here, since PR #432528's own watched leaf, `wkpfStrict`, is unrelated
  to `addrOpts` and was already independently confirmed restored via
  the live corpus check below.)

## Identity/matching boundary: `scan_options` -> `run_target` -> `compare()`

Inspected, not assumed. `run_target`'s own gate-1 lookup
(`options.iter().find(|o| o.path == watched_path)`, `src/main.rs`) and
`declared_defaults_for`'s own predicate-reference lookup
(`options.iter().find(|o| &o.path == r)`) BOTH already do a plain
`Vec<String>` equality check against `OptionDecl.path`. Neither needed
any change: once `path` itself correctly reflects a declaration's real
embedding (S5-F1D's own fix, entirely inside `scan_options`/
`walk_options_block`), the EXISTING equality check already distinguishes
"same leaf, same logical declaration" from "same leaf, different
embedding" correctly -- verified via the full existing integration test
suite (`tests/diff_cli.rs`, `tests/audit_cli.rs`, `tests/check_root.rs`),
all passing unmodified, and via all of this round's own new hostile
tests. `TargetIdentity` (the comparison key `compare()` uses -- module,
test, cfg_ident, option_prefix, watched_path) is unaffected: it is keyed
on the WATCHED QUERY string, never on which declaration resolved it, and
remains correct regardless of how `scan_options` internally computes
`path`.

**Conclusion**: no downstream identity/matching change was required or
made. The provenance loss was entirely contained within
`scan_options`/`walk_options_block`'s own promotion mechanism, and
fixing it there was sufficient. This is disclosed as a verified finding,
not assumed from the mandate's own framing that a downstream change
might be needed.

## Duplicate/first-match interaction: the 8-PR collision corpus, re-checked

All 8 previously-disclosed collision PRs (`fixtures/s5-f1b-r/leaf-collision-scan.json`)
re-checked against a clean-checkout `25c5b54` binary, live-fetched
content:

| PR | field | S5-F1C state | S5-F1D state |
|---|---|---|---|
| `#429967` | prosody `domain` | 2-way collision | **resolved** (0 collisions) |
| `#494314` | prosody `domain`/`extraConfig` | 2-way | **resolved** |
| `#431289` | prosody `domain` | 2-way | **resolved** |
| `#440660` | prosody `domain` | 2-way | **resolved** |
| `#260551` | prosody `domain`/`extraConfig` | 2-way | **resolved** |
| `#463443` | `profiles` | 2-way | **resolved** |
| `#374017` | k3s `enable` | 2-way | **resolved** |
| `#415326` | wstunnel `settings` | 2-way | **unresolved, unchanged** |

Seven of eight are resolved as a direct, verified side effect of
provenance-qualified paths: each colliding submodule's own content is
now qualified by its own real referencing option's name (`muc.domain`
vs. e.g. `uploadHttpOpts`'s own referencing option's name, `manifests.enable`
vs. the module's own top-level `enable`), which are, in every one of
these seven real cases, genuinely different strings. This was not a new
disambiguation policy -- `run_target`'s own `.find()` is completely
unmodified; the paths themselves simply stopped colliding.

`#415326` (wstunnel) remains unresolved for a disclosed, different
reason: its own `option_prefix` (`[services, wstunnel, clients, "*"]`)
is a WILDCARD prefix. S5-F1D's own new reference-following mechanism is
deliberately gated off for wildcard prefixes (`prefix_is_concrete`),
exactly matching S5-F1B's own established precedent (kimai's real
`siteOpts`) -- under a wildcard prefix, every named binding is still
discovered via the separate, unconditional, unchanged mechanism, which
never threads a referencing declaration's own path through at all (a
wildcard segment can never appear in any real declaration's own
attrpath, so there is no "true root reference" to start qualification
from). This is NOT a new gap S5-F1D introduces -- it is the exact
pre-existing wildcard behavior, unrelated to the concrete-prefix
provenance fix, correctly left untouched per this round's own scope
("do not redesign duplicate-candidate selection... do not touch first-
match policy").

**Per the mandate's own instruction ("if provenance makes two
candidates genuinely distinct, retain that distinction rather than
collapsing them")**: this is exactly what happened, for seven of eight
cases, as a direct consequence of the correctness fix -- not a
deliberately engineered disambiguation feature. `.find()` itself did
not need to change anywhere. The remaining wildcard case (`#415326`) is
disclosed, not silently folded into "resolved," and remains open as
S5-F1B-R/F1C-R already flagged it: a separate, real, architectural
question about `run_target`'s own first-match policy under a wildcard
prefix specifically, tracked but not addressed here.
