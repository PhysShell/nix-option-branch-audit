# S6-R1 P1: harness repair and validation -- `HARNESS_PARTIALLY_REPAIRED`

**Parents**: `fixtures/s6-r1/P1-adjudication-and-accounting.md` (`cda489c`,
original P1 result), `fixtures/s6-r1/P1-defect-boundary.md` (`b16cd41`,
corrected `f7674e7`), `fixtures/s6-r1/P1-target-semantics-audit.md`
(`256900c`, `TARGET_INVALID_DEFECT_RETRACTED`). Does not change `oba`.
Does not change `v0.5.0`'s own behavior. Does not change any NC
threshold. Does not run P2. Old targets (`targets/s6-r1-p1.toml`) are
not edited or deleted.

## 1. Formal contract (established by GO-D, restated here, not re-derived)

From `README.md`'s own "P2: nested options idiom fix" + S3-F2 entries,
read before any implementation code: `option_prefix` must name a point
where a real `options={...}` root exists (the module's own top-level
root, or -- per GAP-4 -- a point `find_nested_options_block` itself
recurses into); `watch` must be the FULL remaining relative path from
that root to the leaf, as one multi-segment/dotted form when more than
one hop is involved. `option_prefix` must never terminate mid-way
through an interior `mkOption`-wrapped container's own name, expecting
`watch` to supply only the final hop.

A **second, separate** contract fact, found while re-auditing the new-
module PRs below (not part of GO-D's own scope, newly established
here): `README.md`'s own "One concrete, disqualifying bug found" entry
(the 30-PR K-round section) states plainly that `analyze()` requires
its named module file to exist at all under `--base-root`, and that
this is a **known, disclosed, explicitly unfixed** limitation ("Not
fixed in this round -- a concrete, scoped, named follow-up"). A PR
that introduces a wholly new module file therefore has NO valid `check`/
`diff` target constructible against it at all, under the tool's own
current, documented contract -- independent of how `option_prefix`/
`watch` are split.

## 2-5. Per-PR reconstruction, provenance, and change category (all 15 PRs, frozen sample order)

Re-derived from the real source diff at each PR's own frozen base/head
SHA (`fixtures/s6-r1/P1-target-construction.md`), without consulting
the old `oba` verdict to decide any `option_prefix`/`watch` split.

| # | PR | Old target(s) | New target(s) | Category | Provenance note |
|---|---|---|---|---|---|
| 1 | #565943 | none (step 3) | none | no change | Re-grepped the real diff: no `mkOption`/`options={`/`mkEnableOption`/`mkPackageOption` addition anywhere. Stop point confirmed independently. |
| 2 | #566007 | none (step 3) | none | no change | Same re-check, no declaration touched. |
| 3 | #569867 | none (step 3) | none | no change | Same re-check. |
| 4 | #508090 | `SKIP_GPU` (`option_prefix=[...,"environment"]`, `watch=["SKIP_GPU"]`); `GPU_COLLECTOR` (same shape) | `pr508090_SKIP_GPU`/`pr508090_GPU_COLLECTOR` (`option_prefix=["services","beszel","agent"]`, `watch=["environment.SKIP_GPU"]`/`["environment.GPU_COLLECTOR"]`) | **`PREFIX_ROOT_ERROR` + `WATCH_PATH_ERROR`** (both targets) | Declaration canonical path: `services.beszel.agent.environment.{SKIP_GPU,GPU_COLLECTOR}`. `environment` is itself `mkOption{type=submodule{freeformType=...;options={...};};}` -- an interior container, not a root. |
| 5 | #568429 | `settings`, `settings.server.port`, `settings.oauth.auth-dir`, `openFirewall` | `settings`, `openFirewall` unchanged; `settings.server.port`/`settings.oauth.auth-dir` corrected to `option_prefix=["services","cliproxyapi"]`, `watch=["settings.server.port"]`/`["settings.oauth.auth-dir"]` | **no change** (`settings`, `openFirewall`); **`PREFIX_ROOT_ERROR` + `WATCH_PATH_ERROR`** (the other two) | Canonical paths: `services.cliproxyapi.settings` and `.openFirewall` are direct top-level leaves (no change needed); `services.cliproxyapi.settings.server.port`/`.settings.oauth.auth-dir` sit inside `settings`'s own interior submodule container. |
| 6 | #563823 | `aclPolicies` (`option_prefix=["services","rundeck"]`, `watch=["aclPolicies"]`) | unchanged | **no change** | Verified directly against real head source (`rundeck.nix`): `aclPolicies` is a flat leaf directly inside the module's own top-level `options={...}` block (line 57/141) -- no interior `mkOption`-wrapped container anywhere on its path. Already contract-correct. |
| 7 | #566696 | none (step 3) | none | no change | Re-checked, version bump only, no declaration touched. |
| 8 | #443747 | 10 targets, all on a brand-new module (`gophernicus.nix`, absent in base) | none constructible | **`NEW_MODULE_NO_BASE`** -> **`NOT_EVALUABLE_BY_CURRENT_PROTOCOL`** | Per the second contract fact above (README's own disclosed, unfixed `analyze()`-requires-module-file-under-`--base-root` limitation): no valid `check`/`diff` target exists for a PR whose entire module file is new. This is a tool-contract boundary, not a prefix/watch construction error -- renaming `option_prefix`/`watch` cannot fix it. |
| 9 | #556752 | none (step 3) | none | no change | Re-checked: only default *values* inside an existing config attrset changed, no `options={...}` declaration. |
| 10 | #564688 | none (step 3) | none | no change | Re-checked across all 7 touched module files: zero option declarations in the diff. |
| 11 | #568782 | none (step 5) | none | no change | Re-confirmed: `package = lib.mkPackageOption pkgs "djbdns" {};` is a real new declaration (step 3 passes), but no wired test exists for `dnscache` at head -- step 5 stop is correct, independent of prefix/watch. |
| 12 | #567915 | none (step 2) | none | no change | Re-checked: only a `nixos/tests/**` file changed, no `nixos/modules/**` file in the diff at all. |
| 13 | #568245 | none (step 2) | none | no change | Same re-check. |
| 14 | #561242 | none (step 2) | none | no change | Same re-check. |
| 15 | #568048 | 11 targets, all on a brand-new module (`ollaya.nix`, absent in base) | none constructible | **`NEW_MODULE_NO_BASE`** -> **`NOT_EVALUABLE_BY_CURRENT_PROTOCOL`** | Same reasoning as `#443747` above. |

No `BASE_FILE_SELECTION_ERROR` or `RENAME_MOVE` case was found among
the 15 -- those categories are listed in the spec as possibilities,
not asserted to occur; none of the 15 PRs involved a file rename/move
whose base-file selection could have been wrong.

## 6. New-module determination, in detail

Per the contract fact in Section 1, a brand-new module file produces
`TOOL_ERROR` (exit 3) on `--root .../base` **by the tool's own current,
documented, intentional design** -- not a missing/malformed target. No
artificial base tree was substituted. Both PRs' 21 targets are
reclassified `NOT_EVALUABLE_BY_CURRENT_PROTOCOL` (a distinct label from
`INPUT_OR_HARNESS_FAILURE`, which implies a harness mistake this
pilot's own tooling could have avoided -- this one cannot, without
`oba` itself changing, which is out of this round's scope).

## 7-8. Re-run and re-adjudication

**Re-run**: the 7 targets of Section 2-5 that needed no change were
never re-run (identical binary/source/target -- re-running would only
reproduce `fixtures/s6-r1/P1-oba-raw/{563823,568429}.*.json` already on
record). The 4 corrected targets were already run for real by GO-D
(same `v0.5.0` sha256-verified binary, same real base/head source, no
corpus expansion) -- reused directly rather than re-executed a third
time: `fixtures/s6-r1/P1-target-semantics-evidence/alt-{568429,508090}-{base,head}.json`.
New manifest recording the corrected set, for the record:
`targets/s6-r1-p1-corrected.toml`.

**Re-adjudication, disclosed as NOT blind**: this fork had already read
`P1-target-semantics-audit.md`'s own stated verdicts (`PredicateNotFound`
x3, one real witnessed `PASS`) before this document was written -- full
blind-then-reveal per `adjudication-rubric.md` is not achievable here,
stated honestly rather than pretended. Classification is therefore a
disclosed retrospective judgment against the real evidence, not a fresh
blind prediction:

- `pr568429_settings_server_port` -> **`PredicateNotFound`** (declaration
  found at `['settings','server','port']`, not gated by its own
  predicate -- `cfg.settings.server.port` is consumed as a plain value
  under `openFirewall`'s own, separate predicate). **Classification:
  `CORRECT`** -- this is exactly the FIRST reviewer's own original
  blind prediction ("head should... report `PredicateNotFound`"),
  which was right all along; only the target's own construction was
  wrong, not the reviewer's reasoning.
- `pr568429_settings_oauth_auth_dir` -> **`PredicateNotFound`** (found,
  not consumed by `config` at all). **`CORRECT`**, same reasoning.
- `pr508090_SKIP_GPU` -> **`PASS`**, a real witnessed finding (predicate
  `!cfg.environment.SKIP_GPU` gating `GPU_COLLECTOR`, witnessed at
  `nixos/tests/beszel.nix:101`, `environment.SKIP_GPU = true`).
  **`CORRECT`** -- the tool correctly resolved a real predicate against
  real test evidence once given a valid request.
- `pr508090_GPU_COLLECTOR` -> **`PredicateNotFound`** (found; it has no
  OWN boolean predicate of its own -- it is the VALUE gated by
  `SKIP_GPU`'s predicate, not a predicate subject itself). **`CORRECT`**.

All 4 reclassify from `MISSED_FINDING` to `CORRECT`. The
escalation requirement (`adjudication-rubric.md` item 12) that applied
to all 4 under the old classification no longer applies -- `CORRECT`
does not trigger escalation.

`pr563823_aclPolicies`/`pr568429_settings`/`pr568429_openFirewall`
remain `CORRECT`, unchanged (their construction never changed).

## 9. Accounting -- both numbers, side by side, via `pilot_accounting`'s own tested functions

| | `derived_pr_count` | `derived_target_count` | `substantive_target_count` | `adjudicated_target_count` |
|---|---|---|---|---|
| **Original cohort** (`cda489c`) | 5 | 28 | 7 | 7 |
| **Valid-under-corrected-protocol** | 3 | 7 | 7 | 7 |

```
ORIGINAL COHORT (unchanged, kept for reference, NOT recomputed):
  nc2_pr_level(5)        -> {numerator: 5, denominator: 15, passed: False}   # need >=9
  nc3_target_level(7,28) -> {numerator: 7, denominator: 28, threshold: 20, passed: False}
  nc4_target_level(7,7)  -> {numerator: 7, denominator: 7,  threshold: 5,  passed: True}

VALID-UNDER-CORRECTED-PROTOCOL:
  nc2_pr_level(3)        -> {numerator: 3, denominator: 15, passed: False}   # need >=9
  nc3_target_level(7,7)  -> {numerator: 7, denominator: 7,  threshold: 5,  passed: True}
  nc4_target_level(7,7)  -> {numerator: 7, denominator: 7,  threshold: 5,  passed: True}
```

**The sampled-PR denominator (15) is unchanged in both rows** -- only
the numerators and NC3's own denominator move, per the user's own
explicit instruction not to silently swap denominators while
pretending nothing changed. `derived_pr_count`'s drop from 5 to 3 is
the `NOT_EVALUABLE_BY_CURRENT_PROTOCOL` reclassification of `#443747`/
`#568048`: under the corrected view, they never produced a real,
evaluable target at all, so they do not count as "derived."

**NC2: KILL under both views** (3/15 and 5/15 are both far below 9 --
correcting the harness does not rescue NC2; this pilot's own PR-level
derivation yield is genuinely low, independent of either harness bug).
**NC3: KILL under the original cohort, PASS under the corrected
protocol** (7/28 fails the >=20 threshold; 7/7 clears >=5 easily --
this is the one number the harness repair actually moves). **NC4:
PASS under both views.**

## 10. The GO-D hypothesis, answered directly

**The hypothesis is REFUTED, not confirmed.** Zero of the original 21
`INPUT_OR_HARNESS_FAILURE` cases are explained by the `option_prefix`/
`watch`-split error -- that error affected a *different*, disjoint set
of 4 targets (all classified `MISSED_FINDING`, never
`INPUT_OR_HARNESS_FAILURE`, in the original accounting). All 21
`INPUT_OR_HARNESS_FAILURE` cases are explained by a second, separate,
already-documented, already-disclosed `oba` limitation (new-module
PRs have no valid `check`/`diff` target under `--base-root` at all).
Zero of the 21 remain unresolved/unexplained -- the cause is fully
identified, just not the SAME cause as the other defect, and not one
this round is authorized to fix (it requires `oba` itself to change).
This is the less tidy, more honest answer the user's own closing
remark anticipated might be the case ("было бы почти подозрительно
удобно... иногда мир случайно сотрудничает" -- here, it did not).

## Final verdict

**`HARNESS_PARTIALLY_REPAIRED`**

The `option_prefix`/`watch`-split defect (4 targets, 2 PRs) is fully
repaired and validated: the corrected construction is contract-derived
(not post-hoc-fitted to a desired `oba` outcome), re-run for real, and
re-adjudicated to `CORRECT` with disclosed non-blindness. The
new-module/no-base-file gap (21 targets, 2 PRs) is NOT repairable
within the current protocol -- it is a real, already-documented,
already-disclosed, currently-unfixed limitation of `oba` itself (not
of this pilot's own target construction), and fixing it is explicitly
out of this round's scope (`oba` changes forbidden). The honest
characterization is therefore "partially repaired," not "repaired":
one of the two root causes behind the original NC3 KILL has a harness-
level fix; the other does not and cannot, without touching `oba`.

## What did not happen

No change to `oba`'s source or binary. No change to `README.md`. No
change to `target-construction-protocol.md`, `pilot-accounting.py`, or
`population-and-sampling.py` (the contract/exclusion rules established
here are recorded in this document only -- whether to fold them into
the standing protocol for future rounds is a separate decision, not
made here). No NC threshold changed. No P2 work. Old targets in
`targets/s6-r1-p1.toml` left unedited.

**`S6_R1_P1_GOB_HARNESS_PARTIALLY_REPAIRED`**

**STOP.** No automatic transition into P2. The corrected-protocol
numbers above (NC2 KILL, NC3 PASS, NC4 PASS) are reported for
transparency, not as a passing-batch decision -- `batch_passes_
expansion_gates` requires NC2 AND NC3 AND NC4 to all pass, and NC2
still fails under both views. Any P2 decision requires its own
separate, later, explicit GO, informed by -- but not automatically
triggered by -- this result.
