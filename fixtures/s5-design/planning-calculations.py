#!/usr/bin/env python3
"""S5 design round: planning arithmetic only -- NOT a guarantee of
future sample size, NOT an S5 result.

Reuses `clopper_pearson_lower_bound`/`_binom_sf` verbatim from
`fixtures/s4-live-pr-shadow/generate-report.py` (imported via
importlib, not copy-pasted) -- S4 already built, verified, and
self-tested this exact one-sided exact binomial lower-bound
implementation (against a known reference: 30/30 -> ~0.9050); this
round reuses it rather than re-deriving or re-verifying the same math
a second time, per this project's own reuse-before-reimplementation
discipline (CLAUDE.md).
"""
import importlib.util
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
GEN_PATH = ROOT / "fixtures/s4-live-pr-shadow/generate-report.py"

spec = importlib.util.spec_from_file_location("s4gen", GEN_PATH)
s4gen = importlib.util.module_from_spec(spec)
import sys
sys.modules["s4gen"] = s4gen
spec.loader.exec_module(s4gen)

clopper_pearson_lower_bound = s4gen.clopper_pearson_lower_bound


def wilson_or_exact_ci_width(k: int, n: int, alpha: float = 0.05) -> tuple[float, float]:
    """Two-sided exact Clopper-Pearson interval [lower, upper] for a
    representative-cohort RATE estimate (distinct use from the
    one-sided precision lower bound above -- prevalence/applicability
    estimation wants a two-sided interval, not a one-sided
    "at-least-this-good" claim).

    Built entirely by REUSING `clopper_pearson_lower_bound` twice, not
    a new bisection: the standard Clopper-Pearson closed-form identity
    is `lower(k,n,alpha) = one_sided_lower_bound(k,n,alpha/2)` and
    `upper(k,n,alpha) = 1 - one_sided_lower_bound(n-k,n,alpha/2)` (the
    success/failure symmetry of the binomial -- an upper limit on
    successes is a lower limit on failures, mirrored). A first version
    of this function used its own from-scratch two-tail bisection and
    produced visibly degenerate output (many 0/1 or 1/1 intervals) on
    first real use below -- caught immediately by actually running it,
    not assumed correct from the code alone. Rewritten to reuse the
    ALREADY-verified one-sided primitive (30/30 -> ~0.9050) instead of
    re-deriving the same bisection a second, buggier way.
    """
    if n == 0:
        return 0.0, 1.0
    lower = s4gen.clopper_pearson_lower_bound(k, n, alpha / 2)
    upper = 1.0 - s4gen.clopper_pearson_lower_bound(n - k, n, alpha / 2)
    return lower, upper


def main():
    out = {}

    # --- Section 9: precision denominators and their one-sided lower bounds ---
    denominators = [20, 25, 30, 35, 40, 49]
    precision_bounds = []
    for n in denominators:
        # "N/N correct" (100% observed precision), the pre-registered
        # target shape -- also show one-miss-out-of-N for contrast.
        lb_all_correct = clopper_pearson_lower_bound(n, n, alpha=0.05)
        lb_one_miss = clopper_pearson_lower_bound(n - 1, n, alpha=0.05) if n > 0 else None
        precision_bounds.append({
            "n": n,
            "successes_all_correct": n,
            "one_sided_95_lower_bound_all_correct": round(lb_all_correct, 4),
            "successes_one_miss": n - 1,
            "one_sided_95_lower_bound_one_miss": round(lb_one_miss, 4) if lb_one_miss is not None else None,
        })
    out["precision_lower_bounds"] = precision_bounds

    # --- Section 8: representative-cohort sample size vs two-sided CI width ---
    sample_sizes = [100, 150, 200]
    rates = [0.05, 0.10, 0.25, 0.50]
    ci_table = []
    for n in sample_sizes:
        for rate in rates:
            k = round(rate * n)
            lower, upper = wilson_or_exact_ci_width(k, n, alpha=0.05)
            ci_table.append({
                "n": n,
                "assumed_rate": rate,
                "observed_k": k,
                "two_sided_95_ci": [round(lower, 4), round(upper, 4)],
                "half_width": round((upper - lower) / 2, 4),
            })
    out["representative_cohort_ci_widths"] = ci_table

    # --- Section 7: PR-volume planning arithmetic, from the REAL
    # rule-comparison.jsonl (evaluate-rules.py's own output) -- never
    # hand-typed, and explicitly labelled as planning arithmetic, not a
    # guaranteed future sample size. ---
    rule_comparison_path = Path(__file__).resolve().parent / "rule-comparison.jsonl"
    volume_planning = []
    if rule_comparison_path.exists():
        rules = [json.loads(l) for l in rule_comparison_path.read_text().splitlines() if l.strip()]
        for r in rules:
            yield_per_selected = r["actionable_per_selected_pr"]
            sel_rate = r["selection_rate"]
            row = {"rule": r["rule"], "historical_yield_per_selected_pr": yield_per_selected,
                   "historical_selection_rate": sel_rate, "targets": []}
            for target in (20, 30, 40):
                if yield_per_selected > 0 and sel_rate > 0:
                    selected_needed = target / yield_per_selected
                    candidates_needed = selected_needed / sel_rate
                    row["targets"].append({
                        "actionable_target": target,
                        "selected_prs_needed_approx": round(selected_needed, 1),
                        "candidate_prs_needed_approx": round(candidates_needed),
                    })
                else:
                    row["targets"].append({"actionable_target": target, "selected_prs_needed_approx": None, "candidate_prs_needed_approx": None})
            volume_planning.append(row)
    out["volume_planning_not_a_guarantee"] = volume_planning

    out_path = Path(__file__).resolve().parent / "planning-calculations.json"
    out_path.write_text(json.dumps(out, indent=2) + "\n")
    print(f"wrote {out_path}")
    for row in precision_bounds:
        print(f"  {row['n']}/{row['n']}: one-sided 95% lower bound = {row['one_sided_95_lower_bound_all_correct']:.4f}")
    for row in volume_planning:
        print(f"  {row['rule']}: " + ", ".join(
            f"target={t['actionable_target']}->~{t['candidate_prs_needed_approx']} candidates"
            for t in row["targets"]
        ))


if __name__ == "__main__":
    main()
