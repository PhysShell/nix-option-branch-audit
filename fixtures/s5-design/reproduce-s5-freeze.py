#!/usr/bin/env python3
"""S5: regenerates S5-A and S5-B's frozen orders purely from committed
inputs, and asserts byte-for-byte (PR-number-sequence) equality with
the already-committed `s5a-frozen-order.json`/`s5b-frozen-order.json`.

Mirrors fixtures/s4-live-pr-shadow/reproduce-freeze.py's own exact
pattern: this is VERIFICATION of the existing freeze, not
authorization to redraw it. If reproduction disagrees with the current
frozen orders, this script exits non-zero and prints the discrepancy
-- the correct response is to STOP and report, never to silently
replace the committed cohorts with whatever this script just produced.

Reads only:
  - fixtures/s5-design/population/survivors.json (committed)
  - fixtures/s5-design/population/s5b-all-selection-records.jsonl
    (committed, the exact frozen selector output for every
    remaining-after-S5-A PR)

No network access, no /tmp dependency, no undocumented source of
truth. Runs in CI on every push touching fixtures/s5-design/**.
"""
import json
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent

# The dereferenced COMMIT sha v0.4.5's tag points to (confirmed via
# `git rev-parse --short=7 v0.4.5^{commit}`), matching S4's exact
# convention of seeding from the frozen binary's own commit -- not the
# annotated tag object's own separate sha.
SHORT_SHA = "8f1701a"

S5A_TARGET = 150
S5B_CAP = 219  # mechanically derived in planning-calculations.py; see s5-protocol-final.md


def numbers(rows: list[dict]) -> list[int]:
    return [r["number"] for r in rows]


def main() -> int:
    survivors_path = ROOT / "population" / "survivors.json"
    records_path = ROOT / "population" / "s5b-all-selection-records.jsonl"
    a_path = ROOT / "s5a-frozen-order.json"
    b_path = ROOT / "s5b-frozen-order.json"

    for p in (survivors_path, records_path, a_path, b_path):
        if not p.exists():
            print(f"MISSING: {p} -- freeze not yet performed, nothing to reproduce", file=sys.stderr)
            return 1

    survivors = json.loads(survivors_path.read_text())
    committed_a = json.loads(a_path.read_text())
    committed_b = json.loads(b_path.read_text())

    print(f"survivors: {len(survivors)}")

    # --- S5-A reproduction ---
    seed_a = int(SHORT_SHA, 16)
    pool = list(survivors)
    random.Random(seed_a).shuffle(pool)
    reproduced_a = pool[: min(S5A_TARGET, len(pool))]

    a_match = numbers(reproduced_a) == numbers(committed_a)
    print(f"S5-A reproduction: {'MATCH' if a_match else 'MISMATCH'} "
          f"({len(reproduced_a)} vs {len(committed_a)} committed)")

    # --- S5-B reproduction ---
    a_numbers = set(numbers(reproduced_a))
    remaining = [r for r in pool if r["number"] not in a_numbers]

    selection_records = {}
    for line in records_path.read_text().splitlines():
        if not line.strip():
            continue
        rec = json.loads(line)
        selection_records[rec["pr"]] = rec

    remaining_numbers = {r["number"] for r in remaining}
    record_numbers = set(selection_records.keys())
    if remaining_numbers != record_numbers:
        print(
            f"MISMATCH: selection_records must cover every remaining PR exactly once -- "
            f"missing={remaining_numbers - record_numbers}, extra={record_numbers - remaining_numbers}",
            file=sys.stderr,
        )
        return 1

    eligible = [
        r for r in remaining
        if selection_records[r["number"]]["selected"] is True
        and selection_records[r["number"]]["unresolved"] is False
    ]
    print(f"selected-and-resolved eligible for S5-B: {len(eligible)} / {len(remaining)} remaining")

    seed_b = seed_a + 1
    pool_b = list(eligible)
    random.Random(seed_b).shuffle(pool_b)
    reproduced_b = pool_b[: min(S5B_CAP, len(pool_b))]

    b_match = numbers(reproduced_b) == numbers(committed_b)
    print(f"S5-B reproduction: {'MATCH' if b_match else 'MISMATCH'} "
          f"({len(reproduced_b)} vs {len(committed_b)} committed)")

    # Disjointness, mechanically re-checked here too (not just in
    # test_s5_protocol_machinery.py) since this script is the one that
    # actually runs in CI on every push.
    disjoint = a_numbers.isdisjoint(numbers(reproduced_b))
    print(f"S5-A / S5-B disjoint: {disjoint}")

    if a_match and b_match and disjoint:
        print("\nREPRODUCTION CONFIRMED: both frozen orders regenerate byte-for-byte "
              "identically from committed inputs alone.")
        return 0
    else:
        print("\nREPRODUCTION FAILED -- DO NOT silently replace the committed cohorts. "
              "STOP and report this discrepancy.", file=sys.stderr)
        if not a_match:
            print(f"  S5-A reproduced: {numbers(reproduced_a)[:10]}...", file=sys.stderr)
            print(f"  S5-A committed:  {numbers(committed_a)[:10]}...", file=sys.stderr)
        if not b_match:
            print(f"  S5-B reproduced: {numbers(reproduced_b)[:10]}...", file=sys.stderr)
            print(f"  S5-B committed:  {numbers(committed_b)[:10]}...", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
