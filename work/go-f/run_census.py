#!/usr/bin/env python3
"""GO-F Phase F2: fresh corpus census, reusing fixtures/s6-r1/
population-and-sampling.py's frozen CENSUS_CAP=750/NC1_THRESHOLD=20
mechanism verbatim, with exactly one addition: the GO-F exclusion set
is subtracted from the merged-PR population before systematic_sample
ever runs (reports/go-f-prereg.md Phase F1.2).

NOT EXECUTED TO COMPLETION as of this commit. The preregistered
WINDOW_INCREMENT_DAYS=5 window cannot mature within one continuous
session (only ~7 minutes had elapsed since the LOWER_BOUND anchor at
the time this was written) -- see reports/go-f-evaluation.md's own
"Phase F2 status" section for the honest account of this blocker and
why no shortcut was taken.
"""
import importlib.util
import json
import time
from datetime import datetime, timedelta, timezone

spec = importlib.util.spec_from_file_location(
    "population_and_sampling_r1",
    "/home/tandem/nix-option-branch-audit/fixtures/s6-r1/population-and-sampling.py",
)
ps1 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ps1)

EXCLUDED = set(json.load(open("/home/tandem/nix-option-branch-audit/work/go-f/exclusion-set.json"))["excluded_pr_numbers"])

LOWER_BOUND = datetime(2026, 10, 7, 11, 22, 3, tzinfo=timezone.utc)
WINDOW_INCREMENT_DAYS = 5
WINDOW_HARD_CAP_DAYS = 30


def throttled_page_fetch_fn(query, page, per_page):
    time.sleep(1.2)
    return ps1._gh_api_search_page(query, page, per_page)


def real_search_fn(query):
    return ps1.default_paginated_search(query, page_fetch_fn=throttled_page_fetch_fn, sleep_fn=None)


log = []


def logging_changed_files_fn(pr_number):
    files = ps1.fetch_changed_files(pr_number)
    classification = ps1.obviously_irrelevant_or_potentially_relevant(files)
    log.append({"pr_number": pr_number, "n_files": len(files), "classification": classification})
    return files


def exclusion_filtered_census(lower, upper):
    now = ps1._default_now()
    if now < upper:
        raise ps1.WindowNotMatured(now, upper)
    merged = ps1.fetch_merged_prs_in_window(lower, upper, search_fn=real_search_fn)
    before = len(merged)
    merged = [r for r in merged if r["number"] not in EXCLUDED]
    after = len(merged)
    inspected = ps1.systematic_sample(merged, ps1.CENSUS_CAP)
    relevant = []
    incomplete = []
    for r in inspected:
        try:
            files = logging_changed_files_fn(r["number"])
        except ps1.ChangedFileFetchIncomplete:
            incomplete.append(r["number"])
            continue
        if ps1.obviously_irrelevant_or_potentially_relevant(files) == "potentially_relevant":
            relevant.append(r)
    if incomplete:
        raise ps1.WindowEvaluationIncomplete(incomplete)
    return before, after, len(inspected), relevant


def main():
    upper = LOWER_BOUND + timedelta(days=WINDOW_INCREMENT_DAYS)
    attempts = []
    while True:
        started = ps1._default_now().isoformat()
        before, after, n_inspected, relevant = exclusion_filtered_census(LOWER_BOUND, upper)
        attempts.append({
            "lower": LOWER_BOUND.isoformat(), "upper": upper.isoformat(),
            "raw_population_before_exclusion": before,
            "population_after_exclusion": after,
            "n_inspected": n_inspected,
            "n_relevant": len(relevant),
            "relevant_pr_numbers": sorted(r["number"] for r in relevant),
            "started": started, "finished": ps1._default_now().isoformat(),
        })
        print(json.dumps(attempts[-1], indent=2))
        if len(relevant) >= ps1.NC1_THRESHOLD:
            result = {"outcome": "NC1_PASS", "attempts": attempts, "relevant_pool": relevant, "inspection_log": log}
            break
        if (upper - LOWER_BOUND).days >= WINDOW_HARD_CAP_DAYS:
            result = {"outcome": "NC1_KILL_STOP_LOW_YIELD", "attempts": attempts, "inspection_log": log}
            break
        upper = upper + timedelta(days=WINDOW_INCREMENT_DAYS)

    with open("/home/tandem/nix-option-branch-audit/work/go-f/census-result.json", "w") as f:
        json.dump(result, f, indent=2, default=str)
    print("FINAL:", result["outcome"])


if __name__ == "__main__":
    main()
