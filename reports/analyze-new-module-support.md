# G1: `oba diff` gains support for brand-new (and since-deleted) module files

**Scope**: product engineering on a documented `oba` limitation, triggered
by `fixtures/s6-r1/P1-harness-repair.md`'s own GO-B finding (21/28 P1
targets classified `NOT_EVALUABLE_BY_CURRENT_PROTOCOL` because of this
limitation). This is **not** a reopening of P1, **not** P2, and does
**not** change any frozen S6-R1 metric -- see "Historical frozen P1"
below.

## 1. Prior limitation

`README.md`'s own "One concrete, disqualifying bug found" entry (the
30-PR K-round section): 2 of 30 PRs (both titled "init module")
produced a real `TOOL_ERROR` (CLI exit 3), not a graceful
inapplicability. Root cause, as already documented: "`analyze()`
requires its named module file to exist at all under `--base-root`;
CDC's own half already degrades a missing-on-one-side candidate to a
real `AddedSubject`, OBA's half does not." Marked "**Not fixed in this
round** -- a concrete, scoped, named follow-up."

GO-B's P1 harness-repair re-discovered the identical limitation on real
2026 data: `#443747` (`gophernicus.nix`, 10 targets) and `#568048`
(`ollaya.nix`, 11 targets), both brand-new modules, 21 targets total,
hit the same `TOOL_ERROR` when `oba check --root <base>` found none of
a PR's targets analyzable.

**A second fact, found while tracing this** (before writing any code):
the contract this limitation violates was **already established and
already tested elsewhere in the same codebase**. `run_audit_diff`'s own
`only_on_head`/`only_on_base` partitioning (doc-commented as S1-F1,
with a real regression fixture `fixtures/synthetic/audit-diff-module-lifecycle/`
and 3 passing tests) already treats a target absent on exactly one side
as real PR-diff algebra -- `Added`/`Removed`, never a tool error. Plain
`oba diff` (`run_diff`) simply never received the same treatment; it
called `analyze()` directly on the whole manifest and inherited
`analyze()`'s own strict, all-or-nothing `resolve_within_root`.

## 2. Minimal reproducer

`BASE: nixos/modules/services/x/module.nix` does not exist.
`HEAD`: the same path exists, declaring `services.x.foo.bar`
(`mkOption`).

Before this change (sha256-verified `v0.5.0` release binary):
```
$ oba diff --base-root <base> --head-root <head> --targets <manifest> --json
TOOL_ERROR: target e1_foo_bar: module: resolving nixos/modules/services/x/module.nix
under --root <base>: No such file or directory (os error 2)
exit=3
```

After this change (same fixture, this round's dev build):
```
$ oba diff --base-root <base> --head-root <head> --targets <manifest> --json
{... "diff": {"kind": "added", "head": {"verdict": {"verdict": "PredicateNotFound", ...}}}}
exit=2
```

Full nearby controls (Phase E1), all run for real against the dev build:

| Case | BASE | HEAD | Result |
|---|---|---|---|
| A: existing/unchanged | module exists | module exists, identical | `unchanged`, exit 0 |
| B: existing module, new option added inside it | module exists (no `foo.bar`) | same file, `foo.bar` added | `changed` (`option_not_found -> predicate_not_found`), exit 2 -- goes through the **ordinary** `both_present` path, confirming this is not conflated with file-level birth |
| C: new module (the target limitation) | absent | exists | `added`, exit 2 |
| D: deleted module | exists | absent | `removed`, exit 2 -- **not** the same as C; distinguishable |
| E: rename/move | -- | -- | not separately representable by the current CLI/manifest model: a renamed file surfaces as an independent `Removed` (old path) + `Added` (new path) pair if both paths are declared as separate targets, never as one unified "rename" fact. Characterized here, not built -- out of this round's own minimal scope. |

## 3. Semantic model

`--base-root`/`--head-root` are each a **logical module state at one
point in the PR's own history**, not a mandatory pair of physical
files. `analyze()` itself is unchanged and still requires every target
it's given to physically resolve (Phase E2 Q2: two physical files, by
design, at that layer) -- the fix lives one layer up, at the
diff-orchestration boundary that already existed for `audit-diff`:
partition the manifest by presence on each side *before* calling
`analyze()`, and treat "absent on exactly one side" as the real fact
it is (a PR created or deleted this module), not an error.

For a file absent in BASE, the correct BASE state is **absence of a
module** (Phase E2 Q3) -- not an empty module and not a synthetic empty
option set. Substituting an empty file (Design A, considered and
rejected -- see below) would assert something false: that the module
existed with zero declarations, when the real fact is that the module
did not exist. Nix module evaluation gives no meaning to "an empty
module existed at this commit" when it demonstrably did not (Phase E2
Q5) -- and more concretely, a real module can rely on a relative
import (`import ./shared.nix { ... }`), module arguments, or other
lexical context a fabricated empty file would simply not have (Phase
E2 Q6) -- an empty-file substitute could silently misrepresent what
the module "would have looked like," which this fix never needs to
claim.

## 4. Alternatives considered

| Design | Semantics | Complexity | False-positive risk | False-negative risk | Imports/relative paths | Chosen? |
|---|---|---|---|---|---|---|
| A: synthetic empty BASE source | Invent an empty file where none exists | Low | Low (empty file -> nothing found, usually safe) | None observed, but structurally unprincipled -- an empty file is a FABRICATION, not evidence | Cannot misrepresent imports (there's nothing to import FROM in a one-line stub), but the module's own real HEAD imports are irrelevant to a fake BASE anyway -- the risk is conceptual (asserting a false history), not a concrete bug found here | No |
| B: semantic empty option tree | Don't invent source text; represent BASE as "no declarations" directly in the analysis data structure | Medium (needs a second `AnalysisReport` shape or a sentinel) | Low | Low | N/A (no source involved) | No -- strictly more machinery than C for the same outcome, and `analyze()`'s own contract (always real source) stays simpler if nothing fake is ever constructed in its name |
| **C: asymmetric HEAD-only discovery** (chosen) | For an absent side, skip `analyze()` on that side entirely; run it only where the file is real, and classify by presence (`Added`/`Removed`) | Lowest -- already written, tested, and shipped for `audit-diff`; this round only shares it with `diff` | None added (no new path invents anything) | None added | Structurally immune: no synthetic source is ever read or evaluated, so a real module's own real imports are read correctly or not analyzed at all, never misrepresented | **Yes** |
| D: explicit unsupported subset | Support only provably-safe absence cases, fail closed otherwise | N/A here -- C already covers the full "absent on exactly one side" case without needing a narrower subset; D's own idea (fail closed for cases that can't be proven safe) is exactly what happens for "absent on BOTH sides", which already remains a hard error | -- | -- | -- | Partially -- the "absent on both sides" case already IS D's own fail-closed boundary, inherited unchanged |

Design C was already the project's own chosen answer for `audit-diff`;
this round's own contribution is recognizing that `diff` needed the
identical answer, not inventing a new one, and sharing the
implementation (`compute_oba_diff_entries`) rather than duplicating it.

## 5. Kill-test matrix

| # | Case | Expected | Actual |
|---|---|---|---|
| 1 | New module, one `mkOption` | Added, real verdict, not TOOL_ERROR | `run_diff_module_birth_is_added_not_tool_error`: PASS |
| 2 | New module, nested plain `options={}` attrset | Added, real verdict | covered by `run_diff_new_module_nested_submodule_options_all_discovered` |
| 3 | New module, `mkOption{type=submodule{options=...}}` (GAP-4 shape) | Added, real verdict, declaration genuinely discovered | `run_diff_new_module_nested_submodule_options_all_discovered`: PASS (both `settings.foo.bar`/`settings.baz` found, `PredicateNotFound`, exit 2 not 3) |
| 4 | Multiple newly added options in one new module | All Added independently | same test, 2 options, both correctly classified |
| 5 | Existing module receiving an additional option | `changed`, NOT conflated with module birth | `run_diff_existing_module_both_sides_unchanged_behavior_preserved` (unchanged control) + manual Phase-E1 control B (both present, option added) -- both PASS |
| 6 | Deletion distinguishable from addition | `removed`, not `added` | `run_diff_module_death_is_removed_not_tool_error`: PASS |
| 7 | Missing HEAD file is not silently treated as a new empty module | `removed` (base-only), never `added` | same test as #6 |
| 8 | Malformed HEAD source still fails normally | Inconclusive (parse error), never a silent clean result | `run_diff_malformed_head_on_new_module_still_fails_normally`: PASS, exit 2 |
| 9 | Wrong `option_prefix`/`watch` does not become accepted merely because BASE is missing | `OptionNotFound` on HEAD, never a false discovery | `run_diff_wrong_prefix_on_new_module_is_not_silently_accepted`: PASS |
| 10 | Existing supported BASE/HEAD behavior unchanged | `unchanged`/`changed` via the ordinary path | `run_diff_existing_module_both_sides_unchanged_behavior_preserved`: PASS |

All 10 implemented as real, committed, passing tests (`src/main.rs`,
commit `db4d44c`) plus `fixtures/synthetic/diff-*/` fixtures -- not
merely described.

## 6. Implementation

`src/main.rs`: `compute_oba_diff_entries(base_root, head_root,
&manifest) -> Result<(Vec<ComparisonEntry>, bool, bool, HashSet<PathBuf>)>`
extracted verbatim (zero logic change) from `run_audit_diff`'s own
`only_on_head`/`only_on_base`/`both_present` partitioning. `run_diff`
now calls this shared function instead of `analyze(base)` +
`analyze(head)` + `compare()` directly on the full manifest.
`run_audit_diff` itself calls the same extracted function, so its own
3 pre-existing tests are the regression guard that this round's own
refactor didn't change its behavior. No special-casing of P1 PR
numbers or fixture names anywhere in the implementation -- the fix is
purely structural (partition-before-analyze), and the 21-case replay
below uses the exact same code path as every synthetic kill-test.

## 7. Regression evidence

Full suite (`cargo test --release`): 249 unit tests (`src/main.rs`'s
own `tests` module, including this round's 10 new ones) + 54
`tests/golden.rs` tests + every other integration test binary in the
repo (22, 22, 15, 12, 9, 5, 2, and 1 tests respectively across the
remaining `tests/*.rs` files) -- all green, 0 failed, across all ~390
tests in the repo. `cargo kani`: 6/6
harnesses still `VERIFICATION:- SUCCESSFUL` (K0.1/`eval_known_eq` and
K1/`aggregate` are untouched by this change -- neither is anywhere
near `run_diff`/`run_audit_diff`/`compute_oba_diff_entries`). `cargo
clippy --release`: identical 7 pre-existing warnings, 0 new ones (none
in the touched region).

## 8. Historical engineering replay (NOT part of frozen P1)

```
Historical frozen P1 (fixtures/s6-r1/P1-harness-repair.md):
  unchanged -- NC2 = 3/15 KILL, NC3 = 7/7 PASS, NC4 = 7/7 PASS
  (corrected-protocol cohort); original cohort NC2 = 5/15 KILL,
  NC3 = 7/28 KILL, NC4 = 7/7 PASS. Neither row is touched by this
  document.

Post-fix engineering replay (this round's dev build, same two real
PRs/SHAs GO-B already identified, same real nixpkgs source, raw output
in reports/analyze-new-module-support-evidence/):
  21/21 now evaluable
  0/21 still unsupported
  0/21 fail for another reason
```

Per-target raw results: `#443747` (gophernicus, 10/10 evaluable) --
2x `PASS` (real, witnessed), 2x `TestConfigUnresolved`, 5x
`PredicateNotFound`, 1x `OptionNotFound`. `#568048` (ollaya, 11/11
evaluable) -- 1x `PASS`, 3x `TestConfigUnresolved`, 5x
`PredicateNotFound`, 2x `OptionNotFound`. Full JSON:
`reports/analyze-new-module-support-evidence/replay-{443747,568048}.json`.
**No expansion-gate (NC2/NC3/NC4) recomputation was performed on these
21 targets** -- per this round's own explicit instruction, that would
require a separate, later, explicitly preregistered evaluation.

## 9. Remaining unsupported cases

**Rename/move** (control E, Section 2): not representable as a single
unified fact by the current CLI/manifest model -- only as an
independent `Removed` + `Added` pair if both the old and new paths are
declared as separate targets. No support added for this in this
round; stated as a known gap, not hidden.

**Absent on BOTH sides**: still a hard, fatal error by design (Design
D's own boundary) -- correctly unchanged; this is a genuine
manifest/input problem (nothing in a comparison could say anything
about a target that exists nowhere), not a PR-introduced addition or
removal.

**Permission/read errors, and any resolution failure other than
"genuinely does not exist"**: still propagate as real, fatal errors,
exactly as `resolve_within_root_if_exists`'s own pre-existing contract
already specified -- verified via a nonexistent `--base-root` itself
(distinct error, still TOOL_ERROR/exit 3, not silently treated as "new
module").

## 10. Compatibility / risks

No CLI flag, manifest schema, or JSON envelope shape changed --
`run_diff`'s own output shape (`DiffEnvelope`/`ComparisonEntry`) is
identical before and after; only the SET of entries it can now
produce without erroring is larger (a manifest that previously crashed
the whole invocation on a birth/death case now produces a real `Added`/
`Removed` entry for it, same as `audit-diff` already did). No change to
`check`'s own existing all-absent hard-stop
(`"no target in this manifest is analyzable"`), `target-construction-protocol.md`,
or any S6-R1 artifact. The one live risk already identified and
accepted in Section 9: a PR that renames a module still cannot be
represented as a single semantic "rename" fact -- a future consumer
treating `Removed`+`Added` as two independent, unrelated events (rather
than recognizing the pattern) could double-count or misreport it; out
of scope to fix here, flagged for whoever next touches rename handling.

## Verdict

**`NEW_MODULE_SUPPORT_IMPLEMENTED`**

Semantics were explicit before implementation (Section 3, derived from
`analyze()`'s own documented contract plus the already-shipped
`audit-diff` precedent, not invented to make a test pass). The minimal
new-module repro works (Section 2). All 10 kill-tests pass (Section
5). Pre-existing behavior passes regression in full (Section 7).
Missing BASE files are distinguished safely from unrelated failures
(a nonexistent `--base-root` itself, or a malformed-but-present file,
both still produce real, distinct, non-"new-module" outcomes --
Sections 2 and 9).

**Commit**: `db4d44c` (implementation + kill-tests + fixtures), this
report + replay evidence in a following commit.

**STOP.** No fresh S6 evaluation started. No P2. No version
release/bump. No `README.md` rewrite beyond this report's own separate
file. No further fix attempted (rename/move remains an open, disclosed
gap for a future round, not touched here).
