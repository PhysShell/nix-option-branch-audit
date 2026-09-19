# The exact eligibility/exclusion screen run over raw-population-196.jsonl
# (this directory) to produce sample.md's own funnel + the 30-PR draw.
# Paths below are as originally run (a scratch /tmp working copy of the
# two fixture files this directory now also carries); kept verbatim for
# auditability, not packaged as a re-runnable CLI tool.
import json, re, sys

with open('/tmp/s1-pr-details.jsonl') as f:
    records = [json.loads(l) for l in f if l.strip()]

with open('/tmp/s1-exclusion-names-sorted.txt') as f:
    excl_names = [n.strip() for n in f if n.strip()]

WINDOW_START = "2026-08-21"
WINDOW_END = "2026-09-20"  # inclusive slack for the fetch's own run time

def path_eligible(paths):
    return any(p.startswith("nixos/modules/services/") or p.startswith("nixos/tests/") for p in paths)

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

def matched_excluded_name(paths, title):
    tl = title.lower()
    for name in excl_names:
        nlow = name.lower()
        # path component match: /name/ or /name.nix or name- prefix segment
        for p in paths:
            segs = re.split(r'[/._-]', p.lower())
            if nlow in segs:
                return name
        # title match as a whole word
        if re.search(r'(?<![a-z0-9_-])' + re.escape(nlow) + r'(?![a-z0-9_-])', tl):
            return name
    return None

funnel = {"population": len(records), "eligible": 0, "excl_window": 0, "excl_docs": 0, "excl_mechanical": 0, "excl_name": 0, "survived": 0}
survivors = []
excluded_detail = []

for r in records:
    paths = [f["path"] for f in r.get("files", [])]
    if not path_eligible(paths):
        continue
    funnel["eligible"] += 1

    merged_at = r.get("mergedAt")
    state = r.get("state")
    in_window = True
    if state == "MERGED":
        if not merged_at or not (WINDOW_START <= merged_at[:10] <= WINDOW_END):
            in_window = False
    # OPEN PRs: always considered in-window (live right now)

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

    m = matched_excluded_name(paths, r["title"])
    if m:
        funnel["excl_name"] += 1
        excluded_detail.append((r["number"], f"name:{m}", r["title"]))
        continue

    funnel["survived"] += 1
    survivors.append(r)

print(json.dumps(funnel, indent=2))
print(f"\nsurvivors: {len(survivors)}")

with open('/tmp/s1-survivors.json', 'w') as f:
    json.dump(survivors, f, indent=2)
with open('/tmp/s1-excluded-detail.json', 'w') as f:
    json.dump(excluded_detail, f, indent=2)
