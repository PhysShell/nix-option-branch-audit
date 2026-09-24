#!/usr/bin/env python3
"""S5-F3-R: mechanical comparison of the candidate (05d4a90) replay
output against the F2-R BASELINE (fixtures/s5-f2-r/replay/**), not raw
historical v0.4.5. Adapted from fixtures/s5-f2-r/compare-replay.py with
the comparison target changed.
"""
import json
import sys
from pathlib import Path

D = Path(__file__).resolve().parent
ROOT = D.parent.parent
F2R_REPLAY = ROOT / "fixtures/s5-f2-r/replay"


def load(path):
    return json.loads(path.read_text())


def main():
    manifest = json.loads((D / "applicable-manifest.json").read_text())
    rows = []
    incomplete = []
    for entry in manifest:
        pr = entry["pr"]
        cohort = entry["cohort"]
        baseline_dir = F2R_REPLAY / cohort / str(pr)
        replay_dir = D / "replay" / cohort / str(pr)

        row = {"pr": pr, "cohort": cohort, "position": entry["position"], "baseline": "f2-r"}

        missing = []
        for fname in ("raw.json", "check-base.json", "check-head.json"):
            if not (replay_dir / fname).exists():
                missing.append(f"candidate:{fname}")
            if not (baseline_dir / fname).exists():
                missing.append(f"baseline:{fname}")
        if missing:
            row["status"] = "window_evaluation_incomplete"
            row["missing"] = missing
            incomplete.append(row)
            rows.append(row)
            continue

        diffs = {}
        identical = True
        for fname in ("raw.json", "check-base.json", "check-head.json"):
            base = load(baseline_dir / fname)
            cand = load(replay_dir / fname)
            if base != cand:
                identical = False
                diffs[fname] = {"baseline_f2r": base, "candidate_f3": cand}

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
    print(f"  identical to F2-R baseline: {n_identical}")
    print(f"  changed vs F2-R baseline: {n_changed}")
    if n_changed:
        print("changed PRs:", [r["pr"] for r in rows if r.get("status") == "complete" and not r["identical"]])
    if incomplete:
        print("INCOMPLETE PRs (STOP condition):", [r["pr"] for r in incomplete])
        sys.exit(1)


if __name__ == "__main__":
    main()
