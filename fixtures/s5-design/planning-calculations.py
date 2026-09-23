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
binom_sf = s4gen._binom_sf


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

    # --- Section 9: precision denominators and their one-sided lower
    # bounds. STATISTICAL-UNIT CORRECTION: n/k here count DISTINCT
    # ACTIONABLE PRs, not actionable presentations -- a PR contributing
    # several presentations (S4's own #538802 contributed 4 of the
    # historical corpus's 9) is not several independent Bernoulli
    # trials; it is one PR-level observation, correct only if EVERY
    # presentation within it is independently adjudicated correct.
    # Every individual presentation still gets full two-reviewer
    # adjudication and any single wrong one is still a Tier-1 FAIL --
    # only this confidence-bound denominator changes. The bound's own
    # arithmetic (clopper_pearson_lower_bound) is unit-agnostic, so no
    # code change was needed here, only the interpretation of n/k. ---
    denominators = [20, 25, 30, 35, 40, 49]
    precision_bounds = []
    for n in denominators:
        # "N/N correct" (100% observed precision, N distinct actionable
        # PRs), the pre-registered target shape -- also show
        # one-miss-out-of-N for contrast.
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
    # guaranteed future sample size. STATISTICAL-UNIT CORRECTION: uses
    # `actionable_prs_per_selected_pr` (distinct actionable PRs per
    # selected PR), not `actionable_per_selected_pr` (presentations per
    # selected PR) -- a PR-volume target of 30 means 30 DISTINCT
    # actionable PRs, matching Section 9's own corrected unit above. ---
    rule_comparison_path = Path(__file__).resolve().parent / "rule-comparison.jsonl"
    volume_planning = []
    if rule_comparison_path.exists():
        rules = [json.loads(l) for l in rule_comparison_path.read_text().splitlines() if l.strip()]
        for r in rules:
            yield_per_selected = r["actionable_prs_per_selected_pr"]
            sel_rate = r["selection_rate"]
            row = {"rule": r["rule"], "historical_actionable_prs_per_selected_pr": yield_per_selected,
                   "historical_selection_rate": sel_rate, "targets": []}
            for target in (20, 30, 40):
                if yield_per_selected > 0 and sel_rate > 0:
                    selected_needed = target / yield_per_selected
                    candidates_needed = selected_needed / sel_rate
                    row["targets"].append({
                        "distinct_actionable_pr_target": target,
                        "selected_prs_needed_approx": round(selected_needed, 1),
                        "candidate_prs_needed_approx": round(candidates_needed),
                    })
                else:
                    row["targets"].append({"distinct_actionable_pr_target": target, "selected_prs_needed_approx": None, "candidate_prs_needed_approx": None})
            volume_planning.append(row)
    out["volume_planning_not_a_guarantee"] = volume_planning

    # --- Section 10 (new): S5-B selected-PR cap, mechanically derived
    # (not copied from any prior hand-typed estimate). Under a simple
    # IID planning approximation -- each R4-selected PR independently
    # actionable at R4's own historical whole-corpus rate,
    # actionable_prs_per_selected_pr = 6/34 -- find the smallest
    # selected-PR cap N such that P(Binomial(N, p) >= 30) >= 0.95,
    # reusing s4gen's own `_binom_sf` (already verified against the
    # 30/30 -> ~0.9050 reference) rather than a new tail-probability
    # implementation. This is explicitly PLANNING ARITHMETIC ONLY: the
    # real S5-B stopping rule never uses this probability as a gate
    # input, only as a pre-registered cap chosen before any real draw. ---
    r4_row = None
    if rule_comparison_path.exists():
        for l in rule_comparison_path.read_text().splitlines():
            if not l.strip():
                continue
            r = json.loads(l)
            if r["rule"] == "R4_mkoption_line_edit_v1":
                r4_row = r
                break

    cap_planning = None
    if r4_row is not None:
        p = r4_row["actionable_prs_per_selected_pr"]
        target = 30
        target_prob = 0.95
        n = target
        while n < 2000 and binom_sf(target, n, p) < target_prob:
            n += 1
        derived_cap = n
        cap_planning = {
            "assumed_yield_rate_source": "R4_mkoption_line_edit_v1 whole-S4-corpus actionable_prs_per_selected_pr",
            "assumed_yield_rate": p,
            "distinct_actionable_pr_target": target,
            "target_probability_of_reaching_target": target_prob,
            "mechanically_derived_minimum_cap": derived_cap,
            "p_reach_target_at_derived_cap": round(binom_sf(target, derived_cap, p), 4),
            "expected_actionable_prs_at_derived_cap": round(derived_cap * p, 1),
            "p_reach_target_at_cap_220_for_reference": round(binom_sf(target, 220, p), 4),
            "note": (
                "IID planning approximation only, not a gate input. "
                "The real S5-B stopping rule stops on volume only at "
                "30 distinct actionable PRs or this frozen cap, "
                "whichever comes first; correctness never affects "
                "stopping."
            ),
        }
    out["s5b_selected_pr_cap_planning"] = cap_planning

    out_path = Path(__file__).resolve().parent / "planning-calculations.json"
    out_path.write_text(json.dumps(out, indent=2) + "\n")
    print(f"wrote {out_path}")
    for row in precision_bounds:
        print(f"  {row['n']}/{row['n']}: one-sided 95% lower bound = {row['one_sided_95_lower_bound_all_correct']:.4f}")
    for row in volume_planning:
        print(f"  {row['rule']}: " + ", ".join(
            f"target={t['distinct_actionable_pr_target']}->~{t['candidate_prs_needed_approx']} candidates"
            for t in row["targets"]
        ))
    if cap_planning:
        print(f"  S5-B cap planning: {cap_planning}")


if __name__ == "__main__":
    main()
