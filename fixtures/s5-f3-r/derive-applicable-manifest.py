#!/usr/bin/env python3
"""S5-F3-R: independently re-derive the frozen applicable/non-applicable
partition from the raw S5 ledger. Confirms the re-derived applicable-PR
set is IDENTICAL to fixtures/s5-f2-r/applicable-manifest.json's own PR
set. Adapted verbatim from fixtures/s5-f2-r/derive-applicable-manifest.py."""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
D = Path(__file__).resolve().parent


def main():
    ledger_path = ROOT / "fixtures/s5-live-pr-shadow/adjudication-ledger.jsonl"
    rows = [json.loads(line) for line in ledger_path.read_text().splitlines() if line.strip()]
    pr_summaries = [r for r in rows if r.get("record_type") == "pr_summary"]

    total = len(pr_summaries)
    applicable = [r for r in pr_summaries if r.get("applicable")]
    non_applicable = [r for r in pr_summaries if not r.get("applicable")]

    print(f"total pr_summary records: {total}")
    print(f"applicable: {len(applicable)}")
    print(f"non_applicable: {len(non_applicable)}")

    if total != 369:
        print(f"FATAL: expected 369 total PR records, got {total}")
        sys.exit(1)

    derived_applicable_prs = {r["pr"] for r in applicable}

    f2r_manifest_path = ROOT / "fixtures/s5-f2-r/applicable-manifest.json"
    f2r_manifest = json.loads(f2r_manifest_path.read_text())
    f2r_prs = {e["pr"] for e in f2r_manifest}

    if derived_applicable_prs != f2r_prs:
        print("FATAL: re-derived applicable PR set does not match the frozen S5-F2-R manifest")
        print("  only in ledger-derived:", sorted(derived_applicable_prs - f2r_prs))
        print("  only in f2-r manifest:", sorted(f2r_prs - derived_applicable_prs))
        sys.exit(1)

    print(f"CONFIRMED: {len(derived_applicable_prs)} applicable PRs match fixtures/s5-f2-r/applicable-manifest.json exactly -- same PR selection, independently re-derived from the raw ledger.")

    import shutil
    shutil.copy(f2r_manifest_path, D / "applicable-manifest.json")
    print("wrote fixtures/s5-f3-r/applicable-manifest.json (byte-for-byte copy of the verified-identical f2-r manifest)")


if __name__ == "__main__":
    main()
