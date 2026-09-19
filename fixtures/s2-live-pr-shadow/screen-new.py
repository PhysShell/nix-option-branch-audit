import json, re, sys
sys.path.insert(0, '/tmp')

with open('/tmp/s2-new-pr-details-final.jsonl') as f:
    records = [json.loads(l) for l in f if l.strip()]

with open('/tmp/s1-exclusion-names-sorted.txt') as f:
    excl_names = [n.strip() for n in f if n.strip()]

WINDOW_START = "2026-08-21"
WINDOW_END = "2026-09-20"

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
        for p in paths:
            segs = re.split(r'[/._-]', p.lower())
            if nlow in segs:
                return name
        if re.search(r'(?<![a-z0-9_-])' + re.escape(nlow) + r'(?![a-z0-9_-])', tl):
            return name
    return None

survivors = []
for r in records:
    paths = [f["path"] for f in r.get("files", [])]
    if not path_eligible(paths):
        print(f"{r['number']}: NOT path-eligible"); continue
    if r.get("state") == "MERGED":
        ma = r.get("mergedAt")
        if not ma or not (WINDOW_START <= ma[:10] <= WINDOW_END):
            print(f"{r['number']}: window-excluded"); continue
    if is_docs_only(paths, r["title"]):
        print(f"{r['number']}: docs-excluded"); continue
    if is_mass_mechanical(r.get("changedFiles"), r["title"]):
        print(f"{r['number']}: mechanical-excluded"); continue
    m = matched_excluded_name(paths, r["title"])
    if m:
        print(f"{r['number']}: name-excluded ({m})"); continue
    print(f"{r['number']}: SURVIVES")
    survivors.append(r)

with open('/tmp/s2-new-survivors.json', 'w') as f:
    json.dump(survivors, f, indent=2)
print(f"\n{len(survivors)} new survivors")
