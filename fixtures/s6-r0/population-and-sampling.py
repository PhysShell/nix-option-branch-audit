#!/usr/bin/env python3
"""S6-R0: frozen, deterministic population/census/sampling specification
for the future S6 pilot (P0 census, P1 sample selection).

NOT EXECUTED as part of S6-R0. This file exists so the procedure is
reviewable, reproducible, and committed BEFORE any candidate PR is
examined -- per S6-R0's own mandate: "do not execute the validation
yet... this round exists only to freeze the experiment before seeing
its results." Running this script (or an equivalent re-implementation
that produces the same result given the same inputs) is P0/P1 work,
authorized only by a separate, later, explicit GO.

Every number here (window increment, hard cap, census cap, NC1
threshold, P1 sample size, seed) is frozen in
fixtures/s6-r0/preregistration.md and restated here only so the code
and the prose can never silently drift apart.
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
SEED = int(RELEASE_COMMIT_SHORT, 16)  # same convention as historical S5:
                                       # seed = int(<release-commit-short-sha>, 16)


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


def fetch_merged_prs_in_window(lower, upper):
    """Metadata-only listing (number, merged_at, base ref) of every PR
    merged into `master` in [lower, upper). Cheap: GitHub's search API
    only supports DATE-granularity `merged:` qualifiers, so this fetches
    by whole UTC date range and then filters to the exact timestamp
    bound client-side -- a mechanical post-filter, not a judgment call.
    Does NOT fetch changed-files here (that's the separately-capped,
    more expensive step in `census`).
    """
    date_lo = lower.date().isoformat()
    date_hi = upper.date().isoformat()
    query = f"repo:{REPO} is:pr is:merged base:master merged:{date_lo}..{date_hi}"
    out = subprocess.run(
        ["gh", "api", "-X", "GET", "search/issues", "-f", f"q={query}", "--paginate",
         "--jq", ".items[] | {number, merged_at: .pull_request.merged_at}"],
        capture_output=True, text=True, timeout=120, check=True,
    )
    rows = [json.loads(l) for l in out.stdout.splitlines() if l.strip()]
    filtered = [
        r for r in rows
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


def census(lower, upper):
    """One window-check: returns (inspected_count, potentially_relevant_list)."""
    merged = fetch_merged_prs_in_window(lower, upper)
    inspected = systematic_sample(merged, CENSUS_CAP)
    relevant = []
    for r in inspected:
        files = fetch_changed_files(r["number"])
        if obviously_irrelevant_or_potentially_relevant(files) == "potentially_relevant":
            relevant.append(r)
    return len(inspected), relevant


def find_final_window():
    """The frozen window-extension loop (preregistration.md "Temporal
    freshness" / "Kill-first protocol" NC1). Returns either
    (lower, upper, relevant) on success, or raises on NC1 KILL.
    """
    upper = LOWER_BOUND + timedelta(days=WINDOW_INCREMENT_DAYS)
    while True:
        n_inspected, relevant = census(LOWER_BOUND, upper)
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


def select_p1_sample(relevant):
    """Deterministic seeded shuffle, same convention as historical S5's
    own `seed = int(<short-sha>, 16); random.Random(seed).shuffle(pool)`.
    """
    pool = list(relevant)
    random.Random(SEED).shuffle(pool)
    return pool[:P1_SAMPLE_SIZE]


def check_leakage(pr_numbers, exclusion_path="fixtures/s6-r0/excluded-pr-identities.json"):
    excluded = set(json.load(open(exclusion_path))["excluded_pr_numbers"])
    overlap = sorted(set(pr_numbers) & excluded)
    return overlap  # must be [] -- see preregistration.md "Leakage audit"


if __name__ == "__main__":
    raise SystemExit(
        "This script is a frozen SPECIFICATION for S6-R0. It is committed "
        "but not executed by this round. Running it is P0/P1 work and "
        "requires a separate, later, explicit GO."
    )
