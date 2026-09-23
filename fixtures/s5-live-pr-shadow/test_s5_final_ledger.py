#!/usr/bin/env python3
"""Real-data invariant checks against the final S5 adjudication ledger
-- the same invariants test_s5_protocol_machinery.py already unit-tests
against synthetic data, now re-run against the actual 393-row ledger
this round produced. No network, no oba invocation.

Run with: python3 fixtures/s5-live-pr-shadow/test_s5_final_ledger.py
"""
import importlib.util
import json
import sys
from pathlib import Path

D = Path(__file__).resolve().parent
DESIGN = D.parent / "s5-design"


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


gate = _load("gate", DESIGN / "gate.py")


def load_ledger():
    return [json.loads(l) for l in (D / "adjudication-ledger.jsonl").read_text().splitlines() if l.strip()]


def test_s5a_positions_are_unbroken_1_to_150():
    rows = load_ledger()
    positions = [r["position"] for r in rows if r["record_type"] == "pr_summary" and r["cohort"] == "A"]
    assert gate.is_unbroken_prefix(positions) and len(positions) == 150


def test_s5b_positions_are_unbroken_1_to_219():
    rows = load_ledger()
    positions = [r["position"] for r in rows if r["record_type"] == "pr_summary" and r["cohort"] == "B"]
    assert gate.is_unbroken_prefix(positions) and len(positions) == 219


def test_no_duplicate_pr_numbers_within_either_cohort():
    rows = load_ledger()
    for cohort in ("A", "B"):
        prs = [r["pr"] for r in rows if r["record_type"] == "pr_summary" and r["cohort"] == cohort]
        assert len(prs) == len(set(prs)), f"duplicate PR numbers in cohort {cohort}"


def test_pr_summaries_reconcile_exactly_against_frozen_orders():
    rows = load_ledger()
    s5a = json.loads((DESIGN / "s5a-frozen-order.json").read_text())
    s5b = json.loads((DESIGN / "s5b-frozen-order.json").read_text())
    s5a_by_pos = {i + 1: r["number"] for i, r in enumerate(s5a)}
    s5b_by_pos = {i + 1: r["number"] for i, r in enumerate(s5b)}
    for r in rows:
        if r["record_type"] != "pr_summary":
            continue
        expected = s5a_by_pos[r["position"]] if r["cohort"] == "A" else s5b_by_pos[r["position"]]
        assert r["pr"] == expected, f"cohort {r['cohort']} position {r['position']}: ledger has {r['pr']}, frozen order has {expected}"


def test_every_actionable_presentation_and_pass_has_exactly_two_distinct_reviewers():
    rows = load_ledger()
    for r in rows:
        if r["record_type"] not in ("actionable_presentation", "pass_adjudication"):
            continue
        reviewer_ids = [rev["reviewer_id"] for rev in r["reviews"]]
        assert gate.two_reviewers_ok(reviewer_ids), f"{r.get('presentation_id') or r.get('pass_id')}: reviewers {reviewer_ids}"


def test_report_stats_are_counted_from_ledger_truth_not_stale_pr_summary_counters():
    """S5-R0 regression test: generate-report.py must count
    actionable_presentation_count/pass_verdict_count directly from the
    ledger's own actionable_presentation/pass_adjudication records,
    never from pr_summary's own counters -- those go stale whenever a
    coordinator-driven adjudication adds a record the original
    evidence-gathering pass didn't flag (exactly PR #471312's false
    "Unchanged" and PR #461261's PASS). This test proves the known
    discrepancy is real (so a future accidental revert to the stale
    counters would be caught) and that the committed report already
    reflects the ledger-truth count."""
    rows = load_ledger()
    ap = [r for r in rows if r["record_type"] == "actionable_presentation"]
    pa = [r for r in rows if r["record_type"] == "pass_adjudication"]
    ps_by_pr = {r["pr"]: r for r in rows if r["record_type"] == "pr_summary"}

    stale_ap = [r["pr"] for r in ap if ps_by_pr[r["pr"]]["actionable_count"] < 1]
    stale_pa = [r["pr"] for r in pa if ps_by_pr[r["pr"]]["pass_verdict_count"] < 1]
    assert stale_ap == [471312], f"expected exactly the known PR 471312 staleness, got {stale_ap}"
    assert stale_pa == [461261], f"expected exactly the known PR 461261 staleness, got {stale_pa}"

    report_path = D / "s5-final-report.json"
    if not report_path.exists():
        print("SKIP  test_report_stats_are_counted_from_ledger_truth_not_stale_pr_summary_counters: report not yet generated")
        return
    report = json.loads(report_path.read_text())
    ap_a = sum(1 for r in ap if r["cohort"] == "A")
    ap_b = sum(1 for r in ap if r["cohort"] == "B")
    pa_a = sum(1 for r in pa if r["cohort"] == "A")
    pa_b = sum(1 for r in pa if r["cohort"] == "B")
    assert report["s5a"]["actionable_presentation_count"] == ap_a
    assert report["s5b"]["actionable_presentation_count"] == ap_b
    assert report["s5a"]["pass_verdict_count"] == pa_a
    assert report["s5b"]["pass_verdict_count"] == pa_b


def test_every_disagreement_has_a_coordinator_resolution():
    """A disagreement is any actionable_presentation/pass_adjudication where
    the two raw reviews don't trivially agree on every field -- these must
    never have coordinator_resolution left null."""
    rows = load_ledger()
    for r in rows:
        if r["record_type"] not in ("actionable_presentation", "pass_adjudication"):
            continue
        reviews = r["reviews"]
        if len(reviews) != 2:
            continue
        keys = set(reviews[0].keys()) & set(reviews[1].keys()) - {"reviewer_id", "notes"}
        disagree = any(reviews[0].get(k) != reviews[1].get(k) for k in keys)
        if disagree:
            assert r["coordinator_resolution"] is not None, (
                f"{r.get('presentation_id') or r.get('pass_id')} has disagreeing reviews but no coordinator_resolution"
            )


def test_no_actionable_pr_is_silently_dropped_from_gate_computation():
    """Every distinct PR number appearing in actionable_presentation records
    must appear in the gate's own PR-level aggregation -- a mechanical
    reconciliation between the raw ledger and generate-report.py's output."""
    rows = load_ledger()
    ap_prs = {r["pr"] for r in rows if r["record_type"] == "actionable_presentation"}
    report_path = D / "s5-final-report.json"
    if not report_path.exists():
        print("SKIP  test_no_actionable_pr_is_silently_dropped_from_gate_computation: report not yet generated")
        return
    report = json.loads(report_path.read_text())
    assert report["pr_level_precision"]["distinct_actionable_prs_total"] == len(ap_prs)


def test_gate_verdict_is_fail_given_three_confirmed_tier1_events():
    """A fixed, disclosed assertion about THIS round's own real result --
    not a synthetic test. If this ever fails, the committed report/ledger
    have diverged and must be investigated, not silently regenerated."""
    report_path = D / "s5-final-report.json"
    if not report_path.exists():
        print("SKIP  test_gate_verdict_is_fail_given_three_confirmed_tier1_events: report not yet generated")
        return
    report = json.loads(report_path.read_text())
    assert report["gate"]["verdict"] == "FAIL"
    assert len(report["gate"]["reasons"]) == 3


if __name__ == "__main__":
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    failures = 0
    for t in tests:
        try:
            t()
            print(f"PASS  {t.__name__}")
        except AssertionError as e:
            failures += 1
            print(f"FAIL  {t.__name__}: {e}")
    print(f"\n{len(tests) - failures}/{len(tests)} passed")
    sys.exit(1 if failures else 0)
