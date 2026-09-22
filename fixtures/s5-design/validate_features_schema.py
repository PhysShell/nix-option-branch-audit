#!/usr/bin/env python3
"""S5 design round: schema/leakage validator for a features.jsonl row.

A hand-audited ALLOWLIST of legitimate feature keys -- anything outside
it is rejected, fail-closed. This is the mechanical backstop the
authorization's section 13 asks for: even if a future accidental merge
or a careless edit added an oba-outcome-shaped field to a features row
(e.g. `actionable_count`, `verdict`, `false_pass`), this validator
refuses it rather than silently accepting a poisoned feature file that
could then leak into a real S5-B selection decision.
"""

ALLOWED_KEYS = {
    "pr", "base_sha", "head_sha", "cohort",
    "touches_nixos_modules", "touches_nixos_modules_services", "touches_nixos_tests",
    "num_module_files", "num_test_files", "same_subject_module_and_test",
    "module_file_added", "module_file_removed", "test_file_added", "test_file_removed",
    "module_renamed_or_moved", "module_birth_or_death",
    "option_declaration_changed", "mkoption_edit_count", "mkenableoption_edit_count",
    "default_bool_changed", "type_decl_changed", "submodule_attrsof_changed",
    "predicate_logic_changed", "cfg_ref_edit_count", "config_services_ref_edit_count",
    "literal_comparison_edit_count",
    "test_assignment_changed", "specialisation_configuration_changed", "all_tests_wiring_changed",
    "runtime_side_changed", "execstart_changed", "environment_changed",
    "systemd_service_attrs_changed", "firewall_changed", "users_groups_changed",
    "files_changed_total", "module_diff_lines", "test_diff_lines",
    "package_version_bump_lines", "pure_package_bump_no_option_content",
}

# Any of these appearing in a "features" row is an outcome/label leak,
# never a legitimate pre-analysis feature.
FORBIDDEN_OUTCOME_SHAPED_SUBSTRINGS = [
    "actionable", "verdict", "false_pass", "false_finding", "pass_verdict",
    "tool_error", "applicable", "notable", "inconclusive", "oba_binary",
    "raw_json", "summary_sha", "causal_framing", "pr_relevant", "genuine_evidence",
]


def validate_row(row: dict) -> list[str]:
    errors = []
    for key in row:
        if key not in ALLOWED_KEYS:
            errors.append(f"unexpected key {key!r} not in the audited feature allowlist")
        for forbidden in FORBIDDEN_OUTCOME_SHAPED_SUBSTRINGS:
            if forbidden in key.lower():
                errors.append(f"key {key!r} looks outcome-shaped (contains {forbidden!r}) -- refused")
    return errors


def main():
    import json
    import sys
    from pathlib import Path

    path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent / "features.jsonl"
    total = 0
    bad = 0
    for line in path.read_text().splitlines():
        if not line.strip():
            continue
        total += 1
        row = json.loads(line)
        errors = validate_row(row)
        if errors:
            bad += 1
            print(f"PR #{row.get('pr')}: {errors}")
    print(f"\n{total - bad}/{total} rows valid")
    if bad:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
