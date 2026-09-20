#!/usr/bin/env python3
"""S4: mechanical eligibility screen over the real fresh population.

Same real screen S1/S2/S3 already established (merge-window sanity,
docs-only, mass-mechanical), PLUS the machine-generated exclusion
ledger (PR number, subject name, module path, test path -- all four,
not just names) built in build-exclusion-ledger.py.
"""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ledger = json.loads((ROOT / "fixtures/s4-live-pr-shadow/exclusion-ledger.json").read_text())

excluded_prs = set(ledger["excluded_pr_numbers"])
excluded_names = set(ledger["excluded_subject_names"])
excluded_module_paths = set(ledger["excluded_module_paths"])
excluded_test_paths = set(ledger["excluded_test_paths"])

WINDOW_START = "2026-06-01"
WINDOW_END = "2026-09-20"


def path_eligible(paths):
    return any(
        p.startswith("nixos/modules/services/") or p.startswith("nixos/tests/") for p in paths
    )


def is_docs_only(paths, title):
    if all(p.endswith(".md") for p in paths):
        return True
    t = title.lower()
    if "nixfmt" in t or "typo" in t or ("treewide" in t and "format" in t):
        return True
    return False


def is_mass_mechanical(changed_files, title):
    if changed_files is not None and changed_files > 15:
        return True
    t = title.lower()
    if t.startswith("treewide:") or "by-name migration" in t or t.startswith("maintainers:"):
        return True
    return False


def subject_segments(path: str) -> list[str]:
    return re.split(r"[/._-]", path.lower())


def matched_excluded_path_or_name(paths, title):
    for p in paths:
        if p in excluded_module_paths or p in excluded_test_paths:
            return f"path:{p}"
    tl = title.lower()
    for name in excluded_names:
        for p in paths:
            if name in subject_segments(p):
                return f"name:{name}"
        if re.search(r"(?<![a-z0-9_-])" + re.escape(name) + r"(?![a-z0-9_-])", tl):
            return f"name:{name}"
    return None


def main():
    records = [
        json.loads(line)
        for line in (ROOT / "fixtures/s4-live-pr-shadow" / "population-raw.jsonl").read_text().splitlines()
        if line.strip()
    ]

    funnel = {
        "population": len(records),
        "excl_already_examined_pr_number": 0,
        "eligible_path": 0,
        "excl_window": 0,
        "excl_docs": 0,
        "excl_mechanical": 0,
        "excl_name_or_path": 0,
        "survived": 0,
    }
    survivors = []
    excluded_detail = []

    for r in records:
        if r["number"] in excluded_prs:
            funnel["excl_already_examined_pr_number"] += 1
            excluded_detail.append((r["number"], "already_examined_s1_s2_s3", r["title"]))
            continue

        paths = [f["path"] for f in r.get("files", [])]
        if not path_eligible(paths):
            continue
        funnel["eligible_path"] += 1

        merged_at = r.get("mergedAt")
        state = r.get("state")
        in_window = True
        if state == "MERGED":
            if not merged_at or not (WINDOW_START <= merged_at[:10] <= WINDOW_END):
                in_window = False
        if not in_window:
            funnel["excl_window"] += 1
            excluded_detail.append((r["number"], "window", r["title"]))
            continue

        if is_docs_only(paths, r["title"]):
            funnel["excl_docs"] += 1
            excluded_detail.append((r["number"], "docs", r["title"]))
            continue

        if is_mass_mechanical(r.get("changedFiles"), r["title"]):
            funnel["excl_mechanical"] += 1
            excluded_detail.append((r["number"], "mechanical", r["title"]))
            continue

        m = matched_excluded_path_or_name(paths, r["title"])
        if m:
            funnel["excl_name_or_path"] += 1
            excluded_detail.append((r["number"], m, r["title"]))
            continue

        funnel["survived"] += 1
        survivors.append(r)

    print(json.dumps(funnel, indent=2))
    print(f"\nsurvivors: {len(survivors)}")

    out_dir = ROOT / "fixtures/s4-live-pr-shadow"
    (out_dir / "population-survivors.json").write_text(json.dumps(survivors, indent=2))
    (out_dir / "population-excluded-detail.json").write_text(json.dumps(excluded_detail, indent=2))
    (out_dir / "population-funnel.json").write_text(json.dumps(funnel, indent=2))


if __name__ == "__main__":
    main()
