#!/usr/bin/env python3
"""S5 GO, section 0: diagnostic-only path-overlap annotation.

Metadata-only. Does NOT alter either frozen order, does not exclude
any PR, does not change the selector, does not affect stopping,
adjudication, or the gate. Exists only to make explicit, before any
analyzer result exists, that S5 PRs are fresh PRs but not necessarily
previously-unseen module SUBJECTS: some nixos/modules/services/**
paths touched by S5's frozen cohorts were already touched by S4's own
187 examined PRs, and some paths recur across multiple S5 PRs within
S5 itself.

Reads only already-committed, already-frozen inputs (no network):
  - fixtures/s5-design/s5a-frozen-order.json
  - fixtures/s5-design/s5b-frozen-order.json
  - fixtures/s4-live-pr-shadow/population-raw.jsonl
  - fixtures/s5-design/population/s5-exclusion-addendum.json (S4's
    187 examined PR numbers)

Writes fixtures/s5-design/path-overlap-diagnostics.jsonl, one row per
frozen S5 PR (S5-A then S5-B, in each cohort's own frozen order).
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
D = Path(__file__).resolve().parent


def service_paths(rec: dict) -> list[str]:
    return sorted(
        {f["path"] for f in rec.get("files", []) if f["path"].startswith("nixos/modules/services/")}
    )


def main():
    s4_examined = json.loads((D / "population/s5-exclusion-addendum.json").read_text())["s4_examined_pr_numbers"]
    s4_raw = {
        r["number"]: r
        for r in (json.loads(l) for l in (ROOT / "fixtures/s4-live-pr-shadow/population-raw.jsonl").read_text().splitlines() if l.strip())
    }
    missing = [pr for pr in s4_examined if pr not in s4_raw]
    assert not missing, f"S4-examined PRs missing from population-raw.jsonl: {missing}"

    s4_service_paths: set[str] = set()
    for pr in s4_examined:
        s4_service_paths.update(service_paths(s4_raw[pr]))

    s5a = json.loads((D / "s5a-frozen-order.json").read_text())
    s5b = json.loads((D / "s5b-frozen-order.json").read_text())

    # Path -> list of (cohort, pr) sharing it, across the COMBINED S5
    # population -- this is what "s5_service_path_cluster membership"
    # means: which other frozen S5 PRs touch the exact same path.
    path_to_prs: dict[str, list[tuple[str, int]]] = {}
    for cohort_label, cohort in (("A", s5a), ("B", s5b)):
        for r in cohort:
            for p in service_paths(r):
                path_to_prs.setdefault(p, []).append((cohort_label, r["number"]))

    rows = []
    for cohort_label, cohort in (("A", s5a), ("B", s5b)):
        for r in cohort:
            sp = service_paths(r)
            overlapping = sorted(p for p in sp if p in s4_service_paths)
            cluster = {}
            for p in sp:
                others = [pr for (c, pr) in path_to_prs[p] if pr != r["number"]]
                if others:
                    cluster[p] = sorted(others)
            rows.append({
                "pr": r["number"],
                "cohort": cohort_label,
                "service_module_paths": sp,
                "overlaps_s4_service_path": bool(overlapping),
                "s4_overlapping_paths": overlapping,
                "s5_shared_path_cluster": cluster,
            })

    out_path = D / "path-overlap-diagnostics.jsonl"
    with out_path.open("w") as f:
        for row in rows:
            f.write(json.dumps(row) + "\n")

    # Summary stats, printed and also written for the protocol note to
    # cite verbatim (never hand-typed into prose).
    def summarize(cohort_label, cohort):
        n = len(cohort)
        path_counter: dict[str, int] = {}
        overlap_n = 0
        for r in cohort:
            sp = service_paths(r)
            for p in sp:
                path_counter[p] = path_counter.get(p, 0) + 1
            if any(p in s4_service_paths for p in sp):
                overlap_n += 1
        repeated = {p: c for p, c in path_counter.items() if c > 1}
        return {
            "cohort": cohort_label,
            "n": n,
            "distinct_service_paths": len(path_counter),
            "repeated_paths_within_s5": len(repeated),
            "max_repeat_count": max(path_counter.values()) if path_counter else 0,
            "overlaps_s4_service_path_count": overlap_n,
            "overlaps_s4_service_path_fraction": round(overlap_n / n, 4) if n else None,
        }

    summary = [summarize("A", s5a), summarize("B", s5b)]
    (D / "path-overlap-summary.json").write_text(json.dumps(summary, indent=2) + "\n")

    print(f"wrote {out_path} ({len(rows)} rows)")
    print(f"S4 distinct examined service-module paths: {len(s4_service_paths)}")
    for s in summary:
        print(f"  S5-{s['cohort']}: n={s['n']} distinct_paths={s['distinct_service_paths']} "
              f"repeated={s['repeated_paths_within_s5']} max_repeat={s['max_repeat_count']} "
              f"s4_overlap={s['overlaps_s4_service_path_count']}/{s['n']} "
              f"({s['overlaps_s4_service_path_fraction']*100:.1f}%)")


if __name__ == "__main__":
    main()
