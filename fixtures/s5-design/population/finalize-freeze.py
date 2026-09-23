#!/usr/bin/env python3
"""S5: finalizes the frozen S5-A/S5-B cohort orders from the
satisfying window capacity-search.py found (2025-03-01..2026-09-22),
using ONLY the committed attempt artifacts + the global selection
cache it already built -- no new fetching.

Writes fixtures/s5-design/s5a-frozen-order.json,
fixtures/s5-design/s5b-frozen-order.json,
fixtures/s5-design/population/s5b-all-selection-records.jsonl (one
record per remaining-after-S5-A PR, matching
ledger-schema-draft.json's own invariant), and
fixtures/s5-design/population/freeze-meta.json.
"""
import json
import random
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
DESIGN_DIR = HERE.parent

SHORT_SHA = "8f1701a"
SEED_A = int(SHORT_SHA, 16)
SEED_B = SEED_A + 1
S5A_TARGET = 150
S5B_CAP = 219

SATISFYING_WINDOW = "2025-03-01"


def load_jsonl_dict(path: Path, key: str) -> dict:
    out = {}
    for line in path.read_text().splitlines():
        if line.strip():
            row = json.loads(line)
            out[row[key]] = row
    return out


def main():
    attempt_dir = HERE / "attempts" / SATISFYING_WINDOW
    survivors = json.loads((attempt_dir / "survivors.json").read_text())
    print(f"survivors (window={SATISFYING_WINDOW}): {len(survivors)}", file=sys.stderr)

    pool = list(survivors)
    random.Random(SEED_A).shuffle(pool)
    s5a = pool[: min(S5A_TARGET, len(pool))]
    assert len(s5a) == S5A_TARGET, f"expected exactly {S5A_TARGET}, got {len(s5a)}"

    a_numbers = {r["number"] for r in s5a}
    remainder = [r for r in pool if r["number"] not in a_numbers]
    print(f"S5-A: {len(s5a)}, remainder: {len(remainder)}", file=sys.stderr)

    selection_cache = load_jsonl_dict(HERE / "cache" / "selection-cache.jsonl", "pr")
    remainder_numbers = {r["number"] for r in remainder}
    missing = remainder_numbers - set(selection_cache.keys())
    assert not missing, f"selection cache missing entries for remainder PRs: {missing}"

    records_path = HERE / "s5b-all-selection-records.jsonl"
    with records_path.open("w") as out:
        for r in remainder:
            out.write(json.dumps(selection_cache[r["number"]]) + "\n")
    print(f"wrote {records_path}: {len(remainder)} selection records (one per remainder PR)", file=sys.stderr)

    eligible = [
        r for r in remainder
        if selection_cache[r["number"]]["selected"] is True
        and selection_cache[r["number"]]["unresolved"] is False
    ]
    print(f"eligible for S5-B (selected & resolved): {len(eligible)}", file=sys.stderr)

    pool_b = list(eligible)
    random.Random(SEED_B).shuffle(pool_b)
    s5b = pool_b[: min(S5B_CAP, len(pool_b))]
    print(f"S5-B: {len(s5b)} (cap {S5B_CAP})", file=sys.stderr)

    b_numbers = {r["number"] for r in s5b}
    overlap = a_numbers & b_numbers
    assert not overlap, f"S5-A/S5-B must be disjoint by construction, overlap: {overlap}"

    (DESIGN_DIR / "s5a-frozen-order.json").write_text(json.dumps(s5a, indent=2))
    (DESIGN_DIR / "s5b-frozen-order.json").write_text(json.dumps(s5b, indent=2))

    n_selected = sum(1 for r in remainder if selection_cache[r["number"]]["selected"] is True and not selection_cache[r["number"]]["unresolved"])
    n_not_selected = sum(1 for r in remainder if selection_cache[r["number"]]["selected"] is False and not selection_cache[r["number"]]["unresolved"])
    n_unresolved = sum(1 for r in remainder if selection_cache[r["number"]]["unresolved"] is True)

    meta = {
        "satisfying_window_start": SATISFYING_WINDOW,
        "window_end": "2026-09-22",
        "short_sha": SHORT_SHA,
        "seed_a": SEED_A,
        "seed_b": SEED_B,
        "survivors": len(survivors),
        "s5a_target": S5A_TARGET,
        "s5a_realized": len(s5a),
        "remaining_after_s5a": len(remainder),
        "s5b_selection_selected": n_selected,
        "s5b_selection_not_selected": n_not_selected,
        "s5b_selection_unresolved": n_unresolved,
        "s5b_cap": S5B_CAP,
        "s5b_eligible_pool": len(eligible),
        "s5b_realized": len(s5b),
        "s5a_s5b_disjoint": True,
    }
    (HERE / "freeze-meta.json").write_text(json.dumps(meta, indent=2) + "\n")
    print(json.dumps(meta, indent=2))


if __name__ == "__main__":
    main()
