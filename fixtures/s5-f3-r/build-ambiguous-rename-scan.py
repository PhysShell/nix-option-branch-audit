#!/usr/bin/env python3
"""S5-F3-R section 7: from the migration-edge manifest, group by
(pr, side, from_path) and find every group with >1 edge. For groups
where from_path also equals some watched target's own complete path,
report the actual F3-R verdict recorded for that target (must NOT be
OptionRelocated)."""
import json
from collections import defaultdict
from pathlib import Path

ROOT = Path("/home/tandem/nix-option-branch-audit")
D = ROOT / "fixtures/s5-f3-r"


def main():
    edges = json.loads((D / "migration-edge-manifest.json").read_text())
    groups = defaultdict(list)
    for e in edges:
        key = (e["pr"], e["side"], tuple(e["from_path"]))
        groups[key].append(e)

    ambiguous_groups = {k: v for k, v in groups.items() if len(v) > 1}
    affecting_watched = []
    for (pr, side, from_path), group_edges in ambiguous_groups.items():
        if not any(e["matches_watched_path"] for e in group_edges):
            continue
        cohort = None
        manifest = json.loads((D / "applicable-manifest.json").read_text())
        for m in manifest:
            if m["pr"] == pr:
                cohort = m["cohort"]
        fname = "check-base.json" if side == "base" else "check-head.json"
        check_path = D / "replay" / cohort / str(pr) / fname
        actual_verdict = None
        if check_path.exists():
            data = json.loads(check_path.read_text())
            target_name = group_edges[0]["target"]
            for t in data.get("targets", []):
                if t["name"] != target_name:
                    continue
                for v in t.get("verdicts", []):
                    # the watched option whose full path equals from_path
                    if tuple((t["option_prefix"] if "option_prefix" in t else []) + v["option"].split(".")) == from_path:
                        actual_verdict = v
        affecting_watched.append({
            "pr": pr,
            "side": side,
            "from_path": list(from_path),
            "candidate_destinations": [e["to_path"] for e in group_edges],
            "source_spans": [{"file": e["source_file"], "span": e["span"], "helper_form": e["helper_form"], "kind": e["kind"]} for e in group_edges],
            "actual_f3r_verdict": actual_verdict,
        })

    out = {
        "ambiguous_groups_total": len(ambiguous_groups),
        "ambiguous_groups_affecting_watched_target": affecting_watched,
    }
    if not affecting_watched:
        out["ambiguous_rename_affecting_watched_target_not_observed"] = True

    (D / "ambiguous-rename-scan.json").write_text(json.dumps(out, indent=2) + "\n")
    print(f"ambiguous (pr,side,from_path) groups (any kind, >1 edge): {len(ambiguous_groups)}")
    print(f"of those, affecting a watched target: {len(affecting_watched)}")
    if not affecting_watched:
        print("ambiguous_rename_affecting_watched_target_not_observed: True")
    else:
        for a in affecting_watched:
            print(f"  PR {a['pr']} side={a['side']} from_path={a['from_path']} verdict={a['actual_f3r_verdict']}")


if __name__ == "__main__":
    main()
