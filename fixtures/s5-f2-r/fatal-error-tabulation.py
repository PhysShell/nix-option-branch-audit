#!/usr/bin/env python3
"""S5-F2-R section 8 (fatal/error tabulation): across all 180 applicable
PRs, tabulate parse_errors presence, verdict-kind classes (as a proxy for
exit-code class -- see verdicts[].verdict tag), and any tool-error/
unresolved-class signals, for both the F1D-R baseline and the F2-R
candidate. Report any class present in one but not the other."""
import json
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
D = Path(__file__).resolve().parent
F1DR_REPLAY = ROOT / "fixtures/s5-f1d-r/replay"


def tabulate(replay_root, manifest):
    verdict_kinds = Counter()
    parse_error_prs = []
    for e in manifest:
        pr = e["pr"]
        cohort = e["cohort"]
        pr_dir = replay_root / cohort / str(pr)
        for side in ("check-base.json", "check-head.json"):
            p = pr_dir / side
            if not p.exists():
                continue
            data = json.loads(p.read_text())
            for t in data["targets"]:
                if t.get("parse_errors"):
                    parse_error_prs.append({"pr": pr, "side": side, "target": t["name"]})
                for v in t.get("verdicts", []):
                    kind = v["verdict"] if isinstance(v, dict) and "verdict" in v else str(v)
                    verdict_kinds[kind] += 1
    return verdict_kinds, parse_error_prs


def main():
    manifest = json.loads((D / "applicable-manifest.json").read_text())
    baseline_kinds, baseline_parse_errors = tabulate(F1DR_REPLAY, manifest)
    candidate_kinds, candidate_parse_errors = tabulate(D / "replay", manifest)

    only_baseline = {k: v for k, v in baseline_kinds.items() if k not in candidate_kinds}
    only_candidate = {k: v for k, v in candidate_kinds.items() if k not in baseline_kinds}

    out = {
        "baseline_verdict_kind_counts": dict(baseline_kinds),
        "candidate_verdict_kind_counts": dict(candidate_kinds),
        "verdict_kinds_only_in_baseline": only_baseline,
        "verdict_kinds_only_in_candidate": only_candidate,
        "baseline_parse_error_count": len(baseline_parse_errors),
        "candidate_parse_error_count": len(candidate_parse_errors),
        "baseline_parse_errors": baseline_parse_errors,
        "candidate_parse_errors": candidate_parse_errors,
    }
    (D / "fatal-error-tabulation.json").write_text(json.dumps(out, indent=2) + "\n")
    print("baseline verdict kinds:", dict(baseline_kinds))
    print("candidate verdict kinds:", dict(candidate_kinds))
    print("only in baseline:", only_baseline)
    print("only in candidate:", only_candidate)
    print(f"parse errors baseline={len(baseline_parse_errors)} candidate={len(candidate_parse_errors)}")


if __name__ == "__main__":
    main()
