#!/usr/bin/env python3
"""S5-F3-R section 4: discovered_options must be byte-identical between
the F2-R baseline and the F3-R candidate for EVERY PR (changed or not),
since F3 never touches scan_options (only gate-1's own miss branch and a new analyze()-level post-processing pass). Any delta here is unexpected and
flagged prominently, not explained away."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
D = Path(__file__).resolve().parent
F2R_REPLAY = ROOT / "fixtures/s5-f2-r/replay"


def main():
    manifest = json.loads((D / "applicable-manifest.json").read_text())
    total_checked = 0
    identical = 0
    changed_prs = []
    missing = []
    for e in manifest:
        pr = e["pr"]
        cohort = e["cohort"]
        baseline_dir = F2R_REPLAY / cohort / str(pr)
        cand_dir = D / "replay" / cohort / str(pr)
        for side in ("check-base.json", "check-head.json"):
            base_path = baseline_dir / side
            cand_path = cand_dir / side
            if not base_path.exists() or not cand_path.exists():
                missing.append({"pr": pr, "side": side})
                continue
            total_checked += 1
            base = json.loads(base_path.read_text())
            cand = json.loads(cand_path.read_text())
            base_opts = [t["discovered_options"] for t in base["targets"]]
            cand_opts = [t["discovered_options"] for t in cand["targets"]]
            if base_opts == cand_opts:
                identical += 1
            else:
                changed_prs.append({
                    "pr": pr, "side": side,
                    "baseline_discovered_options": base_opts,
                    "candidate_discovered_options": cand_opts,
                })

    out = {
        "total_checked": total_checked,
        "discovery_identical": identical,
        "discovery_changed_count": len(changed_prs),
        "discovery_changed_prs": changed_prs,
        "missing": missing,
    }
    (D / "discovery-invariance-check.json").write_text(json.dumps(out, indent=2) + "\n")
    print(f"total_checked={total_checked} identical={identical} changed={len(changed_prs)} missing={len(missing)}")
    if changed_prs:
        print("FLAGGED -- discovered_options delta found for PRs:", [c["pr"] for c in changed_prs])


if __name__ == "__main__":
    main()
