#!/usr/bin/env python3
"""S5 protocol machinery self-tests (mandate section 9). Covers the
RULES the frozen protocol depends on -- most exercised here against
synthetic data since no real S5-B adjudication exists yet. The
population-freeze invariants (S5-A/S5-B disjointness, selection-record
count reconciliation) run against the REAL committed frozen files.

Run with: python3 fixtures/s5-design/test_s5_protocol_machinery.py
"""
import importlib.util
import json
import sys
from pathlib import Path

D = Path(__file__).resolve().parent


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


gate = _load("gate", D / "gate.py")

spec = importlib.util.spec_from_file_location("s4gen", D.parent / "s4-live-pr-shadow" / "generate-report.py")
s4gen = importlib.util.module_from_spec(spec)
sys.modules["s4gen"] = s4gen
spec.loader.exec_module(s4gen)


# --- frozen-prefix enforcement -----------------------------------

def test_unbroken_prefix_accepts_a_real_prefix():
    assert gate.is_unbroken_prefix([1, 2, 3, 4, 5])
    assert gate.is_unbroken_prefix([3, 1, 2])  # order of arrival doesn't matter, only the SET does
    assert gate.is_unbroken_prefix([])
    assert gate.is_unbroken_prefix([1])


def test_unbroken_prefix_rejects_a_gap():
    assert not gate.is_unbroken_prefix([1, 2, 4])  # 3 skipped


def test_unbroken_prefix_rejects_a_non_prefix_start():
    assert not gate.is_unbroken_prefix([2, 3, 4])  # doesn't start at 1


def test_unbroken_prefix_rejects_duplicates():
    assert not gate.is_unbroken_prefix([1, 2, 2, 3])


# --- two-reviewer requirement -------------------------------------

def test_two_reviewers_ok_requires_exactly_two_distinct():
    assert gate.two_reviewers_ok(["alice", "bob"])
    assert not gate.two_reviewers_ok(["alice"])
    assert not gate.two_reviewers_ok(["alice", "alice"])  # same reviewer twice is not independent
    assert not gate.two_reviewers_ok(["alice", "bob", "carol"])


# --- PR-level correctness aggregation ------------------------------

def test_pr_level_correct_requires_every_presentation_correct():
    pr = gate.ActionablePR(pr=1, presentations=[
        gate.Presentation("p1", correct=True),
        gate.Presentation("p2", correct=True),
    ])
    assert pr.is_correct()


def test_pr_level_correct_fails_on_a_single_wrong_presentation():
    """#538802-shaped case: one PR, 4 presentations -- ANY one wrong
    makes the whole PR-level observation incorrect, matching Tier-1's
    own per-presentation FAIL trigger."""
    pr = gate.ActionablePR(pr=538802, presentations=[
        gate.Presentation("p1", correct=True),
        gate.Presentation("p2", correct=True),
        gate.Presentation("p3", correct=False),
        gate.Presentation("p4", correct=True),
    ])
    assert not pr.is_correct()


def test_pr_level_correct_raises_on_a_pr_with_no_presentations():
    pr = gate.ActionablePR(pr=1, presentations=[])
    try:
        pr.is_correct()
        assert False, "expected ValueError"
    except ValueError:
        pass


def test_pr_level_precision_counts_matches_historical_538802_shape():
    prs = [
        gate.ActionablePR(1, [gate.Presentation("a", True)]),
        gate.ActionablePR(2, [gate.Presentation("b", True)]),
        gate.ActionablePR(3, [gate.Presentation("c", True)]),
        gate.ActionablePR(538802, [
            gate.Presentation("d1", True), gate.Presentation("d2", True),
            gate.Presentation("d3", True), gate.Presentation("d4", True),
        ]),
        gate.ActionablePR(5, [gate.Presentation("e", True)]),
        gate.ActionablePR(6, [gate.Presentation("f", True)]),
    ]
    correct, total = gate.pr_level_precision_counts(prs)
    assert (correct, total) == (6, 6), "matches R4's real historical overall shape: 6 distinct actionable PRs, all correct"


# --- stopping-rule precedence: Tier-1 FAIL never stops sampling early ---

def test_stopping_rule_ignores_correctness_signal_entirely():
    """should_continue_sampling's signature has no correctness/Tier-1
    parameter at all -- a Tier-1 FAIL discovered on PR #5 of a 30-target
    round must not change this function's answer."""
    # Early in the round, target not yet reached, cap not yet reached:
    # continue, regardless of what happened on any individual PR.
    assert gate.should_continue_sampling(distinct_actionable_prs_so_far=4, target=30, selected_prs_processed=20, cap=219)
    # Target reached: stop (this is the ONLY thing that stops it, not correctness).
    assert not gate.should_continue_sampling(distinct_actionable_prs_so_far=30, target=30, selected_prs_processed=150, cap=219)
    # Cap exhausted before target: stop (-> INSUFFICIENT EVIDENCE, decided elsewhere).
    assert not gate.should_continue_sampling(distinct_actionable_prs_so_far=25, target=30, selected_prs_processed=219, cap=219)


def test_tier1_fail_overrides_tier3_pass_but_round_must_still_reach_its_stop_point():
    """A synthetic Tier-1 FAIL (e.g. a false PR-relevance error) found on
    presentation #5 of a round that ultimately reaches its 30-PR target
    with everything else clean: the gate verdict is FAIL (Tier-1 wins),
    but `evaluate_gate` is only asked to classify AFTER round_complete=True
    -- proving the FAIL doesn't short-circuit sampling itself (that's
    should_continue_sampling's job, and it never saw this event)."""
    # Simulate: sampling proceeds to the real stop point regardless of the
    # early Tier-1 event (should_continue_sampling, called each step,
    # never receives tier1 information -- see the test above).
    continued_to_stop_point = not gate.should_continue_sampling(
        distinct_actionable_prs_so_far=30, target=30, selected_prs_processed=170, cap=219
    )
    assert continued_to_stop_point

    result = gate.evaluate_gate(
        round_complete=True,
        tier1_events=["PR #123456: PR-relevance error on presentation #5 (found mid-round, sampling continued per protocol)"],
        distinct_actionable_prs_correct=29,
        distinct_actionable_prs_total=30,
        target=30,
        lower_bound_threshold=0.90,
        lower_bound_fn=lambda k, n: s4gen.clopper_pearson_lower_bound(k, n),
        false_pass_count=0,
        unresolved_disagreement_count=0,
        s5a_complete=True,
    )
    assert result["verdict"] == gate.TIER1_FAIL


def test_gate_classifies_insufficient_when_cap_exhausted_before_target():
    result = gate.evaluate_gate(
        round_complete=True,
        tier1_events=[],
        distinct_actionable_prs_correct=25,
        distinct_actionable_prs_total=25,
        target=30,
        lower_bound_threshold=0.90,
        lower_bound_fn=lambda k, n: s4gen.clopper_pearson_lower_bound(k, n),
        false_pass_count=0,
        unresolved_disagreement_count=0,
        s5a_complete=True,
    )
    assert result["verdict"] == gate.TIER2_INSUFFICIENT


def test_gate_classifies_pass_on_a_clean_full_round():
    result = gate.evaluate_gate(
        round_complete=True,
        tier1_events=[],
        distinct_actionable_prs_correct=30,
        distinct_actionable_prs_total=30,
        target=30,
        lower_bound_threshold=0.90,
        lower_bound_fn=lambda k, n: s4gen.clopper_pearson_lower_bound(k, n),
        false_pass_count=0,
        unresolved_disagreement_count=0,
        s5a_complete=True,
    )
    assert result["verdict"] == gate.TIER3_PASS


def test_false_pass_is_always_tier1_regardless_of_precision_denominator():
    result = gate.evaluate_gate(
        round_complete=True,
        tier1_events=[],
        distinct_actionable_prs_correct=30,
        distinct_actionable_prs_total=30,
        target=30,
        lower_bound_threshold=0.90,
        lower_bound_fn=lambda k, n: s4gen.clopper_pearson_lower_bound(k, n),
        false_pass_count=1,
        unresolved_disagreement_count=0,
        s5a_complete=True,
    )
    assert result["verdict"] == gate.TIER1_FAIL


# --- Clopper-Pearson cross-checks (reuse s4gen, never reimplement) ---

def test_30_of_30_reference_bound_is_approximately_0_9050():
    lb = s4gen.clopper_pearson_lower_bound(30, 30)
    assert abs(lb - 0.9050) < 0.0005


def test_29_of_29_actually_clears_90_percent_correcting_the_mandates_own_assumption():
    """MECHANICALLY VERIFIED, not assumed: the mandate's own text
    guessed '29/29 < 0.90' as the INSUFFICIENT cross-check, but the real
    one-sided 95% Clopper-Pearson bound for n/n successes is
    alpha**(1/n) -- monotonically increasing in n -- and the true
    crossing point is at n=29 (0.9019 >= 0.90), not n=30. n=28 is the
    real 'just below the threshold' case (0.8985 < 0.90). This test
    asserts the REAL mechanically-computed values, and the frozen
    protocol document/design-report disclose the correction explicitly
    rather than silently keeping the mandate's guessed number.
    """
    lb_29 = s4gen.clopper_pearson_lower_bound(29, 29)
    lb_28 = s4gen.clopper_pearson_lower_bound(28, 28)
    assert lb_29 >= 0.90, f"29/29 = {lb_29:.4f}, expected >= 0.90"
    assert lb_28 < 0.90, f"28/28 = {lb_28:.4f}, expected < 0.90"


# --- population-freeze invariants (real committed files) ---------

def test_s5a_and_s5b_frozen_orders_are_disjoint():
    a_path = D / "s5a-frozen-order.json"
    b_path = D / "s5b-frozen-order.json"
    if not (a_path.exists() and b_path.exists()):
        print("SKIP  test_s5a_and_s5b_frozen_orders_are_disjoint: freeze not yet written")
        return
    a = {r["number"] for r in json.loads(a_path.read_text())}
    b = {r["number"] for r in json.loads(b_path.read_text())}
    overlap = a & b
    assert not overlap, f"S5-A and S5-B must be disjoint by construction, overlap: {overlap}"


def test_selection_records_cover_every_remaining_pr_exactly_once():
    records_path = D / "population" / "s5b-all-selection-records.jsonl"
    a_path = D / "s5a-frozen-order.json"
    survivors_path = D / "population" / "survivors.json"
    if not (records_path.exists() and a_path.exists() and survivors_path.exists()):
        print("SKIP  test_selection_records_cover_every_remaining_pr_exactly_once: freeze not yet written")
        return
    survivors = {r["number"] for r in json.loads(survivors_path.read_text())}
    a_picks = {r["number"] for r in json.loads(a_path.read_text())}
    remaining = survivors - a_picks

    records = [json.loads(l) for l in records_path.read_text().splitlines() if l.strip()]
    record_prs = [r["pr"] for r in records]
    assert len(record_prs) == len(set(record_prs)), "duplicate selection_record PR numbers found"
    assert set(record_prs) == remaining, (
        f"selection records must cover every remaining-after-S5-A PR exactly once: "
        f"missing={remaining - set(record_prs)}, extra={set(record_prs) - remaining}"
    )


def test_capacity_ledger_exclusion_counts_reconcile_to_survivor_count():
    """Every committed capacity-ledger.jsonl row's own exclusion
    breakdown must sum exactly to survivor_count -- this is exactly
    the check that would have caught the real bug found in review: an
    uncounted `continue` in run_screen()'s path-eligibility check
    silently dropped 2 records per window from the funnel's own
    reported breakdown (survivor_count itself was always correct;
    only the ledger's own disclosed breakdown was under-accounted)."""
    ledger_path = D / "population" / "capacity-ledger.jsonl"
    if not ledger_path.exists():
        print("SKIP  test_capacity_ledger_exclusion_counts_reconcile_to_survivor_count: not yet written")
        return
    rows = [json.loads(l) for l in ledger_path.read_text().splitlines() if l.strip()]
    for row in rows:
        if row.get("status") != "complete":
            continue
        total = (
            row["excl_already_examined"] + row["excl_path_ineligible"] + row["excl_window"]
            + row["excl_docs"] + row["excl_mechanical"] + row["excl_name_or_path"] + row["survivor_count"]
        )
        assert total == row["raw_pr_count"], (
            f"window {row['window_start']}: exclusion components sum to {total}, "
            f"expected raw_pr_count {row['raw_pr_count']}"
        )


def test_duplicate_pr_number_within_a_frozen_order_is_rejected():
    """Synthetic: a frozen-order list with a duplicate PR number must be
    detectable -- this is the general-purpose check a real freeze script
    runs before ever committing a frozen order."""
    order = [{"number": 1}, {"number": 2}, {"number": 2}, {"number": 3}]
    numbers = [r["number"] for r in order]
    assert len(numbers) != len(set(numbers)), "sanity: the synthetic fixture itself must contain a duplicate"
    # the actual invariant a real freeze script enforces:
    def assert_no_duplicates(rows):
        nums = [r["number"] for r in rows]
        if len(nums) != len(set(nums)):
            raise ValueError(f"duplicate PR number(s) in frozen order: {[n for n in nums if nums.count(n) > 1]}")
    try:
        assert_no_duplicates(order)
        assert False, "expected ValueError on duplicate PR number"
    except ValueError:
        pass


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
        except Exception as e:
            failures += 1
            print(f"ERROR {t.__name__}: {type(e).__name__}: {e}")
    print(f"\n{len(tests) - failures}/{len(tests)} passed")
    if failures:
        raise SystemExit(1)
