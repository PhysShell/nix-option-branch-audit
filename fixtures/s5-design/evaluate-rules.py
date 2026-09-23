#!/usr/bin/env python3
"""S5 design round: evaluate every candidate enrichment rule against
the S4 187-PR historical design corpus. This is the ONLY point in the
whole design round where features.jsonl and historical-labels.jsonl
are joined -- the join happens here, in a script that produces a
report, never inside the selector logic itself (enrichment-rules.py
stays pure over features alone, proven by test_no_leakage.py).

Writes: fixtures/s5-design/rule-comparison.jsonl (one row per rule)
        fixtures/s5-design/s4-round-stability.json (S4-only
          leave-one-round-out is not meaningful with a single round --
          see design-report.md's own honest account of why Option B
          could not be fully implemented against S1/S2/S3)
"""
import importlib.util
import json
import sys
from pathlib import Path

D = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("rules", D / "enrichment-rules.py")
rules_mod = importlib.util.module_from_spec(spec)
sys.modules["rules"] = rules_mod
spec.loader.exec_module(rules_mod)


def load_jsonl(path):
    return [json.loads(l) for l in path.read_text().splitlines() if l.strip()]


def main():
    features = {r["pr"]: r for r in load_jsonl(D / "features.jsonl")}
    labels = {r["pr"]: r for r in load_jsonl(D / "historical-labels.jsonl")}
    assert set(features) == set(labels), "features/labels PR sets must match exactly"

    joined = []
    for pr, feat in features.items():
        lab = labels[pr]
        joined.append({"pr": pr, "cohort": feat["cohort"], "features": feat, "label": lab})

    results = []
    for rule_name, rule_fn in rules_mod.RULES.items():
        selected = []
        for row in joined:
            sel = rule_fn(row["features"])
            if sel["selected"]:
                selected.append({**row, "selection": sel})

        n_selected = len(selected)
        n_total = len(joined)
        applicable_selected = sum(1 for s in selected if s["label"]["applicable"])
        actionable_total = sum(s["label"]["actionable_count"] for s in selected)
        prs_with_actionable = sum(1 for s in selected if s["label"]["actionable_count"] > 0)
        false_finding_total = sum(s["label"]["false_finding_count"] for s in selected)
        false_pass_total = sum(s["label"]["false_pass_count"] for s in selected)
        pass_verdict_total = sum(s["label"]["pass_verdict_count"] for s in selected)
        tool_error_total = sum(s["label"]["tool_error_ordinary_count"] for s in selected)

        # Baseline (unselected/whole-corpus) comparison for yield lift.
        baseline_actionable_rate = sum(r["label"]["actionable_count"] for r in joined) / n_total if n_total else 0
        selected_actionable_rate = actionable_total / n_selected if n_selected else 0

        # PR-level yield: distinct actionable PRs per selected PR. This,
        # not the presentation-count rate above, is the PRIMARY unit for
        # S5-B planning/stopping-rule arithmetic -- see design-report.md's
        # statistical-unit correction. A PR contributing several
        # presentations (e.g. #538802, 4 presentations) inflates the
        # presentation-rate but counts once here, since presentations
        # from the same PR are not independent Bernoulli trials.
        prs_with_actionable_rate = prs_with_actionable / n_selected if n_selected else 0

        # Cohort-A-only (S4-A, the representative random cohort) subset,
        # since a real S5-B rule applied to a representative population
        # is the thing whose lift actually matters.
        joined_a = [r for r in joined if r["cohort"] == "A"]
        selected_a = [s for s in selected if s["cohort"] == "A"]
        actionable_a_total = sum(s["label"]["actionable_count"] for s in selected_a)
        prs_with_actionable_a = sum(1 for s in selected_a if s["label"]["actionable_count"] > 0)

        results.append({
            "rule": rule_name,
            "corpus_total_prs": n_total,
            "selected_prs": n_selected,
            "selection_rate": round(n_selected / n_total, 4) if n_total else 0,
            "applicable_among_selected": applicable_selected,
            "applicable_fraction_among_selected": round(applicable_selected / n_selected, 4) if n_selected else None,
            "actionable_presentations_in_selected": actionable_total,
            "actionable_per_selected_pr": round(selected_actionable_rate, 4),
            "prs_with_at_least_one_actionable": prs_with_actionable,
            "actionable_prs_per_selected_pr": round(prs_with_actionable_rate, 4),
            "baseline_actionable_per_pr_whole_corpus": round(baseline_actionable_rate, 4),
            "yield_lift_vs_whole_corpus": (
                round(selected_actionable_rate / baseline_actionable_rate, 2)
                if baseline_actionable_rate > 0 else None
            ),
            "false_finding_in_selected": false_finding_total,
            "false_pass_in_selected": false_pass_total,
            "pass_verdicts_in_selected": pass_verdict_total,
            "tool_error_ordinary_in_selected": tool_error_total,
            "s4a_only_selected_prs": len(selected_a),
            "s4a_only_actionable_in_selected": actionable_a_total,
            "s4a_only_prs_with_at_least_one_actionable": prs_with_actionable_a,
            "s4a_only_of_120": f"{len(selected_a)}/{len(joined_a)}",
        })

    out_path = D / "rule-comparison.jsonl"
    with out_path.open("w") as f:
        for r in results:
            f.write(json.dumps(r) + "\n")
    print(f"wrote {out_path}")
    for r in results:
        print(
            f"  {r['rule']}: selected={r['selected_prs']}/{r['corpus_total_prs']} "
            f"({r['selection_rate']*100:.1f}%), actionable_prs={r['prs_with_at_least_one_actionable']}, "
            f"presentations={r['actionable_presentations_in_selected']}, "
            f"actionable_prs_per_selected_pr={r['actionable_prs_per_selected_pr']}, "
            f"lift={r['yield_lift_vs_whole_corpus']}"
        )

    # Selection-record example (section 12's own required shape) for
    # one real PR under the FROZEN rule (R4_mkoption_line_edit_v1),
    # since that is the rule S5-B actually uses -- not R7, which was
    # never recommended or frozen.
    example = None
    for row in joined:
        sel = rules_mod.r4_mkoption_line_edit_v1(row["features"])
        if sel["selected"]:
            example = {
                "pr": row["pr"],
                "selected_by": "R4_mkoption_line_edit_v1",
                "features": {
                    "mkoption_edit_count": row["features"]["mkoption_edit_count"],
                    "mkenableoption_edit_count": row["features"]["mkenableoption_edit_count"],
                },
                "score": sel["score"],
                "reasons": sel["reasons"],
            }
            break
    (D / "selection-record-example.json").write_text(json.dumps(example, indent=2) + "\n")
    print(f"\nexample selection record: {example}")


if __name__ == "__main__":
    main()
