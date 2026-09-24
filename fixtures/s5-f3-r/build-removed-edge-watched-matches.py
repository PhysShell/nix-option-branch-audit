#!/usr/bin/env python3
"""S5-F3-R section 9 (mandate item 14): from the migration-edge
manifest, filter kind=Removed AND matches_watched_path=true across the
WHOLE corpus (not just sshd/gollum). For each, report the F2-R baseline
verdict and F3-R candidate verdict for that exact watched option, and
whether identical."""
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
    edges = json.loads((D / "migration-edge-manifest.json").read_text())
    manifest = json.loads((D / "applicable-manifest.json").read_text())
    cohort_by_pr = {m["pr"]: m["cohort"] for m in manifest}

    removed_watched = [e for e in edges if e["kind"] == "Removed" and e["matches_watched_path"]]
    results = []
    for e in removed_watched:
        pr = e["pr"]
        side = e["side"]
        cohort = cohort_by_pr.get(pr)
        fname = "check-base.json" if side == "base" else "check-head.json"
        # the watched option string is the leaf(s) after option_prefix -- reconstruct from from_path and target's option_prefix isn't directly in the edge, so match by full path length against target's own verdicts list (option field is dotted suffix)
        cand_path = D / "replay" / cohort / str(pr) / fname
        baseline_path = F2R_REPLAY / cohort / str(pr) / fname
        cand_data = json.loads(cand_path.read_text()) if cand_path.exists() else None
        matched_option = None
        cand_verdict = None
        if cand_data:
            for t in cand_data.get("targets", []):
                if t["name"] != e["target"]:
                    continue
                prefix = t["option_prefix"]
                for v in t.get("verdicts", []):
                    full = prefix + v["option"].split(".")
                    if full == e["from_path"]:
                        matched_option = v["option"]
                        cand_verdict = v
        baseline_verdict = find_verdict(baseline_path, e["target"], matched_option) if matched_option else None
        results.append({
            "pr": pr,
            "side": side,
            "target": e["target"],
            "from_path": e["from_path"],
            "option": matched_option,
            "f2r_verdict": baseline_verdict,
            "f3r_verdict": cand_verdict,
            "identical": baseline_verdict == cand_verdict,
        })

    (D / "removed-edge-watched-matches.json").write_text(json.dumps(results, indent=2) + "\n")
    print(f"Removed edges matching a watched path: {len(results)}")
    for r in results:
        print(f"  PR {r['pr']} {r['side']} target={r['target']} option={r['option']} identical={r['identical']}")


if __name__ == "__main__":
    main()
