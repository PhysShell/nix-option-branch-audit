#!/usr/bin/env python3
"""S5-F3-R section 6: enumerate every OptionRelocated verdict across the
whole F3-R replay, paired with the corresponding F2-R baseline verdict
for the same (pr, side, target, option)."""
import json
from pathlib import Path

ROOT = Path("/home/tandem/nix-option-branch-audit")
D = ROOT / "fixtures/s5-f3-r"
F2R_REPLAY = ROOT / "fixtures/s5-f2-r/replay"


def find_verdict(check_json_path, target_name, option):
    if not check_json_path.exists():
        return None
    data = json.loads(check_json_path.read_text())
    for t in data.get("targets", []):
        if t["name"] != target_name:
            continue
        for v in t.get("verdicts", []):
            if v.get("option") == option:
                return v
    return None


def main():
    manifest = json.loads((D / "applicable-manifest.json").read_text())
    results = []
    for e in manifest:
        pr = e["pr"]
        cohort = e["cohort"]
        for side, fname in (("base", "check-base.json"), ("head", "check-head.json")):
            cand_path = D / "replay" / cohort / str(pr) / fname
            if not cand_path.exists():
                continue
            data = json.loads(cand_path.read_text())
            for t in data.get("targets", []):
                for v in t.get("verdicts", []):
                    if v.get("verdict") != "OptionRelocated":
                        continue
                    baseline_path = F2R_REPLAY / cohort / str(pr) / fname
                    f2r_verdict = find_verdict(baseline_path, t["name"], v["option"])
                    results.append({
                        "pr": pr,
                        "side": side,
                        "target": t["name"],
                        "option_prefix": t["option_prefix"],
                        "option": v["option"],
                        "f3r_verdict": v,
                        "f2r_verdict": f2r_verdict,
                    })

    (D / "option-relocated-enumeration.json").write_text(json.dumps(results, indent=2) + "\n")
    prs = sorted({r["pr"] for r in results})
    print(f"OptionRelocated count: {len(results)}")
    print(f"PRs involved: {prs}")


if __name__ == "__main__":
    main()
