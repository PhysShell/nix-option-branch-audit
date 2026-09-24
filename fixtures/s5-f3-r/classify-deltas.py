#!/usr/bin/env python3
"""S5-F3-R: mechanical classification of every PR changed relative to the
F2-R baseline (not raw historical), cross-referenced with
classification-notes.json (hand-verified causal tracing, written after
real source investigation). Adapted from fixtures/s5-f2-r/classify-deltas.py
with the comparison baseline changed to F2-R's own replay output.
"""
import json
from pathlib import Path

D = Path(__file__).resolve().parent
F2R_REPLAY = D.parent / "s5-f2-r" / "replay"


def main():
    manifest = {e["pr"]: e for e in json.loads((D / "applicable-manifest.json").read_text())}
    rows = [json.loads(l) for l in (D / "comparison-report.jsonl").read_text().splitlines() if l.strip()]
    changed = [r for r in rows if r.get("status") == "complete" and not r["identical"]]
    notes = json.loads((D / "classification-notes.json").read_text())

    out = []
    for r in changed:
        pr = r["pr"]
        e = manifest[pr]
        baseline_dir = F2R_REPLAY / e["cohort"] / str(pr)
        cand_dir = D / "replay" / e["cohort"] / str(pr)

        verdict_delta = False
        for fname in ("check-base.json", "check-head.json"):
            base = json.loads((baseline_dir / fname).read_text())
            cand = json.loads((cand_dir / fname).read_text())
            if base["targets"][0]["verdicts"] != cand["targets"][0]["verdicts"]:
                verdict_delta = True

        note = notes.get(str(pr), {})
        out.append({
            "pr": pr,
            "cohort": e["cohort"],
            "baseline": "f2-r",
            "verdict_delta": verdict_delta,
            "classification": note.get("classification", "UNCLASSIFIED"),
            "mechanism": note.get("mechanism"),
        })

    unclassified = [r for r in out if r["classification"] == "UNCLASSIFIED"]
    if unclassified:
        print("ERROR: unclassified PRs found:", [r["pr"] for r in unclassified])

    counts = {}
    for r in out:
        counts[r["classification"]] = counts.get(r["classification"], 0) + 1

    (D / "delta-classification.json").write_text(json.dumps(out, indent=2) + "\n")
    print(f"wrote delta-classification.json, {len(out)} changed PRs classified")
    for k, v in counts.items():
        print(f"  {k}: {v}")


if __name__ == "__main__":
    main()
