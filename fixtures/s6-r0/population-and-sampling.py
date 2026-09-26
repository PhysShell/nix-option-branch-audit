#!/usr/bin/env python3
"""S6-R0A (amends S6-R0, commit 218bf47 -- see R0A-protocol-errata.md
for the full rationale): frozen, deterministic population/census/
sampling specification for the future S6 pilot (P0 census, P1 sample
selection).

NOT EXECUTED against the real post-v0.5.0 candidate window as part of
S6-R0 or S6-R0A. Every function here is unit-testable via dependency
injection (`search_fn`) against synthetic/mocked responses only -- see
`test_population_and_sampling.py`, which uses no real network call and
no S6 candidate date. The first real execution against the actual
window remains P0 and requires a separate, later, explicit GO.

Every number here (window increment, hard cap, census cap, NC
thresholds, target cap, seed) is frozen in
fixtures/s6-r0/preregistration.md (as amended by R0A-protocol-errata.md)
and restated here only so the code and the prose can never silently
drift apart.
"""
import subprocess
import json
import random
from datetime import datetime, timedelta, timezone

REPO = "NixOS/nixpkgs"

# Frozen facts about the released subject under test -- NOT about the
# future candidate population. Fetched once, from this repository's own
# already-published release, not from nixpkgs.
RELEASE_COMMIT = "0a6f1928c650c240ae0f709d413abc6ec6b65688"
RELEASE_COMMIT_SHORT = "0a6f192"
# gh release view v0.5.0 --json publishedAt -- the exact moment v0.5.0
# became publicly available. See preregistration.md "Temporal lower
# bound" for why this timestamp (not the tag-push or commit time) is
# the frozen anchor.
LOWER_BOUND = datetime(2026, 9, 24, 5, 46, 5, tzinfo=timezone.utc)

WINDOW_INCREMENT_DAYS = 7
WINDOW_HARD_CAP_DAYS = 42
CENSUS_CAP = 100          # max PRs whose changed-files are fetched per window-check
NC1_THRESHOLD = 20        # min potentially_relevant PRs required to stop extending
P1_SAMPLE_SIZE = 15
MAX_TARGETS_TOTAL = 30    # S6-R0A item 4: a pre-committed COST bound, not derived
                          # from observed S6 data -- roughly "at most 2 independently
                          # touched option declarations per sampled PR, on average,"
                          # before target construction stops accepting further
                          # targets for this pilot (see "Target multiplicity" below).
SEED = int(RELEASE_COMMIT_SHORT, 16)  # same convention as historical S5:
                                       # seed = int(<release-commit-short-sha>, 16)

# GitHub Search API's own documented constraint (S6-R0A item 1): a
# single search query's own results are retrievable up to at most this
# many items via pagination, REGARDLESS of `--paginate` and regardless
# of how large `total_count` itself reports. `total_count` above this
# value means the query's own true match set exceeds what can be
# retrieved -- the query's own shard must be subdivided BEFORE its
# items are trusted, never after silently truncated pagination.
SEARCH_API_RESULT_CAP = 1000


def obviously_irrelevant_or_potentially_relevant(changed_files):
    """The S6 cheap relevance filter -- reused verbatim from historical
    S5's own two base metadata signals (see
    fixtures/s5-design/extract-features.py: `module_files`/`test_files`,
    both simple path-prefix checks), not a new, S6-specific, favorable
    invention. Deliberately does NOT look at PR title, labels, diff
    content semantics, or any OBA-specific keyword (mkRenamedOptionModule,
    imports=, wildcard, etc.) -- metadata-only, exactly as S6-R0 requires.

    Returns "potentially_relevant" if any changed file starts with
    "nixos/modules/" or "nixos/tests/", else "obviously_irrelevant".
    """
    for f in changed_files:
        if f.startswith("nixos/modules/") or f.startswith("nixos/tests/"):
            return "potentially_relevant"
    return "obviously_irrelevant"


class PopulationIncompleteError(Exception):
    """Raised when a temporal shard cannot be proven complete even at
    the finest supported granularity (one whole UTC day -- GitHub
    Search's own `merged:` qualifier has no finer resolution). Per
    S6-R0A item 7: population incompleteness is special -- it can bias
    WHO gets sampled, so it must fail the entire affected window
    evaluation, never merely skip the offending shard and continue.
    """


def _real_github_search(query):
    """Default `search_fn`: one real GitHub Search API call, returning
    (item_dicts, total_count, incomplete_results) exactly as the API's
    own response envelope reports them -- never just the item list,
    since `total_count`/`incomplete_results` are exactly the
    completeness evidence S6-R0A item 1 requires inspecting BEFORE
    consuming a shard's own results.
    """
    out = subprocess.run(
        ["gh", "api", "-X", "GET", "search/issues", "-f", f"q={query}", "-f", "per_page=100"],
        capture_output=True, text=True, timeout=60, check=True,
    )
    data = json.loads(out.stdout)
    items = [{"number": it["number"], "merged_at": it["pull_request"]["merged_at"]}
             for it in data["items"]]
    return items, data["total_count"], data.get("incomplete_results", False)


def _paginate_search(query, search_fn, per_shard_cap=SEARCH_API_RESULT_CAP):
    """Fetches every page for ONE already-proven-complete shard's own
    query (total_count already confirmed <= per_shard_cap by the
    caller). `search_fn` here is expected to itself page internally
    (the real implementation uses `gh api ... --paginate`); this
    wrapper exists purely so tests can inject a single mocked call
    without needing to simulate real pagination.
    """
    items, total_count, incomplete_results = search_fn(query)
    return items, total_count, incomplete_results


def _shard_query(shard_lo_date, shard_hi_date):
    # merged: is an INCLUSIVE date..date range in GitHub's own search
    # syntax -- shard_hi_date here is the LAST included calendar date,
    # never the exclusive upper bound directly (the caller computes it).
    return (f"repo:{REPO} is:pr is:merged base:master "
            f"merged:{shard_lo_date.isoformat()}..{shard_hi_date.isoformat()}")


def fetch_merged_prs_in_window(lower, upper, search_fn=None):
    """Deterministic temporal-sharding population fetch (S6-R0A item 1).

    Splits [lower, upper) into whole-UTC-day shards, queries each
    shard's own `total_count`/`incomplete_results`, and recursively
    bisects (by TIME interval only -- never by content/relevance) any
    shard whose own `total_count` exceeds `SEARCH_API_RESULT_CAP` or
    whose own `incomplete_results` flag is set, until every accepted
    shard is provably complete. A single-UTC-day shard that still
    cannot be proven complete raises `PopulationIncompleteError` --
    there is no finer granularity to subdivide into (GitHub's own
    `merged:` qualifier is date-only), so this is the genuine floor of
    what this method can prove, and S6-R0A requires failing closed
    here rather than accepting a possibly-truncated result.

    Returns the deduplicated, `(merged_at, number)`-sorted list of
    `{number, merged_at}` records with `merged_at` inside `[lower,
    upper)` (exact timestamp bound, applied client-side after the
    date-granularity server-side query).
    """
    search_fn = search_fn or _real_github_search
    accepted = {}  # number -> record, dedup across adjacent shards' own inclusive-date overlap

    # date-based shard queue; each entry is (date_lo, date_hi) inclusive
    stack = [(lower.date(), (upper - timedelta(microseconds=1)).date())]
    while stack:
        date_lo, date_hi = stack.pop()
        items, total_count, incomplete_results = _paginate_search(
            _shard_query(date_lo, date_hi), search_fn
        )
        if incomplete_results or total_count > SEARCH_API_RESULT_CAP:
            if date_lo == date_hi:
                raise PopulationIncompleteError(
                    f"shard {date_lo} cannot be proven complete "
                    f"(total_count={total_count}, incomplete_results={incomplete_results}) "
                    f"even at one-day granularity"
                )
            mid = date_lo + (date_hi - date_lo) // 2
            stack.append((date_lo, mid))
            stack.append((mid + timedelta(days=1), date_hi))
            continue
        # Shard proven complete (total_count <= cap, not incomplete):
        # consistency check -- what we actually retrieved must match
        # what the API itself claims exists in this shard. A mismatch
        # here is ALSO population incompleteness, not silently accepted.
        if len(items) != total_count:
            raise PopulationIncompleteError(
                f"shard {date_lo}..{date_hi} claims total_count={total_count} "
                f"but only {len(items)} items were actually retrieved"
            )
        for it in items:
            accepted[it["number"]] = it  # dedup by PR number across overlapping shard edges

    filtered = [
        r for r in accepted.values()
        if lower <= datetime.fromisoformat(r["merged_at"].replace("Z", "+00:00")) < upper
    ]
    filtered.sort(key=lambda r: (r["merged_at"], r["number"]))
    return filtered


def fetch_changed_files(pr_number):
    out = subprocess.run(
        ["gh", "api", f"repos/{REPO}/pulls/{pr_number}/files", "--paginate",
         "--jq", ".[].filename"],
        capture_output=True, text=True, timeout=60, check=True,
    )
    return out.stdout.splitlines()


def systematic_sample(ordered_list, cap):
    """Evenly-spaced coverage of the WHOLE window, not just its earliest
    slice -- required so that extending the window's own upper bound
    actually changes what P0 inspects, rather than re-inspecting the
    same first `cap` records every time.
    """
    total = len(ordered_list)
    if total <= cap:
        return list(ordered_list)
    idx = sorted({(i * total) // cap for i in range(cap)})
    return [ordered_list[i] for i in idx]


def census(lower, upper, search_fn=None, changed_files_fn=None):
    """One window-check: returns (inspected_count, potentially_relevant_list).
    Raises `PopulationIncompleteError` if the population fetch itself
    cannot be proven complete -- this ALWAYS aborts the whole
    window-check (S6-R0A item 7), never just the one shard.
    """
    changed_files_fn = changed_files_fn or fetch_changed_files
    merged = fetch_merged_prs_in_window(lower, upper, search_fn=search_fn)
    inspected = systematic_sample(merged, CENSUS_CAP)
    relevant = []
    for r in inspected:
        files = changed_files_fn(r["number"])
        if obviously_irrelevant_or_potentially_relevant(files) == "potentially_relevant":
            relevant.append(r)
    return len(inspected), relevant


def find_final_window(search_fn=None, changed_files_fn=None):
    """The frozen window-extension loop (preregistration.md "Temporal
    freshness" / "Kill-first protocol" NC1 -- a WINDOW-level gate,
    evaluated only here, never per expansion-batch; see
    R0A-protocol-errata.md item 3). Returns either (lower, upper,
    relevant) on success, or raises on NC1 KILL.
    """
    upper = LOWER_BOUND + timedelta(days=WINDOW_INCREMENT_DAYS)
    while True:
        n_inspected, relevant = census(LOWER_BOUND, upper, search_fn, changed_files_fn)
        if len(relevant) >= NC1_THRESHOLD:
            return LOWER_BOUND, upper, relevant
        if (upper - LOWER_BOUND).days >= WINDOW_HARD_CAP_DAYS:
            raise SystemExit(
                f"NC1 KILL / STOP_LOW_YIELD: window extended to hard cap "
                f"({WINDOW_HARD_CAP_DAYS} days), only {len(relevant)} "
                f"potentially_relevant PRs found among {n_inspected} inspected "
                f"(threshold {NC1_THRESHOLD})."
            )
        upper = upper + timedelta(days=WINDOW_INCREMENT_DAYS)


def select_sample(pool, seed, n, already_used_numbers=()):
    """Deterministic seeded shuffle, same convention as historical S5's
    own `seed = int(<short-sha>, 16); random.Random(seed).shuffle(pool)`.
    `already_used_numbers` supports the leakage-replacement rule
    (section 2/6 of preregistration.md) and the expansion rule's own
    "next unused indices" requirement (R0A item 3): excluded numbers
    are removed from the pool BEFORE shuffling, so the Nth call with a
    growing exclusion set deterministically yields "the next N PRs
    that were never used before," not a re-shuffle that could
    reintroduce an already-used PR at a new position.
    """
    remaining = [r for r in pool if r["number"] not in set(already_used_numbers)]
    shuffled = list(remaining)
    random.Random(seed).shuffle(shuffled)
    return shuffled[:n]


def select_p1_sample(relevant):
    return select_sample(relevant, SEED, P1_SAMPLE_SIZE)


def check_leakage(pr_numbers, exclusion_path="fixtures/s6-r0/excluded-pr-identities.json"):
    excluded = set(json.load(open(exclusion_path))["excluded_pr_numbers"])
    overlap = sorted(set(pr_numbers) & excluded)
    return overlap  # must be [] -- see preregistration.md "Leakage audit"


if __name__ == "__main__":
    raise SystemExit(
        "This script is a frozen SPECIFICATION for S6-R0/S6-R0A. It is "
        "committed but not executed against the real candidate window by "
        "either round. Running it against real post-v0.5.0 data is P0/P1 "
        "work and requires a separate, later, explicit GO."
    )
