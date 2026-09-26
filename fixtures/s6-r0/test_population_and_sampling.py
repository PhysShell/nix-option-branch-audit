#!/usr/bin/env python3
"""S6-R0A/S6-R0B protocol-tooling regression tests. Synthetic/mocked
API responses and historical (pre-S6, arbitrary) dates ONLY -- no real
network call, no query against the actual post-v0.5.0 candidate
window. Confirms this file's own compliance is exactly the point of
S6-R0A item 2 ("Preserve blindness"): see `test_no_real_network_calls`
and `test_no_s6_candidate_window_dates_used`, below, which check THIS
FILE's own source text for exactly that.

The R0B section (search pagination correctness, bounded retry
semantics, per-PR/per-window incompleteness statuses) tests the LOWER-
LEVEL primitives (`default_paginated_search`, `with_retry`,
`fetch_changed_files`, `census`'s own incompleteness handling)
introduced to fix the two real execution bugs R0B's own errata
documents; the R0A-era tests above continue to test
`fetch_merged_prs_in_window`'s own shard-bisection logic at the
higher-level `search_fn(query) -> (items, total, incomplete)`
injection point, unchanged and still passing verbatim against the
rewritten implementation.

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


# ======================================================================
# S6-R0B: search pagination correctness + bounded retry semantics
# ======================================================================

def _mock_pages(pages_by_number, total_count, incomplete_results=False):
    """`page_fetch_fn`-shaped mock: `pages_by_number` maps page number
    (1-indexed) -> list of (pr_number, merged_at) tuples for that page.
    Returns a callable matching `_gh_api_search_page`'s own raw-JSON
    return shape, plus `.calls` (a list of page numbers actually
    requested, in order) for asserting exactly which pages were/weren't
    fetched.
    """
    calls = []

    def fn(query, page, per_page):
        calls.append(page)
        items = [{"number": n, "pull_request": {"merged_at": m}} for n, m in pages_by_number.get(page, [])]
        return {"items": items, "total_count": total_count, "incomplete_results": incomplete_results}

    fn.calls = calls
    return fn


def _mock_pages_failing_on(fail_pages, pages_by_number, total_count, fail_times=None):
    """Like `_mock_pages`, but the request for any page number in
    `fail_pages` raises an exception the FIRST `fail_times[page]` times
    it is called (default: every time, for an "always fails" page), then
    (if `fail_times` allows) succeeds normally afterward. Lets a single
    mock express "transient failure then success" and "fails all
    attempts" for the SAME kind of call.
    """
    fail_times = fail_times or {}
    call_counts = {}

    def fn(query, page, per_page):
        call_counts[page] = call_counts.get(page, 0) + 1
        limit = fail_times.get(page, float("inf")) if page in fail_pages else 0
        if call_counts[page] <= limit:
            raise RuntimeError(f"simulated transient failure on page {page}, attempt {call_counts[page]}")
        items = [{"number": n, "pull_request": {"merged_at": m}} for n, m in pages_by_number.get(page, [])]
        return {"items": items, "total_count": total_count, "incomplete_results": False}

    fn.call_counts = call_counts
    return fn


def _rows(numbers, day="2020-01-01T00:00:00Z"):
    return [(n, day) for n in numbers]


def test_total_count_zero_returns_empty_no_extra_pages():
    fn = _mock_pages({1: []}, total_count=0)
    items, total, incomplete = ps.default_paginated_search("q", page_fetch_fn=fn)
    assert items == [] and total == 0 and incomplete is False
    assert fn.calls == [1]


def test_total_count_one_single_page():
    fn = _mock_pages({1: _rows([42])}, total_count=1)
    items, total, incomplete = ps.default_paginated_search("q", page_fetch_fn=fn)
    assert [it["number"] for it in items] == [42]
    assert fn.calls == [1], "a single-item result must never request a second page"


def test_total_count_exactly_one_page_size_no_second_page_fetched():
    n = ps.SEARCH_PAGE_SIZE  # 100
    fn = _mock_pages({1: _rows(range(1, n + 1))}, total_count=n)
    items, total, incomplete = ps.default_paginated_search("q", page_fetch_fn=fn)
    assert len(items) == n
    assert fn.calls == [1], "total_count == one page's own size must not fetch a second page"


def test_total_count_one_over_page_size_fetches_second_page():
    n = ps.SEARCH_PAGE_SIZE + 1  # 101
    fn = _mock_pages({1: _rows(range(1, ps.SEARCH_PAGE_SIZE + 1)), 2: _rows([n])}, total_count=n)
    items, total, incomplete = ps.default_paginated_search("q", page_fetch_fn=fn)
    assert len(items) == n
    assert fn.calls == [1, 2]


def test_multi_page_350_fetches_four_pages_in_order():
    total = 350
    pages = {
        1: _rows(range(1, 101)), 2: _rows(range(101, 201)),
        3: _rows(range(201, 301)), 4: _rows(range(301, 351)),
    }
    fn = _mock_pages(pages, total_count=total)
    items, got_total, incomplete = ps.default_paginated_search("q", page_fetch_fn=fn)
    assert len(items) == total
    assert fn.calls == [1, 2, 3, 4]
    assert {it["number"] for it in items} == set(range(1, 351))


def test_total_count_exactly_at_cap_fetches_all_ten_pages():
    total = ps.SEARCH_API_RESULT_CAP  # 1000
    pages = {p: _rows(range((p - 1) * 100 + 1, p * 100 + 1)) for p in range(1, 11)}
    fn = _mock_pages(pages, total_count=total)
    items, got_total, incomplete = ps.default_paginated_search("q", page_fetch_fn=fn)
    assert len(items) == total
    assert fn.calls == list(range(1, 11))


def test_total_count_one_over_cap_never_fetches_a_second_page():
    # S6-R0B item 2: must not download remaining pages merely to learn
    # total_count > cap -- the whole point of the two-phase design.
    fn = _mock_pages({1: _rows(range(1, 101))}, total_count=ps.SEARCH_API_RESULT_CAP + 1)
    items, total, incomplete = ps.default_paginated_search("q", page_fetch_fn=fn)
    assert total == ps.SEARCH_API_RESULT_CAP + 1
    assert fn.calls == [1], "an oversized shard's own page 1 is fetched (to learn total_count), never page 2+"


def test_incomplete_results_on_page_one_never_fetches_further_pages():
    fn = _mock_pages({1: _rows([1])}, total_count=1, incomplete_results=True)
    items, total, incomplete = ps.default_paginated_search("q", page_fetch_fn=fn)
    assert incomplete is True
    assert fn.calls == [1]


def test_page_mismatch_is_caught_by_the_shard_level_consistency_check():
    # total_count says 350 but the pages together only produce 340
    # unique items -- a real, if different, form of incompleteness the
    # SHARD-level (fetch_merged_prs_in_window) consistency check must
    # still catch, even though default_paginated_search's own per-page
    # fetches all individually "succeeded."
    total = 350
    pages = {
        1: _rows(range(1, 101)), 2: _rows(range(101, 201)),
        3: _rows(range(201, 301)), 4: _rows(range(301, 341)),  # only 40 here, not 50
    }
    page_fn = _mock_pages(pages, total_count=total)

    def search_fn(query):
        return ps.default_paginated_search(query, page_fetch_fn=page_fn)

    try:
        ps.fetch_merged_prs_in_window(HIST_LOWER, HIST_UPPER, search_fn=search_fn)
        raise AssertionError("expected PopulationIncompleteError")
    except ps.PopulationIncompleteError:
        pass


def test_duplicate_item_within_one_shards_own_pages_is_caught():
    # PR 5 appears on both page 1 and page 2 of the SAME shard's own
    # result -- total_count=101 literal item count, but only 100
    # UNIQUE PR numbers -- must be caught, not silently deduplicated
    # into a false "complete" shard.
    pages = {1: _rows(range(1, 101)), 2: _rows([5])}  # 5 already on page 1
    page_fn = _mock_pages(pages, total_count=101)

    def search_fn(query):
        return ps.default_paginated_search(query, page_fetch_fn=page_fn)

    try:
        ps.fetch_merged_prs_in_window(HIST_LOWER, HIST_UPPER, search_fn=search_fn)
        raise AssertionError("expected PopulationIncompleteError")
    except ps.PopulationIncompleteError:
        pass


def test_later_page_failure_after_retries_raises_population_query_incomplete():
    fn = _mock_pages_failing_on(
        fail_pages={2}, pages_by_number={1: _rows(range(1, 101)), 2: _rows([101])},
        total_count=101,
    )
    try:
        ps.default_paginated_search("q", page_fetch_fn=fn)
        raise AssertionError("expected PopulationQueryIncomplete")
    except ps.PopulationQueryIncomplete:
        pass
    assert fn.call_counts[2] == ps.MAX_FETCH_ATTEMPTS


# --- retry/status behavior ---------------------------------------------

def test_population_query_transient_failure_then_success():
    fn = _mock_pages_failing_on(
        fail_pages={1}, pages_by_number={1: _rows([1])}, total_count=1, fail_times={1: 1},
    )
    items, total, incomplete = ps.default_paginated_search("q", page_fetch_fn=fn)
    assert [it["number"] for it in items] == [1]
    assert fn.call_counts[1] == 2, "must succeed on the second attempt, not retry needlessly further"


def test_population_query_fails_all_attempts_raises_population_query_incomplete():
    fn = _mock_pages_failing_on(fail_pages={1}, pages_by_number={}, total_count=1)
    try:
        ps.default_paginated_search("q", page_fetch_fn=fn)
        raise AssertionError("expected PopulationQueryIncomplete")
    except ps.PopulationQueryIncomplete:
        pass
    assert fn.call_counts[1] == ps.MAX_FETCH_ATTEMPTS


def _flaky_raw_fetch(fail_times):
    calls = []

    def fn(pr_number):
        calls.append(pr_number)
        n = len([c for c in calls if c == pr_number])
        if n <= fail_times:
            raise RuntimeError(f"simulated transient failure, attempt {n}")
        return ["nixos/modules/services/foo.nix"]

    fn.calls = calls
    return fn


def test_changed_file_transient_failure_then_success():
    fn = _flaky_raw_fetch(fail_times=1)
    files = ps.fetch_changed_files(123, raw_fetch_fn=fn, metadata_fetch_fn=lambda n: 1)
    assert files == ["nixos/modules/services/foo.nix"]
    assert fn.calls.count(123) == 2


def test_changed_file_fails_all_attempts_raises_changed_file_fetch_incomplete():
    fn = _flaky_raw_fetch(fail_times=999)  # always fails
    try:
        ps.fetch_changed_files(123, raw_fetch_fn=fn, metadata_fetch_fn=lambda n: 1)
        raise AssertionError("expected ChangedFileFetchIncomplete")
    except ps.ChangedFileFetchIncomplete as e:
        assert e.pr_number == 123
    assert fn.calls.count(123) == ps.MAX_FETCH_ATTEMPTS


def test_failed_changed_file_classification_does_not_become_obviously_irrelevant():
    # census() must raise WindowEvaluationIncomplete for this PR, never
    # silently classify it either way.
    q = ps._shard_query(HIST_LOWER.date(), (HIST_UPPER - timedelta(microseconds=1)).date())
    merged_items = _items([1, 2], ["2020-01-01T00:00:00Z", "2020-01-02T00:00:00Z"])
    search = _mock_search({q: (merged_items, 2, False)})

    def changed_files_fn(n):
        if n == 1:
            return ["nixos/modules/services/foo.nix"]  # relevant
        raise ps.ChangedFileFetchIncomplete(n, "simulated")

    try:
        ps.census(HIST_LOWER, HIST_UPPER, search_fn=search, changed_files_fn=changed_files_fn)
        raise AssertionError("expected WindowEvaluationIncomplete")
    except ps.WindowEvaluationIncomplete as e:
        assert e.incomplete_pr_numbers == [2]


def test_incomplete_census_propagates_through_find_final_window_not_extended_as_low_yield():
    # A census that raises WindowEvaluationIncomplete must propagate
    # OUT of find_final_window directly -- it must never be caught and
    # treated as "not enough relevant PRs yet, extend the window."
    # Every date here is DERIVED from ps.LOWER_BOUND itself (the
    # already-frozen constant), never a hand-typed literal S6-window
    # date -- see this file's own "preserve blindness" meta-tests.
    one_day_in = ps.LOWER_BOUND + timedelta(days=1)
    q = ps._shard_query(ps.LOWER_BOUND.date(), (ps.LOWER_BOUND + timedelta(days=ps.WINDOW_INCREMENT_DAYS) - timedelta(microseconds=1)).date())
    merged_items = _items([1], [one_day_in.isoformat().replace("+00:00", "Z")])
    search = _mock_search({q: (merged_items, 1, False)})

    def changed_files_fn(n):
        raise ps.ChangedFileFetchIncomplete(n, "simulated")

    # Matured well past even the hard-cap window, so S6-R0C's own
    # maturity guard never interferes with what THIS test is actually
    # checking (WindowEvaluationIncomplete propagation) -- maturity has
    # its own dedicated tests, below.
    matured_now = lambda: ps.LOWER_BOUND + timedelta(days=ps.WINDOW_HARD_CAP_DAYS + 1)

    try:
        ps.find_final_window(search_fn=search, changed_files_fn=changed_files_fn, now_fn=matured_now)
        raise AssertionError("expected WindowEvaluationIncomplete to propagate")
    except ps.WindowEvaluationIncomplete:
        pass
    except SystemExit:
        raise AssertionError(
            "an incomplete census must never be silently reinterpreted as a "
            "genuine NC1 low-yield result triggering window extension/KILL"
        )


# ======================================================================
# S6-R0C: temporal-window maturity guard + changed-file completeness
# ======================================================================

def _fixed_now(dt):
    return lambda: dt


# --- window maturity ----------------------------------------------------

def test_now_before_upper_is_blocked_with_zero_population_calls():
    upper = HIST_UPPER
    now_fn = _fixed_now(upper - timedelta(seconds=1))

    def search_fn(query):
        raise AssertionError("no GitHub candidate query may be performed before maturity")

    try:
        ps.census(HIST_LOWER, upper, search_fn=search_fn, now_fn=now_fn)
        raise AssertionError("expected WindowNotMatured")
    except ps.WindowNotMatured as e:
        assert e.required_upper_bound == upper


def test_now_exactly_at_upper_is_permitted():
    upper = HIST_UPPER
    now_fn = _fixed_now(upper)  # now == upper -- the exclusive bound has just elapsed
    q = ps._shard_query(HIST_LOWER.date(), (upper - timedelta(microseconds=1)).date())
    search = _mock_search({q: ([], 0, False)})
    # Must not raise WindowNotMatured -- reaching the (empty) relevant
    # list at all proves the population query was actually attempted.
    n_inspected, relevant = ps.census(HIST_LOWER, upper, search_fn=search, now_fn=now_fn)
    assert relevant == []


def test_now_after_upper_is_permitted():
    upper = HIST_UPPER
    now_fn = _fixed_now(upper + timedelta(days=1))
    q = ps._shard_query(HIST_LOWER.date(), (upper - timedelta(microseconds=1)).date())
    search = _mock_search({q: ([], 0, False)})
    n_inspected, relevant = ps.census(HIST_LOWER, upper, search_fn=search, now_fn=now_fn)
    assert relevant == []


def test_extension_window_immature_does_not_reinterpret_earlier_result_as_low_yield():
    # First (7-day) window matures and evaluates cleanly with too few
    # relevant PRs (NC1 fails); the SECOND (14-day) extension's own
    # upper bound has not matured -- must surface as WindowNotMatured
    # carrying THAT 14-day upper bound, never silently skipped past and
    # never reported as STOP_LOW_YIELD for the (already fully evaluated,
    # genuinely low-yield) first window.
    first_upper = ps.LOWER_BOUND + timedelta(days=ps.WINDOW_INCREMENT_DAYS)
    second_upper = ps.LOWER_BOUND + timedelta(days=2 * ps.WINDOW_INCREMENT_DAYS)
    now_fn = _fixed_now(first_upper + timedelta(seconds=1))  # first matured, second not

    q1 = ps._shard_query(ps.LOWER_BOUND.date(), (first_upper - timedelta(microseconds=1)).date())
    search = _mock_search({q1: ([], 0, False)})  # zero relevant -- genuinely low yield

    try:
        ps.find_final_window(search_fn=search, now_fn=now_fn)
        raise AssertionError("expected WindowNotMatured for the second (14-day) window")
    except ps.WindowNotMatured as e:
        assert e.required_upper_bound == second_upper
    except SystemExit:
        raise AssertionError(
            "the first window's own genuine low yield must never be reported as "
            "STOP_LOW_YIELD when the real reason execution stopped is that the "
            "SECOND window has not matured yet"
        )
    assert search.calls == [q1], "the second window's own query must never be attempted before maturity"


# --- changed-file completeness -------------------------------------------

def _metadata(count):
    return lambda pr_number: count


def _files(n):
    return [f"nixos/modules/services/f{i}.nix" for i in range(n)]


def test_changed_files_reported_zero_retrieved_zero():
    files = ps.fetch_changed_files(1, raw_fetch_fn=lambda n: [], metadata_fetch_fn=_metadata(0))
    assert files == []


def test_changed_files_reported_one_retrieved_one():
    files = ps.fetch_changed_files(1, raw_fetch_fn=lambda n: _files(1), metadata_fetch_fn=_metadata(1))
    assert len(files) == 1


def test_changed_files_reported_2999_retrieved_2999():
    files = ps.fetch_changed_files(1, raw_fetch_fn=lambda n: _files(2999), metadata_fetch_fn=_metadata(2999))
    assert len(files) == 2999


def test_changed_files_reported_exactly_3000_retrieved_3000():
    files = ps.fetch_changed_files(1, raw_fetch_fn=lambda n: _files(3000), metadata_fetch_fn=_metadata(3000))
    assert len(files) == 3000


def test_changed_files_reported_3001_fails_closed_without_attempting_file_list_fetch():
    def raw_fetch_fn(n):
        raise AssertionError("the /files endpoint must never be queried once reported_count exceeds the cap")

    try:
        ps.fetch_changed_files(1, raw_fetch_fn=raw_fetch_fn, metadata_fetch_fn=_metadata(3001))
        raise AssertionError("expected ChangedFileFetchIncomplete")
    except ps.ChangedFileFetchIncomplete as e:
        assert "3001" in str(e) and "exceeds_github_3000" in str(e)


def test_changed_files_count_mismatch_is_incomplete():
    files_fn = lambda n: _files(49)  # only 49 actually retrieved
    try:
        ps.fetch_changed_files(1, raw_fetch_fn=files_fn, metadata_fetch_fn=_metadata(50))
        raise AssertionError("expected ChangedFileFetchIncomplete")
    except ps.ChangedFileFetchIncomplete as e:
        assert "49" in str(e) and "50" in str(e)


def test_changed_files_metadata_transient_failure_then_success():
    calls = []

    def metadata_fetch_fn(n):
        calls.append(n)
        if len(calls) == 1:
            raise RuntimeError("simulated transient failure")
        return 1

    files = ps.fetch_changed_files(1, raw_fetch_fn=lambda n: _files(1), metadata_fetch_fn=metadata_fetch_fn)
    assert len(files) == 1
    assert len(calls) == 2


def test_changed_files_metadata_fails_all_attempts():
    def metadata_fetch_fn(n):
        raise RuntimeError("simulated permanent failure")

    try:
        ps.fetch_changed_files(1, raw_fetch_fn=lambda n: _files(1), metadata_fetch_fn=metadata_fetch_fn)
        raise AssertionError("expected ChangedFileFetchIncomplete")
    except ps.ChangedFileFetchIncomplete as e:
        assert e.pr_number == 1


def test_incomplete_file_list_case_does_not_become_obviously_irrelevant_r0c():
    # Same shape as R0B's own equivalent test, but driven through the
    # NEW completeness-guard failure mode (count mismatch) rather than a
    # raw fetch exception -- confirms census() treats EVERY
    # ChangedFileFetchIncomplete cause identically (never inspects
    # `.cause` to decide relevance).
    q = ps._shard_query(HIST_LOWER.date(), (HIST_UPPER - timedelta(microseconds=1)).date())
    merged_items = _items([1, 2], ["2020-01-01T00:00:00Z", "2020-01-02T00:00:00Z"])
    search = _mock_search({q: (merged_items, 2, False)})

    def changed_files_fn(n):
        if n == 1:
            return ["nixos/modules/services/foo.nix"]
        raise ps.ChangedFileFetchIncomplete(n, "reported_changed_files_exceeds_github_3000_file_api_limit (reported_count=5000)")

    try:
        ps.census(HIST_LOWER, HIST_UPPER, search_fn=search, changed_files_fn=changed_files_fn)
        raise AssertionError("expected WindowEvaluationIncomplete")
    except ps.WindowEvaluationIncomplete as e:
        assert e.incomplete_pr_numbers == [2]


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
    # Checks for the real S6 candidate window's own dates used either as
    # an actual `datetime(...)` construction OR as a quoted ISO date
    # string literal (the shape a hand-typed mock merged_at would take)
    # -- not this function's own necessary mention of the patterns it
    # forbids, and not the module docstring's own factual, unquoted
    # prose reference to when v0.5.0 was released (a fact about the
    # ALREADY-PUBLISHED release, not the future candidate population --
    # see preregistration.md section 3). A quoted-string check
    # specifically requires a leading quote character before the date,
    # which a prose mention like "(2026-09-24)" never has.
    src = _source_excluding_named_functions("test_no_s6_candidate_window_dates_used")
    forbidden = (
        "datetime(2026, 9,", "datetime(2026,9,", "datetime(2026, 10,", "datetime(2026,10,",
        '"2026-09-2', "'2026-09-2", '"2026-10-', "'2026-10-",
    )
    assert not any(pat in src for pat in forbidden), (
        "no real S6 candidate-window date may be constructed or quoted in this test file"
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
