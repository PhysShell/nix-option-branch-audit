#!/usr/bin/env python3
"""S6-R0: build the frozen leakage-exclusion reference from ALREADY-EXISTING
historical data (the S5 adjudication ledger) -- not a query against live
nixpkgs, and not a query against the future S6 candidate population.
This script reads only fixtures/s5-live-pr-shadow/adjudication-ledger.jsonl
(frozen since S5, untouched by every remediation round) and writes the
exact PR-number set every S6 sample must be diffed against.

Rationale for why the ledger alone is sufficient (see preregistration.md
section "Leakage-prevention method" for the full argument): every PR
number referenced anywhere in S5/F1/F1B/F1C/F1D/F1D-R/F2/F2-R/F3/F3-R --
including every synthetic fixture derived from a real PR, and every
"additional sound correction" the -R rounds themselves discovered -- is
provably a member of this same 369-PR ledger, because every one of those
rounds' own replay pipelines only ever operated on the frozen
applicable-manifest.json derived from this exact ledger (independently
re-derived and confirmed byte-identical at every single round, per each
round's own report). No round ever introduced a PR number the ledger
doesn't already contain.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
D = Path(__file__).resolve().parent


def main():
    ledger_path = ROOT / "fixtures/s5-live-pr-shadow/adjudication-ledger.jsonl"
    rows = [json.loads(line) for line in ledger_path.read_text().splitlines() if line.strip()]
    pr_summaries = [r for r in rows if r.get("record_type") == "pr_summary"]

    prs = sorted({r["pr"] for r in pr_summaries})

    if len(prs) != 369:
        raise SystemExit(f"FATAL: expected 369 unique PRs in the ledger, got {len(prs)}")

    out = {
        "source": "fixtures/s5-live-pr-shadow/adjudication-ledger.jsonl",
        "generated_by": "fixtures/s6-r0/build-exclusion-list.py",
        "total_excluded_pr_count": len(prs),
        "excluded_pr_numbers": prs,
        "min_pr_number": min(prs),
        "max_pr_number": max(prs),
        "note": (
            "Every PR number ever touched by S5, its remediation "
            "(F1/F1B/F1C/F1D/F1D-R/F2/F2-R/F3/F3-R), or any synthetic "
            "fixture derived from a real PR, is a member of this set -- "
            "no round ever introduced a PR number outside the original "
            "369-PR ledger. This list is the mechanical leakage check "
            "S6's own sampling step must diff every sampled PR number "
            "against, in addition to (not instead of) the structural "
            "argument that GitHub PR numbers are monotonically "
            "increasing and never reused, so a genuinely-freshly-merged "
            "PR after the v0.5.0 release date cannot coincide with a "
            "historical S5 PR number by construction."
        ),
    }
    (D / "excluded-pr-identities.json").write_text(json.dumps(out, indent=2) + "\n")
    print(f"wrote excluded-pr-identities.json: {len(prs)} PRs, range [{min(prs)}, {max(prs)}]")


if __name__ == "__main__":
    main()
