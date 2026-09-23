#!/usr/bin/env python3
"""S5-F1-R: mechanical comparison of the candidate (cdfb4ca) replay
output against the original, frozen historical S5 output, for every
applicable PR. Pure JSON structural comparison -- no judgment, no
classification (that's the coordinator's own job, done separately and
by hand for every real delta this script finds). Both sides self-report
`tool.version: "0.4.5"` identically (no version bump), so a straight
parsed-JSON equality check is the correct, sufficient comparison; no
field needs to be filtered out.

Reads: applicable-manifest.json (180 entries, each with orig_dir).
       replay/<cohort>/<pr>/{raw.json,check-base.json,check-head.json}
       (written by the replay execution step).
Writes: comparison-report.jsonl (one row per applicable PR).
"""
import json
import sys
from pathlib import Path

D = Path(__file__).resolve().parent
ROOT = D.parent.parent


def load(path):
    return json.loads(path.read_text())


def main():
    manifest = json.loads((D / "applicable-manifest.json").read_text())
    rows = []
    incomplete = []
    for entry in manifest:
        pr = entry["pr"]
        cohort = entry["cohort"]
        orig_dir = ROOT / entry["orig_dir"]
        replay_dir = D / "replay" / cohort / str(pr)

        row = {"pr": pr, "cohort": cohort, "position": entry["position"]}

        missing = []
        for fname in ("raw.json", "check-base.json", "check-head.json"):
            if not (replay_dir / fname).exists():
                missing.append(fname)
        if missing:
            row["status"] = "window_evaluation_incomplete"
            row["missing"] = missing
            incomplete.append(row)
            rows.append(row)
            continue

        diffs = {}
        identical = True
        for fname in ("raw.json", "check-base.json", "check-head.json"):
            orig = load(orig_dir / fname)
            cand = load(replay_dir / fname)
            if orig != cand:
                identical = False
                diffs[fname] = {"orig": orig, "candidate": cand}

        row["status"] = "complete"
        row["identical"] = identical
        if not identical:
            row["diffs"] = diffs
        rows.append(row)

    out_path = D / "comparison-report.jsonl"
    with out_path.open("w") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")

    n_total = len(rows)
    n_incomplete = len(incomplete)
    n_complete = n_total - n_incomplete
    n_identical = sum(1 for r in rows if r.get("status") == "complete" and r["identical"])
    n_changed = sum(1 for r in rows if r.get("status") == "complete" and not r["identical"])

    print(f"total applicable PRs: {n_total}")
    print(f"incomplete (window_evaluation_incomplete): {n_incomplete}")
    print(f"complete: {n_complete}")
    print(f"  identical: {n_identical}")
    print(f"  changed: {n_changed}")
    if n_changed:
        print("changed PRs:", [r["pr"] for r in rows if r.get("status") == "complete" and not r["identical"]])
    if incomplete:
        print("INCOMPLETE PRs (STOP condition):", [r["pr"] for r in incomplete])
        sys.exit(1)


if __name__ == "__main__":
    main()
