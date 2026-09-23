#!/usr/bin/env python3
"""S5 population freeze: exclusion addendum.

NEVER edits fixtures/s4-live-pr-shadow/exclusion-ledger.json (a frozen
S4 artifact, read-only reference here). This addendum adds exactly one
thing on top of it: every PR number S4 actually examined, derived
mechanically from the S4 ledger's own real `pr_summary` records (never
hand-typed), independently cross-checked against features.jsonl's own
PR set (itself derived from the same ledger by a different script,
extract-features.py) as a consistency proof before writing anything.

S1-S3's own excluded_pr_numbers (130, already in the S4 ledger) are
NOT duplicated here -- the S5 screen unions this addendum with the S4
ledger's own fields directly, so this file holds only the NEW set S4
itself contributes.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
S4_LEDGER = ROOT / "fixtures/s4-live-pr-shadow/adjudication-ledger.jsonl"
S5_FEATURES = ROOT / "fixtures/s5-design/features.jsonl"
OUT = Path(__file__).resolve().parent / "s5-exclusion-addendum.json"


def main():
    records = [json.loads(l) for l in S4_LEDGER.read_text().splitlines() if l.strip()]
    ledger_prs = sorted({r["pr"] for r in records if r.get("record_type") == "pr_summary"})

    feature_prs = sorted({json.loads(l)["pr"] for l in S5_FEATURES.read_text().splitlines() if l.strip()})

    assert ledger_prs == feature_prs, (
        f"S4 ledger pr_summary PR set ({len(ledger_prs)}) must exactly match "
        f"features.jsonl's own PR set ({len(feature_prs)}) before this addendum is trusted -- "
        "mismatch would mean features.jsonl and the S4 ledger have silently diverged"
    )
    assert len(ledger_prs) == 187, f"expected exactly 187 S4-examined PRs, got {len(ledger_prs)}"

    out = {
        "generated_by": "fixtures/s5-design/population/build-exclusion-addendum.py",
        "derived_from": "fixtures/s4-live-pr-shadow/adjudication-ledger.jsonl (pr_summary records)",
        "cross_checked_against": "fixtures/s5-design/features.jsonl (independent re-derivation, exact match required)",
        "s4_examined_pr_numbers": ledger_prs,
        "s4_examined_pr_count": len(ledger_prs),
        "note": (
            "Union this list with fixtures/s4-live-pr-shadow/exclusion-ledger.json's own "
            "excluded_pr_numbers (S1+S2+S3's 130) to get S5's full PR-number exclusion set. "
            "This addendum intentionally does not duplicate the S1-S3 names/paths/pr-numbers "
            "already in the S4 ledger."
        ),
    }
    OUT.write_text(json.dumps(out, indent=2) + "\n")
    print(f"wrote {OUT}: {len(ledger_prs)} S4-examined PR numbers, cross-check PASSED")


if __name__ == "__main__":
    main()
