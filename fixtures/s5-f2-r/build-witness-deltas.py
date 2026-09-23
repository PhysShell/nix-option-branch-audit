#!/usr/bin/env python3
"""S5-F2-R section 6: raw before/after witness evidence for every PR
whose comparison-report.jsonl row shows identical=false vs the F1D-R
baseline. No classification -- structured raw extraction only."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
D = Path(__file__).resolve().parent
F1DR_REPLAY = ROOT / "fixtures/s5-f1d-r/replay"


def main():
    manifest = {e["pr"]: e for e in json.loads((D / "applicable-manifest.json").read_text())}
    rows = [json.loads(l) for l in (D / "comparison-report.jsonl").read_text().splitlines() if l.strip()]
    changed = [r for r in rows if r.get("status") == "complete" and not r["identical"]]

    out = {}
    for r in changed:
        pr = r["pr"]
        e = manifest[pr]
        cohort = e["cohort"]
        baseline_dir = F1DR_REPLAY / cohort / str(pr)
        cand_dir = D / "replay" / cohort / str(pr)

        pr_rows = []
        for side, fname in (("base", "check-base.json"), ("head", "check-head.json")):
            baseline = json.loads((baseline_dir / fname).read_text())
            cand = json.loads((cand_dir / fname).read_text())
            for bt, ct in zip(baseline["targets"], cand["targets"]):
                assert bt["name"] == ct["name"]
                if bt["verdicts"] == ct["verdicts"]:
                    continue
                pr_rows.append({
                    "side": side,
                    "target": bt["name"],
                    "option_prefix": bt["option_prefix"],
                    "cfg_ident": bt["cfg_ident"],
                    "baseline_verdicts": bt["verdicts"],
                    "candidate_verdicts": ct["verdicts"],
                })
        # also raw.json (audit-diff) delta, if any
        base_raw = json.loads((baseline_dir / "raw.json").read_text())
        cand_raw = json.loads((cand_dir / "raw.json").read_text())
        raw_diff = None
        if base_raw != cand_raw:
            raw_diff = {"baseline": base_raw, "candidate": cand_raw}

        out[str(pr)] = {"cohort": cohort, "verdict_deltas": pr_rows, "raw_diff": raw_diff}

    (D / "witness-deltas.json").write_text(json.dumps(out, indent=2) + "\n")
    print(f"wrote witness-deltas.json for {len(out)} changed PRs")
    for pr, rec in out.items():
        print(f"  PR {pr}: {len(rec['verdict_deltas'])} verdict deltas, raw_diff={'yes' if rec['raw_diff'] else 'no'}")


if __name__ == "__main__":
    main()
