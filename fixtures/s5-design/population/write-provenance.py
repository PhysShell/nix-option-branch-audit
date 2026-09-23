#!/usr/bin/env python3
"""Assembles fixtures/s5-design/population/provenance.json from the
real committed outputs of the multi-window capacity search
(capacity-search.py -> capacity-ledger.jsonl + attempts/*) and the
final freeze (finalize-freeze.py -> freeze-meta.json) -- no hand-typed
numbers, every field sourced from a committed file this round itself
produced.
"""
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def load_jsonl(path: Path) -> list:
    return [json.loads(l) for l in path.read_text().splitlines() if l.strip()]


def main():
    ledger = load_jsonl(HERE / "capacity-ledger.jsonl")
    freeze_meta = json.loads((HERE / "freeze-meta.json").read_text())
    addendum = json.loads((HERE / "s5-exclusion-addendum.json").read_text())

    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
    ).stdout.strip()

    satisfying_row = next((r for r in ledger if r.get("criterion_met")), None)

    out = {
        "generated_by": "fixtures/s5-design/population/write-provenance.py",
        "frozen_against_commit": commit,
        "window_expansion_procedure": (
            "deterministic backward quarterly expansion from 2026-06-01, hard cap 2024-01-01; "
            "see capacity-search.py and s5-protocol-final.md section 5"
        ),
        "capacity_ledger_row_count": len(ledger),
        "capacity_ledger_summary": [
            {k: r[k] for k in (
                "window_start", "window_end", "status", "survivor_count",
                "s5a_count", "r4_selected_count_after_a", "criterion_met",
            ) if k in r}
            for r in ledger
        ],
        "satisfying_window": satisfying_row,
        "exclusion_addendum": {
            "s4_examined_pr_count": addendum["s4_examined_pr_count"],
            "cross_checked_against": addendum["cross_checked_against"],
        },
        "freeze": freeze_meta,
    }
    out_path = HERE / "provenance.json"
    out_path.write_text(json.dumps(out, indent=2) + "\n")
    print(f"wrote {out_path}")
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
