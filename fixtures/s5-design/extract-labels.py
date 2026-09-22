#!/usr/bin/env python3
"""S5 design round: historical outcome labels, extracted mechanically
from S4's own real adjudication-ledger.jsonl. These labels are used
ONLY to evaluate candidate enrichment selectors on the already-seen
design corpus -- NEVER as an input to any future S5 selection.

Kept in a physically separate file from features.jsonl (produced by
extract-features.py, which never reads this ledger's own outcome
fields) specifically so the "features never see labels" boundary is
auditable file-by-file, not just by code review of one script.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
LEDGER = ROOT / "fixtures/s4-live-pr-shadow/adjudication-ledger.jsonl"
OUT = Path(__file__).resolve().parent / "historical-labels.jsonl"


def resolved_judgment(rec):
    reviews = rec["reviews"]
    r1, r2 = reviews[0], reviews[1]
    keys = ["scanner_substance_correct", "pr_relevant", "causal_framing_correct", "rendered_presentation_correct"]
    if all(r1[k] == r2[k] for k in keys):
        return r1
    return rec.get("coordinator_resolution")


def resolved_pass_judgment(rec):
    reviews = rec["reviews"]
    r1, r2 = reviews[0], reviews[1]
    if r1["genuine_evidence"] == r2["genuine_evidence"]:
        return r1
    return rec.get("coordinator_resolution")


def main():
    records = [json.loads(l) for l in LEDGER.read_text().splitlines() if l.strip()]
    pr_summaries = {(r["cohort"], r["pr"]): r for r in records if r["record_type"] == "pr_summary"}
    actionable = [r for r in records if r["record_type"] == "actionable_presentation"]
    passes = [r for r in records if r["record_type"] == "pass_adjudication"]
    tool_errors = [r for r in records if r["record_type"] == "tool_error"]

    by_pr_actionable: dict[tuple, list] = {}
    for r in actionable:
        by_pr_actionable.setdefault((r["cohort"], r["pr"]), []).append(r)
    by_pr_pass: dict[tuple, list] = {}
    for r in passes:
        by_pr_pass.setdefault((r["cohort"], r["pr"]), []).append(r)
    by_pr_tool_error: dict[tuple, list] = {}
    for r in tool_errors:
        by_pr_tool_error.setdefault((r["cohort"], r["pr"]), []).append(r)

    rows = []
    for key, summary in pr_summaries.items():
        cohort, pr = key
        acts = by_pr_actionable.get(key, [])
        pss = by_pr_pass.get(key, [])
        tes = by_pr_tool_error.get(key, [])

        actionable_count = len(acts)
        actionable_all_correct = all(
            resolved_judgment(a) is not None
            and resolved_judgment(a)["scanner_substance_correct"]
            and resolved_judgment(a)["pr_relevant"]
            and resolved_judgment(a)["causal_framing_correct"]
            and resolved_judgment(a)["rendered_presentation_correct"]
            for a in acts
        ) if acts else None
        false_finding_count = sum(
            1 for a in acts
            if resolved_judgment(a) is not None and not resolved_judgment(a)["scanner_substance_correct"]
        )
        pr_relevance_error_count = sum(
            1 for a in acts
            if resolved_judgment(a) is not None and not resolved_judgment(a)["pr_relevant"]
        )
        causal_framing_error_count = sum(
            1 for a in acts
            if resolved_judgment(a) is not None and not resolved_judgment(a)["causal_framing_correct"]
        )

        pass_count = len(pss)
        false_pass_count = sum(
            1 for p in pss
            if resolved_pass_judgment(p) is not None and not resolved_pass_judgment(p)["genuine_evidence"]
        )

        tool_error_count = len(tes)
        tool_error_ordinary_count = sum(1 for t in tes if not t["attributable_to_unsupported_input"])

        rows.append({
            "pr": pr,
            "cohort": cohort,
            "applicable": summary["applicable"],
            "actionable_count": actionable_count,
            "actionable_all_correct": actionable_all_correct,
            "false_finding_count": false_finding_count,
            "pr_relevance_error_count": pr_relevance_error_count,
            "causal_framing_error_count": causal_framing_error_count,
            "pass_verdict_count": pass_count,
            "false_pass_count": false_pass_count,
            "tool_error_count": tool_error_count,
            "tool_error_ordinary_count": tool_error_ordinary_count,
            "manual_effort_bucket": summary.get("manual_effort_bucket"),
            "gap_classification": summary.get("gap_classification"),
        })

    OUT.write_text("\n".join(json.dumps(r) for r in rows) + "\n")
    print(f"wrote {len(rows)} historical label rows to {OUT}")
    print(f"  applicable: {sum(1 for r in rows if r['applicable'])}")
    print(f"  total actionable: {sum(r['actionable_count'] for r in rows)}")
    print(f"  total false_finding: {sum(r['false_finding_count'] for r in rows)}")
    print(f"  total false_pass: {sum(r['false_pass_count'] for r in rows)}")
    print(f"  total tool_error_ordinary: {sum(r['tool_error_ordinary_count'] for r in rows)}")


if __name__ == "__main__":
    main()
