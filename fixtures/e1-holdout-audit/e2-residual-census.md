# E2: residual inconclusive census (the 13 left after E1-R)

Research-only, zero `src/` changes, against the exact same frozen
40-candidate holdout as E1/E1-R, at commit `b0c598c` (unchanged since
E1-R). For each of the 13 `INCONCLUSIVE` candidates: the FIRST blocking
cause only, not every oddity the file happens to contain -- a target
with five strange shapes only counts once, at whichever one the real
gate chain actually stops on first. Verified against real source and
the tool's own real JSON output, not inferred.

## Per-candidate

### libinput -- `enable` -- `DefaultUnresolved`
- **first blocker**: declaration
- **concrete unsupported shape**: the option's own real default IS
  found (P1's `//`-override extraction works correctly), but the
  override's `default` field is `config.services.xserver.enable` -- a
  cross-module reference to a SIBLING option, not a literal. Neither
  `classify_value` nor `classify_known_value` can classify a
  `NODE_SELECT` as a known value.
- **frequency among residual 13**: 1/13, no other candidate hits this
  exact shape.
- **counterfactual**: uncertain, not confirmed. Would require chasing
  ANOTHER option's own declared default recursively (`config.services.
  xserver.enable`'s own default) -- a real, deeper feature (cross-option
  default-value resolution), not touched here, and whether `xserver.
  enable`'s own default is itself classifiable isn't checked (that
  module isn't vendored in this holdout).

### jitsi-meet -- `jigasi.enable` -- `OptionNotFound`
- **first blocker**: declaration
- **concrete unsupported shape**: `options.services.jitsi-meet = with
  types; { ... };` -- the value is a `NODE_WITH` wrapping the real
  attrset, not a literal `NODE_ATTR_SET`. `scan_options`'s own loop
  requires `value.kind() == NODE_ATTR_SET` before doing ANYTHING else,
  so the entire block (dozens of real options, including `jigasi.enable`
  at line 156) is invisible -- this is GAP-3 from E1's own original
  census, confirmed still unfixed (P0-P2 never touched it).
- **frequency among residual 13**: part of a 2/13 cluster with `i2pd`
  below -- same underlying mechanism (`scan_options` never unwraps a
  value expression before checking `NODE_ATTR_SET`), two different real
  wrapper kinds.
- **counterfactual**: plausible, not confirmed. `jigasi.enable` is a
  plain `mkOption`; its one real predicate site (`optionalString
  cfg.jigasi.enable ...`) is a shape H2 already lowers. Whether the real
  test ever sets `jigasi.enable` wasn't checked this round.

### i2pd -- `enable` -- `OptionNotFound`
- **first blocker**: declaration
- **concrete unsupported shape**: `options.services.i2pd = let
  freeformType = ...; in { ... };` -- the value is a `NODE_LET_IN`
  wrapping the real attrset, same root cause as `jitsi-meet` above (a
  different concrete Nix wrapper, same `value.kind() != NODE_ATTR_SET`
  gate).
- **frequency among residual 13**: see `jitsi-meet` -- 2/13 for this
  cluster.
- **counterfactual**: plausible, not confirmed. `enable` is `mkEnableOption`
  (P1-recognized once reached); its predicate (`mkIf cfg.enable`)
  matches an already-supported shape. Real test evidence not checked.

### matterjs-server -- `openFirewall`, `bluetoothSupport` -- `TestConfigUnresolved`
- **first blocker**: test-config
- **concrete unsupported shape**: the real test's ENTIRE node definition
  is one fully-flattened combined attrpath --
  `nodes.machine.services.matterjs-server.enable = true;` -- not the
  `nodes.machine = { services.matterjs-server = { ... }; };` nested-
  attrset shape `scan_test_assignments`'s own documented scope names
  ("flat `nodes.foo = ...` or nested `nodes = { foo = ...; };`"). A
  5-segment combined attrpath is a third shape neither form anticipates.
- **frequency among residual 13**: 1/13 in this EXACT form, but see
  `nimdow` below -- the same broad "combined multi-segment attrpath,
  not walked the same as a nested attrset" pattern recurs on the
  DECLARATION side too, in a DIFFERENT scanner. Not counted as the same
  fix (different code, `scan_test_assignments` vs `scan_options`), but
  flagged as the same underlying insight.
- **counterfactual**: CONFIRMED, not just plausible -- E1's own original
  census already hand-verified, by reading the real source directly,
  that `openFirewall`/`bluetoothSupport` are genuinely never set
  anywhere in this test file. If this shape were walkable, both would
  become real `FINDING`s (`OBA001`), not `PASS`.

### ferretdb -- `enable` -- `TestConfigUnresolved`
- **first blocker**: test-config
- **concrete unsupported shape**: the real test file is `{ runTest, pkgs
  }: { postgresql = runTest {...}; sqlite = runTest {...}; }` -- the
  real `services.ferretdb.enable = true;` sits three levels deep inside
  a function-call ARGUMENT (`runTest { ...; nodes.machine = { ...;
  services.ferretdb.enable = true; }; }`), never at a literal top-level
  `nodes` position at all.
- **frequency among residual 13**: 1/13 in this exact shape.
- **counterfactual**: plausible (E1's own original note), not re-verified
  deeper here. The assignment IS real and present; whether reaching
  INTO a function-call argument structurally would also require solving
  "which call is `runTest`" (a further alias/identity question) isn't
  resolved.

### monado -- `enable`, `forceDefaultRuntime` -- `TestConfigUnresolved`; `highPriority` -- `OptionNotFound` (two distinct first blockers on one candidate, disclosed explicitly rather than picking one to represent it)
- **first blocker (`enable`/`forceDefaultRuntime`)**: test-config --
  import opacity, `imports = [ ./common/openxr.nix ];`, a real SHARED
  test helper file this project doesn't vendor. The opacity gate
  correctly refuses to certify a conclusion -- this is the design
  working as intended, not a bug, and almost certainly NOT fixable
  without expanding this project's own stated scope (reading arbitrary
  external test-helper files it doesn't already vendor).
- **first blocker (`highPriority`)**: declaration -- a DIFFERENT, novel
  edge case: `highPriority = mkEnableOption "..." // mkOption { default
  = true; };` -- the `//`-merge's right-hand side is itself an
  `mkOption {...}` CALL, not a literal attrset. P1's
  `classify_option_helper_call` only recognizes a literal
  `NODE_ATTR_SET` as the override's RHS; a call that ITSELF evaluates to
  one isn't unwrapped, so this entry matches neither branch and is
  silently invisible.
- **frequency among residual 13**: import-opacity, 1/13 in this exact
  form (see the "test-side opacity" thematic note below for the
  broader, non-identical cluster); the `// mkOption{...}` merge edge
  case, 1/13, unique.
- **counterfactual**: import-opacity -- almost certainly not a
  worthwhile fix target (out of project scope). `// mkOption{...}`
  case -- uncertain, not verified whether real test evidence for
  `highPriority` even exists.

### nimdow -- `enable` -- `OptionNotFound`
- **first blocker**: declaration
- **concrete unsupported shape**: `services.xserver.windowManager.
  nimdow.enable = mkEnableOption "nimdow";` -- ONE combined 5-segment
  attrpath (`services.xserver.windowManager.nimdow.enable`) declared
  directly inside the top `options = { ... };` block, never split into
  nested attrsets. P2's own fix (reset the path when the ACCUMULATED
  path from walking nested attrsets equals `option_prefix`) never
  fires here, because there's no separate attrset-recursion step at
  all -- the whole container+leaf path arrives on ONE entry.
- **frequency among residual 13**: the declaration-side half of the
  "combined multi-segment attrpath" pattern also seen in
  `matterjs-server` (test-config side) -- 2/13 total for the pattern,
  in two DIFFERENT scanners.
- **counterfactual**: likely, not fully confirmed. The real test sets
  `services.xserver.windowManager.nimdow.enable = true;` (default
  `false`) directly inline in the same node -- a real opposite
  transition. The test node also has `imports = [ ./common/x11.nix
  ./common/user-account.nix ];`, unrelated files not vendored here;
  whether the opacity gate would (correctly) judge them irrelevant to
  THIS specific option isn't independently confirmed.

### tor -- `enable` -- `TestConfigUnresolved`
- **first blocker**: test-config
- **concrete unsupported shape**: real `services.tor.enable = true;`
  text exists, but only inside helper functions (`mkDANode`/
  `mkRelayNode`/`mkExitNode`) later `map`ped over name lists to build
  the actual `nodes` attrset -- never literal `nodes.<name> = {...};`
  text at all.
- **frequency among residual 13**: 1/13 in this exact form (part of the
  broader, non-uniform "test-side indirection" theme, not the same
  concrete mechanism as `ferretdb`'s or `akkoma`'s own forms).
- **counterfactual**: uncertain, likely deep. E1's own original note
  already flagged that even if this specific opacity were resolved,
  tor's real multi-node topology likely raises further, separate
  per-instance questions -- not a clean single-cause fix.

### bees -- `filesystems` -- `PredicateNotFound`
- **first blocker**: predicate
- **concrete unsupported shape**: `lib.mkIf (cfg.filesystems != { })`
  -- a comparison against an empty ATTRSET literal. `ValueExpr::Literal`
  only represents `Null`/`Bool`/`Str` -- no list/attrset variant exists
  at all. Already named in E1's own census as GAP-6.
- **frequency among residual 13**: 1/13 (E1's own original census also
  saw the LIST-literal sibling of this exact gap on `flame`'s `apps`/
  `categories` watches, but those candidates are `FINDING`s overall in
  E1-R, not part of these 13 -- so within the RESIDUAL 13 specifically,
  this is a singleton).
- **counterfactual**: uncertain, not verified whether the real test ever
  sets `filesystems` to a non-empty value.

### authelia -- `instances.*.enable` -- `PredicateNotFound`
- **first blocker**: predicate
- **concrete unsupported shape**: the real gating logic is
  `enabledInstances = lib.filterAttrs (name: instance: instance.enable)
  cfg.instances;` -- a plain function call (`filterAttrs`) mediating
  which instances get a systemd service, never a textual `mkIf`/
  `optional*` site referencing `instance.enable` at all. Already named
  in E1's own census as GAP-7.
- **frequency among residual 13**: 1/13, unique.
- **counterfactual**: uncertain, likely a genuinely harder, more
  open-ended feature (recognizing an option used inside a lambda
  passed to a standard higher-order function) than anything P0-P2
  touched -- not a quick, bounded fix.

### akkoma -- `enable` -- `TestConfigUnresolved`
- **first blocker**: test-config
- **concrete unsupported shape**: the real `services.akkoma.enable =
  true;` assignment lives inside `serverConfig`, a separate `let`-bound
  module FUNCTION, referenced only by NAME via `imports = [ commonConfig
  serverConfig ];` inside the `akkoma-a`/`akkoma-b` node bodies -- not
  inline, and not a file-path import either (a local identifier alias
  to a function value, structurally traceable in principle the same way
  H2's own alias resolution already chases identifiers for predicates,
  but `scan_test_assignments` doesn't do this today).
- **frequency among residual 13**: 1/13 in this exact form (named-
  local-function import, as opposed to `monado`'s external-file import
  or `ferretdb`'s function-call-argument nesting -- related in spirit,
  not the same mechanism).
- **counterfactual**: plausible. The real assignment does exist and is
  a genuine `false -> true` transition; whether reaching it needs full
  alias-chasing (a real, nontrivial addition to `scan_test_assignments`)
  or something narrower wasn't resolved further here.

### spacecookie -- `enable`, `openFirewall` -- `TestConfigUnresolved`
- **first blocker**: test-config
- **concrete unsupported shape**: `nodes = { ${gopherHost} = {...};
  ${gopherClient} = {}; };` -- the node NAMES themselves are dynamic,
  interpolated `let`-bound string keys, not literal identifiers.
  `scan_test_assignments`'s own `nodes.foo`/`nodes = { foo = ...; }`
  handling presumes literal instance names; a `${...}`-interpolated key
  is a structurally different (`NODE_DYNAMIC`) attribute the walker
  can't resolve to a concrete instance at all -- even though the actual
  option assignments one level in (`enable = true; openFirewall =
  true;`) are perfectly plain and inline.
- **frequency among residual 13**: 1/13, unique.
- **counterfactual**: the STRONGEST, cleanest counterfactual of the
  entire test-config cluster. Both real assignments are directly inline
  (no further indirection once the node-name problem itself is solved)
  -- if dynamic node-name keys were resolved to their real `let`-bound
  string values (a bounded, well-defined feature: evaluate a `NODE_
  DYNAMIC` key against the same lexical-scope resolver H2 already has
  for identifiers), both would almost certainly become real, clean
  `PASS`es.

### pomerium -- `enable`, `useACMEHost` -- `OptionNotFound`
- **first blocker**: declaration
- **concrete unsupported shape**: `cfg = config.services.pomerium;` is
  bound INSIDE `config`'s own value expression (`config = let cfg = ...;
  in mkIf cfg.enable {...};`), a sibling scope to `options`, never an
  ancestor of the `options.services.pomerium = {...};` declaration's own
  tree position. `resolve_cfg_root`'s ancestor-walk (used as the flat-
  dotted branch's own safety gate) can never reach it, so the safety
  condition fails and the WHOLE declaration block is skipped -- zero
  options discovered, not just one entry missing. Already named in E1's
  own census as GAP-5, confirmed never touched by P0-P2 (a different
  mechanism from P2's own fix, which only affects walking WITHIN an
  already-accepted block).
- **frequency among residual 13**: 1/13, unique.
- **counterfactual**: uncertain, not verified whether real predicate/test
  evidence for `enable`/`useACMEHost` exists beyond what's already known
  (a real `mkIf cfg.enable {...}` almost certainly exists, per the
  module's own `config = ... mkIf cfg.enable {...};` structure quoted
  above, but this wasn't independently re-derived this round).

## Grouped by concrete root cause (not by declaration/predicate/test-config
stage alone -- that 3-way split is misleadingly close to even and hides
that neither "declaration" nor "test-config" is actually ONE fix)

| root cause | candidates | count/13 | one reusable fix? |
|---|---|---|---|
| value-wrapper (`with`/`let...in`) not unwrapped before requiring `NODE_ATTR_SET` | jitsi-meet, i2pd | 2 | **yes** -- one change to `scan_options`, both fixed |
| combined multi-segment dotted attrpath not walked like a nested attrset | nimdow (declaration side), matterjs-server (test-config side) | 2 | **no** -- two different scanners, two separate fixes, related only in spirit |
| test-side import/reference/function indirection (4 genuinely different concrete mechanisms) | monado (external file import -- likely out of scope entirely), akkoma (named local function import), tor (function-call-built topology), ferretdb (function-call argument nesting) | 4 | **no** -- not one fix; at best a loose family, and one member may be unfixable within this project's own stated scope |
| dynamic/interpolated node-name key | spacecookie | 1 | yes, in isolation, but 1/13 alone |
| `cfg` bound inside `config`'s own value (safety-gate scope) | pomerium | 1 | yes, in isolation, but 1/13 alone |
| `mkEnableOption ... // mkOption {...}` (RHS is a call, not a literal attrset) | monado (`highPriority`) | 1 | yes, in isolation, but 1/13 alone |
| cross-module default-value reference (`config.<other option>`) | libinput | 1 | a real, deeper feature, not a quick fix |
| empty list/attrset literal comparison (`!= []`/`!= {}`) -- GAP-6, already named in E1 | bees | 1 | yes, in isolation, but 1/13 alone |
| predicate mediated through a plain HOF call (`filterAttrs`) -- GAP-7, already named in E1 | authelia | 1 | a real, deeper feature, not a quick fix |

## Decision, against the rule fixed before this census started

> if one root cause ≥ 4/13: targeted OBA fix
> if two root causes together ≥ ~60%: consider two sequential fixes
> if diffuse: OBA freeze at current level → CDC generated-config work

**No single concrete root cause reaches 4/13.** The largest genuine
single-mechanism cluster (value-wrapper unwrapping) is 2/13 (~15%). The
largest THEMATIC cluster (test-side indirection, 4/13 ≈ 31%) is not one
fix at all -- it's (at least) three distinct mechanisms plus one item
that's arguably out of this project's own stated scope entirely, and
reporting it as "one gap, 31%" would be exactly the bucket-counting
fantasy this project's own K2c round got corrected for once already.
Even generously combining declaration-side and test-config-side into
two coarse buckets (6/13 each) doesn't change the underlying fact: ~9
genuinely distinct concrete causes sit behind these 13 candidates, most
hit exactly once.

**Residual is diffuse. Per the pre-fixed rule: OBA freeze at the current
level (P0-P2, E1-R's 21 PASS / 6 FINDING / 13 INCONCLUSIVE / 0
TOOL_ERROR). Next: the CDC generated-config-artifact work** -- E1's own
35%-of-holdout signal, not touched by this census, still the strongest
single lead in either direction.

No app-specific special case was needed or considered anywhere in this
census -- every concrete cause above, where a fix is plausible at all,
would be a general, reusable change to `scan_options`/
`scan_test_assignments`, never a per-app branch. This gate never had to
be invoked to reject anything, since nothing here cleared the frequency
bar in the first place.
