#!/usr/bin/env python3
"""S5-F1B-R: independently re-derive the frozen applicable/non-applicable
partition from the raw S5 ledger (fixtures/s5-live-pr-shadow/adjudication-ledger.jsonl),
rather than assuming the S5-F1-R applicable-manifest.json is still correct.
This is a reproducibility/integrity check, not a re-adjudication -- the
ledger itself is the frozen ground truth, untouched by this round.

Verifies:
  - 369 total pr_summary records
  - the applicable/non-applicable partition (expected 180/189, but derived,
    not assumed)
  - that the re-derived applicable-PR set is IDENTICAL to the frozen
    fixtures/s5-f1-r/applicable-manifest.json's own PR set (the same 180
    identities must be replayed -- "same PR selection" per the mandate)

Writes nothing to the ledger or to fixtures/s5-f1-r/ (read-only reference).
Writes fixtures/s5-f1b-r/applicable-manifest.json as a byte-for-byte copy
of fixtures/s5-f1-r/applicable-manifest.json, ONLY after confirming the PR
sets match exactly -- i.e. this script's own output is provably the same
180 frozen identities, not independently reconstructed base_sha/head_sha
values (those still come from the S5-F1-R manifest, itself already
S5-R0/S5-verified).
"""
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

    f1r_manifest_path = ROOT / "fixtures/s5-f1-r/applicable-manifest.json"
    f1r_manifest = json.loads(f1r_manifest_path.read_text())
    f1r_prs = {e["pr"] for e in f1r_manifest}

    if derived_applicable_prs != f1r_prs:
        print("FATAL: re-derived applicable PR set does not match the frozen S5-F1-R manifest")
        print("  only in ledger-derived:", sorted(derived_applicable_prs - f1r_prs))
        print("  only in f1-r manifest:", sorted(f1r_prs - derived_applicable_prs))
        sys.exit(1)

    print(f"CONFIRMED: {len(derived_applicable_prs)} applicable PRs match fixtures/s5-f1-r/applicable-manifest.json exactly -- same PR selection, independently re-derived from the raw ledger.")

    import shutil
    shutil.copy(f1r_manifest_path, D / "applicable-manifest.json")
    print("wrote fixtures/s5-f1b-r/applicable-manifest.json (byte-for-byte copy of the verified-identical f1-r manifest)")


if __name__ == "__main__":
    main()
