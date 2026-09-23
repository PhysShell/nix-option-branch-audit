# S5-F1C: investigation and traces

Required by the mandate: explicit traces for one `with types` case, one
`with lib.types` case, the portmaster alias chain, the k3s ordering case,
and the angrr counterexample -- each showing AST/source -> lexical
resolution -> type/reference resolution -> provenance edge(s) ->
discovered declaration -> final candidate order.

## `with types` case -- rspamd `workerOpts` (PR #484133)

- **AST/source**: `workers = mkOption { type = with types; attrsOf
  (submodule workerOpts); ... };`, a true-root declaration
  (`option_prefix = [services, rspamd]`). The WHOLE FILE also opens with
  its own `with lib;`, unrelated to this specific reference.
- **Lexical resolution**: the `workerOpts` identifier's nearest ancestor
  is the LOCAL `with types;` node (directly wrapping this one type
  field) -- closer than the enclosing `let` that actually binds
  `workerOpts`.
- **Type/reference resolution**: `with_namespace_is_recognized_type_namespace`
  recognizes the bare identifier `types` -> `WithPolicy::TransparentForRecognizedTypeNamespace`
  continues the walk outward past this `with`, reaching the enclosing
  `let`, where `workerOpts` IS bound -> resolves to `workerOpts`'s own
  value.
- **Provenance edge**: `workers` (true root) -> `workerOpts` (one hop,
  now resolvable).
- **Discovered declaration**: `workerOpts`'s own direct leaves
  (`count`, `includes`, `bindSockets`, ...) are promoted, bare-recorded.
  `bindSocketOpts` (referenced from `bindSockets`'s own type field,
  itself only discovered because `workerOpts` promoted) is a SECOND hop
  -- stays excluded by the unwidened one-hop cap, unrelated to whether
  F1C-A resolves it or not (confirmed: `count`/`includes`/`bindSockets`
  now present; `socket`/`mode`/`owner`/`rawEntry` -- `bindSocketOpts`'s
  own leaves -- remain absent, matching the design exactly).
- **Final candidate order**: `workerOpts`'s own content sorts by its own
  source position, interleaved correctly with the rest of the file's
  true-root content (F1C-C).

## `with lib.types` case -- prosody `vHostOpts` (PRs #429967/#494314/#431289/#440660/#260551)

- **AST/source**: `virtualHosts = mkOption { type = with types;
  attrsOf (submodule vHostOpts); ... };` (prosody.nix uses the bare
  `types` form too, resolved via the file's own `with lib;` -- but the
  distinguishing, previously-blocking wrapper here is specifically this
  LOCAL `with`, whichever spelling). A second, independently-confirmed
  case in the corpus (rspamd's `mountType`-shaped file, and other real
  nixpkgs modules) genuinely spells this `with lib.types;` -- both forms
  are recognized identically by `with_namespace_is_recognized_type_namespace`.
- **Lexical resolution**: same shape as rspamd's `workerOpts` above --
  the local `with` is nearer than the enclosing `let` that binds
  `vHostOpts`.
- **Type/reference resolution**: resolves via the same
  `TransparentForRecognizedTypeNamespace` path.
- **Provenance edge**: `virtualHosts` (true root, declared directly at
  `option_prefix = [services, prosody]`) -> `vHostOpts` (one hop).
- **Discovered declaration**: `vHostOpts`'s own `enabled`/`ssl`/`domain`/
  `extraConfig` leaves are now promoted and bare-recorded. `domain`
  collides with `mucOpts.domain`/`uploadHttpOpts.domain` (both also
  legitimately promoted, one hop each) -- a real, PRE-EXISTING collision
  (present, even more severely, in the historical v0.4.5 baseline --
  see `fixtures/s5-f1b-r/leaf-collision-scan.json`), not introduced by
  this fix, not resolved by it either (see "Leaf-collision disclosure"
  below).
- **Final candidate order**: verified field-level restoration for all 5
  affected PRs -- `429967`/`494314`/`260551` now fully byte-identical to
  historical (`discovered_options`, not just the watched verdict);
  `431289`/`440660` were already verdict-correct (real S5-F1-R
  regressions restored under S5-F1B) and are now ALSO fully
  discovery-identical.

## Portmaster alias chain (PR #557329)

- **AST/source**: `packages = mkOption { type = listOf
  profilePackageType; ... };` (a real declaration, found via `profiles`'
  own inline submodule -- true-root-anchored). `profilePackageType =
  types.coercedTo types.package (package: {inherit package;})
  packageMatchType;` -- a named `let`-binding whose own value is a type
  COMBINATOR call, not itself a submodule. `packageMatchType =
  types.submodule ({...}: {options = {directory = ...; name = ...;
  package = ...; storeNameRegex = ...; strictHead = ...; strictLast =
  ...; wrapped = ...;};});` -- the real submodule.
- **Lexical resolution**: `packages`'s own `type =` field resolves
  `profilePackageType` directly (dotted, no `with` involved at all).
- **Type/reference resolution**: `resolve_type_reference` resolves
  `profilePackageType` to its own value (the `coercedTo ...` call).
  `find_nested_options_block` on that value finds no `options = {...}`
  block anywhere within it -- classified as a transparent alias step,
  not a submodule -- `collect_type_reference_ranges_rec` recurses into
  it, finding the bare identifier `packageMatchType`, which DOES resolve
  (again dotted, no `with`) to a value that DOES contain its own
  `options = {...}` block -- classified as the real submodule, recursion
  stops there (its own content is walked separately, by the existing
  promoted-candidate mechanism).
- **Provenance edge**: `packages` (true root) -> `profilePackageType`
  (transparent alias, hop 1) -> `packageMatchType` (real submodule,
  logical hop 2, but reached through a transparent alias step, not
  through a second `options={}`-shaped candidate -- F1C-B's own
  recursion is deliberately NOT bounded by the same one-hop cap that
  gates named-binding PROMOTION, since it is not promoting a second
  candidate, only following a reference chain to find what a SINGLE
  promoted candidate's own type field ultimately points at).
- **Discovered declaration**: all 7 of `packageMatchType`'s own leaves
  (`directory`, `name`, `package`, `storeNameRegex`, `strictHead`,
  `strictLast`, `wrapped`) now resolve, bare-recorded, verified
  field-level identical to the historical v0.4.5 output (0 missing, 0
  added).
- **Final candidate order**: unaffected (no pre-existing collision on
  this file).

## K3s ordering case (PR #374017)

- **AST/source**: `manifestModule = let mkTarget = ...; in
  types.submodule ({...}: {options = {enable = ...; target = ...;
  content = ...; source = ...;};});` declared early in the file (inside
  the module's own top-level `let`). `manifests = mkOption { type =
  types.attrsOf manifestModule; };`, declared later, inside `options.
  services.k3s = {...}` -- which ALSO directly declares its own,
  unrelated `enable = mkEnableOption "k3s";`, textually much later in
  the file (a genuine, pre-existing duplicate `enable` path).
- **Lexical resolution / type reference resolution**: `manifestModule`
  resolves directly (dotted, no `with`) from `manifests`'s own type
  field -- one hop, promotes.
- **Provenance edge**: `manifests` (true root) -> `manifestModule` (one
  hop).
- **Discovered declaration**: `manifestModule`'s own `enable`/`target`/
  `content`/`source` are recorded (as they already were under S5-F1B,
  since this is a one-hop, no-`with` case -- F1C-A/B do not change
  WHETHER this content is found).
- **Final candidate order**: this is what F1C-C fixes. Before it, the
  two-pass split appended `manifestModule`'s own content (discovered in
  Pass 2) after ALL of Pass 1's true-root content, moving its `enable`
  (the module's EARLIEST declaration, source-wise) past the file's own
  top-level `enable` (declared much later). Sorting the final vector by
  `(span.line, span.col)` restores the exact historical order: verified
  element-for-element identical to the historical `discovered_options`
  array (not just as a set) against the real, live-fetched PR content.

## Angrr counterexample (PR #471312) -- must remain excluded throughout

- **AST/source**: `services.angrr.settings = mkOption { type =
  types.submodule settingsOptions; };` (true root). `settingsOptions =
  {freeformType = toml.type; options = {temporary-root-policies =
  mkOption {type = with lib.types; attrsOf (submodule
  temporaryRootPolicyOptions); };};};` -- itself a genuine submodule
  (has its own direct `options = {...}` block). `temporaryRootPolicyOptions`
  is a SEPARATE named binding, declaring the real, unrelated `period`.
- **Lexical / type reference resolution**: `settingsOptions` resolves
  directly (dotted, hop 1) -- promotes. Its own `temporary-root-policies`
  field IS now resolvable under F1C-A (the local `with lib.types;` is
  recognized) -- but this resolution happens while WALKING
  `settingsOptions`'s own PROMOTED content, which uses a throwaway,
  discarded reference set (unchanged since S5-F1B) -- so even though the
  reference itself now resolves, it never reaches the SHARED set Pass 2
  decides promotions from.
- **F1C-B's own, independent safety property**: separately,
  `collect_type_reference_ranges_rec`'s recursion, when resolving
  `settingsOptions` from `settings`'s own type field, checks
  `find_nested_options_block` on `settingsOptions`'s resolved value --
  finds its own direct `options = {...}` block -- classifies it as a
  genuine submodule, NOT a transparent alias -- and deliberately does
  NOT recurse into it looking for further references (that's what
  distinguishes it from `profilePackageType` above). This is a second,
  independent reason `temporaryRootPolicyOptions` is never reached via
  the SHARED reference set, on top of the throwaway-set mechanism.
- **Provenance edge**: `settings` -> `settingsOptions` (hop 1, real) ->
  `temporary-root-policies` -> `temporaryRootPolicyOptions` (hop 2, not
  established from the true root).
- **Discovered declaration**: `temporaryRootPolicyOptions`'s own
  `period` stays excluded. Verified via the real, live-fetched PR
  content (`predicate_not_found -> option_not_found`, Changed) AND via
  three dedicated unit tests reproducing this exact shape (one testing
  F1C-A alone, one testing F1C-B alone via a synthetic settingsOptions
  fixture, one testing BOTH active together).

## Leaf-collision disclosure -- unchanged by F1C

`fixtures/s5-f1b-r/leaf-collision-scan.json`'s own 8 flagged PRs were
re-checked against the F1C candidate: every one still shows either the
exact same collision set as historical (`415326`, `374017`, `463443`)
or a STRICT SUBSET of it (the prosody `domain`/`extraConfig`/`ssl`
cases, now SMALLER since `vHostOpts` is legitimately promoted and no
longer silently excluded -- reducing, never worsening, the pre-existing
ambiguity). No new collision introduced anywhere. `run_target`'s own
`.find()` (first-match) selection policy is unmodified, per the
mandate's explicit scope boundary -- this remains open as a separate,
disclosed follow-up question, not addressed here.

## What is supported vs. deliberately unresolved

**Supported**: a one-hop reference to a named submodule, whether written
dotted (`types.submodule <name>`) or wrapped in a recognized `with
types;`/`with lib.types;` (F1C-A); a chain of any number of transparent
type-alias steps (bindings whose own value is not itself a submodule
declaration) leading to the real submodule, cycle-safe (F1C-B); stable,
historical-matching output order regardless of which pass discovered a
declaration (F1C-C).

**Deliberately unresolved, unchanged from S5-F1B**: a second `options=
{...}`-shaped submodule reached only through ANOTHER submodule's own
promoted content (the one-hop cap itself -- tayga's `addrOpts`, rspamd's
`bindSocketOpts`, angrr's `temporaryRootPolicyOptions`); a `with` whose
own namespace expression is not statically `types`/`lib.types`.

**Would require real Nix evaluation, out of scope entirely**: resolving
what a `with someDynamicExpression;` namespace actually contains;
following an alias chain through a computed/dynamic attribute access;
any general dead-code or reachability analysis beyond static, lexical
reference-following.
