#!/usr/bin/env python3
"""S5 design round: candidate result-blind enrichment selectors.

Every function here is a PURE function of a single feature dict
(produced by `extract-features.py`, itself computed only from a real
PR's diff -- never from `oba`'s own output). No filesystem access, no
network access, no `oba` invocation, no ledger read -- enforced by
`test_no_leakage.py::test_enrichment_rules_module_never_touches_oba_or_result_files`.

Each rule returns (selected: bool, score: int, reasons: list[str]) --
`reasons` names the EXACT source-diff facts that caused selection, so
a future reviewer can prove selection happened on diff features, not
`oba` results (see section 12/13 of the design authorization).
"""
from typing import TypedDict


class Selection(TypedDict):
    selected: bool
    score: int
    reasons: list[str]


def r1_module_and_test_cochange(f: dict) -> Selection:
    """R1: module + test co-change."""
    ok = f["touches_nixos_modules_services"] and f["touches_nixos_tests"]
    reasons = []
    if f["touches_nixos_modules_services"]:
        reasons.append("touches_nixos_modules_services")
    if f["touches_nixos_tests"]:
        reasons.append("touches_nixos_tests")
    return {"selected": ok, "score": int(ok), "reasons": reasons}


def r2_option_declaration_and_test_change(f: dict) -> Selection:
    """R2: option declaration + test change."""
    ok = f["option_declaration_changed"] and f["test_assignment_changed"]
    reasons = []
    if f["option_declaration_changed"]:
        reasons.append("option_declaration_changed")
    if f["test_assignment_changed"]:
        reasons.append("test_assignment_changed")
    return {"selected": ok, "score": int(ok), "reasons": reasons}


def r3_predicate_change_and_test_change(f: dict) -> Selection:
    """R3: predicate-bearing module change + test change."""
    ok = f["predicate_logic_changed"] and f["touches_nixos_tests"]
    reasons = []
    if f["predicate_logic_changed"]:
        reasons.append("predicate_logic_changed")
    if f["touches_nixos_tests"]:
        reasons.append("touches_nixos_tests")
    return {"selected": ok, "score": int(ok), "reasons": reasons}


def r4_mkoption_line_edit_v1(f: dict) -> Selection:
    """R4_mkoption_line_edit_v1: select iff at least one ADDED or
    REMOVED diff line in a `nixos/modules/services/**` file contains
    the literal token `mkOption` or `mkEnableOption`
    (`mkoption_edit_count`/`mkenableoption_edit_count`, both computed
    by `extract-features.py` via a plain regex match over real unified
    -diff `+`/`-` lines -- see `RE_MKOPTION`/`RE_MKENABLEOPTION` and
    `diff_lines()`/`count_matches()` there).

    This is a LEXICAL line-edit detector, not an option-lifecycle
    detector: it makes no add/remove/rename claim, performs no AST or
    structural diffing, and cannot tell an added declaration from a
    removed one, a genuine semantic edit from a pure reformat, or one
    edited option from another in the same hunk. A PURELY COSMETIC
    edit to a line that happens to contain `mkOption`/`mkEnableOption`
    (e.g. reindentation, a comment moved past the token, a
    reformat-only diff) selects under this rule -- this is stated
    explicitly as ACCEPTABLE for an enrichment selector (it only
    changes which PRs get reviewed, never a verdict), not a defect to
    be fixed by this rule.
    """
    ok = f["mkoption_edit_count"] > 0 or f["mkenableoption_edit_count"] > 0
    reasons = []
    if f["mkoption_edit_count"] > 0:
        reasons.append(f"mkoption_edit_count={f['mkoption_edit_count']}")
    if f["mkenableoption_edit_count"] > 0:
        reasons.append(f"mkenableoption_edit_count={f['mkenableoption_edit_count']}")
    return {"selected": ok, "score": int(ok), "reasons": reasons}


def r5_module_lifecycle(f: dict) -> Selection:
    """R5: module lifecycle -- module file birth/death, plus any
    associated test/module wiring change.
    """
    ok = f["module_birth_or_death"]
    reasons = []
    if f["module_file_added"]:
        reasons.append("module_file_added")
    if f["module_file_removed"]:
        reasons.append("module_file_removed")
    if f["all_tests_wiring_changed"]:
        reasons.append("all_tests_wiring_changed")
    return {"selected": ok, "score": int(ok), "reasons": reasons}


def r6_branch_relevant_configuration_change(f: dict) -> Selection:
    """R6: branch-relevant configuration change -- any of option
    declaration, predicate, test assignment, runtime-side (generated
    config/environment/command construction/firewall/socket/user/group).
    """
    checks = {
        "option_declaration_changed": f["option_declaration_changed"],
        "predicate_logic_changed": f["predicate_logic_changed"],
        "test_assignment_changed": f["test_assignment_changed"],
        "runtime_side_changed": f["runtime_side_changed"],
    }
    ok = any(checks.values())
    reasons = [k for k, v in checks.items() if v]
    return {"selected": ok, "score": int(ok), "reasons": reasons}


def r7_compound_transparent_score(f: dict, threshold: int = 3) -> Selection:
    """R7: a small deterministic point system. Every point corresponds
    to one interpretable source-diff fact; weights are small integers;
    no model training, no oba output anywhere in the computation.
    """
    points = []
    score = 0
    if f["option_declaration_changed"]:
        score += 2
        points.append("+2 option_declaration_changed")
    if f["test_assignment_changed"]:
        score += 2
        points.append("+2 test_assignment_changed")
    if f["predicate_logic_changed"]:
        score += 2
        points.append("+2 predicate_logic_changed")
    if f["same_subject_module_and_test"]:
        score += 1
        points.append("+1 same_subject_module_and_test")
    if f["module_birth_or_death"]:
        score += 1
        points.append("+1 module_birth_or_death")
    if f["specialisation_configuration_changed"] or f["all_tests_wiring_changed"]:
        score += 1
        points.append("+1 specialisation_or_wiring_changed")
    ok = score >= threshold
    return {"selected": ok, "score": score, "reasons": points}


RULES = {
    "R1_module_and_test_cochange": r1_module_and_test_cochange,
    "R2_option_declaration_and_test_change": r2_option_declaration_and_test_change,
    "R3_predicate_change_and_test_change": r3_predicate_change_and_test_change,
    "R4_mkoption_line_edit_v1": r4_mkoption_line_edit_v1,
    "R5_module_lifecycle": r5_module_lifecycle,
    "R6_branch_relevant_configuration_change": r6_branch_relevant_configuration_change,
    "R7_compound_transparent_score": r7_compound_transparent_score,
}
