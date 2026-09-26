#!/usr/bin/env python3
"""S6-R0A protocol-tooling regression tests. Synthetic/mocked API
responses and historical (pre-S6, arbitrary) dates ONLY -- no real
network call, no query against the actual post-v0.5.0 candidate
window. Confirms this file's own compliance is exactly the point of
S6-R0A item 2 ("Preserve blindness"): see `test_no_real_network_calls`
and `test_no_s6_candidate_window_dates_used`, below, which check THIS
FILE's own source text for exactly that.

Run with: python3 fixtures/s6-r0/test_population_and_sampling.py
"""
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

import importlib.util as _ilu

_HERE = Path(__file__).resolve().parent


def _load_by_path(module_name, filename):
    # Both "population-and-sampling.py" and "pilot-accounting.py" have
    # hyphens in their own filenames (matching this directory's own
    # existing naming convention) -- not importable via a normal
    # `import` statement, so both are loaded explicitly by path.
    spec = _ilu.spec_from_file_location(module_name, _HERE / filename)
    mod = _ilu.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


ps = _load_by_path("population_and_sampling", "population-and-sampling.py")
pa = _load_by_path("pilot_accounting", "pilot-accounting.py")


# Arbitrary HISTORICAL dates, well before v0.5.0's own release
# (2026-09-24) and well before this project even existed -- used only
# to exercise the sharding arithmetic, never to query anything real.
HIST_LOWER = datetime(2020, 1, 1, tzinfo=timezone.utc)
HIST_UPPER = datetime(2020, 1, 8, tzinfo=timezone.utc)  # 7-day window


def _mock_search(responses_by_query):
    """Returns a `search_fn` that looks up its own canned response by
    exact query string -- forces every test to be explicit about which
    shard query it expects, rather than accidentally matching the wrong
    one.
    """
    calls = []

    def fn(query):
        calls.append(query)
        if query not in responses_by_query:
            raise AssertionError(f"unexpected query, no mock response registered: {query!r}")
        return responses_by_query[query]

    fn.calls = calls
    return fn


def _items(numbers, merged_ats):
    return [{"number": n, "merged_at": m} for n, m in zip(numbers, merged_ats)]


# --- 1. search result exactly below cap -------------------------------

def test_result_well_below_cap_is_accepted_without_subdivision():
    items = _items([1, 2, 3], ["2020-01-01T00:00:00Z"] * 3)
    q = ps._shard_query(HIST_LOWER.date(), (HIST_UPPER - timedelta(microseconds=1)).date())
    search = _mock_search({q: (items, 3, False)})
    result = ps.fetch_merged_prs_in_window(HIST_LOWER, HIST_UPPER, search_fn=search)
    assert len(result) == 3
    assert len(search.calls) == 1, "a shard well below the cap must never be subdivided"


# --- 2. result count at cap boundary requiring subdivision (or not) ---

def test_total_count_exactly_at_cap_is_complete_no_subdivision():
    n = ps.SEARCH_API_RESULT_CAP
    items = _items(list(range(1, n + 1)), ["2020-01-01T00:00:00Z"] * n)
    q = ps._shard_query(HIST_LOWER.date(), (HIST_UPPER - timedelta(microseconds=1)).date())
    search = _mock_search({q: (items, n, False)})
    result = ps.fetch_merged_prs_in_window(HIST_LOWER, HIST_UPPER, search_fn=search)
    assert len(result) == n
    assert len(search.calls) == 1, "total_count == cap is fully retrievable, must not subdivide"


def test_total_count_one_over_cap_requires_subdivision():
    n = ps.SEARCH_API_RESULT_CAP + 1
    whole_shard_q = ps._shard_query(HIST_LOWER.date(), (HIST_UPPER - timedelta(microseconds=1)).date())
    mid = HIST_LOWER.date() + (((HIST_UPPER - timedelta(microseconds=1)).date() - HIST_LOWER.date()) // 2)
    hi_date = (HIST_UPPER - timedelta(microseconds=1)).date()
    left_q = ps._shard_query(HIST_LOWER.date(), mid)
    right_q = ps._shard_query(mid + timedelta(days=1), hi_date)
    left_items = _items(list(range(1, 4)), ["2020-01-02T00:00:00Z"] * 3)
    right_items = _items(list(range(4, 7)), ["2020-01-06T00:00:00Z"] * 3)
    search = _mock_search({
        whole_shard_q: ([], n, False),  # only total_count matters here; items ignored on split
        left_q: (left_items, 3, False),
        right_q: (right_items, 3, False),
    })
    result = ps.fetch_merged_prs_in_window(HIST_LOWER, HIST_UPPER, search_fn=search)
    assert {r["number"] for r in result} == {1, 2, 3, 4, 5, 6}
    assert whole_shard_q in search.calls, "the oversized shard must actually be queried (to learn its own total_count) before being split"
    assert len(search.calls) == 3, "one oversized query, then exactly its two children"


# --- 3. recursive shard subdivision (more than one level) -------------

def test_recursive_multi_level_subdivision():
    # A 4-day window where the whole-window and BOTH halves are
    # oversized, forcing a second level of splitting down to
    # individual days.
    lower = datetime(2020, 1, 1, tzinfo=timezone.utc)
    upper = datetime(2020, 1, 5, tzinfo=timezone.utc)  # days 1,2,3,4
    over = ps.SEARCH_API_RESULT_CAP + 1
    d1, d2, d3, d4 = [datetime(2020, 1, d, tzinfo=timezone.utc).date() for d in (1, 2, 3, 4)]
    whole_q = ps._shard_query(d1, d4)
    left_q = ps._shard_query(d1, d2)     # days 1-2
    right_q = ps._shard_query(d3, d4)    # days 3-4
    day1_q = ps._shard_query(d1, d1)
    day2_q = ps._shard_query(d2, d2)
    day3_items = _items([30], ["2020-01-03T00:00:00Z"])
    day4_items = _items([40], ["2020-01-04T00:00:00Z"])
    day1_items = _items([10], ["2020-01-01T00:00:00Z"])
    day2_items = _items([20], ["2020-01-02T00:00:00Z"])
    search = _mock_search({
        whole_q: ([], over, False),
        left_q: ([], over, False),      # left half STILL oversized -> must split again
        right_q: (day3_items + day4_items, 2, False),  # right half fine as-is
        day1_q: (day1_items, 1, False),
        day2_q: (day2_items, 1, False),
    })
    result = ps.fetch_merged_prs_in_window(lower, upper, search_fn=search)
    assert {r["number"] for r in result} == {10, 20, 30, 40}
    assert day1_q in search.calls and day2_q in search.calls, "left half must recurse down to single days"


# --- 4. incomplete_results=true forces subdivision even if total_count looks fine --

def test_incomplete_results_flag_forces_subdivision_regardless_of_total_count():
    whole_q = ps._shard_query(HIST_LOWER.date(), (HIST_UPPER - timedelta(microseconds=1)).date())
    mid = HIST_LOWER.date() + (((HIST_UPPER - timedelta(microseconds=1)).date() - HIST_LOWER.date()) // 2)
    hi_date = (HIST_UPPER - timedelta(microseconds=1)).date()
    left_q = ps._shard_query(HIST_LOWER.date(), mid)
    right_q = ps._shard_query(mid + timedelta(days=1), hi_date)
    search = _mock_search({
        whole_q: ([{"number": 99, "merged_at": "2020-01-01T00:00:00Z"}], 1, True),  # tiny total_count, but flagged incomplete
        left_q: ([{"number": 99, "merged_at": "2020-01-01T00:00:00Z"}], 1, False),
        right_q: ([], 0, False),
    })
    result = ps.fetch_merged_prs_in_window(HIST_LOWER, HIST_UPPER, search_fn=search)
    assert [r["number"] for r in result] == [99]
    assert whole_q in search.calls and left_q in search.calls, "incomplete_results=true must trigger a split even with a tiny total_count"


def test_single_day_shard_still_incomplete_raises_population_incomplete_error():
    day = datetime(2020, 1, 1, tzinfo=timezone.utc)
    lower, upper = day, day + timedelta(days=1)
    q = ps._shard_query(day.date(), day.date())
    search = _mock_search({q: ([], 1, True)})  # flagged incomplete, no finer shard possible
    try:
        ps.fetch_merged_prs_in_window(lower, upper, search_fn=search)
        raise AssertionError("expected PopulationIncompleteError")
    except ps.PopulationIncompleteError:
        pass


def test_retrieved_item_count_mismatching_total_count_also_fails_closed():
    q = ps._shard_query(HIST_LOWER.date(), (HIST_UPPER - timedelta(microseconds=1)).date())
    # total_count says 5, but the mock only "returns" 2 items -- a real,
    # if different, form of incompleteness the consistency check must catch.
    search = _mock_search({q: (_items([1, 2], ["2020-01-01T00:00:00Z"] * 2), 5, False)})
    try:
        ps.fetch_merged_prs_in_window(HIST_LOWER, HIST_UPPER, search_fn=search)
        raise AssertionError("expected PopulationIncompleteError")
    except ps.PopulationIncompleteError:
        pass


# --- 5. duplicate PR returned across adjacent date shards -------------

def test_duplicate_pr_across_adjacent_shards_is_deduplicated():
    whole_q = ps._shard_query(HIST_LOWER.date(), (HIST_UPPER - timedelta(microseconds=1)).date())
    mid = HIST_LOWER.date() + (((HIST_UPPER - timedelta(microseconds=1)).date() - HIST_LOWER.date()) // 2)
    hi_date = (HIST_UPPER - timedelta(microseconds=1)).date()
    left_q = ps._shard_query(HIST_LOWER.date(), mid)
    right_q = ps._shard_query(mid + timedelta(days=1), hi_date)
    over = ps.SEARCH_API_RESULT_CAP + 1
    # PR 7 (merged right at the shard boundary) appears in BOTH halves --
    # a real possibility if a merge timestamp sits exactly on the split
    # instant and both queries' own inclusive date ranges touch it.
    left_items = _items([7], ["2020-01-04T23:59:59Z"])
    right_items = _items([7, 8], ["2020-01-04T23:59:59Z", "2020-01-05T00:00:01Z"])
    search = _mock_search({
        whole_q: ([], over, False),
        left_q: (left_items, 1, False),
        right_q: (right_items, 2, False),
    })
    result = ps.fetch_merged_prs_in_window(HIST_LOWER, HIST_UPPER, search_fn=search)
    numbers = [r["number"] for r in result]
    assert numbers.count(7) == 1, f"PR 7 must appear exactly once after dedup, got {numbers}"
    assert set(numbers) == {7, 8}


# --- 6. exact lower/upper timestamp filtering --------------------------

def test_exact_timestamp_bounds_are_applied_client_side():
    q = ps._shard_query(HIST_LOWER.date(), (HIST_UPPER - timedelta(microseconds=1)).date())
    items = [
        {"number": 1, "merged_at": "2019-12-31T23:59:59Z"},  # before lower -- must be excluded
        {"number": 2, "merged_at": "2020-01-01T00:00:00Z"},  # exactly at lower -- included (inclusive)
        {"number": 3, "merged_at": "2020-01-07T23:59:59Z"},  # just before upper -- included
        {"number": 4, "merged_at": "2020-01-08T00:00:00Z"},  # exactly at upper -- excluded (exclusive)
    ]
    search = _mock_search({q: (items, 4, False)})
    result = ps.fetch_merged_prs_in_window(HIST_LOWER, HIST_UPPER, search_fn=search)
    assert {r["number"] for r in result} == {2, 3}, result


# --- 7. deterministic final ordering -----------------------------------

def test_final_ordering_is_merged_at_then_pr_number():
    q = ps._shard_query(HIST_LOWER.date(), (HIST_UPPER - timedelta(microseconds=1)).date())
    items = [
        {"number": 30, "merged_at": "2020-01-03T00:00:00Z"},
        {"number": 10, "merged_at": "2020-01-01T00:00:00Z"},
        {"number": 11, "merged_at": "2020-01-01T00:00:00Z"},  # same timestamp as 10, tie-break by number
        {"number": 20, "merged_at": "2020-01-02T00:00:00Z"},
    ]
    search = _mock_search({q: (items, 4, False)})
    result = ps.fetch_merged_prs_in_window(HIST_LOWER, HIST_UPPER, search_fn=search)
    assert [r["number"] for r in result] == [10, 11, 20, 30]


# --- 8. deterministic seeded P1 sample ---------------------------------

def test_seeded_sample_is_reproducible_across_repeated_calls():
    pool = [{"number": n, "merged_at": f"2020-01-01T00:00:{n:02d}Z"} for n in range(1, 51)]
    a = ps.select_sample(pool, ps.SEED, 15)
    b = ps.select_sample(pool, ps.SEED, 15)
    assert a == b, "the same pool and seed must always produce the same sample"
    assert len(a) == 15
    assert len(set(r["number"] for r in a)) == 15, "no duplicate PRs within one sample"


# --- 9. leakage exclusion/replacement behavior -------------------------

def test_leakage_replacement_is_deterministic_and_excludes_the_leaked_pr():
    # preregistration.md section 6 promises only "replace using the
    # frozen sampling rule, not manual choice" -- a fresh, deterministic
    # re-draw over the reduced (leaked-PR-removed) pool satisfies that
    # exactly. It does NOT promise the other 14 keep their exact prior
    # positions (Fisher-Yates over a changed-length list is not
    # incremental by construction) -- asserting that stronger property
    # would be inventing an extra guarantee the frozen text never made.
    pool = [{"number": n, "merged_at": f"2020-01-01T00:00:{n:02d}Z"} for n in range(1, 51)]
    first = ps.select_sample(pool, ps.SEED, 15)
    leaked = first[0]["number"]
    replacement_a = ps.select_sample(pool, ps.SEED, 15, already_used_numbers=[leaked])
    replacement_b = ps.select_sample(pool, ps.SEED, 15, already_used_numbers=[leaked])
    assert replacement_a == replacement_b, "replacement draw must itself be deterministic"
    assert leaked not in [r["number"] for r in replacement_a]
    assert len(replacement_a) == 15


def test_check_leakage_reports_real_overlap_and_none_for_disjoint_sets(tmp_path=None):
    import json
    import tempfile
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump({"excluded_pr_numbers": [1, 2, 3, 244302]}, f)
        path = f.name
    assert ps.check_leakage([4, 5, 6], exclusion_path=path) == []
    assert ps.check_leakage([2, 5, 244302], exclusion_path=path) == [2, 244302]


# --- 10. multi-target PR accounting ------------------------------------

def test_target_cap_accepts_up_to_the_budget_then_records_the_rest_as_cut():
    per_pr = [
        (100, ["t1"]),
        (200, ["t1", "t2", "t3"]),
        (300, ["t1", "t2"]),
    ]
    accepted, cut = pa.apply_target_cap(per_pr, cap=4)
    assert accepted == [(100, "t1"), (200, "t1"), (200, "t2"), (200, "t3")]
    assert cut == [(300, "t1"), (300, "t2")]
    assert len(accepted) == 4
    assert len(cut) == 2


def test_target_cap_no_cut_when_total_within_budget():
    per_pr = [(100, ["t1"]), (200, ["t1"])]
    accepted, cut = pa.apply_target_cap(per_pr, cap=pa.MAX_TARGETS_TOTAL)
    assert cut == []
    assert len(accepted) == 2


# --- 11. NC2/NC3/NC4 denominator calculations --------------------------

def test_nc2_denominator_is_fixed_sampled_pr_count_not_target_count():
    r = pa.nc2_pr_level(derived_pr_count=9)
    assert r == {"numerator": 9, "denominator": 15, "passed": True}
    r2 = pa.nc2_pr_level(derived_pr_count=8)
    assert r2["passed"] is False


def test_nc3_denominator_is_derived_target_count_not_pr_count():
    # 20 derived targets, 14 substantive -> threshold = ceil(0.7*20)=14 -> passes exactly at the boundary
    r = pa.nc3_target_level(substantive_target_count=14, derived_target_count=20)
    assert r["denominator"] == 20
    assert r["threshold"] == 14
    assert r["passed"] is True
    r2 = pa.nc3_target_level(substantive_target_count=13, derived_target_count=20)
    assert r2["passed"] is False


def test_nc4_denominator_is_substantive_target_count():
    r = pa.nc4_target_level(adjudicable_target_count=7, substantive_target_count=10)
    assert r["denominator"] == 10
    assert r["threshold"] == 7
    assert r["passed"] is True


def test_nc3_and_nc4_zero_denominator_is_a_defined_fail_not_a_crash():
    r3 = pa.nc3_target_level(substantive_target_count=0, derived_target_count=0)
    assert r3["passed"] is False
    r4 = pa.nc4_target_level(adjudicable_target_count=0, substantive_target_count=0)
    assert r4["passed"] is False


# --- 12. expansion batch stopping calculation --------------------------

def test_batch_gate_ignores_nc1_and_uses_target_level_nc3_nc4():
    passed, detail = pa.batch_passes_expansion_gates(
        derived_pr_count=10, substantive_target_count=15,
        derived_target_count=20, adjudicable_target_count=15,
    )
    assert passed is True
    assert set(detail.keys()) == {"nc2", "nc3", "nc4"}, "NC1 must never appear in a per-batch gate"


def test_expansion_stops_at_cumulative_thirty_substantive_targets():
    history = [
        {"derived_pr_count": 10, "substantive_target_count": 15, "derived_target_count": 18, "adjudicable_target_count": 15},
        {"derived_pr_count": 10, "substantive_target_count": 16, "derived_target_count": 18, "adjudicable_target_count": 16},
    ]
    assert pa.expansion_stopping_decision(history) == "STOP_CUMULATIVE_TARGET_COUNT"


def test_expansion_stops_after_three_consecutive_passing_batches_below_cumulative_cap():
    passing_batch = {"derived_pr_count": 10, "substantive_target_count": 5, "derived_target_count": 6, "adjudicable_target_count": 5}
    history = [dict(passing_batch) for _ in range(3)]
    assert sum(b["substantive_target_count"] for b in history) < pa.CUMULATIVE_SUBSTANTIVE_TARGET_STOP
    assert pa.expansion_stopping_decision(history) == "STOP_CONSECUTIVE_PASSING_BATCHES"


def test_expansion_continues_when_neither_stop_condition_met():
    weak_batch = {"derived_pr_count": 5, "substantive_target_count": 2, "derived_target_count": 6, "adjudicable_target_count": 1}
    history = [weak_batch]
    assert pa.expansion_stopping_decision(history) == "CONTINUE"


# --- meta: this test file's own compliance with "preserve blindness" ---

def _source_excluding_named_functions(*fn_names):
    """This file's own source, with the named functions' own bodies
    (identified by their `def <name>` header line up to the next
    top-level `def`/EOF) removed -- lets a meta-test name the exact
    pattern it forbids without that naming making the check trip over
    itself. Every OTHER function's real code is still fully scanned.
    """
    lines = Path(__file__).read_text().splitlines()
    out = []
    skipping = False
    for line in lines:
        if any(line.startswith(f"def {name}(") for name in fn_names):
            skipping = True
            continue
        if skipping and line.startswith("def "):
            skipping = False
        if not skipping:
            out.append(line)
    return "\n".join(out)


def test_no_real_network_calls():
    src = _source_excluding_named_functions(
        "test_no_real_network_calls", "_source_excluding_named_functions"
    )
    forbidden = ("import subprocess", "subprocess.run(", "subprocess.Popen(", "os.system(")
    assert not any(pat in src for pat in forbidden), "this test file must never shell out to gh/network itself"


def test_no_s6_candidate_window_dates_used():
    # Checks for the real S6 candidate window's own dates used as an
    # actual `datetime(...)` construction, not this function's own
    # necessary mention of the pattern it forbids, and not the module
    # docstring's own factual reference to when v0.5.0 was released
    # (a fact about the ALREADY-PUBLISHED release, not the future
    # candidate population -- see preregistration.md section 3).
    src = _source_excluding_named_functions("test_no_s6_candidate_window_dates_used")
    forbidden = ("datetime(2026, 9,", "datetime(2026,9,", "datetime(2026, 10,", "datetime(2026,10,")
    assert not any(pat in src for pat in forbidden), (
        "no real S6 candidate-window date may be constructed in this test file"
    )


if __name__ == "__main__":
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failures = 0
    for t in tests:
        try:
            t()
            print(f"PASS  {t.__name__}")
        except AssertionError as e:
            failures += 1
            print(f"FAIL  {t.__name__}: {e}")
        except Exception as e:  # noqa: BLE001 -- surface any unexpected error as a failure, not a crash
            failures += 1
            print(f"ERROR {t.__name__}: {type(e).__name__}: {e}")
    print(f"\n{len(tests) - failures}/{len(tests)} passed")
    if failures:
        raise SystemExit(1)
