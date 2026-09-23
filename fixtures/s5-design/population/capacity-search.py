#!/usr/bin/env python3
"""S5 population-capacity search: deterministic backward window
expansion, per the user's frozen procedure (and its correction: the
quarterly steps never land exactly on the 2024-01-01 hard cap by
construction, so 2024-01-01 is an explicit final step, a short partial
step from 2024-03-01, evaluated regardless of whether 2024-03-01 itself
satisfied -- never skipped, and 2023-12-01 is never evaluated).

The 2026-06-01..2026-09-22 window (already fetched, 94 survivors) is
row 1 of the capacity ledger -- a legitimate pre-adjudication
population-capacity finding, not an S5 result. Every candidate window
shares the SAME end date (2026-09-22) and the SAME frozen mechanical
screens/exclusion policy/selector/seeds; only the start date widens.

No OBA invocation anywhere. No adjudication. No outcome data of any
kind -- pure population/selector mechanics, same boundary as
everything else in this round.

Guardrail (explicit, per the user's own correction): an API/rate-limit
failure partway through a window's measurement is NEVER recorded as
`criterion_met: false` -- that would silently treat a broken
measurement as a real negative result. It is recorded as a distinct
`status: "window_evaluation_incomplete"` ledger row and the WHOLE
procedure stops immediately, never advancing to a wider window on an
incomplete measurement.

Efficiency (implementation detail only -- the frozen procedure's
RESULT must be byte-identical regardless of how it's fetched; if any
caching/batching shortcut here would ever produce a different
candidate set, order, or stop/freeze decision than a naive full
re-fetch, that is a bug in the shortcut, not an acceptable variance):
  - commit SHAs are fetched incrementally, only the newly-opened
    calendar slice each step, and unioned into a persistent cache;
  - commit -> PR association is resolved via batched GraphQL
    (associatedPullRequests), not one REST call per commit;
  - PR detail+files are fetched via batched GraphQL, cached globally
    by PR number -- fetched at most once across the ENTIRE multi
    -window search;
  - each PR's real Compare-API diff (needed for the selector's own
    line-level `patch` text -- GraphQL has no equivalent) is cached
    globally by PR number too, fetched at most once.
"""
import hashlib
import json
import random
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
DESIGN_DIR = HERE.parent
ROOT = DESIGN_DIR.parents[1]

sys.path.insert(0, str(DESIGN_DIR / "selector"))
from production_selector import select  # noqa: E402

REPO_OWNER, REPO_NAME = "NixOS", "nixpkgs"
REPO = f"{REPO_OWNER}/{REPO_NAME}"
PATHS = ["nixos/modules/services", "nixos/tests"]
WINDOW_END = "2026-09-22"

SHORT_SHA = "8f1701a"
SEED_A = int(SHORT_SHA, 16)
SEED_B = SEED_A + 1
S5A_TARGET = 150
S5B_CAP = 219

# Frozen deterministic step sequence. Quarterly steps back from
# 2026-06-01 would land on 2023-12-01 next, which is PAST the hard cap
# -- so the sequence is explicitly truncated and the final entry is
# the clamped hard cap itself (a short ~2-month partial step from
# 2024-03-01), always evaluated, never skipped, never followed by
# anything earlier.
WINDOW_STEPS = [
    "2026-06-01",  # row 1, already fetched -- no refetch
    "2026-03-01", "2025-12-01", "2025-09-01", "2025-06-01",
    "2025-03-01", "2024-12-01", "2024-09-01", "2024-06-01",
    "2024-03-01", "2024-01-01",  # explicit clamped hard cap
]

CACHE_DIR = HERE / "cache"
CACHE_DIR.mkdir(exist_ok=True)
PR_RAW_CACHE = CACHE_DIR / "pr-raw-cache.jsonl"          # global, by PR number
COMMIT_PR_CACHE = CACHE_DIR / "commit-pr-cache.json"      # sha -> [pr numbers]
SELECTION_CACHE = CACHE_DIR / "selection-cache.jsonl"     # global, by PR number
COVERAGE_STATE = CACHE_DIR / "coverage-state.json"        # earliest date already commit-resolved

LEDGER_PATH = HERE / "capacity-ledger.jsonl"
ATTEMPTS_DIR = HERE / "attempts"
ATTEMPTS_DIR.mkdir(exist_ok=True)

RATE_LIMIT_SAFETY_MARGIN = 300


class WindowIncomplete(Exception):
    """Raised when a window's measurement could not be completed for a
    real infrastructure reason (rate limit, API error). NEVER caught
    and coerced into criterion_met=False -- always propagates to a
    distinct ledger status and a full-procedure stop."""


def sh256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_rate_limits():
    proc = subprocess.run(["gh", "api", "rate_limit"], capture_output=True, text=True)
    if proc.returncode != 0:
        raise WindowIncomplete(f"rate_limit check itself failed: {proc.stderr}")
    data = json.loads(proc.stdout)
    core = data["resources"]["core"]["remaining"]
    graphql = data["resources"]["graphql"]["remaining"]
    print(f"  [rate limit] core={core} graphql={graphql}", file=sys.stderr, flush=True)
    if core < RATE_LIMIT_SAFETY_MARGIN or graphql < RATE_LIMIT_SAFETY_MARGIN:
        raise WindowIncomplete(
            f"rate limit safety margin hit: core={core} graphql={graphql} "
            f"(margin={RATE_LIMIT_SAFETY_MARGIN})"
        )


def load_json(path: Path, default):
    if path.exists():
        return json.loads(path.read_text())
    return default


def load_jsonl_dict(path: Path, key: str) -> dict:
    out = {}
    if path.exists():
        for line in path.read_text().splitlines():
            if line.strip():
                row = json.loads(line)
                out[row[key]] = row
    return out


def append_jsonl(path: Path, row: dict):
    with path.open("a") as f:
        f.write(json.dumps(row) + "\n")


def gh_api_paginate(query: str, attempts: int = 4) -> list:
    last_err = None
    for attempt in range(attempts):
        proc = subprocess.run(["gh", "api", query, "--paginate", "--slurp"], capture_output=True, text=True)
        if proc.returncode == 0:
            pages = json.loads(proc.stdout)
            if pages and isinstance(pages[0], list):
                return [item for page in pages for item in page]
            return pages
        last_err = proc.stderr
        if attempt < attempts - 1:
            time.sleep(2 * (attempt + 1))
    raise WindowIncomplete(f"gh api --paginate failed for {query} after {attempts} attempts: {last_err}")


def fetch_commit_slice(path: str, since: str, until: str) -> list[str]:
    q = f"repos/{REPO}/commits?path={path}&since={since}T00:00:00Z&until={until}T00:00:00Z&per_page=100"
    commits = gh_api_paginate(q)
    return [c["sha"] for c in commits]


def graphql_query_file(query: str, attempts: int = 4) -> dict:
    """Retried with backoff, matching the same discipline already used
    by gh_api_paginate/gh_compare -- a transient truncated/empty HTTP
    response (seen in practice: 'unexpected end of JSON input') is a
    network blip, not a real capacity/rate-limit wall, and must not
    stop the whole multi-window search on the first hiccup. Only
    exhausting all attempts raises WindowIncomplete.
    """
    tmp = CACHE_DIR / "_tmp_query.graphql"
    tmp.write_text(query)
    last_err = None
    for attempt in range(attempts):
        proc = subprocess.run(["gh", "api", "graphql", "-F", f"query=@{tmp}"], capture_output=True, text=True)
        if proc.returncode == 0:
            try:
                data = json.loads(proc.stdout)
            except json.JSONDecodeError as e:
                last_err = f"JSON decode error: {e}; stdout head: {proc.stdout[:300]!r}"
                if attempt < attempts - 1:
                    time.sleep(2 * (attempt + 1))
                    continue
                raise WindowIncomplete(f"graphql query failed after {attempts} attempts: {last_err}\nquery head: {query[:300]}")
            if "errors" in data:
                raise WindowIncomplete(f"graphql errors: {data['errors']}\nquery head: {query[:300]}")
            return data["data"]
        last_err = proc.stderr
        if attempt < attempts - 1:
            time.sleep(2 * (attempt + 1))
    raise WindowIncomplete(f"graphql query failed after {attempts} attempts: {last_err}\nquery head: {query[:300]}")


def resolve_commits_to_prs_batched(shas: list[str], commit_pr_cache: dict, batch_size: int = 40) -> dict[str, list[int]]:
    """Persists commit_pr_cache to disk after EVERY batch (not only
    once at the end) so a later batch's failure never discards already
    -resolved earlier batches within the same call."""
    out = {}
    for i in range(0, len(shas), batch_size):
        batch = shas[i:i + batch_size]
        parts = []
        for j, sha in enumerate(batch):
            parts.append(
                f'c{j}: object(oid: "{sha}") {{ ... on Commit {{ oid '
                f'associatedPullRequests(first: 3) {{ nodes {{ number }} }} }} }}'
            )
        query = "query { repository(owner: \"%s\", name: \"%s\") { %s } }" % (
            REPO_OWNER, REPO_NAME, " ".join(parts)
        )
        data = graphql_query_file(query)
        repo = data["repository"]
        for j, sha in enumerate(batch):
            node = repo.get(f"c{j}")
            if node is None:
                out[sha] = []
                continue
            prs = [n["number"] for n in node.get("associatedPullRequests", {}).get("nodes", [])]
            out[sha] = prs
        commit_pr_cache.update(out)
        COMMIT_PR_CACHE.write_text(json.dumps(commit_pr_cache))
        print(f"    resolved commit batch {i + len(batch)}/{len(shas)}", file=sys.stderr, flush=True)
    return out


def fetch_pr_details_batched(numbers: list[int], pr_raw_cache: dict, batch_size: int = 15) -> dict[int, dict]:
    """Persists newly-fetched records into pr_raw_cache (and appends to
    PR_RAW_CACHE on disk) after EVERY batch, not only once at the end,
    for the same resume-safety reason as resolve_commits_to_prs_batched."""
    out = {}
    for i in range(0, len(numbers), batch_size):
        batch = numbers[i:i + batch_size]
        parts = []
        for j, n in enumerate(batch):
            parts.append(f'''p{j}: pullRequest(number: {n}) {{
              number title state mergedAt baseRefOid headRefOid additions deletions changedFiles
              files(first: 100) {{ nodes {{ path additions deletions changeType }} }}
            }}''')
        query = "query { repository(owner: \"%s\", name: \"%s\") { %s } }" % (
            REPO_OWNER, REPO_NAME, " ".join(parts)
        )
        data = graphql_query_file(query)
        repo = data["repository"]
        for j, n in enumerate(batch):
            node = repo.get(f"p{j}")
            if node is None or node.get("state") != "MERGED":
                continue
            files = [
                {"path": f["path"], "additions": f["additions"], "deletions": f["deletions"], "changeType": f["changeType"]}
                for f in node["files"]["nodes"]
            ]
            out[n] = {
                "number": node["number"],
                "title": node["title"],
                "state": node["state"],
                "mergedAt": node["mergedAt"],
                "baseRefOid": node["baseRefOid"],
                "headRefOid": node["headRefOid"],
                "additions": node["additions"],
                "deletions": node["deletions"],
                "changedFiles": node["changedFiles"],
                "files": files,
            }
        for n, rec in out.items():
            if n not in pr_raw_cache:
                pr_raw_cache[n] = rec
                append_jsonl(PR_RAW_CACHE, rec)
        print(f"    fetched PR detail batch {i + len(batch)}/{len(numbers)}", file=sys.stderr, flush=True)
    return out


def gh_compare(base_sha: str, head_sha: str, attempts: int = 4) -> dict:
    last_err = None
    for attempt in range(attempts):
        proc = subprocess.run(
            ["gh", "api", f"repos/{REPO}/compare/{base_sha}...{head_sha}"],
            capture_output=True, text=True,
        )
        if proc.returncode == 0:
            return json.loads(proc.stdout)
        last_err = proc.stderr
        if attempt < attempts - 1:
            time.sleep(2 * (attempt + 1))
    raise WindowIncomplete(f"gh api compare failed for {base_sha}...{head_sha} after {attempts} attempts: {last_err}")


def load_exclusion_set() -> set[int]:
    ledger = json.loads((ROOT / "fixtures/s4-live-pr-shadow/exclusion-ledger.json").read_text())
    addendum = json.loads((HERE / "s5-exclusion-addendum.json").read_text())
    return set(ledger["excluded_pr_numbers"]) | set(addendum["s4_examined_pr_numbers"])


def load_screen_funcs():
    """Import screen.py's own pure functions (not its main()) so the
    exact same mechanical screen logic runs here, never reimplemented.
    """
    import importlib.util
    spec = importlib.util.spec_from_file_location("s5screen", HERE / "screen.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def run_screen(records: list[dict], window_start: str, screen_mod) -> tuple[list[dict], dict]:
    excluded_prs = screen_mod.excluded_prs
    funnel = {
        "population": len(records), "excl_already_examined_pr_number": 0,
        "excl_path_ineligible": 0,
        "eligible_path": 0, "excl_window": 0, "excl_docs": 0,
        "excl_mechanical": 0, "excl_name_or_path": 0, "survived": 0,
    }
    survivors = []
    for r in records:
        if r["number"] in excluded_prs:
            funnel["excl_already_examined_pr_number"] += 1
            continue
        paths = [f["path"] for f in r.get("files", [])]
        if not screen_mod.path_eligible(paths):
            funnel["excl_path_ineligible"] += 1
            continue
        funnel["eligible_path"] += 1
        merged_at = r.get("mergedAt")
        state = r.get("state")
        in_window = True
        if state == "MERGED":
            if not merged_at or not (window_start <= merged_at[:10] <= WINDOW_END):
                in_window = False
        if not in_window:
            funnel["excl_window"] += 1
            continue
        if screen_mod.is_docs_only(paths, r["title"]):
            funnel["excl_docs"] += 1
            continue
        if screen_mod.is_mass_mechanical(r.get("changedFiles"), r["title"]):
            funnel["excl_mechanical"] += 1
            continue
        if screen_mod.matched_excluded_path_or_name(paths, r["title"]):
            funnel["excl_name_or_path"] += 1
            continue
        funnel["survived"] += 1
        survivors.append(r)
    return survivors, funnel


def build_selector_input(pr_record: dict, compare: dict) -> dict:
    files = compare.get("files", [])
    files_complete = len(files) == pr_record.get("changedFiles")
    return {
        "pr": pr_record["number"],
        "base_sha": pr_record["baseRefOid"],
        "head_sha": pr_record["headRefOid"],
        "files": [{k: f[k] for k in ("filename", "status", "patch") if k in f} for f in files],
        "files_complete": files_complete,
    }


def evaluate_window(window_start: str, excluded: set[int], screen_mod,
                     pr_raw_cache: dict, commit_pr_cache: dict,
                     selection_cache: dict, coverage: dict) -> dict:
    """Evaluates exactly one window and returns its full result dict.
    Raises WindowIncomplete on any real infrastructure failure -- never
    returns a result with a guessed/partial count."""
    if window_start != "2026-06-01":
        check_rate_limits()
        earliest = coverage["earliest_covered_since"]
        print(f"  fetching new slice [{window_start}, {earliest}) ...", file=sys.stderr)
        new_shas = set()
        for p in PATHS:
            shas = fetch_commit_slice(p, window_start, earliest)
            print(f"    path={p}: {len(shas)} new commits in slice", file=sys.stderr)
            new_shas.update(shas)
        new_shas = sorted(s for s in new_shas if s not in commit_pr_cache)
        print(f"  {len(new_shas)} genuinely new commit shas to resolve", file=sys.stderr)

        if new_shas:
            resolve_commits_to_prs_batched(new_shas, commit_pr_cache)

        all_new_pr_numbers = set()
        for sha in new_shas:
            all_new_pr_numbers.update(commit_pr_cache.get(sha, []))
        genuinely_new = sorted(n for n in all_new_pr_numbers if n not in excluded and n not in pr_raw_cache)
        print(f"  {len(genuinely_new)} genuinely new candidate PR numbers to fetch detail for", file=sys.stderr)

        if genuinely_new:
            fetch_pr_details_batched(genuinely_new, pr_raw_cache)

        coverage["earliest_covered_since"] = window_start
        COVERAGE_STATE.write_text(json.dumps(coverage, indent=2))

    cumulative_records = list(pr_raw_cache.values())
    survivors, funnel = run_screen(cumulative_records, window_start, screen_mod)
    print(f"  cumulative raw pool: {len(cumulative_records)}, survivors: {len(survivors)}", file=sys.stderr)

    pool = list(survivors)
    random.Random(SEED_A).shuffle(pool)
    s5a = pool[: min(S5A_TARGET, len(pool))]
    a_numbers = {r["number"] for r in s5a}
    remainder = [r for r in pool if r["number"] not in a_numbers]

    if window_start != "2026-06-01":
        check_rate_limits()

    need_diff = [r for r in remainder if r["number"] not in selection_cache]
    print(f"  remainder: {len(remainder)}, need diff/selector for {len(need_diff)} new PRs", file=sys.stderr)
    for i, r in enumerate(need_diff, 1):
        compare = gh_compare(r["baseRefOid"], r["headRefOid"])
        selector_input = build_selector_input(r, compare)
        result = select(selector_input)
        selection_cache[r["number"]] = result
        append_jsonl(SELECTION_CACHE, result)
        if i % 50 == 0 or i == len(need_diff):
            print(f"    selector run {i}/{len(need_diff)}", file=sys.stderr, flush=True)

    selected_in_remainder = [
        r for r in remainder
        if selection_cache[r["number"]]["selected"] is True
        and selection_cache[r["number"]]["unresolved"] is False
    ]
    count_selected = len(selected_in_remainder)
    criterion_met = len(s5a) == S5A_TARGET and count_selected >= S5B_CAP
    print(f"  S5-A: {len(s5a)}/{S5A_TARGET}, R4-selected remainder: {count_selected}/{S5B_CAP}, "
          f"criterion_met={criterion_met}", file=sys.stderr)

    attempt_dir = ATTEMPTS_DIR / window_start
    attempt_dir.mkdir(exist_ok=True)
    (attempt_dir / "raw.jsonl").write_text("".join(json.dumps(r) + "\n" for r in cumulative_records))
    (attempt_dir / "funnel.json").write_text(json.dumps(funnel, indent=2))
    (attempt_dir / "survivors.json").write_text(json.dumps(survivors, indent=2))

    ledger_row = {
        "window_start": window_start, "window_end": WINDOW_END, "status": "complete",
        "raw_pr_count": funnel["population"],
        "resolved_count": funnel["population"],
        "excl_already_examined": funnel["excl_already_examined_pr_number"],
        "excl_path_ineligible": funnel["excl_path_ineligible"],
        "excl_window": funnel["excl_window"],
        "excl_docs": funnel["excl_docs"],
        "excl_mechanical": funnel["excl_mechanical"],
        "excl_name_or_path": funnel["excl_name_or_path"],
        "survivor_count": funnel["survived"],
        "s5a_count": len(s5a),
        "survivors_after_a_count": len(remainder),
        "r4_selected_count_after_a": count_selected,
        "criterion_met": criterion_met,
        "raw_jsonl_sha256": sh256(attempt_dir / "raw.jsonl"),
        "survivors_json_sha256": sh256(attempt_dir / "survivors.json"),
    }
    append_jsonl(LEDGER_PATH, ledger_row)
    print(f"  ledger row written: {json.dumps(ledger_row)}", file=sys.stderr)

    return {
        "ledger_row": ledger_row, "criterion_met": criterion_met,
        "survivors": survivors, "s5a": s5a, "remainder": remainder,
        "selected_in_remainder": selected_in_remainder,
    }


def main():
    excluded = load_exclusion_set()
    print(f"exclusion set size: {len(excluded)}", file=sys.stderr)
    screen_mod = load_screen_funcs()

    pr_raw_cache = load_jsonl_dict(PR_RAW_CACHE, "number")
    commit_pr_cache: dict = load_json(COMMIT_PR_CACHE, {})
    selection_cache = load_jsonl_dict(SELECTION_CACHE, "pr")
    coverage = load_json(COVERAGE_STATE, {"earliest_covered_since": None})

    existing_raw_path = HERE / "raw.jsonl"
    if not pr_raw_cache and existing_raw_path.exists():
        for line in existing_raw_path.read_text().splitlines():
            if line.strip():
                rec = json.loads(line)
                pr_raw_cache[rec["number"]] = rec
                append_jsonl(PR_RAW_CACHE, rec)
        coverage["earliest_covered_since"] = "2026-06-01"
        COVERAGE_STATE.write_text(json.dumps(coverage, indent=2))
        print(f"seeded pr-raw-cache from existing raw.jsonl: {len(pr_raw_cache)} records", file=sys.stderr)

    if LEDGER_PATH.exists():
        LEDGER_PATH.unlink()

    for step_idx, window_start in enumerate(WINDOW_STEPS):
        print(f"\n=== attempt {step_idx + 1}/{len(WINDOW_STEPS)}: window_start={window_start} ===", file=sys.stderr)
        try:
            result = evaluate_window(window_start, excluded, screen_mod, pr_raw_cache,
                                      commit_pr_cache, selection_cache, coverage)
        except WindowIncomplete as e:
            row = {"window_start": window_start, "window_end": WINDOW_END,
                   "status": "window_evaluation_incomplete", "reason": str(e)}
            append_jsonl(LEDGER_PATH, row)
            print(f"\nWINDOW EVALUATION INCOMPLETE for {window_start}: {e}", file=sys.stderr)
            print("STOPPING -- not advancing to a wider window on an incomplete measurement, "
                  "not recording this as a real criterion_met=false.", file=sys.stderr)
            print(json.dumps({"result": "INCOMPLETE", "window_start": window_start, "reason": str(e)}))
            return 2

        if result["criterion_met"]:
            print(f"\nSATISFYING WINDOW FOUND: {window_start}", file=sys.stderr)
            print(json.dumps({"result": "SATISFIED", "window_start": window_start}))
            return 0

    print("\nS5 POPULATION CAPACITY INSUFFICIENT: hard cap 2024-01-01 exhausted without meeting criterion",
          file=sys.stderr)
    print(json.dumps({"result": "INSUFFICIENT"}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
