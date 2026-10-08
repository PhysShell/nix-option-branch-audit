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

## GO-H1B: closing the three protocol gaps

### H1B.0 Provenance

- Branch `claude/go-h-protocol-redesign`. Start HEAD `6d806c1` (clean). Parent: GO-H, GO-H1, GO-H1A. CI on `6d806c1`: success, run 37744079684.
- Protocol and rubric: `eligibility-rubric.json` v4 (`go-h-eligibility-rubric/4.0`); v3.1 kept as `eligibility-rubric.v3.1.json`. `target-validity-rubric.json` gained `source_loading`; prior version kept as `target-validity-rubric.v1.json`.
- Existing solutions checked (CLAUDE.md order), recorded per the reuse rule:
  - Git at a pinned commit (`git grep` on a blobless clone): closest and used. It supplies pinned, complete, reproducible search and gives line-level evidence. Limitation: it is text search, so it has no Nix parse; it cannot resolve `with`, `inherit`, or let bindings by itself.
  - `gh search code`: rejected for absence. Default branch only, no SHA pin, file-level results only (`rundeck.nix` for `aclPolicies`, no line numbers).
  - ripgrep: same text-search role as git grep but has no SHA pin; rejected for absence evidence.
  - Nix evaluation and CodeQL: not run. Too heavy for a per-target check and neither gives a pinned, reproducible absence result without a separate build of each SHA.
  - Gap that remains: no parser-level receiver resolution. A5 is therefore a textual closure over known receiver forms, not a semantic one.

### H1B.1–H1B.3 Exact-tree absence evidence

`work/go-h/exact_tree_search.sh <repo> <sha> <scope> <regex> [exclude] [receiver_regex]` resolves the SHA with `git rev-parse --verify --quiet <sha>^{commit}`, requires an exact match and a non-empty scope, and runs `git grep -a -n -E`. Exit 0 is MATCH; exit 1 is ZERO_COMPLETE; anything else is ERROR. Only ZERO_COMPLETE can support absence. Controls are in `work/go-h/h1b3_controls.sh`, output in `work/go-h/independent-review/h1b-exact-tree-controls.txt`:

- P1 positive: `config\.boot\.zfs\.enabled` in beszel-agent.nix @ e2497c3a: MATCH, 3 matches (lines 139, 186, 188). The earlier count of 2 was wrong.
- P2 positive: `enabled = lib\.mkOption` in zfs.nix @ e812ab65: MATCH, 1 match (line 304).
- N1 owner-key `\bgophernicus\b` outside gophernicus.nix @ e4c7d977: 13 matches, all non-consumer (1 release note, module-list.nix:914, all-tests.nix:791, 10 in tests/gophernicus.nix).
- N2 leaf `\.rootDir\b`: 8 matches, all in other receivers (cloudflare-warp, darkhttpd, nspawn-container; suwayomi-server.nix:224 is a string).
- N3 container path `services\.gophernicus\.`: 2 matches (release note, one test read).
- PC1/PC2 `aclPolicies` @ b063b8f9 and @ 3d35b67b: MATCH, 2 each, both in rundeck.nix. The gh result (rundeck.nix, no SHA) agrees on the file set; the exact tree gives pinned line-level evidence.

Controller check of the whole-namespace A5 receivers at e4c7d977:

- `ncdns.nix:8` `cfgs = config.services`: closed use (only literal `ncdns`, `pdns-recursor`, `namecoind`).
- `with config.services;` (pki.nix:48, mautrix-signal:177, mautrix-whatsapp:179, certmgr test:94) and `inherit (config.services) …` (mailman:405, public-inbox test:30): reach K only by naming it; A3 covers them.
- Iterations over `config.services.<name>` (restic, redis, uhub, vmalert, bitcoind, fedimintd, gitwatch, and others): named sub-attrsets, not the namespace.
- `cgit.nix:8` `cfgs = config.services.cgit`: a named sub-attrset; the cgit iterations do not read the namespace.
- No bare iteration over `config.services` itself was found. The earlier suspected gap is not present at e4c7d977.
- Computed keys `config.services.${x}`: rancher (`"k3s"`, `"rke2"`), watt (literal list), vault-agent (three literals), web-app enums; all closed.

### H1B.4–H1B.7 Predicate roles and lambdas

Rule (v4): roles P_GATING, P_CONTROL, P_SELECTION, P_DATA_ONLY, P_UNKNOWN; eligible roles are GATING, CONTROL, SELECTION. A predicate lambda passed to a P_SELECTION function is a predicate; a transform lambda (map, mapAttrs, concatMap) is not. Let bindings are transparent: the category follows the declaration module of the traced paths. This resolves the v3.1 conflict between `indirect_via_bindings` and `lambda_over_option_attrset`.

The fresh reviewer (below) applied this on C1–C6. Each `filter` predicate was classed P_SELECTION and eligible: C1, C2, C4 E_DIRECT; C3, C5 E_INDIRECT; C6 P_GATING, E_INDIRECT. The controller did not separately re-derive these categories; they are the reviewer's reported results.

Open: builtins are unbound in-file, so `builtins.isString` and `builtins.isInt` are P_UNKNOWN. A predicate that traces both in-module and foreign paths has no combination rule. A boolean written as an attribute value (C6 lines 186, 188) sits on the P_GATING / P_DATA_ONLY boundary.

### H1B.8–H1B.11 EC10 equivalence levels

Rule (v4): EQ0 textual; EQ1 AST; EQ2 bounded (alpha-renaming of lambda and let bindings, attribute-path sugar, same-file let alias that binds exactly the option path); EQ3 everything else. EC10_AUTOMATIC is true for EQ0–EQ2 only. Forbidden automatic normalizations: reordering `&&` / `||` operands, `!(!x)` → `x`, list reordering, `mkDefault` / `mkForce` wrapper changes, literal substitution, bindings outside the expression.

Reviewer on the ten synthetic pairs (C9a–j): C9c and C9h EQ2 equivalent; the other eight are EQ3 with equivalent = no, unknown (C9e, C9i, C9j). All EC10_AUTOMATIC = false except C9c and C9h. Two wording points are open: whether a head-introduced let around an option path is covered by EQ2 (c), and EQ3 library-binding equivalences, which need a SHA.

### H1B.12 Diagnostic dry run (non-scoring)

Re-read this turn: 508090 (`filterAttrs`, reviewed above as C2) and 566007 (sandbox boolean). Not re-read this turn; these rest on GO-H1A notes: 566696, 569867, 564688. The dry run is diagnostic only and does not change any GO-H1A category.

### H1B.13 Bounded fresh reviewer

One reviewer run on the v4 prompt (`work/go-h/independent-review/h1b-reviewer-prompt.txt`). Its report, transcribed, is in `work/go-h/independent-review/h1b-review-record.txt`. It returned category agreement on every decidable case. Its material objections are: the A5 wording (self-contradictory), P_GATING / P_DATA_ONLY, let-binding defaults (`GPU_COLLECTOR` reads `services.xserver.videoDrivers`), builtins resolution, the aggregation rule, and EQ3 library bindings.

One refinement is applied: the A5 wording is corrected (`with` and `inherit` are covered by A3, not open receivers; whole-namespace iteration is named explicitly). This is a wording correction only; no rule was added. It was not re-reviewed. Re-running the reviewer on it would not close the definitional items below, so the verdict is not changed by it. C7 and C8 are therefore not decided by the reviewer; the controller check above gives T_NONE for `services.gophernicus.rootDir` under the corrected A5, but that is not reviewer-confirmed.

### H1B.14 Pipeline

R (file relevance) → E (eligibility) → D (derivable target) → V (target validity, exact-SHA source) → A (analyzer outcome). W (witnessability) remains an annotation and is not in the pipeline's pass/fail path.

### H1B.15 GO-I handoff

Mechanical responsibilities for GO-I, if the protocol is later declared READY:

- `work/go-h/validate_targets.py:46–49` reads the working tree (`Path(root) / target["module"]` with `read_text()`). This violates `target-validity-rubric.json` `source_loading`. It must read `git show <sha>:<module>` at the side SHA, and an unresolved SHA is ERROR. The validator is not modified in GO-H1B.
- Exact-tree search for V4 and S1 uses `exact_tree_search.sh` with the side SHA.

### Remaining items (reviewer-owned or definitional)

1. Builtins resolution: whether builtins are a fixed table of P_SELECTION / P_DATA_ONLY functions.
2. Combination rule when a predicate traces both in-module and foreign paths.
3. Let-bound defaults: whether an unforced value under `?` counts as a read.
4. Option defaults: traced paths stop at the named option; defaults are not followed.
5. P_GATING vs P_DATA_ONLY for a boolean written as an attribute value.
6. Selection inside a message interpolation (C4).
7. Aggregation rule across per-site results of one hunk.
8. EC10 (c): whether a head-introduced let around an option path is covered.
9. EQ3 library-binding equivalences: need a SHA, not decidable from the expression.

### Verdict (GO-H1B)

**PROTOCOL-REDESIGN-PARTIAL.** The three named gaps are closed for absence evidence (exact-SHA search with positive and negative controls) and for EC10 levels (EQ0–EQ3 with forbidden normalizations). They are not closed for predicate roles: items 1–7 above are definitional, not wording, and the reviewer left the boundary between P_GATING and P_DATA_ONLY unresolved. READY requires those to be decided, so the verdict is PARTIAL.

GO-I, GO-G1 and GO-J are not started. oba is not modified.

## GO-H1C

Scope: close the definitional items left open by GO-H1B and decide READY or PARTIAL. Rubric v5 is `work/go-h/eligibility-rubric.json` (schema `go-h-eligibility-rubric/5.0`). v4 is kept as `work/go-h/eligibility-rubric.v4.json`. Probes are DIAGNOSTIC / NON-SCORING. The P1 and GO-F verdicts are not altered. oba is not modified. The validator is not implemented. GO-G1, GO-J and GO-I are not started.

### Rulings (rubric v5, `h1c_*` blocks)

- **Callee resolution (`h1c_callee_resolution`).** Callees are FQ_BUILTIN (`builtins.<name>`), FQ_LIB (`lib.<path>`, resolved at the exact SHA), LOCAL_ALIAS (unique binding to a resolved callee), BARE, or DYNAMIC. Anything not resolved mechanically is CALLEE_UNKNOWN, which makes the predicate P_UNKNOWN. Matching identifier spelling is never resolution (R3).
- **Mixed predicates (`h1c_mixed_predicate`).** A predicate with at least one proven watched dependency is eligible for the watched option (M1). Foreign dependencies are recorded separately and do not disqualify it. A predicate with only foreign dependencies makes the target invalid (M2). The record makes no sole-cause claim (M3). Category follows declaration_module_governs (M4).
- **Causal relevance (`h1c_causal_relevance`).** A textual mention is not causal. The watched option must reach the predicate by an accepted def-use edge. An unbounded path is DEPENDENCY_UNBOUNDED and is never eligible.
- **Let aliases (`h1c_let_alias_transparency`).** A let binding is transparent only if it is a pure alias satisfying T1–T6 (unique name, no shadowing, static option path, direct use, no intervening call or transform, proven chain). Any other let binding is not transparent. An unproven link is ALIAS_UNPROVEN, and the equivalence is EQ3.
- **Let-bound defaults (`h1c_let_bound_defaults`).** A let change is recorded at the option property it reaches (D1). It is eligible only if that option has a branch relation (D3). A shared binding records every reached property, and unrelated consumers are recorded as not relevant (D2).
- **Gating versus data (`h1c_gating_vs_data`).** G0 lists the gating positions: an if condition; the first argument of a resolved conditional-inclusion callee, with the contract read from its definition; an assertion or warning condition; a P_SELECTION predicate; and an operand of && or || that is itself in a gating position. G1 makes an attribute value, list element, interpolated text, serialized value, or external text P_DATA_ONLY. G2 makes an unidentified consumer P_UNKNOWN. G3: && and || are gating only when the whole result is in a G0 position. G4: no speculative downstream gating.
- **Higher-order calls (`h1c_higher_order`).** Selection callbacks (filter, filterAttrs, partition, findFirst) are predicates (H1). Transform callbacks (map, mapAttrs, concatMap, and similar) are not predicates (H2). A custom higher-order function counts only when its body is resolved (H3). The table is a set of resolved callees, not a name whitelist (H4).
- **EC10 (`ec10_equivalence`).** EQ2 gains (d) transparent alias introduction or removal under T1–T6, and (e) library alias substitution under L1. Forbidden automatic normalizations are unchanged, with one exception: the L1 library alias chain.
- **Library binding (`h1c_library_binding`).** L1: a library name defined by an inherit or attribute-alias chain of at most 4 steps, all in lib/ at one SHA, ending at a builtins member, is EQ2(e). L2: a non-alias library definition is EQ3. L3: no SHA gives no equivalence claim. L4: a name is never evidence.
- **Exact-source identity (`h1c_exact_source_identity`).** Every source fact records its SHA. Working-tree and default-branch evidence is diagnostic only.
- **Ambiguity (`h1c_ambiguity_policy`).** Ambiguity is a first-class outcome: CALLEE_UNKNOWN, P_UNKNOWN, E_AMBIGUOUS, DEPENDENCY_UNBOUNDED, ALIAS_UNPROVEN, EQ3, and T_UNKNOWN.
- **GO-I boundary (`h1c_go_i_boundary`).** GO-I must do the mechanical work: source loading from an exact SHA, AST parsing, lexical resolution, transparent let aliases, R1 resolution, selection-versus-transform split, watched-edge tracing, G0/G1 classification where decidable, V1–V4, side-aware checks, EQ0–EQ2 (a)–(e), and A1–A6 search. GO-I must fall into reviewer-owned states for CALLEE_UNKNOWN, dynamic paths, P_UNKNOWN, E_AMBIGUOUS, EQ3, DEPENDENCY_UNBOUNDED and ALIAS_UNPROVEN. It must not manufacture certainty.

### Correction to absence evidence (A1)

v4 A1 required status ZERO_COMPLETE. That status is only returned when the raw search has zero matches, but A3 decides absence after A4 classification. Under v4 wording, T_NONE was impossible whenever a non-consumer match existed. Gophernicus has 13 such matches, and rundeck has 42. The GO-H1B note that gophernicus gave T_NONE "under the corrected A5" was therefore not valid under the frozen A1. v5 A1 requires a pinned, completed search (MATCH or ZERO_COMPLETE, git grep exit 0 or 1, no ERROR). Absence is then decided by A3 after A4 classification. This was applied before the reviewer was spawned.

### Probes (DIAGNOSTIC / NON-SCORING)

Probe script: `work/go-h/h1c_probes.sh`. Output: `work/go-h/independent-review/h1c-probe-excerpts.txt`. Each excerpt is printed from an exact SHA: E = e4c7d977, B = e2497c3a, F = fdc33963, RUN = b063b8f9. P05s is synthetic, with no SHA. P11 is synthetic.

Controller checks, outside the reviewed excerpt:
- P09 owner key (`\bgophernicus\b`, E): 13 matches. All are A4 non-consumer: 1 release note, 1 module-list line, 1 all-tests line, 10 lines in nixos/tests/gophernicus.nix. No enum at E contains `gophernicus`. Enums checked: freshrss, invoiceplane, drupal, wordpress, zabbix frontend and proxy, dokuwiki, limesurvey, movim, kimai.
- P09 receiver query (A5, E): 23 matches. Classified as: `with` and `inherit` (covered by A3), named sub-attrset (cgit.nix:8), whole-namespace with named leaves (ncdns.nix:8), literal sets (rancher, watt, vault-agent), and enum-typed computed keys (web apps). The rancher closure is inherited from the literal import list. This classification is controller work. The reviewer could not verify it from the excerpt, so it is unreviewed.
- P09b owner key (`\brundeck\b`, RUN): 42 matches, all A4 non-consumer: 39 in nixos/tests/rundeck.nix, 1 release note, 1 module-list line, 1 all-tests line. A5 at RUN returns the same 23 matches, with the same classification. Rundeck T_NONE therefore depends on the same unreviewed A5 classification.
- P08b (`locate.nix:276`): `lib.boolToYesNo` is an element of the argument list passed to `utils.escapeSystemdExecArgs`. This is data (G1).

### Reviewer (bounded, fresh, one run)

Agent `ae2f9ae548c1870f7`, prompt `work/go-h/independent-review/h1c-reviewer-prompt.txt`, inputs: the rubric and the probe excerpt file only. No git commands. Full report, transcribed: `work/go-h/independent-review/h1c-review-record.txt`.

- Classified P01–P11 with role, category and rule clause.
- Six pairs: 5 separated (P03/P04, P01/P08a, P05/P05f, P10/P10b, P06/P07). P09/P09b are not separated by the rules. Both use the same A1–A6 procedure, so this is expected.
- Listed 15 rule gaps (in the record).
- No wording refinement was applied. The gaps are definitional, not wording, so the one permitted refinement would not close them.

One reviewer claim is wrong. The reviewer said the lib/default.nix step for `lib.filter` was missing. The step is at `lib/default.nix:278` (`filter` re-exported from `inherit (self.lists)` at line 268), and `lib/lists.nix:25` inherits `filter` from builtins. Three steps, all in lib/, at e4c7d977. The L1 chain is complete, so `lib.filter` is EQ2(e) equivalent to `builtins.filter`. The reviewer saw only the excerpt window 266–270. The excerpt file is not edited after review. `lib.boolToYesNo` (P10) is a non-alias lambda, so it stays L2, EQ3.

### Verdict (GO-H1C)

**PROTOCOL-REDESIGN-PARTIAL.** READY requires that the listed definitional conditions hold. The following constructs remain unresolved, each with its exact trigger:

1. **Control dependence from an enclosing conditional, and the category for invalid targets (P02, M2).** A changed assertion inside `lib.mkIf` on the watched option is not a def-use edge, so it is not reached by the accepted-edge rule. M2 makes the target invalid, but invalid targets are not one of the four categories, so the aggregation rule cannot place them.
2. **Name-based roles for unresolved callees (P11a).** `predicate_taxonomy` names `lib.filter` as P_SELECTION. R1 requires an exact SHA. For `let f = lib.filter`, with no SHA in the probe, the rules give CALLEE_UNKNOWN on a strict reading and P_SELECTION on the taxonomy reading. The rules conflict. Taxonomy names must apply only to SHA-resolved callees.
3. **Keys inside freeform attrset options (P03, P04b).** `declaration_module_governs` does not say which module declares a key such as `GPU_COLLECTOR` inside an attrset option.
4. **Option-free helper with an internal conditional (P08a, P10).** `lib.boolToYesNo` branches on its parameter. The rules are silent on helpers whose parameter receives option state at call sites.
5. **A5 closure for whole-namespace bindings and enum-typed computed keys (P09, P09b).** The rules do not say whether use-site narrowing closes a whole-namespace domain (`ncdns.nix:8`), or how enum-typed computed-key domains are read. The rancher closure is not re-printed. Gophernicus and rundeck T_NONE both depend on A5 classification that the reviewer could not verify.
6. **Evidence not yet printed at the SHA (P05f, P05s, P07c, P09 A1).** The G0(ii) definitions of `lib.optionals`, `lib.optionalString` and `lib.mkIf` must be read at the SHA. The rev-parse check for A1 was not in the excerpt.

Not applied: the E_NO list does not name transform lambdas (P04). This is inferable from H2, so it is a wording point that was not spent on the review's one allowed refinement.

GO-I, GO-G1 and GO-J are not started. oba is not modified.

## GO-H1D

Status: DIAGNOSTIC / NON-SCORING probes. P1 and GO-F classifications are not altered. Rubric: `work/go-h/eligibility-rubric.json`, schema go-h-eligibility-rubric/6.0 (`h1d_*` keys govern; v5 kept as `eligibility-rubric.v5.json`). Target validity: `work/go-h/target-validity-rubric.v2.json` (schema /2; v1 unchanged). Reviewer prompt: `work/go-h/independent-review/h1d-reviewer-prompt.txt`. Record: `work/go-h/independent-review/h1d-review-record.txt`. Raw probe output: `work/go-h/independent-review/h1d-probe-raw.txt` (from `work/go-h/h1d_probes.sh`).

### Rules frozen before the reviewer

1. Control dependence (C1–C4): DATA_DEPENDENT, CONTROL_DEPENDENT_ONLY, INDEPENDENT. Only DATA_DEPENDENT predicates are dependencies of W (C1). A gate that is itself DATA_DEPENDENT is a predicate on W (C2). An option-semantic change is eligible only when a DATA_DEPENDENT predicate reads W (C4).
2. Validity (v2): V_VALID, V_INVALID, V_AMBIGUOUS. Named invalid reasons: WRONG_BASE_HEAD_SIDE, PREFIX_CONTRACT_VIOLATION, WATCH_PATH_NOT_DECLARED, FREEFORM_KEY_NOT_INDEPENDENT_DECLARATION, FOREIGN_ONLY_DEPENDENCY, NO_WATCHED_PREDICATE_DEPENDENCY. V_INVALID carries no E category. A derivation precondition was added before the reviewer ran: a V state is computed only for a target derived from a changed predicate on W or an option-semantic change to W.
3. Callee resolution (K1–K6): a name is not a resolution (K1). A lib path is resolved only by exact-SHA identity (K2). K1 settles the H1C conflict between R1 and the taxonomy example `lib.filter`.
4. G0(ii) pinning: tree and blob SHAs of the pinned primitives were verified against the pinned repository. Trees: E `21fd84ae`, B `d181338d`, F `6e06015b`. Blobs at E: `lib/modules.nix` `74ff0f04`, `lib/lists.nix` `819b6d7f`, `lib/strings.nix` `ccd84572`, `lib/attrsets.nix` `a7056726`, `lib/trivial.nix` `f333440e`, `lib/default.nix` `56baba87`.
5. Freeform (F1–F6): a declared suboption is an independent declaration (F1). A freeform-only key is not (F2).
6. Namespace closure (N1–N6): literal-leaf closure (N1), whole-namespace consumption is open (N2), computed keys are closed only over an exact domain (N3), enum expansion needs four pieces of evidence (N4). A grep is a screen, not a closure (N6).
7. Interprocedural (I1–I5): a boolean's role is fixed at its own consumer position (I1). One W-derived call site is positive evidence (I2). A data-position call-site change is E_NO for that hunk (I3). Eligibility and equivalence are separate decisions (I4).
8. boolToYesNo ruling: the helper's internal `if` is P_CONTROL at its own position (I1). A change to `lib/trivial.nix:308` is E_INDIRECT for `services.locate.pruneBindMounts` (I3). Rewriting `locate.nix:276` inline is EC10 with L2 and EQ3, reviewer-owned.

### Reviewer result

One fresh general-purpose reviewer, 17 cases, seven distinctions (D1–D7), 17 rule-gap entries. The reviewer separated all seven distinctions. Full record in `h1d-review-record.txt`. Controller notes are kept separate in that file.

- D1 (control vs data): C1a is CONTROL_DEPENDENT_ONLY, V_INVALID NO_WATCHED_PREDICATE_DEPENDENCY. C1b is DATA_DEPENDENT, V_VALID.
- D2 (foreign-only vs no-watched-gate): C2a is FOREIGN_ONLY_DEPENDENCY. C2b is NO_WATCHED_PREDICATE_DEPENDENCY.
- D3 (invalid vs ambiguous): C3a is V_INVALID (freeform, mechanical). C3b is V_AMBIGUOUS (open dynamic key on the only path).
- D4 (resolved vs unresolved callee): C4d resolves at F to `lib.filter` (P_SELECTION, V_VALID). C4a–C4c are CALLEE_UNKNOWN (V_AMBIGUOUS).
- D5 (declared vs freeform-only key): C5a is a declared suboption (V_VALID). C5b is freeform-only (V_INVALID).
- D6 (closed vs open): an open receiver off the dependency path does not change V (C6c). An open computed key on the path makes V_AMBIGUOUS (C6d). The closed enum at movim (C6b) is closed by N4.
- D7 (helper vs caller position vs equivalence): C7a is E_INDIRECT (V_VALID). C7b is E_NO (not derived). C7c is E_AMBIGUOUS (EC10, reviewer-owned).

The reviewer's answer for C1a contains a self-contradiction: V_INVALID together with category E_NO. The rule text says V_INVALID carries no E category.

### Why the verdict is PARTIAL

The blocking construct is control-only predicate change inside a W-gated block, exemplified by C1a (`beszel-agent.nix:211`, inside `lib.mkIf cfg.openFirewall` at :209).

- C1 says CONTROL_DEPENDENT_ONLY never establishes a dependency on W. C4 then makes an option-semantic change eligible only through a DATA_DEPENDENT gate. Under that rule C1a is V_INVALID, and by aggregation it has no E category.
- The reviewer's answer gives E_NO. Under the rule as written that is not available, so the case has no category under the frozen rules.
- Deriving a category from the enclosing gate would need a different rule. The open question is whether a predicate reached only through a W gate counts as a dependency of W.
- U3 as ruled (GO-H1A/H1C) concerns "conditional option-semantic eligibility through a branch relation". C1 and C4 read that branch relation narrowly, as a direct DATA_DEPENDENT gate. That narrowing is H1D drafting, not a user ruling. Whether it matches what U3 intended is a scope question for the user. The directive forbids reopening U3 without exact contradicting repository evidence. C1a exposes the question, but it does not show that the repository contradicts U3.

No mechanical, reviewer-owned, ambiguity, or D-capability classification suffices for this construct without deciding that scope question. Ambiguity would mark a hunk the rubric itself says has no dependency, and leaves the V_INVALID versus E_AMBIGUOUS conflict open. A reviewer-owned call would hand a scope decision to each case. Because the question is about intent, this round does not decide it.

Secondary, non-blocking items for the next round:

- I1 versus I3 for a call-site rewrite in a data position (C7c). The EC10 route gives E_AMBIGUOUS, so the category is decided, but the routing conflict should be fixed.
- Synthetic cases with `lib.mkIf` or `lib.filter` and no SHA take P_UNKNOWN under K5. The prompt said "classify as written", which pointed the other way. This was a defect in the prompt, not in the rules, and changes no V result.
- Gaps about a parent-option default reaching a child key (C4, C3a), a submodule as an intermediate container under V3 (C5a), and the module used for V1–V3 when the changed predicate is in another module (C7a). Each is covered by an ambiguity state or by a mechanical reason that does not depend on it.

Wording refinement: none applied. The reviewer's gaps are rule silences and conflicts, not wording defects. The one permitted refinement is not used.

### Verdict (GO-H1D)

**PROTOCOL-REDESIGN-PARTIAL.** Five constructs have rules, mechanical or reviewer-owned classifications, or explicit ambiguity states, and the reviewer separated the distinctions for them. Control dependence from an enclosing gate, and whether such a dependency is eligible under U3, are not decided. The GO-I handoff is not written, because READY was not reached. GO-I, GO-G1 and GO-J are not started. oba is not modified.

Next step, for the user: decide whether a predicate reached only through a W gate (CONTROL_DEPENDENT_ONLY) is a dependency of W for eligibility under U3. The answer determines whether C1a becomes V_VALID with a category, or stays V_INVALID with no category. Until that is decided, a further round would only restate this one.

## GO-H1D closure (owner ruling, 2026-10-08)

Status: verdict PROTOCOL-REDESIGN-READY, conditional on the closure clauses being accepted. No fresh reviewer has checked the closure clauses. GO-I is not started.

Ruling applied: control dependence alone does not make an inner predicate a valid Layer-A target for W. C1a (beszel-agent.nix:211, inside the lib.mkIf at :209 gated by openFirewall) is CONTROL_DEPENDENT_ONLY. Its V state is V_INVALID with NO_WATCHED_PREDICATE_DEPENDENCY, and it has no E category. The gate at :209 is a valid predicate on W when it is itself changed (C1b, V_VALID, E_DIRECT). No composite predicate is synthesized.

Closure rules added (all in work/go-h/eligibility-rubric.json, h1d_closure and the h1d_* sections it names):
- derivation_scope: a hunk derives W when it changes W's own attribute, a predicate with a W path or a W gate, or an ancestor whose reach is undecided. Data terms do not derive W. A predicate with no W relation does not derive W.
- Blob identity per commit (h1d_gating_primitives.per_commit_blobs): mkIf is pinned at B by blob equality; lib.filter is pinned at F through lib/lists.nix at F, whose blob differs from E.
- Clarifications: "transparent" in the causal-relevance edge means tracing under let_binding_rule, which is how C4d reaches GPU_COLLECTOR. K5 governs over case parentheticals. The I3 E_NO clause applies only to all-data hunks. A predicate introduced by an EC10 rewrite is a changed predicate. Predicate hunks are validated against the side where W is declared.
- New states: UNRESOLVED (dependency), PARENT_DEFAULT_REACH (V_AMBIGUOUS), DECLARING_MODULE_UNSTATED (E_AMBIGUOUS).

Reviewer's 17 gaps, by class: 12 MECHANICAL, 1 OWNER_RULING (gap 3), 3 AMBIGUITY_STATE (gaps 8, 9, 17), 1 REVIEWER_OWNED (gap 16, the V3 submodule container for C5a). The reviewer record is not edited.

Open after closure, reported as such and not decided: C5a depends on the reviewer-owned V3 decision. C7c is E_AMBIGUOUS pending the EC10 reviewer decision on the boolToYesNo rewrite (EQ3). C3a is V_AMBIGUOUS (PARENT_DEFAULT_REACH). C3b, C4a-C4c and C6d are UNRESOLVED. C6c's category is E_AMBIGUOUS (DECLARING_MODULE_UNSTATED).

Next step for the user: decide whether the closure clauses need one fresh review round before GO-I. GO-I, GO-G1 and GO-J are not started, and oba is not modified.
