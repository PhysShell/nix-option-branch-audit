#!/usr/bin/env python3
"""S5 population freeze: real, live fetch of the fresh candidate
population from GitHub's real API (`gh api`, already authenticated).

NOT part of CI -- this requires live network access and produces a
non-replayable snapshot (nixpkgs keeps merging PRs). The FROZEN
artifact is this run's own committed output (raw.jsonl + the recorded
fetch timestamp in provenance.json), never a re-fetch. Only
`reproduce-s5-freeze.py` (which reads committed files, no network) is
CI-safe.

Same real screen methodology as S4 (see
fixtures/s4-live-pr-shadow/screen.py's own docstring): real commits
touching nixos/modules/services/** or nixos/tests/** via GitHub's own
path-filtered commits API, real merge-window, resolved to their real
merge PRs.

Window: 2026-06-01 through 2026-09-22 (today), same window S1-S4 used
-- non-overlap with S1-S4 is guaranteed by PR-NUMBER exclusion (this
script drops already-examined/excluded numbers BEFORE fetching PR
detail), not by picking a disjoint calendar window. This window still
has substantial real survivor material S1-S4 never touched, since S4
itself only ever adjudicated 187 of a much larger real eligible
population.
"""
import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

REPO = "NixOS/nixpkgs"
WINDOW_START = "2026-06-01T00:00:00Z"
WINDOW_END = "2026-09-22T23:59:59Z"
PATHS = ["nixos/modules/services", "nixos/tests"]

ROOT = Path(__file__).resolve().parents[3]
OUT_DIR = Path(__file__).resolve().parent
S4_LEDGER_EXCLUSION = ROOT / "fixtures/s4-live-pr-shadow/exclusion-ledger.json"
S5_ADDENDUM = OUT_DIR / "s5-exclusion-addendum.json"


def gh_api_paginate(path_query: str, attempts: int = 4) -> list:
    for attempt in range(attempts):
        proc = subprocess.run(
            ["gh", "api", path_query, "--paginate", "--slurp"],
            capture_output=True, text=True,
        )
        if proc.returncode == 0:
            pages = json.loads(proc.stdout)
            if pages and isinstance(pages[0], list):
                return [item for page in pages for item in page]
            return pages
        if attempt < attempts - 1:
            time.sleep(2 * (attempt + 1))
            continue
        raise RuntimeError(f"gh api --paginate failed for {path_query}: {proc.stderr}")


def gh_api(path: str, attempts: int = 4):
    for attempt in range(attempts):
        proc = subprocess.run(["gh", "api", path], capture_output=True, text=True)
        if proc.returncode == 0:
            return json.loads(proc.stdout)
        if proc.returncode != 0 and "404" in proc.stderr:
            return None
        if attempt < attempts - 1:
            time.sleep(2 * (attempt + 1))
            continue
        raise RuntimeError(f"gh api failed for {path}: {proc.stderr}")


def load_exclusion_set() -> set[int]:
    ledger = json.loads(S4_LEDGER_EXCLUSION.read_text())
    addendum = json.loads(S5_ADDENDUM.read_text())
    excluded = set(ledger["excluded_pr_numbers"]) | set(addendum["s4_examined_pr_numbers"])
    return excluded


def fetch_candidate_commits() -> set[str]:
    shas: set[str] = set()
    for path in PATHS:
        query = f"repos/{REPO}/commits?path={path}&since={WINDOW_START}&until={WINDOW_END}&per_page=100"
        commits = gh_api_paginate(query)
        print(f"  path={path}: {len(commits)} commits", file=sys.stderr, flush=True)
        for c in commits:
            shas.add(c["sha"])
    return shas


def resolve_commits_to_prs(shas: set[str]) -> set[int]:
    pr_numbers: set[int] = set()
    total = len(shas)
    for i, sha in enumerate(sorted(shas), 1):
        prs = gh_api(f"repos/{REPO}/commits/{sha}/pulls")
        if prs:
            for pr in prs:
                pr_numbers.add(pr["number"])
        if i % 100 == 0 or i == total:
            print(f"  resolved {i}/{total} commits -> {len(pr_numbers)} distinct PR numbers so far", file=sys.stderr, flush=True)
    return pr_numbers


def fetch_pr_record(number: int) -> dict | None:
    detail = gh_api(f"repos/{REPO}/pulls/{number}")
    if detail is None:
        return None
    if not detail.get("merged"):
        return None
    files_raw = gh_api_paginate(f"repos/{REPO}/pulls/{number}/files?per_page=100")
    files = [
        {
            "path": f["filename"],
            "additions": f.get("additions", 0),
            "deletions": f.get("deletions", 0),
            "changeType": f.get("status", "modified").upper(),
        }
        for f in files_raw
    ]
    return {
        "number": number,
        "title": detail.get("title", ""),
        "state": "MERGED",
        "mergedAt": detail.get("merged_at"),
        "baseRefOid": detail.get("base", {}).get("sha"),
        "headRefOid": detail.get("head", {}).get("sha"),
        "additions": detail.get("additions"),
        "deletions": detail.get("deletions"),
        "changedFiles": detail.get("changed_files"),
        "files": files,
        # extra field beyond S4's own shape: independently-provable
        # file-list completeness, for the production selector's own
        # fail-closed truncation check (Compare API caps at 300; PR
        # files endpoint is independently paginated to completion
        # here, and this equality check proves it).
        "files_complete": len(files) == detail.get("changed_files"),
    }


def main():
    fetch_started_at = datetime.now(timezone.utc).isoformat()
    print(f"fetch started: {fetch_started_at}", file=sys.stderr)

    excluded = load_exclusion_set()
    print(f"exclusion set size: {len(excluded)}", file=sys.stderr)

    print("fetching candidate commits...", file=sys.stderr)
    shas = fetch_candidate_commits()
    print(f"total distinct commit shas: {len(shas)}", file=sys.stderr)

    print("resolving commits to PRs...", file=sys.stderr)
    pr_numbers = resolve_commits_to_prs(shas)
    print(f"total distinct PR numbers (pre-exclusion): {len(pr_numbers)}", file=sys.stderr)

    survivors_pre_screen = sorted(n for n in pr_numbers if n not in excluded)
    dropped_by_exclusion = len(pr_numbers) - len(survivors_pre_screen)
    print(f"dropped by PR-number exclusion: {dropped_by_exclusion}", file=sys.stderr)
    print(f"candidates remaining for detail fetch: {len(survivors_pre_screen)}", file=sys.stderr)

    out_path = OUT_DIR / "raw.jsonl"
    written = 0
    with out_path.open("w") as out:
        for i, number in enumerate(survivors_pre_screen, 1):
            record = fetch_pr_record(number)
            if record is not None:
                out.write(json.dumps(record) + "\n")
                written += 1
            if i % 50 == 0 or i == len(survivors_pre_screen):
                print(f"  fetched detail {i}/{len(survivors_pre_screen)} ({written} written)", file=sys.stderr, flush=True)

    fetch_finished_at = datetime.now(timezone.utc).isoformat()
    meta = {
        "fetch_started_at": fetch_started_at,
        "fetch_finished_at": fetch_finished_at,
        "window_start": WINDOW_START,
        "window_end": WINDOW_END,
        "paths": PATHS,
        "exclusion_set_size": len(excluded),
        "distinct_commit_shas": len(shas),
        "distinct_pr_numbers_pre_exclusion": len(pr_numbers),
        "dropped_by_pr_number_exclusion": dropped_by_exclusion,
        "candidates_fetched_detail_for": len(survivors_pre_screen),
        "raw_records_written": written,
    }
    (OUT_DIR / "fetch-meta.json").write_text(json.dumps(meta, indent=2) + "\n")
    print(f"\nwrote {out_path}: {written} raw records", file=sys.stderr)
    print(json.dumps(meta, indent=2))


if __name__ == "__main__":
    main()
