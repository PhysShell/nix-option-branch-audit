#!/usr/bin/env python3
"""S5-F2-R section 8: targeted F1D-anchor / guacamole / cgit / fatal-error
checks against the F1D-R baseline, for specific named PRs."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
D = Path(__file__).resolve().parent
F1DR_REPLAY = ROOT / "fixtures/s5-f1d-r/replay"

ANCHOR_PRS = {
    "angrr_471312": 471312,
    "f1_regression_431289": 431289,
    "f1_regression_506644": 506644,
    "f1_regression_508427": 508427,
    "f1_regression_440660": 440660,
    "f1_regression_428153": 428153,
    "f1_regression_397967": 397967,
    "f1_regression_427260": 427260,
    "rspamd_484133": 484133,
    "tayga_432528": 432528,
    "k3s_374017": 374017,
    "guacamole_462487": 462487,
    "cgit_475112": 475112,
}


def find_cohort(manifest, pr):
    for e in manifest:
        if e["pr"] == pr:
            return e["cohort"]
    return None


def main():
    manifest = json.loads((D / "applicable-manifest.json").read_text())
    out = {}
    for label, pr in ANCHOR_PRS.items():
        cohort = find_cohort(manifest, pr)
        if cohort is None:
            out[label] = {"pr": pr, "status": "NOT_IN_APPLICABLE_MANIFEST"}
            continue
        baseline_dir = F1DR_REPLAY / cohort / str(pr)
        cand_dir = D / "replay" / cohort / str(pr)
        rec = {"pr": pr, "cohort": cohort}
        for fname in ("raw.json", "check-base.json", "check-head.json"):
            bp = baseline_dir / fname
            cp = cand_dir / fname
            if not bp.exists() or not cp.exists():
                rec[fname] = {"status": "missing", "baseline_exists": bp.exists(), "candidate_exists": cp.exists()}
                continue
            b = json.loads(bp.read_text())
            c = json.loads(cp.read_text())
            rec[fname] = {"identical": b == c}
            if b != c:
                rec[fname]["baseline"] = b
                rec[fname]["candidate"] = c
        out[label] = rec

    (D / "anchors-check.json").write_text(json.dumps(out, indent=2) + "\n")
    for label, rec in out.items():
        if "status" in rec:
            print(f"{label}: {rec['status']}")
            continue
        idents = {fname: rec[fname].get("identical") for fname in ("raw.json", "check-base.json", "check-head.json") if fname in rec}
        print(f"{label} (PR {rec['pr']}): {idents}")


if __name__ == "__main__":
    main()
