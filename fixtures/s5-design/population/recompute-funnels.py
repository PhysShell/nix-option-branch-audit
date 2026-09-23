#!/usr/bin/env python3
"""Bookkeeping-only fix: adds the `excl_path_ineligible` funnel field
(previously silently dropped by an uncounted `continue` in
run_screen()/screen.py's path-eligibility check -- 2 records per
window, real PRs found via commit-path search whose own full PR file
list doesn't actually touch a watched path) and regenerates every
attempt's funnel.json plus capacity-ledger.jsonl's exclusion-breakdown
fields.

Reads ONLY already-committed attempts/*/raw.jsonl -- NO network, NO
re-fetch. Verifies the recomputed survivors SET is byte-identical
(same PR numbers) to the existing committed survivors.json for every
window before writing anything -- if it isn't, this stops and raises
rather than silently changing the frozen population.
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
DESIGN_DIR = HERE.parent

sys.path.insert(0, str(DESIGN_DIR / "selector"))

import importlib.util
spec = importlib.util.spec_from_file_location("capsearch", HERE / "capacity-search.py")
capsearch = importlib.util.module_from_spec(spec)
sys.modules["capsearch"] = capsearch
spec.loader.exec_module(capsearch)


def main():
    screen_mod = capsearch.load_screen_funcs()
    ledger = [json.loads(l) for l in (HERE / "capacity-ledger.jsonl").read_text().splitlines() if l.strip()]

    new_ledger = []
    for row in ledger:
        window_start = row["window_start"]
        attempt_dir = HERE / "attempts" / window_start
        records = [json.loads(l) for l in (attempt_dir / "raw.jsonl").read_text().splitlines() if l.strip()]
        survivors, funnel = capsearch.run_screen(records, window_start, screen_mod)

        existing_survivors = json.loads((attempt_dir / "survivors.json").read_text())
        existing_numbers = sorted(r["number"] for r in existing_survivors)
        new_numbers = sorted(r["number"] for r in survivors)
        assert existing_numbers == new_numbers, (
            f"SURVIVOR SET CHANGED for window {window_start} -- stopping, not overwriting. "
            f"existing={len(existing_numbers)} new={len(new_numbers)}"
        )
        assert funnel["survived"] == row["survivor_count"], (
            f"survivor_count changed for {window_start}: {row['survivor_count']} -> {funnel['survived']}"
        )

        total = (
            funnel["excl_already_examined_pr_number"] + funnel["excl_path_ineligible"]
            + funnel["excl_window"] + funnel["excl_docs"] + funnel["excl_mechanical"]
            + funnel["excl_name_or_path"] + funnel["survived"]
        )
        assert total == funnel["population"], (
            f"reconciliation still fails for {window_start}: {total} != {funnel['population']}"
        )

        (attempt_dir / "funnel.json").write_text(json.dumps(funnel, indent=2))

        new_row = dict(row)
        new_row["excl_path_ineligible"] = funnel["excl_path_ineligible"]
        # re-insert to keep excl_path_ineligible positioned right after excl_already_examined,
        # matching capacity-search.py's own field order
        ordered = {}
        for k, v in row.items():
            ordered[k] = v
            if k == "excl_already_examined":
                ordered["excl_path_ineligible"] = funnel["excl_path_ineligible"]
        new_ledger.append(ordered)
        print(f"{window_start}: raw={funnel['population']} already_examined={funnel['excl_already_examined_pr_number']} "
              f"path_ineligible={funnel['excl_path_ineligible']} window={funnel['excl_window']} "
              f"docs={funnel['excl_docs']} mechanical={funnel['excl_mechanical']} "
              f"name_or_path={funnel['excl_name_or_path']} survived={funnel['survived']} "
              f"(reconciles: {total == funnel['population']})", file=sys.stderr)

    with (HERE / "capacity-ledger.jsonl").open("w") as f:
        for row in new_ledger:
            f.write(json.dumps(row) + "\n")
    print(f"\nrewrote capacity-ledger.jsonl ({len(new_ledger)} rows) and every attempts/*/funnel.json", file=sys.stderr)

    # Top-level canonical funnel.json mirrors the satisfying (final) window.
    final_window = next(r["window_start"] for r in new_ledger if r.get("criterion_met"))
    (HERE / "funnel.json").write_text((HERE / "attempts" / final_window / "funnel.json").read_text())
    print(f"refreshed top-level funnel.json from attempts/{final_window}/funnel.json", file=sys.stderr)


if __name__ == "__main__":
    main()
