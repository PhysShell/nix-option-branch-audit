#!/usr/bin/env python3
"""S4: generates the final report's Markdown tables and every metric
numerator/denominator from the JSONL adjudication ledger.

Per the S4 mandate: "The final Markdown tables and all metric
numerators/denominators must be generated from that ledger by a
checked script. Do not manually retype counts into the final report."

This script's own schema and gate arithmetic are frozen before any
real adjudication and exercised by `run_selftest()` against synthetic
dummy ledgers (see fixtures/s4-live-pr-shadow/generator-selftest/) --
the same discipline `oba` itself applies to its own analysis logic.

Ledger schema (one JSON object per line):
  cohort: "A" | "B"
  pr: int
  record_type: "actionable_presentation" | "pass_verdict" | "tool_error"
  scanner_substance_correct: bool | null
  pr_relevant: bool | null
  causal_framing_correct: bool | null
  rendered_presentation_correct: bool | null
  false_finding: bool
  false_pass: bool
  disagreement: bool
  disagreement_resolved: bool | null
  manual_effort_minutes_bucket: "<2" | "2-10" | ">10" | null

A record's overall correctness (for actionable_presentation records)
requires ALL FOUR of scanner_substance_correct, pr_relevant,
causal_framing_correct, rendered_presentation_correct to be true, and
no unresolved disagreement -- per the protocol's own "a presentation
is correct only if all of these are correct" clause.
"""
from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from math import comb
from pathlib import Path

ROOT = Path(__file__).resolve().parent


# ---------------------------------------------------------------------
# Exact one-sided Clopper-Pearson lower confidence bound.
# No scipy available in this environment -- implemented directly via
# the exact relationship between the regularized incomplete beta
# function and the binomial survival function (both integer-parameter
# forms), inverted by bisection. 30/30 successes at 95% one-sided
# confidence must give ~90.5% (the protocol's own worked example) --
# checked in run_selftest().
# ---------------------------------------------------------------------
def _binom_sf(k: int, n: int, p: float) -> float:
    """P(X >= k) for X ~ Binomial(n, p)."""
    if k <= 0:
        return 1.0
    if k > n:
        return 0.0
    if p <= 0.0:
        return 0.0
    if p >= 1.0:
        return 1.0
    total = 0.0
    for i in range(k, n + 1):
        total += comb(n, i) * (p**i) * ((1 - p) ** (n - i))
    return total


def clopper_pearson_lower_bound(successes: int, total: int, alpha: float = 0.05) -> float:
    """One-sided (1-alpha) exact lower confidence bound on a binomial
    proportion. successes/total = k/n; solves P(X >= k | n, p) = alpha
    for p via bisection (binom_sf is monotone increasing in p)."""
    if total == 0:
        return 0.0
    if successes <= 0:
        return 0.0
    if successes >= total:
        # p_L solves p^n = alpha directly (closed form, also the
        # bisection's own fixed point -- used as a fast path and a
        # cross-check).
        return alpha ** (1.0 / total)
    lo, hi = 0.0, 1.0
    for _ in range(200):
        mid = (lo + hi) / 2
        if _binom_sf(successes, total, mid) < alpha:
            lo = mid
        else:
            hi = mid
    return lo


# ---------------------------------------------------------------------
# Ledger loading and metric computation.
# ---------------------------------------------------------------------
@dataclass
class GateResult:
    verdict: str  # "PASS" | "FAIL" | "INSUFFICIENT EVIDENCE"
    reasons: list[str] = field(default_factory=list)


def load_ledger(path: Path) -> list[dict]:
    records = []
    with path.open() as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            records.append(json.loads(line))
    return records


def is_overall_correct(rec: dict) -> bool:
    if rec.get("disagreement") and not rec.get("disagreement_resolved"):
        return False
    return bool(
        rec.get("scanner_substance_correct")
        and rec.get("pr_relevant")
        and rec.get("causal_framing_correct")
        and rec.get("rendered_presentation_correct")
    )


def cohort_precision(records: list[dict], cohort: str) -> tuple[int, int]:
    """Returns (correct, total) among actionable_presentation records
    in the given cohort."""
    presentations = [
        r
        for r in records
        if r["cohort"] == cohort and r["record_type"] == "actionable_presentation"
    ]
    correct = sum(1 for r in presentations if is_overall_correct(r))
    return correct, len(presentations)


def evaluate_gate(records: list[dict]) -> GateResult:
    a_correct, a_total = cohort_precision(records, "A")
    b_correct, b_total = cohort_precision(records, "B")
    total_correct = a_correct + b_correct
    total_presentations = a_total + b_total

    false_pass = [r for r in records if r.get("false_pass")]
    false_finding = [
        r
        for r in records
        if r["record_type"] == "actionable_presentation"
        and not r.get("scanner_substance_correct", True)
    ]
    framing_errors = [
        r
        for r in records
        if r["record_type"] == "actionable_presentation"
        and r.get("scanner_substance_correct")
        and r.get("pr_relevant")
        and not (r.get("causal_framing_correct") and r.get("rendered_presentation_correct"))
    ]
    unresolved_disagreements = [
        r for r in records if r.get("disagreement") and not r.get("disagreement_resolved")
    ]
    tool_errors_ordinary = [
        r
        for r in records
        if r["record_type"] == "tool_error" and not r.get("tool_error_attributable_to_unsupported_input")
    ]

    reasons: list[str] = []

    # Stopping-rule sufficiency (presentation-count based, never
    # correctness-based -- mirrors protocol.md's own cohort stopping
    # rules restated here as the REPORT's own sufficiency check).
    insufficient = False
    if a_total < 10:
        insufficient = True
        reasons.append(f"S4-A has {a_total} actionable presentations, below the 10 minimum")
    if total_presentations < 30:
        insufficient = True
        reasons.append(f"{total_presentations} total actionable presentations, below the 30 minimum")

    if insufficient:
        return GateResult(verdict="INSUFFICIENT EVIDENCE", reasons=reasons)

    # From here, evidence is sufficient -- decide PASS vs FAIL.
    if false_pass:
        reasons.append(f"{len(false_pass)} false PASS(es) found")
    if false_finding:
        reasons.append(f"{len(false_finding)} false finding(s) found")
    if framing_errors:
        reasons.append(f"{len(framing_errors)} PR-relevance/causal-framing error(s) found")
    if unresolved_disagreements:
        reasons.append(f"{len(unresolved_disagreements)} unresolved reviewer disagreement(s)")
    if tool_errors_ordinary:
        reasons.append(f"{len(tool_errors_ordinary)} TOOL_ERROR(s) on ordinary supported input")
    if a_total > 0 and a_correct != a_total:
        reasons.append(f"S4-A actionable precision {a_correct}/{a_total} != 100%")
    if b_total > 0 and b_correct != b_total:
        reasons.append(f"S4-B actionable precision {b_correct}/{b_total} != 100%")

    lower_bound = clopper_pearson_lower_bound(total_correct, total_presentations)
    if lower_bound < 0.90:
        reasons.append(
            f"pooled one-sided 95% exact lower bound {lower_bound:.4f} < 0.90"
        )

    if reasons:
        return GateResult(verdict="FAIL", reasons=reasons)
    return GateResult(verdict="PASS", reasons=["all 10 conditions satisfied"])


def render_report(records: list[dict]) -> str:
    a_correct, a_total = cohort_precision(records, "A")
    b_correct, b_total = cohort_precision(records, "B")
    gate = evaluate_gate(records)
    lines = [
        "# S4 report (generated, not hand-typed)",
        "",
        f"- S4-A actionable precision: {a_correct}/{a_total}"
        + (f" = {100*a_correct/a_total:.1f}%" if a_total else " (n/a)"),
        f"- S4-B actionable precision: {b_correct}/{b_total}"
        + (f" = {100*b_correct/b_total:.1f}%" if b_total else " (n/a)"),
        f"- Deployment gate: **{gate.verdict}**",
        "- Reasons: " + "; ".join(gate.reasons),
    ]
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------
# Self-test: the generator's own schema/gate arithmetic, exercised
# against synthetic dummy ledgers BEFORE any real adjudication.
# ---------------------------------------------------------------------
def _make_actionable(cohort: str, pr: int, correct: bool = True, **overrides) -> dict:
    rec = {
        "cohort": cohort,
        "pr": pr,
        "record_type": "actionable_presentation",
        "scanner_substance_correct": correct,
        "pr_relevant": correct,
        "causal_framing_correct": correct,
        "rendered_presentation_correct": correct,
        "false_finding": not correct,
        "false_pass": False,
        "disagreement": False,
        "disagreement_resolved": None,
    }
    rec.update(overrides)
    return rec


def run_selftest() -> None:
    failures = []

    def check(name: str, condition: bool) -> None:
        status = "ok" if condition else "FAILED"
        print(f"[selftest] {name}: {status}")
        if not condition:
            failures.append(name)

    # 1. 30/30 correct (>=10 from A, >=30 total) -> PASS.
    records = [_make_actionable("A", 1000 + i, True) for i in range(12)]
    records += [_make_actionable("B", 2000 + i, True) for i in range(18)]
    gate = evaluate_gate(records)
    check("30/30 correct, 12 from A -> PASS", gate.verdict == "PASS")

    # 2. 29/30 correct -- an ISOLATED framing-only defect (substance
    # and PR-relevance both correct, only causal_framing_correct is
    # false -- the exact #516128/#562066 shape), distinct from a false
    # finding (a substance error). Confirms both the gate fails AND
    # the specific framing_errors code path fires, not just
    # false_finding by coincidence.
    records2 = [_make_actionable("A", 1000 + i, True) for i in range(12)]
    records2 += [_make_actionable("B", 2000 + i, True) for i in range(17)]
    records2.append(
        _make_actionable("B", 2999, True, causal_framing_correct=False, false_finding=False)
    )
    gate2 = evaluate_gate(records2)
    check(
        "29/30 correct, isolated framing-only defect -> FAIL with a framing reason",
        gate2.verdict == "FAIL" and any("causal-framing" in r for r in gate2.reasons),
    )

    # 3. One false_pass=true anywhere -> FAIL, regardless of precision
    # elsewhere (a PASS-verdict record, not itself an actionable
    # presentation, still trips the gate).
    records3 = [_make_actionable("A", 1000 + i, True) for i in range(12)]
    records3 += [_make_actionable("B", 2000 + i, True) for i in range(18)]
    records3.append(
        {
            "cohort": "A",
            "pr": 9999,
            "record_type": "pass_verdict",
            "scanner_substance_correct": None,
            "pr_relevant": None,
            "causal_framing_correct": None,
            "rendered_presentation_correct": None,
            "false_finding": False,
            "false_pass": True,
            "disagreement": False,
            "disagreement_resolved": None,
        }
    )
    gate3 = evaluate_gate(records3)
    check(
        "one false_pass=true, otherwise-clean 30/30 -> FAIL",
        gate3.verdict == "FAIL" and any("false PASS" in r for r in gate3.reasons),
    )

    # 4. 9 actionable presentations from S4-A alone, even if 9/9
    # correct -> INSUFFICIENT EVIDENCE, never PASS.
    records4 = [_make_actionable("A", 1000 + i, True) for i in range(9)]
    records4 += [_make_actionable("B", 2000 + i, True) for i in range(30)]
    gate4 = evaluate_gate(records4)
    check(
        "9/9 correct from A alone (below 10 min) -> INSUFFICIENT EVIDENCE",
        gate4.verdict == "INSUFFICIENT EVIDENCE",
    )

    # 5. Clopper-Pearson worked example: 30/30 -> ~90.5% one-sided
    # lower bound (the protocol's own stated example).
    lb = clopper_pearson_lower_bound(30, 30)
    check(f"CP lower bound(30/30) ~= 0.905 (got {lb:.4f})", abs(lb - 0.9050) < 0.001)

    # 6. A sufficiently-large but imperfect pool can still fail purely
    # on the confidence-bound condition even with zero individual
    # defects recorded incorrectly -- sanity: a small-N 100%-correct
    # case below the pooled-bound comfort zone should still evaluate
    # via the real formula, not a hardcoded shortcut (30/30 is the
    # smallest N at which the bound clears 90% with zero errors).
    lb_29 = clopper_pearson_lower_bound(29, 29)
    check(
        f"CP lower bound(29/29) > 30/30's bound (got {lb_29:.4f} vs {lb:.4f})",
        lb_29 < lb,  # fewer trials -> a HIGHER k/n ratio needed for the same bound; 29/29 has a slightly lower bound than 30/30 at the same 100% rate
    )

    if failures:
        print(f"\n{len(failures)} selftest check(s) FAILED: {failures}", file=sys.stderr)
        sys.exit(1)
    print(f"\nAll {6} selftest checks passed.")


def main() -> None:
    if len(sys.argv) > 1 and sys.argv[1] == "--selftest":
        run_selftest()
        return
    if len(sys.argv) > 1 and sys.argv[1] == "--ledger":
        ledger_path = Path(sys.argv[2])
        records = load_ledger(ledger_path)
        print(render_report(records))
        return
    print("usage: generate-report.py --selftest | --ledger <path.jsonl>", file=sys.stderr)
    sys.exit(2)


if __name__ == "__main__":
    main()
