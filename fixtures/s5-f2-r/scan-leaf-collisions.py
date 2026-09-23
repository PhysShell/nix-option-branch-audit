#!/usr/bin/env python3
"""S5-F2-R: full-corpus leaf-collision scan (adapted from
fixtures/s5-f1d-r/scan-leaf-collisions.py). discovered_options is
untouched by F2 (only path_matches_prefix/evaluate_predicate_witness/
run_target changed), so this is expected to be byte-identical to
F1D-R's own leaf-collision-scan.json -- comparison baseline is F1D-R's
own replay output, matching this round's own baseline convention."""
import json
from pathlib import Path
from collections import Counter

ROOT = Path(__file__).resolve().parents[2]
D = Path(__file__).resolve().parent
F1DR_REPLAY = ROOT / "fixtures/s5-f1d-r/replay"


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
        baseline_dir = F1DR_REPLAY / cohort / str(pr)
        cand_dir = D / "replay" / cohort / str(pr)

        for side in ("check-base.json", "check-head.json"):
            cand_path = cand_dir / side
            baseline_path = baseline_dir / side
            if not cand_path.exists():
                missing.append((pr, side))
                continue
            scanned += 1
            cand = json.loads(cand_path.read_text())
            baseline = json.loads(baseline_path.read_text()) if baseline_path.exists() else None
            for t in cand["targets"]:
                cand_collisions = collisions_in(t["discovered_options"])
                if not cand_collisions:
                    continue
                baseline_target = None
                if baseline is not None:
                    baseline_target = next((bt for bt in baseline["targets"] if bt["name"] == t["name"]), None)
                baseline_collisions = collisions_in(baseline_target["discovered_options"]) if baseline_target else None
                findings.append({
                    "pr": pr,
                    "side": side,
                    "target": t["name"],
                    "module": t["module"],
                    "colliding_paths_candidate": cand_collisions,
                    "colliding_paths_f1dr_baseline": baseline_collisions,
                    "pre_existing_in_f1dr_baseline": baseline_collisions == cand_collisions if baseline_collisions is not None else None,
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
        print(f"  PR {f['pr']} ({f['side']}, target={f['target']}): {f['colliding_paths_candidate']} pre_existing={f['pre_existing_in_f1dr_baseline']}")


if __name__ == "__main__":
    main()
