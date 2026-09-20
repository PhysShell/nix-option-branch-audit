import json, re

with open('/tmp/s3-pr-details.jsonl') as f:
    records = [json.loads(l) for l in f if l.strip()]

with open('/home/tandem/nix-option-branch-audit/fixtures/s3-live-pr-shadow/exclusion-name-list-253.txt') as f:
    excl_names = [n.strip() for n in f if n.strip()]

# explicit PR-number exclusion: all 30 S1 + all 50 S2 drawn
s1_drawn = [295514,492803,496303,519494,525702,527821,539076,543675,549506,550960,
            551955,553349,553770,554779,555805,556710,556729,557545,558121,558854,
            559055,559588,559627,560647,561557,561669,561845,563958,564021,564357]
s2a_drawn = [295514,452303,469112,479381,485251,503263,528118,545231,552737,552774,
             554703,555377,555625,556253,556989,557327,558091,558149,558283,559009,
             559344,559798,559860,560892,560968,561051,562104,562703,563778,564257]
s2b_drawn = [532540,534100,547038,549553,549618,552038,552640,553474,554495,554949,
             555612,555635,556119,556413,558400,559239,559393,560664,561130,561424]
already_drawn = set(s1_drawn) | set(s2a_drawn) | set(s2b_drawn)
print(f"already-drawn PR count (union, dedup): {len(already_drawn)}")

WINDOW_START = "2026-08-01"
WINDOW_END = "2026-09-21"

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

funnel = {"population": len(records), "excl_already_drawn": 0, "eligible": 0, "excl_window": 0, "excl_docs": 0, "excl_mechanical": 0, "excl_name": 0, "survived": 0}
survivors = []
excluded_detail = []

for r in records:
    if r['number'] in already_drawn:
        funnel["excl_already_drawn"] += 1
        excluded_detail.append((r["number"], "already_drawn_s1_or_s2", r["title"]))
        continue

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

with open('/tmp/s3-survivors.json', 'w') as f:
    json.dump(survivors, f, indent=2)
with open('/tmp/s3-excluded-detail.json', 'w') as f:
    json.dump(excluded_detail, f, indent=2)
