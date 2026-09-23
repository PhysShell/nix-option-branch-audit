#!/usr/bin/env python3
"""S5-F1D-R: mechanical classification of every changed PR's delta type
(verdict-level vs. discovery-representation-only), cross-referenced
with the coordinator's own hand-verified causal tracing (recorded in
classification-notes.json, written by hand after real source
investigation AND after reconcile-discoveries.py's own span-based
provenance reconciliation -- this script does NOT automate causal
classification, only the verdict-vs-representation split, which is a
mechanical JSON comparison). Adapted verbatim from
fixtures/s5-f1-r/classify-deltas.py.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
D = Path(__file__).resolve().parent


def main():
    manifest = {e["pr"]: e for e in json.loads((D / "applicable-manifest.json").read_text())}
    rows = [json.loads(l) for l in (D / "comparison-report.jsonl").read_text().splitlines() if l.strip()]
    changed = [r for r in rows if r.get("status") == "complete" and not r["identical"]]
    notes = json.loads((D / "classification-notes.json").read_text())

    out = []
    for r in changed:
        pr = r["pr"]
        e = manifest[pr]
        orig_dir = ROOT / e["orig_dir"]
        cand_dir = D / "replay" / e["cohort"] / str(pr)

        verdict_delta = False
        for fname in ("check-base.json", "check-head.json"):
            orig = json.loads((orig_dir / fname).read_text())
            cand = json.loads((cand_dir / fname).read_text())
            if orig["targets"][0]["verdicts"] != cand["targets"][0]["verdicts"]:
                verdict_delta = True

        note = notes.get(str(pr), {})
        out.append({
            "pr": pr,
            "cohort": e["cohort"],
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
