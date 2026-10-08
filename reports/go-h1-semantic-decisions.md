# GO-H1 — Semantic decisions

Status: the H1.1 stop is superseded by the owner's scope ruling (README Layer A retained). Sections H1.0–H1.13 below are the first pass and are kept for provenance. The current rulings, the three-PR re-adjudication, W, the funnel and the bounded review are in the section "GO-H1A: applied rulings" at the end. Where the two disagree, the GO-H1A section governs.

## H1.0 Provenance

- Branch `claude/go-h-protocol-redesign`, base GO-H commit `3aa3ac6` (CI run 37734845684, success).
- Uncommitted and excluded from the commit: `work/go-f/oba-candidate-9ff7c04` (frozen binary), `work/go-h/__pycache__/`, `work/go-h/h12/src/` (fetched upstream sources).
- GO-E ancestry: `db4d44c` (new-module support) is an ancestor of `9ff7c04` (the frozen candidate). The current post-GO-E state therefore includes new-module support.
- H13 blind reviewer: its verbatim final hand-back is saved as `work/go-h/independent-review/h13-blind-report-verbatim.txt`. It is model output, recorded as data. Its per-PR table is also summarized in `reports/go-h-evaluation-protocol.md` section 9.

## H1.1 Product-goal conflict (STOP)

The H1.1 premise treats oba's unit as a public or runtime contract. The repository documents a narrower declared goal:

- `README.md` lines 3–12 define three separated layers. A is option branch activation (OBA, "this tool"). B is consumer contract drift (CDC, "not built"). C is runtime observability (ROB, "not built").
- `README.md` lines 16–18: "`B PASS` does not imply `C PASS`, and this tool does not attempt either — it answers exactly one question: did a test ever drive this branch's predicate to the *opposite* boolean outcome from what the option's own default produces."
- `README.md` non-goals, lines 170–171: "**No claim about runtime behavior.** `PASS` means 'a test drove this predicate to the opposite outcome from its default', full stop."

A rule that classifies a change to a warning, a system check, or a config block as a change to the option's "runtime contract" would make oba's eligibility depend on behavior the README says the tool does not claim. Under the declared goal, E is defined by option branch semantics only. Under the runtime reading, E is broader. These are two different products.

The brief says: "If repository documentation demonstrates that this statement contradicts the actual declared product goal, STOP and report the conflict rather than forcing these rulings." That condition is met. This report therefore records the conflict and the factual evidence, and does not close U3 or U4.

Scope ruling required from the user: is oba's E defined by (a) the declared goal (option branch activation, README layer A), or (b) a runtime/public contract (which would extend the tool beyond layer A and reopen the README non-goal)?

## H1.2 Factual traces for the three disputed PRs

These traces are scope-independent. They come from the patch hunks and the source at the cited SHAs.

| PR | Changed construct | Operands read | In-module? | Option-contract property changed (default/type/apply/readOnly/declaration) | E under (a) declared goal | E under (b) runtime reading |
|---|---|---|---|---|---|---|
| 566007 | `path` list gains `lib.optionals config.boot.zfs.enabled [config.boot.zfs.package]`. `PrivateDevices` and `PrivateUsers` each gain a conjunct `!config.boot.zfs.enabled`. | `config.boot.zfs.enabled` (new, foreign: declared in `nixos/modules/tasks/filesystems/zfs.nix`). `cfg.smartmon.enable` (existing, in-module, unchanged). | Mixed | None | DEPENDS ON U4: the new operand is foreign; the in-module operand was already read. | ELIGIBLE (predicate change on the service's sandbox and path). |
| 569867 | `system.checks` derivation renamed to `ensure-all-wrappers-paths-exist`. `nativeBuildInputs` drops `pkgs.file`. ELF `file --mime` failure branch removed. | `security.wrappers` values, which are in-module, read by a build-time check. | Yes (inputs) | None | INELIGIBLE: a build-time validation is not a branch predicate of an option. | DEPENDS: validation over in-module option values; not a branch. Ruling required. |
| 566696 | `corePkgs` gains `cosmic-osk`. `excludedCorePkgs` (let-binding over `config.environment.cosmic.excludePackages`, declared at `cosmic.nix` line 63) feeds the `warnings` condition. `cosmic-viewer` added to `systemPackages`. `qt.*` and `switcherooControl.enable` set with `mkDefault`. | `environment.cosmic.excludePackages` (in-module, declared line 63). | Yes (warning operand) | None (the `mkDefault` lines are config assignments: EC11, P2 separate mode) | Warning-only change: DEPENDS ON U4 (a warning is not a declared option branch). Config assignments: INELIGIBLE (P2). | ELIGIBLE under the rubric's EC13 rule (warning reads an option declared in this module). |

Notes:

- The v1 eligibility rubric's EC13 rule ("ELIGIBLE iff the condition reads a module option declared in this module") is satisfied for 566696 by source inspection (`cosmic.nix` line 63 declares `excludePackages`). The blind reviewer's classification is therefore consistent with the rubric's own text. The disagreement with the declaration-only proxy is about scope, not reading error.
- For 566007 the in-module operand `cfg.smartmon.enable` was already present before the change. Whether a predicate that gains only a foreign operand counts as a change to an in-module option's branch is exactly the question U4 asks.
- No `mkPackageOption` change is involved in these three PRs.

## H1.3–H1.7 Rulings (status)

| ID | Question | Status | Basis |
|---|---|---|---|
| U1 | Documentation-only change in scope? | Closed: OUT under both readings | README layer A has no documentation semantics. The runtime reading does not add documentation either. |
| U2 | Configuration-value assignments in scope? | Closed for the current product: separate mode (P2), not in E | README: "No Nix evaluation", so config values are not evaluated. Under reading (b), config values could be in scope; this closes only under (a). |
| U3 | Value-only behavioral options (no predicate) in E? | NOT CLOSED: needs scope ruling | Under (a) layer A cannot witness them; under (b) they are E with an analyzer-scope outcome. |
| U4 | Predicate-only and indirect changes (EC12, EC13) in E? | NOT CLOSED: needs scope ruling | Depends on (a) vs (b). The three disputed PRs above show the split. |

Not decided in this report (blocked behind U3/U4): the E tag set and the W definition, the funnel order with W as a side branch, metric ownership, and the final rubric scope. These are recorded as proposals in `reports/go-h-evaluation-protocol.md` section 13, not as rulings.

## H1.8 Historical wording (forward note only)

`reports/go-h-evaluation-protocol.md` line 115 (section 10 item 2) describes the 21 new-module targets as a KNOWN PRODUCT LIMITATION with P1 outcomes `NEW_MODULE_NO_BASE` / `NOT_EVALUABLE_BY_CURRENT_PROTOCOL`. Those are P1 frozen analyzer outcomes from before GO-E. Post-GO-E (`db4d44c`, ancestor of `9ff7c04`) they are not a current product limitation. The forward note in protocol section 13 labels this as historical. The P1 results are not rewritten.

## H1.9–H1.12 Not run

- H1.9 (funnel finalization) and H1.10 (metric ownership) depend on U3/U4 and are not finalized.
- H1.12 mkPackageOption: declaration semantics only, consistent with EC17 in the rubric. The lib/options.nix source at 82c6719 confirms the expansion is a plain `mkOption` with a package type and default. Discovery belongs to the analyzer owner. The defect itself is not fixed here (GO-G1 scope).

## H1.13 Rubric v2

`work/go-h/eligibility-rubric.json` is v2 with per-class scope flags. `work/go-h/eligibility-rubric.v1.json` preserves v1. Classes whose eligibility depends on the unresolved scope are marked `DEPENDS_ON_RULING` and are not decided.

## H1.14 Bounded fresh reviewer

Not run. Its only purpose would be to apply v2 to the three disputed PRs and controls. Under v2 those verdicts depend on the scope ruling, so a reviewer run now would produce a consistency check against an unsettled rule. Run it after the ruling.

## What the user needs to decide

1. Which product is oba: (a) option branch activation as declared in README layer A, or (b) a runtime/public contract?
2. Under (a): 569867 is INELIGIBLE. 566007 and 566696 (warning-only part) depend on U4, which then needs a ruling on predicates and warnings that read a foreign or in-module operand. The v2 rubric records this as `DEPENDS_ON_RULING`.
3. Under (b): the README non-goals and layer C must be revised first. That is a change to the declared product and is not made here.

## Verdict

PROTOCOL-REDESIGN-PARTIAL. U1 and U2 are closed. U3, U4, the funnel, the metrics and the bounded review are blocked on the scope ruling. GO-I, GO-G1 and GO-J are not started.

## GO-H1A: applied rulings

Scope (owner ruling): README Layer A is retained. oba is an option-dependent source-level branch-activation analyzer. It is not a runtime or public-contract analyzer, and runtime observability is a separate layer.

### Rulings

| ID | Ruling |
|---|---|
| U1 | Documentation-only change: OUT. |
| U2 | Configuration-value assignments: separate mode, OUT for current Layer A evaluation. |
| U3 | Option semantic change (declaration, default, type, apply, readOnly, path, helper-generated declaration): eligible only when a traceable relationship exists from the changed option to an option-dependent branch predicate. Traceability is searched as specified in `work/go-h/eligibility-rubric.json` (`traceability_search`). |
| U4 | Predicate change: eligible when the changed boolean predicate directly or indirectly consumes option state. Shell or external-command tests (including inside derivation strings) are not option-branch predicates. A change that only alters data read by an unchanged predicate is OUT. |

Eligibility categories: `E_DIRECT`, `E_INDIRECT`, `E_AMBIGUOUS`, `E_NO`. E-positive means `E_DIRECT` or `E_INDIRECT`. The category is the strongest hunk category.

### Re-adjudication of the three H13 disagreements

Each row cites the source trace recorded in H1.2. The decisive hunk is the one that changes the category.

| PR | Decisive hunk | Trace | Category | Dependency recorded |
|---|---|---|---|---|
| 569867 | `system.checks` derivation: `file --mime` ELF branch removed (shell `if` inside the build string) | The removed test is a shell predicate on `file(1)` output. Option state (`security.wrappers`) enters only as the iteration domain, which the PR does not change. The Nix expressions changed are the derivation name and `nativeBuildInputs`, neither of which is option semantics. Contrary evidence was checked and not found. | `E_NO` | Not an option-branch predicate (`predicate_rule.not_nix_predicate`). |
| 566007 | `PrivateDevices` and `PrivateUsers` gain `&& !config.boot.zfs.enabled`; `path` gains `lib.optionals config.boot.zfs.enabled [config.boot.zfs.package]` | The changed predicates still read `cfg.smartmon.enable`, an option declared in the changed module (`T_IN`, unchanged context). The added operand `config.boot.zfs.enabled` is declared in another module (`nixos/modules/tasks/filesystems/zfs.nix`). The changed term therefore consumes option state declared outside the module. | `E_INDIRECT` | `cfg.smartmon.enable` (in-module, existing, unchanged) and `config.boot.zfs.enabled` (foreign, added) feed the same changed predicates. |
| 566696 | `cosmic.nix`: `corePkgs` gains `cosmic-osk`; `cosmic-viewer` added to `systemPackages`; `qt.*` and `services.switcherooControl.enable` set with `mkDefault` | The `warnings` condition (`cfg.showExcludedPkgsWarning && excludedCorePkgs != [ ]`) is unchanged in the diff (context line only). The changed expressions are the `corePkgs` list literal (data read by the unchanged condition, `EC21`), a `systemPackages` list entry, and config-value assignments (`EC11`, U2). No option declaration, default, type or apply changed. | `E_NO` | None. The warning content changes (lists more packages) but the predicate does not, so this is OUT under the owner's rule. |

These three adjudications were fixed before the bounded review (below). They are not revised to match the reviewer.

### W: witnessability (separate axis)

W is an annotation. It is never a condition for R, E, D or V. Labels, from the H5 evidence:

- `W_YES`: a test at the relevant side drives the traceable predicate to the opposite outcome from the option's default.
- `W_NO_TEST`: no test file exists or the test is absent on that side (H5: check exit 3).
- `W_NOT_WITNESSED`: a test exists and references the option, but never drives the opposite outcome (H5: OBA001, check exit 2).
- `W_NOT_REQUIRED`: E_NO or no predicate to witness.
- `W_AMBIGUOUS`: the witness could not be determined.

### Funnel

`R → E → D → V → A`, with W as an annotation on the final stage.

- R: census file-path relevance (`nixos/modules/**` or `nixos/tests/**`). File-path rule only.
- E: the rubric categories above, from source semantics only.
- D: a mechanically derivable target from frozen steps 1–7.
- V: target validity against source, independent of oba (`validate_targets.py`, `target-validity-rubric.json`).
- A: analyzer outcome from oba. Measured only after V.

Preserved rules:

- `D != V`. D is "derived", V is "valid against source". A derived target can be invalid.
- Known prefix and watch mistakes are recorded as `D=yes, V=no`. The GO-F pre-correction defect (option_prefix ending in an interior `mkOption` container) is an example. V2 catches it.
- Historical P1 new-module outcomes (`NEW_MODULE_NO_BASE`, `NOT_EVALUABLE_BY_CURRENT_PROTOCOL`) are historical only. They describe the pre-GO-E analyzer. They are not current post-GO-E limitations, since `db4d44c` (ancestor of the candidate `9ff7c04`) supports new-module analysis.

### mkPackageOption

For protocol purposes `mkPackageOption` is an option declaration. Its arguments define default and type (EC17). Traceability applies as for any declaration. Its discovery by the analyzer is not investigated here and not fixed (GO-G1 scope).

### Metric ownership

| Metric | Numerator / denominator | Owner |
|---|---|---|
| M1 E/R | E-positive PRs / R PRs | ELIGIBILITY RUBRIC |
| M2 D/E | derivable E-positive PRs / E-positive PRs | TARGET CONSTRUCTOR |
| M3 V/D | valid targets / derived targets | TARGET VALIDATOR |
| M4 A/V | substantive analyzer outcomes / valid targets | OBA ANALYSIS |
| M5 correct/adjudicated | correct substantive outcomes / adjudicated substantive outcomes | adjudication (named reviewer) |
| M6 witnessed/witnessable | witnessed targets / witnessable targets (threshold-free) | WITNESS INFRASTRUCTURE |

No threshold is set in this report.

### Bounded fresh review (H1.14)

Run 1 applied `eligibility-rubric.v3-prereview.json` to the three disputed PRs and eight controls. Run 2 applied `work/go-h/eligibility-rubric.json` (v3.1) to five affected cases. Both are fresh agents. Neither was given GO-G classifications, the original H13 answers, expected outcomes, or the list of which PRs are controls. Run 1 is kept as a compact per-PR record in `work/go-h/independent-review/h1a-review-results.txt`; its verbatim output is not retained in this repository. Run 2 is verbatim in `work/go-h/independent-review/h1a-review-rerun.txt`. Results are in "Review result" below.

### Review result

**Run 1 (rubric v3, eight controls and three disputed PRs).** All three disputed categories agree with the fixed adjudications: 566007 E_INDIRECT, 569867 E_NO, 566696 E_NO. Controls: 443747 E_DIRECT, 508090 E_DIRECT, 563823 E_DIRECT, 568429 E_AMBIGUOUS (search cap), 565943 E_NO, 564688 E_NO, 567915 E_NO, 556752 E_NO. The reviewer raised six rubric gaps. One was a real internal contradiction: `traceability_search.result_map` assigned E_DIRECT to a same-file consumer of a foreign operand, which contradicted `predicate_rule`. All six were addressed once in v3.1, recorded in its `amendments`.

**Run 2 (rubric v3.1, five affected cases).** Blind, as above.

| PR | Fixed adjudication | Run 2 PR category | Decisive hunk (run 2) |
|---|---|---|---|
| 566007 | E_INDIRECT | E_INDIRECT | `PrivateDevices`/`PrivateUsers`/`path`: foreign operand `config.boot.zfs.enabled` |
| 508090 | E_DIRECT (control) | E_DIRECT | `services.beszel.agent.environment.SKIP_GPU`, read by `lib.optionals` in the same module |
| 563823 | E_DIRECT (control) | E_DIRECT | `jaasLoginModuleClass`: reads `cfg.package.version`, `cfg.package` declared in the same module |
| 564688 | E_NO (control) | E_NO | No E-positive hunk; every nixos/modules hunk is an EC10 equivalent rewrite |
| 443747 | E_DIRECT (control) | E_DIRECT | `services.gophernicus.enable`, read by `lib.mkIf` in the same module |

All five agree on the PR category. The decisive hunks match run 1 where run 1 named one. The rerun did not change any fixed adjudication, and none was revised to match it. A category disagreement did not occur, so there was no reason to refine again or rerun. No further iteration was run.

**Remaining rubric gaps raised in run 2.** These are not category disagreements, and they do not change any of the five categories.

- Wording, to be stated explicitly in a later version: hunk-level aggregation when a hunk mixes categories (the strongest term was used); precedence between EC10 and `lambda_over_option_attrset` (EC10 was applied); non-boolean let-bound value functions reading option state, such as `collectorAttrs` and list concatenations, are outside `scope.in` (E_NO was applied); a data-derived type on a new option; whether to search by full option path or last segment; whether a declaring module may be established by absence of a declaration; files outside `nixos/modules` and `nixos/tests` (`ci/`, `lib/`, `pkgs/`); a default change whose only consumer generates shell or data output (E_NO, per U3).
- **S1, the primary semantic ambiguity.** `gh search code` searches the default branch and cannot be pinned to a base or head SHA. A T_NONE result (which gives E_NO) and a T_FOREIGN result (which gives E_INDIRECT) both depend on that search being complete for the side under test. The protocol has no rule for when an unpinned index is adequate, and no available tool supplies a pinned search. In the five cases, no PR category depends on S1: 443747's decisive hunk is in-module, and 563823's `aclPolicies` E_NO is also given by scope. The ambiguity still applies to any general absence claim.
- **S2.** The branch-construct list does not include lambda predicates in `lib.filter` or `filterAttrs`. It is not settled whether such a boolean is an option-branch predicate for E. The 508090 `filterAttrs` hunk is one instance. It does not change that PR's category.
- **S3.** EC10 equivalence is stated by reading lib definitions at a pinned SHA. The rubric does not say how far builtins semantics must be verified. In 564688 (`param-lib.nix`), the duplicate-key rule of `listToAttrs` was not checked. If the equivalence fails, an E_NO would be wrong.

### Verdict (GO-H1A)

**PROTOCOL-REDESIGN-PARTIAL.** The exact remaining semantic ambiguity is S1: whether an absence claim (T_NONE → E_NO, or the absence of a foreign consumer) obtained from an unpinned default-branch code search is verified evidence. The protocol needs a complete search at the base or head SHA for that claim, and no current tool provides one. S2 and S3 must also be decided before the protocol is declared READY. The rubric is not changed by this section. GO-I, GO-G1 and GO-J are not started.
