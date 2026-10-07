# S6-R1 P1: oba execution, adjudication, and accounting (NC2/NC3/NC4)

**Parent**: `fixtures/s6-r1/P1-target-construction.md`. Re-reads
`fixtures/s6-r0/adjudication-rubric.md` and `pilot-accounting.py`
verbatim before writing any blinded answer below.

## Procedural note, disclosed rather than hidden

1. **Wrong binary caught before it tainted the record.** The `oba` on
   this machine's `PATH` reported `"version": "0.4.2"` -- a stale dev
   build predating the `v0.5.0` release, not the subject under test.
   Caught on the very first invocation (version field is printed in
   every `--json` envelope) before any verdict was adjudicated. Fixed
   by downloading the real release asset
   (`oba-x86_64-unknown-linux-musl.tar.xz` from the `v0.5.0` GitHub
   release) and verifying its `sha256.sum` before any further use. All
   results below are from that verified `v0.5.0` binary
   (`"version": "0.5.0"` in every JSON envelope in
   `fixtures/s6-r1/P1-oba-raw/`).
2. **One blinding-order breach, disclosed.** While diagnosing the
   version issue, `pr508090_SKIP_GPU`/`pr508090_GPU_COLLECTOR`'s real
   verdicts were looked at before a written blinded answer was
   recorded for them -- a genuine deviation from the rubric's own
   ordering ("write the answer BEFORE oba's verdict is revealed").
   Both are adjudicated below with that breach stated plainly, not
   silently corrected after the fact. The remaining 5 substantive
   targets (`#568429`'s four, `#563823`'s one) were adjudicated in the
   correct blind-then-reveal order.

## Phase B: real `oba` execution

Ran `oba check --root <base/head> --targets <manifest> --json` per PR
(gives full per-target verdicts/evidence) and `oba diff --base-root
--head-root --targets --json` (gives the transition summary). Raw
output for all 3 PRs that produced a result: `fixtures/s6-r1/P1-oba-raw/
{508090,568429,563823}.{base,head}.json` (+ `.json` for the diff mode).

**#443747 and #568048 (21 targets): `TOOL_ERROR`, exit 3, on `--root
.../base`.** Both PRs introduce a brand-new module file; the base
tree therefore has no such file to resolve at all --
`fixtures/s6-r1/P1-oba-raw/{443747,568048}.base.stderr`:
`module: ... not found under --root ...`. This is `oba` correctly
refusing to analyze a target whose module file plainly does not
exist, exactly as its own `--root` security-boundary contract
documents ("an absolute `module`/`test`, a `../` escape... all become
a TOOL_ERROR"). **This is a harness-construction gap in this pilot's
own target-construction step (step 1), not an `oba` defect**: steps
1-6 were already frozen before this was discovered, and per the
protocol's own ordering guarantee, the frozen target records are not
edited after step 7 -- so all 21 of these targets are recorded exactly
as `INPUT_OR_HARNESS_FAILURE`, honestly, rather than retried with a
different base-tree representation after the fact.

## Phase C: adjudication (7 substantive targets)

### `pr563823_aclPolicies`

**Evidence**: `aclPolicies` declared `options.services.rundeck.aclPolicies
= mkOption { type = attrsOf str; default = {...}; }`; only the
*default value* (an ACL-policy text blob) changed between base and
head. Consumption site: `lib.mapAttrsToList (name: content: install
...) cfg.aclPolicies` -- a generic attribute-set iteration, not a
boolean/Truthy gate. `nixos/tests/rundeck.nix` never references
`aclPolicies`/`acl` at all, in either version.

**Blinded answer** (written before opening the verdict JSON): no
provable predicate-outcome transition exists -- the only change is to
a default value, not to any predicate or config-generation site oba's
Truthy/enable model would watch; expect the same inconclusive-style
verdict in both base and head.

**Revealed**: base `PredicateNotFound`, head `PredicateNotFound`
(identical). **Classification: `CORRECT`.**

### `pr568429_settings`

**Evidence**: `settings`'s own `mkOption` block has its `type` field
changed (flat freeform -> submodule with nested `options`), but
`cfg.settings` itself is still only ever consumed as opaque data
(passed to `secretsReplacement`/the YAML generator), never gated by
its own boolean predicate.

**Blinded answer**: no predicate exists for `settings` as a whole in
either version; expect `PredicateNotFound` both sides, no transition.

**Revealed**: base `PredicateNotFound`, head `PredicateNotFound`.
**Classification: `CORRECT`.**

### `pr568429_openFirewall`

**Evidence**: `networking.firewall = lib.mkIf cfg.openFirewall {
allowedTCPPorts = [ <port expr> ]; };` -- a real Truthy/`mkIf`
predicate, present in both base and head at the same structural
position (only the port *value* expression inside the body changed,
from a let-bound `port` var to `cfg.settings.server.port`; the
predicate condition `cfg.openFirewall` itself is untouched).

**Blinded answer**: a resolvable predicate exists in both versions,
structurally identical; expect the same verdict both sides, no
transition.

**Revealed**: base and head both `OBA001`, identical predicate span
shape (`Truthy: mkIf` on `cfg.openFirewall`), both `default_outcome:
false`, both predicate attempts `witnessed: false` (the real
`nixos/tests/cliproxyapi.nix` test fixture never sets
`openFirewall = true`, so the predicate is legitimately never
witnessed in either version). **Classification: `CORRECT`.**

### `pr568429_settings_server_port` and `pr568429_settings_oauth_auth_dir`

**Evidence**: in base, `settings`'s type has no nested `options` block
at all (flat `format.type` freeform) -- no declaration for
`server.port`/`oauth.auth-dir` can exist. In head, both are declared
as `options.server.port`/`options.oauth.auth-dir` *inside*
`settings`'s own `mkOption { type = lib.types.submodule { freeformType
= ...; options = { ... } } }` block -- the exact "option declared
inside another option's own nested submodule `options={...}` block"
shape this project's own `README.md` ("P0: the xandikos
nested-submodule declaration collision (GAP-4), fixed") documents as a
supported capability (`find_nested_options_block` recursing into a
nested `options={...}` found inside an outer `mkOption{...}` call,
e.g. `nginx.enable` correctly found at `["nginx","enable"]`).

**Blinded answer**: base should be `OptionNotFound` (declaration
genuinely absent). Head *should*, per the documented capability above,
locate the nested declaration and then report `PredicateNotFound`
(the declaration exists, but neither field is gated by any boolean
predicate anywhere in the module -- `server.port` is consumed as a
plain value in `allowedTCPPorts`, under `openFirewall`'s *own*
predicate, not its own; `oauth.auth-dir` is not consumed by the Nix
module's `config` section at all). If head instead reports
`OptionNotFound`, that would mean the documented nested-recursion
capability fails for this shape despite the option genuinely existing
in source -- flagged in advance as a possible defect, not assumed.

**Revealed**: base `OptionNotFound` (both) -- matches. **Head also
`OptionNotFound` (both)** -- does **not** match; the declaration
genuinely exists in the real head source (confirmed by direct
reading, not inferred) but was not found.

**Follow-up repro, run after this adjudication (not used to change the
blinded answer above, only to characterize the finding)**: a minimal
synthetic module (`options.services.foo.environment = mkOption { type
= submodule { options = { SKIP_X = mkOption {...}; }; }; };`, no
`freeformType` at all, structurally as close to the documented
`nginx.enable` example as this repo's own README states it) was
checked with the same verified `v0.5.0` binary and **also** produced
`OptionNotFound` for `SKIP_X` -- so the gap is not specific to
`freeformType` coexisting with `options` (an earlier hypothesis,
ruled out); it reproduces even for the plainest possible "option
nested inside another option's own submodule-typed field" shape, one
level below the module's own `cfg_ident` binding. Root cause was NOT
isolated at the `src/main.rs` level within this pilot's own scope --
only the user-visible behavioral gap is confirmed, reproducibly, via
one minimal synthetic case plus two independent real nixpkgs PRs
(this one and `#508090`, below).

No match in `README.md`/`fixtures/s5-remediation-closeout/closeout.md`
for this exact shape (`freeformType` is not mentioned anywhere in
`README.md`; the only documented nested-submodule fix, GAP-4, is
stated as already covering exactly this shape) -- this does not look
like an already-disclosed, intentional limitation.

**Classification: `MISSED_FINDING`** (the analyzer's own claimed,
documented capability -- discover an option nested inside another
option's own submodule type -- fails to surface a genuinely-present
declaration, under a materially identical shape to the one its own
docs cite as the fixed/working case).

### `pr508090_SKIP_GPU` and `pr508090_GPU_COLLECTOR` (blinding-order breach disclosed above)

**Evidence**: `environment = mkOption { type = submodule { freeformType
= attrsOf str; options = { SKIP_SYSTEMD = ...; SKIP_GPU = mkOption
{...}; GPU_COLLECTOR = mkOption {...}; }; }; }` -- base has only
`SKIP_SYSTEMD`; head adds `SKIP_GPU`/`GPU_COLLECTOR` as new nested
declarations, same shape as `#568429` above.

**Answer** (written after already having looked at the verdict JSON,
per the disclosed breach -- not a genuine blind prediction, a
retrospective judgment): base `OptionNotFound` is correct (truly
absent). Head `OptionNotFound` is the same gap as `#568429`'s pair --
both options are real, present declarations
(`cfg.environment.SKIP_GPU`/`GPU_COLLECTOR`, directly read by
`activeCollectors = lib.optionals (!cfg.environment.SKIP_GPU)
cfg.environment.GPU_COLLECTOR;`, itself feeding real predicate-shaped
logic elsewhere in the same file), silently not found.

**Classification: `MISSED_FINDING`**, same root-cause family as
`#568429`'s pair, reported with the blinding caveat above rather than
presented as cleanly independent confirmation.

## Escalation (adjudication-rubric.md item 12)

Per the rubric's own trigger list, a second, independent reviewer is
required for every `MISSED_FINDING`/`FALSE_FINDING`/
`MISLEADING_PRESENTATION` classification. **All 4 targets classified
`MISSED_FINDING` above** (`pr568429_settings_server_port`,
`pr568429_settings_oauth_auth_dir`, `pr508090_SKIP_GPU`,
`pr508090_GPU_COLLECTOR`) **require this escalation.** A genuinely
independent second reviewer (a fresh process with no memory of this
writeup) was not spawned within this fork -- stated honestly rather
than faked. The 3 `CORRECT` targets do not trigger escalation.

## Phase D: accounting (via `pilot_accounting`'s own tested functions, not hand-arithmetic)

| Counter | Value |
|---|---|
| `sampled_pr_count` | 15 |
| `derived_pr_count` | 5 |
| `derived_target_count` | 28 (`apply_target_cap`: `accepted=28, cut=0`) |
| `substantive_target_count` | 7 (28 minus 21 `INPUT_OR_HARNESS_FAILURE`) |
| `adjudicated_target_count` | 7 (all 7 substantive targets reached a definite, non-`ORACLE_AMBIGUOUS` classification: 3 `CORRECT` + 4 `MISSED_FINDING`) |

```
nc2_pr_level(5)                 -> {numerator: 5,  denominator: 15, passed: False}   # need >=9
nc3_target_level(7, 28)         -> {numerator: 7,  denominator: 28, threshold: 20, passed: False}
nc4_target_level(7, 7)          -> {numerator: 7,  denominator: 7,  threshold: 5,  passed: True}
batch_passes_expansion_gates(5, 7, 28, 7) -> False
```

**NC2: KILL** (5 of 15 sampled PRs yielded a mechanically-derivable
target; need >=9). **NC3: KILL** (7 of 28 derived targets reached a
substantive result; need >=20 -- 21 of the 28 were brand-new-module
`INPUT_OR_HARNESS_FAILURE` cases, a harness gap, not analyzer
behavior). **NC4: PASS** (7 of 7 substantive targets received a
trustworthy, non-ambiguous adjudication; need >=5).

## Taxonomy breakdown

| Label | Count | Targets |
|---|---|---|
| `CORRECT` | 3 | `pr563823_aclPolicies`, `pr568429_settings`, `pr568429_openFirewall` |
| `MISSED_FINDING` | 4 | `pr568429_settings_server_port`, `pr568429_settings_oauth_auth_dir`, `pr508090_SKIP_GPU`, `pr508090_GPU_COLLECTOR` |
| `INPUT_OR_HARNESS_FAILURE` | 21 | all 10 of `#443747`'s targets + all 11 of `#568048`'s targets |
| `FALSE_FINDING` / `MISLEADING_PRESENTATION` / `CONSERVATIVE_INCONCLUSIVE` / `ORACLE_AMBIGUOUS` / `KNOWN_LIMITATION_REOBSERVED` | 0 | -- |

## What this does and does not establish

This batch's own `batch_passes_expansion_gates` is `False` -- per
`preregistration.md`'s staged design, this is informational, not an
instruction to draw a second P1 batch or otherwise expand; that
decision belongs to a separate, later, explicit GO. The `MISSED_FINDING`
finding above (nested-submodule-declared options not discovered) is a
real, reproducible result of this pilot, pending the second-reviewer
escalation the rubric itself requires before it could be called a
confirmed new S6 defect per `adjudication-rubric.md`'s own four-part
"new defect" test.

**`S6_R1_P1_ACCOUNTING_COMPLETE_NC2_KILL_NC3_KILL_NC4_PASS`**

**STOP.** No P2/expansion decision made here. That requires a
separate, later, explicit GO.
