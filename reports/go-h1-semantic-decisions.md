# GO-H1 — Semantic decisions

Status: STOPPED at H1.1. The declared product goal conflicts with the H1.1 premise. No U1–U4 ruling that depends on scope was made. Verdict: PROTOCOL-REDESIGN-PARTIAL.

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
