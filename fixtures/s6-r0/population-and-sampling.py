#!/usr/bin/env python3
"""S6-R0D (amends S6-R0C, commit 62bdec7, which amends S6-R0B, commit
ccdfbe9, which amends S6-R0A, commit 0cb69b0, which amends S6-R0,
commit 218bf47 -- see R0D-window-increment-amendment.md for the full
rationale): frozen, deterministic population/census/sampling
specification for the future S6 pilot (P0 census, P1 sample selection),
with a correctly PAGINATING real search implementation,
bounded/observable retry semantics, a temporal-window MATURITY guard (a
window is never queried before its own exclusive upper bound has
actually elapsed), a changed-file COMPLETENESS guard (GitHub's own
`/pulls/{n}/files` endpoint cannot represent more than 3000 files; the
PR's own authoritative `changed_files` metadata count is cross-checked
before any file list is trusted), and a cheaper initial P0 kill-gate
window (3 days, not 7 -- P0 only needs to answer one cheap density
question, and 3 days is a deliberately chosen middle ground between a
single, potentially unrepresentative day and an overpriced week).

NOT EXECUTED against the real post-v0.5.0 candidate window as part of
S6-R0/S6-R0A/S6-R0B/S6-R0C/S6-R0D. Every function here is unit-testable
via dependency injection (`search_fn`, `page_fetch_fn`, `raw_fetch_fn`,
`metadata_fetch_fn`, `sleep_fn`, `now_fn`) against synthetic/mocked
responses only -- see `test_population_and_sampling.py`, which uses no
real network call and no S6 candidate date. The first real execution
against the actual window remains P0 and requires a separate, later,
explicit GO -- and cannot succeed before the frozen window's own upper
bound has actually elapsed, enforced mechanically by this file's own
maturity guard, not merely by operator discipline.

Every number here (window increment, hard cap, census cap, NC
thresholds, target cap, seed, MAX_FETCH_ATTEMPTS,
GITHUB_PR_FILES_HARD_CAP) is frozen in fixtures/s6-r0/preregistration.md
as amended by R0A-protocol-errata.md, R0B-execution-tooling-errata.md,
R0C-final-pre-P0-guards.md, and R0D-window-increment-amendment.md, and
restated here only so the code and
the prose can never silently drift apart.
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

WINDOW_INCREMENT_DAYS = 3  # S6-R0D: was 7 -- P0 only needs to answer one
                            # cheap question ("is there enough potentially
                            # relevant volume in the fresh stream?"); 7 days
                            # overpays for that first kill-gate, 1 day risks
                            # landing in an unrepresentative slice, 3 is the
                            # deliberately chosen middle -- see
                            # R0D-window-increment-amendment.md. Same clean
                            # arithmetic extension shape as before (3 -> +3
                            # -> +3 -> ...), just a smaller unit.
WINDOW_HARD_CAP_DAYS = 42  # unchanged -- the overall "give up" ceiling is
                            # untouched by S6-R0D; only the per-check
                            # granularity got cheaper (14 three-day steps
                            # instead of 6 seven-day steps to reach it).
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

# GitHub Search API's own documented constraint: a single search
# query's own results are retrievable up to at most this many items via
# pagination, REGARDLESS of how large `total_count` itself reports.
# `total_count` above this value means the query's own true match set
# exceeds what can be retrieved -- the query's own shard must be
# subdivided BEFORE its items are trusted, never after silently
# truncated pagination.
SEARCH_API_RESULT_CAP = 1000
SEARCH_PAGE_SIZE = 100

# S6-R0B item 5: frozen, bounded retry budget for every real network
# call this tooling makes. No unbounded retry loop anywhere.
MAX_FETCH_ATTEMPTS = 3

# S6-R0C item 3: GitHub's own documented hard ceiling on how many files
# `GET /repos/{owner}/{repo}/pulls/{pull_number}/files` can represent,
# REGARDLESS of pagination -- a PR with more changed files than this
# cannot have its full file list proven complete via that endpoint at
# all (unlike the Search API's own 1000-result cap, this one cannot be
# worked around by subdividing the QUERY -- there is only one PR, its
# own file count is what it is). The PR resource's own top-level
# `changed_files` field is a SEPARATE, independently-computed diff-stat
# integer, not itself subject to this same 3000-item listing cap, which
# is exactly why comparing "reported changed_files" against "retrieved
# file count" is a valid authoritative completeness check.
GITHUB_PR_FILES_HARD_CAP = 3000


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


def with_retry(fn, max_attempts=MAX_FETCH_ATTEMPTS, sleep_fn=None):
    """S6-R0B item 5: the one, shared, bounded-retry primitive every
    real network call in this file goes through. Calls `fn()` up to
    `max_attempts` times, returning on first success; re-raises the
    LAST exception once attempts are exhausted. `sleep_fn(attempt)` is
    an optional injectable backoff hook (a real production wiring MAY
    pass real GitHub rate-limit-aware backoff here later; every test in
    this round passes `None`, so no test ever sleeps in real time).
    Attempt count is observable indirectly: a mock's own call count,
    asserted by the caller after the fact.
    """
    last_exc = None
    for attempt in range(1, max_attempts + 1):
        try:
            return fn()
        except Exception as e:  # noqa: BLE001 -- any failure is a retry candidate here
            last_exc = e
            if sleep_fn and attempt < max_attempts:
                sleep_fn(attempt)
    raise last_exc


class PopulationIncompleteError(Exception):
    """Raised when a temporal shard cannot be proven complete even at
    the finest supported granularity (one whole UTC day -- GitHub
    Search's own `merged:` qualifier has no finer resolution), OR when
    a population query's own retry budget is exhausted (see
    `PopulationQueryIncomplete`, below). Per S6-R0A item 7: population
    incompleteness is special -- it can bias WHO gets sampled, so it
    must fail the entire affected window evaluation, never merely skip
    the offending shard and continue.
    """


class PopulationQueryIncomplete(PopulationIncompleteError):
    """S6-R0B item 3: a population query (first-page metadata fetch, or
    a later page of an already-proven-bounded shard) failed all
    `MAX_FETCH_ATTEMPTS` retries -- a DIFFERENT failure mode from "this
    shard is too large to trust" (`PopulationIncompleteError`'s own
    day-granularity-floor case): here the population's own true size is
    simply UNKNOWN due to a transient fetch failure, not known-and-too-
    large. Both fail the whole window evaluation identically -- this
    subclass exists only to carry a more specific, correctly-labeled
    cause, not to be handled differently by any caller.
    """


class ChangedFileFetchIncomplete(Exception):
    """S6-R0B item 3: a single PR's own changed-file list could not be
    retrieved after `MAX_FETCH_ATTEMPTS` attempts. PER-PR, not
    window-wide -- the caller (`census`) is responsible for recording
    this PR's own number rather than silently dropping it, and per
    item 3's own explicit instruction, must NEVER reclassify it as
    "obviously_irrelevant" (unknown is not the same fact as irrelevant).
    """
    def __init__(self, pr_number, cause):
        self.pr_number = pr_number
        self.cause = cause
        super().__init__(f"changed_file_fetch_incomplete: PR {pr_number}: {cause}")


class WindowEvaluationIncomplete(Exception):
    """S6-R0B item 4: at least one systematically-sampled P0 census
    record's own changed-file fetch failed all retries. An unknown
    relevance classification must never silently count as irrelevant
    (which would just be absorbed into a possibly-too-low NC1 count)
    and must never be treated as a genuine low-density NC1 result
    (which would incorrectly trigger window EXTENSION -- extension is
    for observed low relevance density, never for missing
    classification data). This window-check is itself incomplete and
    must be reported as such, distinctly from both `PASS NC1` and
    `STOP_LOW_YIELD`.
    """
    def __init__(self, incomplete_pr_numbers):
        self.incomplete_pr_numbers = list(incomplete_pr_numbers)
        super().__init__(
            f"window_evaluation_incomplete: changed-file fetch failed for "
            f"{len(self.incomplete_pr_numbers)} census record(s) after "
            f"{MAX_FETCH_ATTEMPTS} attempts each: {self.incomplete_pr_numbers}"
        )


class WindowNotMatured(Exception):
    """S6-R0C item 1-2: raised BEFORE any GitHub candidate query is
    performed (population or otherwise) when `now_utc < upper` for the
    window under evaluation -- the frozen window's own population does
    not fully exist yet; querying it now would silently measure a
    truncated, still-in-progress population and call it "the frozen
    window," which is a different, unauthorized experiment. Applies
    identically to the initial window AND to every later NC1 extension
    (`find_final_window`'s own loop calls `census` again for each
    extended `upper`, and this same guard fires again there -- no
    separate extension-specific logic is needed). Distinct from both
    `STOP_LOW_YIELD` (a genuine low-density result) and
    `window_evaluation_incomplete` (missing classification data for an
    already-fully-existing population) -- an immature window is neither
    of those; it simply cannot be evaluated yet.
    """
    def __init__(self, now_utc, required_upper_bound):
        self.now_utc = now_utc
        self.required_upper_bound = required_upper_bound
        super().__init__(
            f"window_not_matured: now={now_utc.isoformat()} < required upper "
            f"bound {required_upper_bound.isoformat()} -- no GitHub candidate "
            f"query was performed"
        )


def _default_now():
    return datetime.now(timezone.utc)


def _shard_query(shard_lo_date, shard_hi_date):
    # merged: is an INCLUSIVE date..date range in GitHub's own search
    # syntax -- shard_hi_date here is the LAST included calendar date,
    # never the exclusive upper bound directly (the caller computes it).
    return (f"repo:{REPO} is:pr is:merged base:master "
            f"merged:{shard_lo_date.isoformat()}..{shard_hi_date.isoformat()}")


# --- real (production) primitives -- each independently injectable ----

def _gh_api_search_page(query, page, per_page):
    """One real HTTP call: page `page` (1-indexed) of `query`, `per_page`
    items per page. Returns the raw parsed JSON response body.
    """
    out = subprocess.run(
        ["gh", "api", "-X", "GET", "search/issues",
         "-f", f"q={query}", "-f", f"page={page}", "-f", f"per_page={per_page}"],
        capture_output=True, text=True, timeout=60, check=True,
    )
    return json.loads(out.stdout)


def _raw_fetch_changed_files(pr_number):
    out = subprocess.run(
        ["gh", "api", f"repos/{REPO}/pulls/{pr_number}/files", "--paginate",
         "--jq", ".[].filename"],
        capture_output=True, text=True, timeout=60, check=True,
    )
    return out.stdout.splitlines()


def _raw_fetch_pr_changed_files_count(pr_number):
    """The PR resource's OWN top-level `changed_files` field (`GET
    /repos/{owner}/{repo}/pulls/{pull_number}`) -- an independently
    computed diff-stat integer, not the `/files` LISTING endpoint's own
    output, and not subject to that endpoint's own 3000-item cap. This
    is the authoritative count `fetch_changed_files` cross-checks the
    actually-retrieved file list against, per S6-R0C item 3.
    """
    out = subprocess.run(
        ["gh", "api", f"repos/{REPO}/pulls/{pr_number}", "--jq", ".changed_files"],
        capture_output=True, text=True, timeout=60, check=True,
    )
    return int(out.stdout.strip())


def _search_page(query, page, page_fetch_fn=None):
    """One page, normalized to `(items, total_count, incomplete_results)`
    -- the shared shape both the metadata-only first-page check and the
    full-retrieval loop below consume identically.
    """
    page_fetch_fn = page_fetch_fn or _gh_api_search_page
    raw = page_fetch_fn(query, page, SEARCH_PAGE_SIZE)
    items = [{"number": it["number"], "merged_at": it["pull_request"]["merged_at"]}
             for it in raw["items"]]
    return items, raw["total_count"], raw.get("incomplete_results", False)


def default_paginated_search(query, page_fetch_fn=None, sleep_fn=None):
    """S6-R0B items 1-2: the REAL default `search_fn`. Two-phase:

    Phase 1 (cheap): fetch page 1 only, to learn `total_count`/
    `incomplete_results` BEFORE deciding whether full retrieval is even
    safe -- never downloads pages 2..N merely to discover the shard is
    oversized (item 2's own cost requirement).

    Phase 2 (only for a shard already proven `total_count <= cap` and
    not `incomplete_results`): retrieve every remaining page. Each
    individual page fetch goes through the same bounded `with_retry`
    budget; exhausting it on ANY page raises `PopulationQueryIncomplete`
    (a `PopulationIncompleteError` subclass), which the caller
    (`fetch_merged_prs_in_window`) does not catch -- it propagates and
    fails the whole window evaluation, exactly like the existing
    day-granularity-floor case.

    Returns `(items, total_count, incomplete_results)` -- for an
    oversized/incomplete shard, `items` from phase 1 alone is returned
    (the caller's own bisection logic never consumes it in that case,
    only `total_count`/`incomplete_results`, so returning a partial list
    here is harmless and avoids wasted phase-2 fetches).
    """
    try:
        items_p1, total_count, incomplete_results = with_retry(
            lambda: _search_page(query, 1, page_fetch_fn), sleep_fn=sleep_fn
        )
    except Exception as e:
        raise PopulationQueryIncomplete(
            f"query {query!r}: page 1 fetch failed after {MAX_FETCH_ATTEMPTS} attempts: {e}"
        )

    if incomplete_results or total_count > SEARCH_API_RESULT_CAP:
        return items_p1, total_count, incomplete_results

    all_items = list(items_p1)
    total_pages = -(-total_count // SEARCH_PAGE_SIZE)  # ceil division
    for page in range(2, total_pages + 1):
        try:
            page_items, _, page_incomplete = with_retry(
                lambda page=page: _search_page(query, page, page_fetch_fn), sleep_fn=sleep_fn
            )
        except Exception as e:
            raise PopulationQueryIncomplete(
                f"query {query!r}: page {page} fetch failed after {MAX_FETCH_ATTEMPTS} attempts: {e}"
            )
        if page_incomplete:
            raise PopulationQueryIncomplete(
                f"query {query!r}: page {page} itself reported incomplete_results"
            )
        all_items.extend(page_items)

    return all_items, total_count, incomplete_results


def fetch_merged_prs_in_window(lower, upper, search_fn=None, sleep_fn=None):
    """Deterministic temporal-sharding population fetch.

    Splits [lower, upper) into whole-UTC-day shards, queries each
    shard's own `total_count`/`incomplete_results` (via `search_fn`,
    defaulting to `default_paginated_search`), and recursively bisects
    (by TIME interval only -- never by content/relevance) any shard
    whose own `total_count` exceeds `SEARCH_API_RESULT_CAP` or whose own
    `incomplete_results` flag is set, until every accepted shard is
    provably complete. A single-UTC-day shard that still cannot be
    proven complete raises `PopulationIncompleteError` -- there is no
    finer granularity to subdivide into (GitHub's own `merged:`
    qualifier is date-only), so this is the genuine floor of what this
    method can prove.

    Returns the deduplicated, `(merged_at, number)`-sorted list of
    `{number, merged_at}` records with `merged_at` inside `[lower,
    upper)` (exact timestamp bound, applied client-side after the
    date-granularity server-side query).
    """
    search_fn = search_fn or (lambda q: default_paginated_search(q, sleep_fn=sleep_fn))
    accepted = {}  # number -> record, dedup across adjacent shards' own inclusive-date overlap

    stack = [(lower.date(), (upper - timedelta(microseconds=1)).date())]
    while stack:
        date_lo, date_hi = stack.pop()
        items, total_count, incomplete_results = search_fn(_shard_query(date_lo, date_hi))
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
        # Dedup BEFORE comparing to total_count -- a duplicate item
        # returned twice within a shard's own pagination is a distinct,
        # real possibility to guard against, not merely a cross-shard one.
        unique_numbers = {it["number"] for it in items}
        if len(unique_numbers) != total_count:
            raise PopulationIncompleteError(
                f"shard {date_lo}..{date_hi} claims total_count={total_count} "
                f"but only {len(unique_numbers)} unique items were actually retrieved"
            )
        for it in items:
            accepted[it["number"]] = it  # dedup by PR number across overlapping shard edges

    filtered = [
        r for r in accepted.values()
        if lower <= datetime.fromisoformat(r["merged_at"].replace("Z", "+00:00")) < upper
    ]
    filtered.sort(key=lambda r: (r["merged_at"], r["number"]))
    return filtered


def fetch_changed_files(pr_number, raw_fetch_fn=None, metadata_fetch_fn=None, sleep_fn=None):
    """Per-PR changed-file fetch, bounded-retried (`MAX_FETCH_ATTEMPTS`)
    at every step, now with S6-R0C's own completeness guard:

    1. Fetch the PR's own authoritative `changed_files` metadata count
       FIRST (cheap: one object, no pagination) -- before attempting the
       potentially-expensive file-list fetch at all.
    2. If that count exceeds `GITHUB_PR_FILES_HARD_CAP` (3000), fail
       closed immediately -- the `/files` listing endpoint literally
       cannot represent this PR's own complete file list, no amount of
       pagination fixes that, and no file-list fetch is even attempted
       (item 6's own network-cost discipline).
    3. Otherwise, fetch the actual file list (bounded-retried,
       pagination-complete per S6-R0B) and require
       `len(retrieved) == reported_count` -- any mismatch is ALSO
       incompleteness, never silently accepted.

    Raises `ChangedFileFetchIncomplete` (never a raw `subprocess`/
    network exception) for every failure mode above -- the caller
    (`census`) is responsible for recording this per-PR, not for the
    whole window.
    """
    metadata_fetch_fn = metadata_fetch_fn or _raw_fetch_pr_changed_files_count
    raw_fetch_fn = raw_fetch_fn or _raw_fetch_changed_files

    try:
        reported_count = with_retry(lambda: metadata_fetch_fn(pr_number), sleep_fn=sleep_fn)
    except Exception as e:
        raise ChangedFileFetchIncomplete(
            pr_number, f"metadata fetch failed after {MAX_FETCH_ATTEMPTS} attempts: {e}"
        )

    if reported_count > GITHUB_PR_FILES_HARD_CAP:
        raise ChangedFileFetchIncomplete(
            pr_number,
            f"reported_changed_files_exceeds_github_{GITHUB_PR_FILES_HARD_CAP}_file_api_limit "
            f"(reported_count={reported_count})",
        )

    try:
        files = with_retry(lambda: raw_fetch_fn(pr_number), sleep_fn=sleep_fn)
    except Exception as e:
        raise ChangedFileFetchIncomplete(
            pr_number, f"file list fetch failed after {MAX_FETCH_ATTEMPTS} attempts: {e}"
        )

    if len(files) != reported_count:
        raise ChangedFileFetchIncomplete(
            pr_number,
            f"retrieved_file_count({len(files)}) != reported_changed_file_count({reported_count})",
        )

    return files


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


def census(lower, upper, search_fn=None, changed_files_fn=None, sleep_fn=None, now_fn=None):
    """One window-check: returns `(inspected_count, potentially_relevant_list)`.

    Raises `WindowNotMatured` (S6-R0C items 1-2), BEFORE any network
    call of any kind, if `now_utc < upper` -- the window's own
    population does not fully exist yet.

    Raises `PopulationIncompleteError` (or its `PopulationQueryIncomplete`
    subclass) if the population fetch itself cannot be proven complete --
    this ALWAYS aborts the whole window-check.

    Raises `WindowEvaluationIncomplete` (S6-R0B item 4) if any
    systematically-sampled census record's own changed-file fetch
    exhausts its retry budget -- this ALSO always aborts the whole
    window-check (never silently treated as "irrelevant," never treated
    as a genuine NC1 low-density result that would trigger extension).
    """
    now_fn = now_fn or _default_now
    now = now_fn()
    if now < upper:
        raise WindowNotMatured(now, upper)

    changed_files_fn = changed_files_fn or (lambda n: fetch_changed_files(n, sleep_fn=sleep_fn))
    merged = fetch_merged_prs_in_window(lower, upper, search_fn=search_fn, sleep_fn=sleep_fn)
    inspected = systematic_sample(merged, CENSUS_CAP)
    relevant = []
    incomplete = []
    for r in inspected:
        try:
            files = changed_files_fn(r["number"])
        except ChangedFileFetchIncomplete:
            incomplete.append(r["number"])
            continue
        if obviously_irrelevant_or_potentially_relevant(files) == "potentially_relevant":
            relevant.append(r)
    if incomplete:
        raise WindowEvaluationIncomplete(incomplete)
    return len(inspected), relevant


def find_final_window(search_fn=None, changed_files_fn=None, sleep_fn=None, now_fn=None):
    """The frozen window-extension loop (preregistration.md "Temporal
    freshness" / "Kill-first protocol" NC1 -- a WINDOW-level gate,
    evaluated only here, never per expansion-batch; see
    R0A-protocol-errata.md item 3). Returns either (lower, upper,
    relevant) on success, raises `SystemExit` on NC1 KILL
    (`STOP_LOW_YIELD`), or propagates `WindowNotMatured` (S6-R0C)/
    `WindowEvaluationIncomplete`/`PopulationIncompleteError` UNCAUGHT --
    an immature or incomplete window-check is never silently treated as
    either a pass or a genuine low-yield extension trigger (S6-R0B item
    4, S6-R0C items 1-2). Applying the SAME per-call maturity check
    inside `census` to every extension step, with no separate
    extension-specific logic, is exactly what makes "the 14-day
    extension's own upper bound hasn't elapsed yet" surface correctly
    as `WindowNotMatured`, carrying that exact 14-day upper bound, the
    first time `census` is called with it -- never skipped ahead to.
    """
    upper = LOWER_BOUND + timedelta(days=WINDOW_INCREMENT_DAYS)
    while True:
        n_inspected, relevant = census(LOWER_BOUND, upper, search_fn, changed_files_fn, sleep_fn, now_fn)
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
    "next unused indices" requirement: excluded numbers are removed
    from the pool BEFORE shuffling, then the SAME seed re-shuffles the
    reduced pool deterministically -- a fresh, reproducible draw, not a
    promise that every other previously-selected PR keeps its exact
    prior position (see R0A-protocol-errata.md item 4's own note on
    this).
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
        "This script is a frozen SPECIFICATION for S6-R0/S6-R0A/S6-R0B/"
        "S6-R0C/S6-R0D. It is committed but not executed against the "
        "real candidate window by any of these rounds. Running it "
        "against real post-v0.5.0 data is P0/P1 work and requires a "
        "separate, later, explicit GO -- and cannot succeed before the "
        "frozen initial window's own upper bound has actually elapsed."
    )
