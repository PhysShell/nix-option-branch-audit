#!/usr/bin/env python3
"""S5-F1B-R: search the FULL replayed corpus (all 180 applicable PRs, both
base and head, not just PRs whose comparison shows a delta from historical)
for the disclosed-but-unresolved S5-F1B edge case: two or more discovered
declarations sharing the exact same `path` within a single target's own
`discovered_options` list, in the CANDIDATE (6c2c2aa) output. This is a
targeted search per the mandate ("detect whether any frozen S5 PR actually
exercises this ambiguity"), independent of whether that PR's own comparison
against historical shows any delta at all -- the collision could exist
identically in the historical output too (pre-existing) or be new.

For every occurrence found, also checks whether the SAME collision (same
path, same target) already existed in the historical output, to distinguish
"F1B introduces a new collision" from "the collision already existed and
F1B doesn't change that."

Writes leaf-collision-scan.json.
"""
import json
from pathlib import Path
from collections import Counter

ROOT = Path(__file__).resolve().parents[2]
D = Path(__file__).resolve().parent


def collisions_in(discovered_options):
    counts = Counter(tuple(o["path"]) for o in discovered_options)
    return [list(p) for p, n in counts.items() if n > 1]


def main():
    manifest = json.loads((D / "applicable-manifest.json").read_text())
    findings = []
    scanned = 0
    missing = []
    for e in manifest:
        pr = e["pr"]
        cohort = e["cohort"]
        orig_dir = ROOT / e["orig_dir"]
        cand_dir = D / "replay" / cohort / str(pr)

        for side in ("check-base.json", "check-head.json"):
            cand_path = cand_dir / side
            orig_path = orig_dir / side
            if not cand_path.exists():
                missing.append((pr, side))
                continue
            scanned += 1
            cand = json.loads(cand_path.read_text())
            orig = json.loads(orig_path.read_text()) if orig_path.exists() else None
            for t in cand["targets"]:
                cand_collisions = collisions_in(t["discovered_options"])
                if not cand_collisions:
                    continue
                orig_target = None
                if orig is not None:
                    orig_target = next((ot for ot in orig["targets"] if ot["name"] == t["name"]), None)
                orig_collisions = collisions_in(orig_target["discovered_options"]) if orig_target else None
                findings.append({
                    "pr": pr,
                    "side": side,
                    "target": t["name"],
                    "module": t["module"],
                    "colliding_paths_candidate": cand_collisions,
                    "colliding_paths_historical": orig_collisions,
                    "pre_existing_in_historical": orig_collisions == cand_collisions if orig_collisions is not None else None,
                })

    out = {
        "scanned_check_files": scanned,
        "missing_check_files": missing,
        "findings": findings,
        "findings_count": len(findings),
    }
    (D / "leaf-collision-scan.json").write_text(json.dumps(out, indent=2) + "\n")
    print(f"scanned {scanned} check-*.json files across {len(manifest)} PRs")
    if missing:
        print(f"WARNING: {len(missing)} check files missing (replay incomplete for these): {missing[:10]}")
    print(f"leaf-collision findings: {len(findings)}")
    for f in findings:
        print(f"  PR {f['pr']} ({f['side']}, target={f['target']}): {f['colliding_paths_candidate']} pre_existing={f['pre_existing_in_historical']}")


if __name__ == "__main__":
    main()
