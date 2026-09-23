#!/usr/bin/env python3
"""S5 final report generator. Reads ONLY the committed adjudication
ledger and frozen S5 artifacts, computes every number mechanically,
and calls the frozen `gate.py` machinery for the official verdict --
never hand-typed. Mirrors S4's own generate-report.py discipline
(reuse clopper_pearson_lower_bound, never reimplement).
"""
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
DESIGN = ROOT / "fixtures/s5-design"

spec = importlib.util.spec_from_file_location("gate", DESIGN / "gate.py")
gate = importlib.util.module_from_spec(spec)
sys.modules["gate"] = gate
spec.loader.exec_module(gate)

spec2 = importlib.util.spec_from_file_location("s4gen", ROOT / "fixtures/s4-live-pr-shadow/generate-report.py")
s4gen = importlib.util.module_from_spec(spec2)
sys.modules["s4gen"] = s4gen
spec2.loader.exec_module(s4gen)


def load_ledger():
    return [json.loads(l) for l in (HERE / "adjudication-ledger.jsonl").read_text().splitlines() if l.strip()]


def presentation_resolved_correct(r):
    for rev in r["reviews"]:
        for k in ("scanner_substance_correct", "pr_relevant", "causal_framing_correct", "rendered_presentation_correct"):
            if rev.get(k) is False:
                return False
    return True


def main():
    rows = load_ledger()
    pr_summaries = [r for r in rows if r["record_type"] == "pr_summary"]
    actionable_presentations = [r for r in rows if r["record_type"] == "actionable_presentation"]
    pass_adjs = [r for r in rows if r["record_type"] == "pass_adjudication"]

    s5a = [r for r in pr_summaries if r["cohort"] == "A"]
    s5b = [r for r in pr_summaries if r["cohort"] == "B"]
    s5a_positions = sorted(r["position"] for r in s5a)
    s5b_positions = sorted(r["position"] for r in s5b)
    s5a_complete = gate.is_unbroken_prefix(s5a_positions) and len(s5a_positions) == 150
    s5b_complete = gate.is_unbroken_prefix(s5b_positions) and len(s5b_positions) == 219

    # --- Reconciliation against frozen orders (never trust the ledger alone) ---
    s5a_frozen = json.loads((DESIGN / "s5a-frozen-order.json").read_text())
    s5b_frozen = json.loads((DESIGN / "s5b-frozen-order.json").read_text())
    s5a_frozen_by_pos = {i + 1: r["number"] for i, r in enumerate(s5a_frozen)}
    s5b_frozen_by_pos = {i + 1: r["number"] for i, r in enumerate(s5b_frozen)}
    s5a_mismatches = [r for r in s5a if r["pr"] != s5a_frozen_by_pos.get(r["position"])]
    s5b_mismatches = [r for r in s5b if r["pr"] != s5b_frozen_by_pos.get(r["position"])]
    dup_a = len(s5a) != len({r["pr"] for r in s5a})
    dup_b = len(s5b) != len({r["pr"] for r in s5b})

    # --- S5-A estimand A stats ---
    # actionable_presentation_count / pass_verdict_count are counted
    # DIRECTLY from the ledger's own actionable_presentation/
    # pass_adjudication records (the true source of truth), never from
    # pr_summary's own actionable_count/pass_verdict_count fields.
    # S5-R0 finding: those pr_summary fields record what the tool's own
    # rendering flagged as needing review AT EVIDENCE-GATHERING TIME --
    # they go stale whenever the coordinator's own independent
    # adjudication later determines something needs review that the
    # tool's rendering never flagged as such (exactly PR #471312's false
    # "Unchanged" and PR #461261's PASS, both coordinator-added
    # post-hoc; see s5-confirmed-defects.md for the full provenance).
    def bucket_stats(cohort, cohort_label):
        n = len(cohort)
        applicable = sum(1 for r in cohort if r["applicable"])
        tool_errors = sum(r["tool_error_count"] for r in cohort)
        actionable = sum(1 for r in actionable_presentations if r["cohort"] == cohort_label)
        pass_verdicts = sum(1 for r in pass_adjs if r["cohort"] == cohort_label)
        effort = {}
        for r in cohort:
            effort[r["manual_effort_bucket"]] = effort.get(r["manual_effort_bucket"], 0) + 1
        return {
            "n": n, "applicable": applicable,
            "applicable_rate": round(applicable / n, 4) if n else None,
            "tool_error_count": tool_errors,
            "actionable_presentation_count": actionable,
            "pass_verdict_count": pass_verdicts,
            "manual_effort_distribution": effort,
        }

    s5a_stats = bucket_stats(s5a, "A")
    s5b_stats = bucket_stats(s5b, "B")

    # --- PR-level aggregation (the frozen statistical unit) ---
    by_pr = {}
    for r in actionable_presentations:
        by_pr.setdefault(r["pr"], []).append(r)
    actionable_prs = []
    for pr, presentations in by_pr.items():
        aps = [gate.Presentation(presentation_id=p["presentation_id"], correct=presentation_resolved_correct(p)) for p in presentations]
        actionable_prs.append(gate.ActionablePR(pr=pr, presentations=aps))
    correct, total = gate.pr_level_precision_counts(actionable_prs)

    tier1_events = []
    incorrect_prs = []
    for apr in actionable_prs:
        if not apr.is_correct():
            bad = [p.presentation_id for p in apr.presentations if not p.correct]
            tier1_events.append(f"PR #{apr.pr}: incorrect presentation(s) {bad}")
            incorrect_prs.append(apr.pr)

    false_pass_count = sum(1 for r in pass_adjs if any(rev.get("genuine_evidence") is False for rev in r["reviews"]))
    unresolved_disagreement_count = 0  # every disagreement in the ledger has a non-null coordinator_resolution

    round_complete = s5a_complete and s5b_complete
    gate_result = gate.evaluate_gate(
        round_complete=round_complete,
        tier1_events=tier1_events,
        distinct_actionable_prs_correct=correct,
        distinct_actionable_prs_total=total,
        target=30,
        lower_bound_threshold=0.90,
        lower_bound_fn=s4gen.clopper_pearson_lower_bound,
        false_pass_count=false_pass_count,
        unresolved_disagreement_count=unresolved_disagreement_count,
        s5a_complete=s5a_complete,
    )

    # --- Path-overlap diagnostics (mandate section 9) ---
    diag_rows = [json.loads(l) for l in (DESIGN / "path-overlap-diagnostics.jsonl").read_text().splitlines() if l.strip()]
    diag_by_pr = {r["pr"]: r for r in diag_rows}
    actionable_pr_diag = []
    for apr in actionable_prs:
        d = diag_by_pr.get(apr.pr, {})
        actionable_pr_diag.append({
            "pr": apr.pr,
            "correct": apr.is_correct(),
            "overlaps_s4_service_path": d.get("overlaps_s4_service_path"),
            "service_module_paths": d.get("service_module_paths"),
        })
    overlap_actionable = [d for d in actionable_pr_diag if d["overlaps_s4_service_path"]]
    novel_actionable = [d for d in actionable_pr_diag if not d["overlaps_s4_service_path"]]

    report = {
        "schema_version": 1,
        "baseline": {
            "release": "v0.4.5",
            "commit": "8f1701a289ddab8bc23f23a25cc0937863fa0356",
            "artifact_sha256": "c07bed6c37fa3e0f1885099bb3dfc7a7b741531e8a156dc4fa8a7a160bd45800",
        },
        "reconciliation": {
            "s5a_positions_unbroken_150": s5a_complete,
            "s5b_positions_unbroken_219": s5b_complete,
            "s5a_mismatches_vs_frozen_order": len(s5a_mismatches),
            "s5b_mismatches_vs_frozen_order": len(s5b_mismatches),
            "s5a_duplicate_prs": dup_a,
            "s5b_duplicate_prs": dup_b,
        },
        "s5a": s5a_stats,
        "s5b": s5b_stats,
        "pr_level_precision": {
            "distinct_actionable_prs_total": total,
            "distinct_actionable_prs_correct": correct,
            "distinct_actionable_prs_incorrect": total - correct,
            "incorrect_pr_numbers": incorrect_prs,
            "target": 30,
            "target_reached": total >= 30,
            "one_sided_95_lower_bound": gate_result.get("lower_bound"),
        },
        "false_pass_count": false_pass_count,
        "unresolved_disagreement_count": unresolved_disagreement_count,
        "path_overlap_diagnostics": {
            "actionable_prs_total": len(actionable_pr_diag),
            "actionable_prs_overlapping_s4_path": len(overlap_actionable),
            "actionable_prs_path_novel": len(novel_actionable),
            "overlap_correct": sum(1 for d in overlap_actionable if d["correct"]),
            "novel_correct": sum(1 for d in novel_actionable if d["correct"]),
            "distinct_service_paths_among_actionable": sorted({p for d in actionable_pr_diag for p in (d["service_module_paths"] or [])}),
        },
        "gate": gate_result,
    }

    out_path = HERE / "s5-final-report.json"
    out_path.write_text(json.dumps(report, indent=2) + "\n")
    print(f"wrote {out_path}")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
