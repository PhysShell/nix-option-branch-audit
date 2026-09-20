#!/usr/bin/env python3
"""S4: validates the JSONL adjudication ledger against a strict schema
and generates every S4 metric/table/inventory/gate verdict from it.

Per the S4 mandate: "The final Markdown tables and all metric
numerators/denominators must be generated from that ledger by a
checked script. Do not manually retype counts into the final report."
And, on review: "The generator must reject malformed evidence rather
than silently interpreting it." A schema-validation failure is a
report-generation TOOL_ERROR, never an analytical PASS/FAIL result.

===========================================================================
LEDGER SCHEMA
===========================================================================

One JSON object per line. `record_type` selects the shape:

pr_summary -- one per PROCESSED PR, in frozen-order position order:
  record_type: "pr_summary"
  cohort: "A" | "B"
  position: int (1-indexed position in that cohort's frozen order)
  pr: int
  base_sha: str (real 40-hex-char git SHA)
  head_sha: str (real 40-hex-char git SHA)
  applicable: bool
  module_paths: list[str]
  test_paths: list[str]
  oba_binary_version: str
  oba_artifact_sha256: str (64-hex-char)
  command: str
  raw_json_ref: str
  raw_json_sha256: str (64-hex-char)
  rendered_summary_ref: str
  rendered_summary_sha256: str (64-hex-char)
  actionable_count: int (>=0)
  pass_verdict_count: int (>=0)
  tool_error_count: int (>=0)
  manual_effort_bucket: "<2" | "2-10" | ">10" | null
  gap_classification: "known_gap" | "new_gap" | null
  # optional, default 0/empty when absent (e.g. oba=no PRs never carry these):
  inconclusive_count: int (>=0, optional)
  inconclusive_root_causes: list[str] (optional, one entry per inconclusive instance)
  origin_unclear_count: int (>=0, optional)

actionable_presentation -- one per actionable presentation, with a
stable presentation_id and TWO independent reviewer judgments:
  record_type: "actionable_presentation"
  cohort: "A" | "B"
  pr: int
  presentation_id: str (stable, derived -- see presentation_id())
  engine: str
  subject: str
  bucket: str
  raw_output_index: int
  reviews: [review, review]  -- EXACTLY 2, distinct reviewer_id
    review = {
      reviewer_id: str
      scanner_substance_correct: bool
      pr_relevant: bool
      causal_framing_correct: bool
      rendered_presentation_correct: bool
      notes: str
    }
  coordinator_resolution: null | {
      resolved_by: str
      scanner_substance_correct: bool
      pr_relevant: bool
      causal_framing_correct: bool
      rendered_presentation_correct: bool
      rationale: str
    }

pass_adjudication -- one per PASS verdict requiring review:
  record_type: "pass_adjudication"
  cohort: "A" | "B"
  pr: int
  pass_id: str (stable, e.g. "<pr>:<engine>:<subject>")
  genuine_evidence: bool
  reviewer_id: str
  notes: str

tool_error -- one per TOOL_ERROR encountered:
  record_type: "tool_error"
  cohort: "A" | "B"
  pr: int
  attributable_to_unsupported_input: bool
  notes: str

protocol_note -- optional, documents an explicit, pre-declared reason
for a stopping-rule position that would otherwise be rejected:
  record_type: "protocol_note"
  cohort: "A" | "B"
  reason: str

There is deliberately no `false_finding` field anywhere: a
presentation's false-finding status is ALWAYS derived from
`scanner_substance_correct == False` on its resolved judgment, never
carried as a separate, potentially-inconsistent flag (item 10).
"""
from __future__ import annotations

import hashlib
import json
import sys
from dataclasses import dataclass, field
from math import comb
from pathlib import Path

ROOT = Path(__file__).resolve().parent

SHA40 = None  # set below after import re
import re  # noqa: E402

SHA40_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


# ---------------------------------------------------------------------
# Exact one-sided Clopper-Pearson lower confidence bound (unchanged
# from the prior version -- see its own docstring for why this is
# implemented from first principles, no scipy available here).
# ---------------------------------------------------------------------
def _binom_sf(k: int, n: int, p: float) -> float:
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
    if total == 0:
        return 0.0
    if successes <= 0:
        return 0.0
    if successes >= total:
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
# Schema validation.
# ---------------------------------------------------------------------
class ValidationError(Exception):
    def __init__(self, errors: list[str]):
        super().__init__("; ".join(errors))
        self.errors = errors


RECORD_TYPES = {
    "pr_summary",
    "actionable_presentation",
    "pass_adjudication",
    "tool_error",
    "protocol_note",
}

PR_SUMMARY_REQUIRED = {
    "cohort": str,
    "position": int,
    "pr": int,
    "base_sha": str,
    "head_sha": str,
    "applicable": bool,
    "module_paths": list,
    "test_paths": list,
    "oba_binary_version": str,
    "oba_artifact_sha256": str,
    "command": str,
    "raw_json_ref": str,
    "raw_json_sha256": str,
    "rendered_summary_ref": str,
    "rendered_summary_sha256": str,
    "actionable_count": int,
    "pass_verdict_count": int,
    "tool_error_count": int,
}

REVIEW_REQUIRED = {
    "reviewer_id": str,
    "scanner_substance_correct": bool,
    "pr_relevant": bool,
    "causal_framing_correct": bool,
    "rendered_presentation_correct": bool,
    "notes": str,
}

PRESENTATION_REQUIRED = {
    "cohort": str,
    "pr": int,
    "presentation_id": str,
    "engine": str,
    "subject": str,
    "bucket": str,
    "raw_output_index": int,
}

PASS_REQUIRED = {
    "cohort": str,
    "pr": int,
    "pass_id": str,
    "genuine_evidence": bool,
    "reviewer_id": str,
    "notes": str,
}

TOOL_ERROR_REQUIRED = {
    "cohort": str,
    "pr": int,
    "attributable_to_unsupported_input": bool,
    "notes": str,
}


def _check_fields(rec: dict, required: dict, ctx: str, errors: list[str]) -> None:
    for field_name, typ in required.items():
        if field_name not in rec:
            errors.append(f"{ctx}: missing required field '{field_name}'")
            continue
        val = rec[field_name]
        if typ is bool:
            if not isinstance(val, bool):
                errors.append(f"{ctx}: field '{field_name}' must be a bool, got {type(val).__name__}")
        elif typ is int:
            if not isinstance(val, int) or isinstance(val, bool):
                errors.append(f"{ctx}: field '{field_name}' must be an int, got {type(val).__name__}")
        elif typ is str:
            if not isinstance(val, str) or not val:
                errors.append(f"{ctx}: field '{field_name}' must be a non-empty str")
        elif typ is list:
            if not isinstance(val, list):
                errors.append(f"{ctx}: field '{field_name}' must be a list")


def validate_ledger(
    records: list[dict],
    s4a_order: list[int] | None = None,
    s4b_order: list[int] | None = None,
) -> list[str]:
    """Returns a list of validation errors. Empty list == valid.
    `s4a_order`/`s4b_order` are the frozen PR-number orders (position 1
    == index 0) -- when supplied, frozen-membership/position/prefix
    checks run too; when omitted (e.g. a unit test not exercising that
    axis), those specific checks are skipped."""
    errors: list[str] = []

    pr_summaries: dict[tuple[str, int], dict] = {}
    presentation_ids: set[str] = set()
    pass_ids: dict[tuple[str, int], set[str]] = {}
    seen_pr_in_cohort: set[tuple[str, int]] = set()

    for i, rec in enumerate(records):
        ctx = f"record[{i}]"
        rt = rec.get("record_type")
        if rt not in RECORD_TYPES:
            errors.append(f"{ctx}: unknown or missing record_type {rt!r}")
            continue

        cohort = rec.get("cohort")
        if rt != "protocol_note" or "cohort" in rec:
            if cohort not in ("A", "B"):
                errors.append(f"{ctx} ({rt}): cohort must be 'A' or 'B', got {cohort!r}")

        if rt == "pr_summary":
            _check_fields(rec, PR_SUMMARY_REQUIRED, f"{ctx} (pr_summary)", errors)
            if "base_sha" in rec and isinstance(rec["base_sha"], str) and not SHA40_RE.match(rec["base_sha"]):
                errors.append(f"{ctx}: base_sha is not a real 40-hex-char SHA: {rec['base_sha']!r}")
            if "head_sha" in rec and isinstance(rec["head_sha"], str) and not SHA40_RE.match(rec["head_sha"]):
                errors.append(f"{ctx}: head_sha is not a real 40-hex-char SHA: {rec['head_sha']!r}")
            for shafield in ("oba_artifact_sha256", "raw_json_sha256", "rendered_summary_sha256"):
                v = rec.get(shafield)
                if isinstance(v, str) and not SHA256_RE.match(v):
                    errors.append(f"{ctx}: {shafield} is not a real 64-hex-char SHA256: {v!r}")
            key = (cohort, rec.get("pr"))
            if key in seen_pr_in_cohort:
                errors.append(f"{ctx}: duplicate pr_summary for cohort={cohort} pr={rec.get('pr')}")
            seen_pr_in_cohort.add(key)
            pr_summaries[key] = rec
            if rec.get("manual_effort_bucket") not in ("<2", "2-10", ">10", None):
                errors.append(f"{ctx}: manual_effort_bucket has an invalid value {rec.get('manual_effort_bucket')!r}")
            if rec.get("gap_classification") not in ("known_gap", "new_gap", None):
                errors.append(f"{ctx}: gap_classification has an invalid value {rec.get('gap_classification')!r}")
            order = s4a_order if cohort == "A" else s4b_order
            if order is not None:
                pr = rec.get("pr")
                pos = rec.get("position")
                if isinstance(pr, int) and pr not in order:
                    errors.append(f"{ctx}: pr {pr} is not a member of the frozen S4-{cohort} order")
                elif isinstance(pr, int) and isinstance(pos, int):
                    try:
                        expected_pos = order.index(pr) + 1
                    except ValueError:
                        expected_pos = None
                    if expected_pos is not None and pos != expected_pos:
                        errors.append(
                            f"{ctx}: pr {pr} has position {pos}, but its real frozen-order position is {expected_pos}"
                        )

        elif rt == "actionable_presentation":
            _check_fields(rec, PRESENTATION_REQUIRED, f"{ctx} (actionable_presentation)", errors)
            pid = rec.get("presentation_id")
            if isinstance(pid, str):
                if pid in presentation_ids:
                    errors.append(f"{ctx}: duplicate presentation_id {pid!r}")
                presentation_ids.add(pid)
            key = (cohort, rec.get("pr"))
            if key not in seen_pr_in_cohort and key not in {
                (r.get("cohort"), r.get("pr")) for r in records if r.get("record_type") == "pr_summary"
            }:
                errors.append(f"{ctx}: presentation for pr={rec.get('pr')} cohort={cohort} has no matching pr_summary (unknown/non-processed PR)")
            reviews = rec.get("reviews")
            if not isinstance(reviews, list) or len(reviews) != 2:
                errors.append(f"{ctx}: actionable_presentation must have EXACTLY 2 reviews, got {len(reviews) if isinstance(reviews, list) else type(reviews).__name__}")
            else:
                for j, rv in enumerate(reviews):
                    _check_fields(rv, REVIEW_REQUIRED, f"{ctx}.reviews[{j}]", errors)
                ids = [rv.get("reviewer_id") for rv in reviews if isinstance(rv, dict)]
                if len(ids) == 2 and ids[0] == ids[1]:
                    errors.append(f"{ctx}: the two reviews must have DISTINCT reviewer_id, both are {ids[0]!r}")
                if len(ids) == 2 and all(isinstance(rv, dict) and all(k in rv for k in REVIEW_REQUIRED) for rv in reviews):
                    disagree = any(
                        reviews[0][f] != reviews[1][f]
                        for f in (
                            "scanner_substance_correct",
                            "pr_relevant",
                            "causal_framing_correct",
                            "rendered_presentation_correct",
                        )
                    )
                    # NOTE: a disagreement WITHOUT a coordinator_resolution
                    # is NOT a schema violation -- it is a legitimate,
                    # representable "unresolved disagreement" state,
                    # handled by the GATE itself (tier 2: INSUFFICIENT
                    # EVIDENCE, unless a hard failure elsewhere already
                    # decides FAIL first). Only the reverse is invalid: a
                    # resolution attached to reviews that don't actually
                    # disagree is nonsensical (there was nothing to
                    # resolve), so that alone is rejected here.
                    cr = rec.get("coordinator_resolution")
                    if not disagree and cr is not None:
                        errors.append(f"{ctx}: reviewers agree but a coordinator_resolution is present (must be null when there is no disagreement)")
                    if cr is not None:
                        cr_required = {
                            "resolved_by": str,
                            "scanner_substance_correct": bool,
                            "pr_relevant": bool,
                            "causal_framing_correct": bool,
                            "rendered_presentation_correct": bool,
                            "rationale": str,
                        }
                        _check_fields(cr, cr_required, f"{ctx}.coordinator_resolution", errors)

        elif rt == "pass_adjudication":
            _check_fields(rec, PASS_REQUIRED, f"{ctx} (pass_adjudication)", errors)
            key = (cohort, rec.get("pr"))
            pid = rec.get("pass_id")
            if isinstance(pid, str):
                pass_ids.setdefault(key, set())
                if pid in pass_ids[key]:
                    errors.append(f"{ctx}: duplicate pass_id {pid!r} for pr={rec.get('pr')}")
                pass_ids[key].add(pid)

        elif rt == "tool_error":
            _check_fields(rec, TOOL_ERROR_REQUIRED, f"{ctx} (tool_error)", errors)

        elif rt == "protocol_note":
            if "reason" not in rec or not isinstance(rec.get("reason"), str) or not rec["reason"]:
                errors.append(f"{ctx} (protocol_note): missing non-empty 'reason'")

    # Cross-record: complete PASS coverage -- every pr_summary's own
    # pass_verdict_count must equal the number of pass_adjudication
    # records for that PR (item 9: a missing PASS review is incomplete
    # evidence, not "zero false PASSes").
    for key, summary in pr_summaries.items():
        expected = summary.get("pass_verdict_count")
        actual = len(pass_ids.get(key, set()))
        if isinstance(expected, int) and expected != actual:
            errors.append(
                f"pr_summary cohort={key[0]} pr={key[1]}: pass_verdict_count={expected} but {actual} pass_adjudication record(s) present -- incomplete PASS coverage"
            )

    # Cross-record: frozen-order PREFIX property (item 5) -- processed
    # PRs for each cohort must be an unbroken prefix of the frozen
    # order, no skips, no reordering.
    for cohort_label, order in (("A", s4a_order), ("B", s4b_order)):
        if order is None:
            continue
        processed = sorted(
            (r["position"], r["pr"])
            for r in pr_summaries.values()
            if r.get("cohort") == cohort_label and isinstance(r.get("position"), int)
        )
        for idx, (pos, pr) in enumerate(processed, start=1):
            if pos != idx:
                errors.append(
                    f"S4-{cohort_label}: processed positions are not a contiguous 1..N prefix (found position {pos} at rank {idx})"
                )
                break
            if idx - 1 < len(order) and order[idx - 1] != pr:
                errors.append(
                    f"S4-{cohort_label}: position {pos} processed pr={pr}, but the frozen order's own entry at that position is {order[idx - 1]} -- skipped or reordered"
                )

    return errors


# ---------------------------------------------------------------------
# Mechanical stopping-rule check (item 6).
# ---------------------------------------------------------------------
def check_stopping_rule(records: list[dict], s4a_order: list[int], s4b_order: list[int]) -> list[str]:
    errors: list[str] = []
    notes = {r["cohort"]: r["reason"] for r in records if r.get("record_type") == "protocol_note"}

    a_summaries = sorted(
        (r for r in records if r.get("record_type") == "pr_summary" and r.get("cohort") == "A"),
        key=lambda r: r["position"],
    )
    b_summaries = sorted(
        (r for r in records if r.get("record_type") == "pr_summary" and r.get("cohort") == "B"),
        key=lambda r: r["position"],
    )
    a_actionable = _actionable_counts_by_pr(records, "A")
    b_actionable = _actionable_counts_by_pr(records, "B")

    # S4-A: process >=60; after 60, stop at first position where
    # cumulative A actionable count reaches 10; otherwise continue
    # through position min(120, len(s4a_order)).
    a_cap = min(120, len(s4a_order))
    cumulative = 0
    required_stop_a = None
    for i, r in enumerate(a_summaries, 1):
        cumulative += a_actionable.get(r["pr"], 0)
        if i >= 60 and cumulative >= 10:
            required_stop_a = i
            break
    if required_stop_a is None:
        required_stop_a = a_cap

    if len(a_summaries) < required_stop_a and "A" not in notes:
        errors.append(
            f"S4-A stopped at {len(a_summaries)}, before the required stopping point {required_stop_a}"
        )
    if len(a_summaries) > required_stop_a and "A" not in notes:
        errors.append(
            f"S4-A continued to {len(a_summaries)}, past the first valid stopping point {required_stop_a}, with no documented protocol_note reason"
        )

    # S4-B: process >=40; after 40, stop at first position where
    # combined A+B actionable reaches 30 with A contributing >=10;
    # otherwise continue through the realized cap.
    b_cap = min(80, len(s4b_order))
    total_a_actionable = sum(a_actionable.values())
    cumulative_b = 0
    required_stop_b = None
    for i, r in enumerate(b_summaries, 1):
        cumulative_b += b_actionable.get(r["pr"], 0)
        if i >= 40 and (total_a_actionable + cumulative_b) >= 30 and total_a_actionable >= 10:
            required_stop_b = i
            break
    if required_stop_b is None:
        required_stop_b = b_cap

    if len(b_summaries) < required_stop_b and "B" not in notes:
        errors.append(
            f"S4-B stopped at {len(b_summaries)}, before the required stopping point {required_stop_b}"
        )
    if len(b_summaries) > required_stop_b and "B" not in notes:
        errors.append(
            f"S4-B continued to {len(b_summaries)}, past the first valid stopping point {required_stop_b}, with no documented protocol_note reason"
        )

    return errors


def _actionable_counts_by_pr(records: list[dict], cohort: str) -> dict[int, int]:
    counts: dict[int, int] = {}
    for r in records:
        if r.get("record_type") == "actionable_presentation" and r.get("cohort") == cohort:
            counts[r["pr"]] = counts.get(r["pr"], 0) + 1
    return counts


# ---------------------------------------------------------------------
# Resolved judgment + metric computation.
# ---------------------------------------------------------------------
def resolved_judgment(rec: dict) -> dict | None:
    """The presentation's own final judgment: the coordinator
    resolution if one is present; either (agreeing) review when both
    reviewers agree; **None** when the two reviewers genuinely
    disagree and no coordinator_resolution has been recorded yet --
    that presentation's substance/relevance/framing/rendering status
    is UNDETERMINED, not defaulted to either reviewer's own opinion.
    Every caller must treat None as "not confirmed either way", never
    silently fall back to a reviewer's own judgment -- an unresolved
    disagreement is handled exclusively by the gate's own tier-2
    INSUFFICIENT EVIDENCE check, never smuggled into a tier-1 "confirmed
    hard failure" via an arbitrary reviewer pick."""
    if rec.get("coordinator_resolution") is not None:
        return rec["coordinator_resolution"]
    if _has_unresolved_disagreement(rec):
        return None
    return rec["reviews"][0]


def is_overall_correct(rec: dict) -> bool:
    j = resolved_judgment(rec)
    if j is None:
        return False
    return bool(
        j.get("scanner_substance_correct")
        and j.get("pr_relevant")
        and j.get("causal_framing_correct")
        and j.get("rendered_presentation_correct")
    )


def cohort_precision(records: list[dict], cohort: str) -> tuple[int, int]:
    """Denominator excludes presentations with a genuinely unresolved
    disagreement -- their correctness is undetermined, not "0" (that
    would silently penalize precision for a disagreement the gate's
    own tier-2 check already handles as INSUFFICIENT EVIDENCE)."""
    presentations = [
        r
        for r in records
        if r["cohort"] == cohort
        and r["record_type"] == "actionable_presentation"
        and resolved_judgment(r) is not None
    ]
    correct = sum(1 for r in presentations if is_overall_correct(r))
    return correct, len(presentations)


@dataclass
class GateResult:
    verdict: str
    reasons: list[str] = field(default_factory=list)


def _has_unresolved_disagreement(rec: dict) -> bool:
    reviews = rec.get("reviews") or []
    if len(reviews) != 2:
        return False
    disagree = any(
        reviews[0].get(f) != reviews[1].get(f)
        for f in (
            "scanner_substance_correct",
            "pr_relevant",
            "causal_framing_correct",
            "rendered_presentation_correct",
        )
    )
    return disagree and rec.get("coordinator_resolution") is None


def evaluate_gate(records: list[dict]) -> GateResult:
    """3-tier precedence, frozen in protocol.md:
    1. any hard correctness failure -> FAIL, unconditionally.
    2. else insufficient volume/undefined precision/unresolved
       disagreement/exhausted cap -> INSUFFICIENT EVIDENCE.
    3. else evaluate precision/CI-bound conditions -> PASS or FAIL.
    """
    presentations = [r for r in records if r["record_type"] == "actionable_presentation"]
    a_correct, a_total = cohort_precision(records, "A")
    b_correct, b_total = cohort_precision(records, "B")
    total_correct = a_correct + b_correct
    total_presentations = a_total + b_total

    # NOTE: every inventory below is built ONLY from presentations with
    # a determined judgment (resolved_judgment(r) is not None) -- a
    # presentation with a genuinely unresolved disagreement is neither
    # "confirmed correct" nor "confirmed wrong" in any of these
    # categories; it surfaces exclusively via unresolved_disagreements
    # (tier 2), never double-counted into a tier-1 hard failure.
    determined = [r for r in presentations if resolved_judgment(r) is not None]
    false_pass = [r for r in records if r["record_type"] == "pass_adjudication" and not r.get("genuine_evidence", True)]
    false_finding = [r for r in determined if not resolved_judgment(r).get("scanner_substance_correct")]
    pr_relevance_errors = [
        r
        for r in determined
        if resolved_judgment(r).get("scanner_substance_correct") and not resolved_judgment(r).get("pr_relevant")
    ]
    framing_errors = [
        r
        for r in determined
        if resolved_judgment(r).get("scanner_substance_correct")
        and resolved_judgment(r).get("pr_relevant")
        and not resolved_judgment(r).get("causal_framing_correct")
    ]
    rendering_errors = [
        r
        for r in determined
        if resolved_judgment(r).get("scanner_substance_correct")
        and resolved_judgment(r).get("pr_relevant")
        and resolved_judgment(r).get("causal_framing_correct")
        and not resolved_judgment(r).get("rendered_presentation_correct")
    ]
    unresolved_disagreements = [r for r in presentations if _has_unresolved_disagreement(r)]
    tool_errors_ordinary = [
        r for r in records if r["record_type"] == "tool_error" and not r.get("attributable_to_unsupported_input")
    ]

    # --- Tier 1: hard correctness failures ---
    hard_reasons: list[str] = []
    if false_pass:
        hard_reasons.append(f"{len(false_pass)} false PASS(es) found")
    if false_finding:
        hard_reasons.append(f"{len(false_finding)} false finding(s)/scanner-substance error(s) found")
    if pr_relevance_errors:
        hard_reasons.append(f"{len(pr_relevance_errors)} PR-relevance error(s) found")
    if framing_errors:
        hard_reasons.append(f"{len(framing_errors)} causal-framing error(s) found")
    if rendering_errors:
        hard_reasons.append(f"{len(rendering_errors)} rendered-presentation error(s) found")
    if tool_errors_ordinary:
        hard_reasons.append(f"{len(tool_errors_ordinary)} TOOL_ERROR(s) on ordinary supported input")
    if hard_reasons:
        return GateResult(verdict="FAIL", reasons=hard_reasons)

    # --- Tier 2: insufficient evidence ---
    insufficient_reasons: list[str] = []
    if a_total < 10:
        insufficient_reasons.append(f"S4-A has {a_total} actionable presentations, below the 10 minimum")
    if total_presentations < 30:
        insufficient_reasons.append(f"{total_presentations} total actionable presentations, below the 30 minimum")
    if b_total == 0:
        insufficient_reasons.append("S4-B has zero actionable presentations -- its precision is undefined")
    if unresolved_disagreements:
        insufficient_reasons.append(f"{len(unresolved_disagreements)} unresolved reviewer disagreement(s)")
    if insufficient_reasons:
        return GateResult(verdict="INSUFFICIENT EVIDENCE", reasons=insufficient_reasons)

    # --- Tier 3: precision / CI-bound evaluation ---
    reasons: list[str] = []
    if a_correct != a_total:
        reasons.append(f"S4-A actionable precision {a_correct}/{a_total} != 100%")
    if b_correct != b_total:
        reasons.append(f"S4-B actionable precision {b_correct}/{b_total} != 100%")
    lower_bound = clopper_pearson_lower_bound(total_correct, total_presentations)
    if lower_bound < 0.90:
        reasons.append(f"pooled one-sided 95% exact lower bound {lower_bound:.4f} < 0.90")

    if reasons:
        return GateResult(verdict="FAIL", reasons=reasons)
    return GateResult(verdict="PASS", reasons=["all 10 conditions satisfied"])


# ---------------------------------------------------------------------
# Error inventories (item 10) -- each its own machine-derived list.
# ---------------------------------------------------------------------
def build_inventories(records: list[dict]) -> dict[str, list[dict]]:
    presentations = [r for r in records if r["record_type"] == "actionable_presentation"]
    # Same rule as evaluate_gate: a genuinely unresolved disagreement is
    # excluded from every determined-judgment inventory below (it has
    # its own, separate inventory) -- never defaulted into any of them.
    determined = [r for r in presentations if resolved_judgment(r) is not None]
    return {
        "false_finding": [
            r for r in determined if not resolved_judgment(r).get("scanner_substance_correct")
        ],
        "pr_relevance_error": [
            r
            for r in determined
            if resolved_judgment(r).get("scanner_substance_correct")
            and not resolved_judgment(r).get("pr_relevant")
        ],
        "causal_framing_error": [
            r
            for r in determined
            if resolved_judgment(r).get("scanner_substance_correct")
            and resolved_judgment(r).get("pr_relevant")
            and not resolved_judgment(r).get("causal_framing_correct")
        ],
        "rendered_presentation_error": [
            r
            for r in determined
            if resolved_judgment(r).get("scanner_substance_correct")
            and resolved_judgment(r).get("pr_relevant")
            and resolved_judgment(r).get("causal_framing_correct")
            and not resolved_judgment(r).get("rendered_presentation_correct")
        ],
        "false_pass": [
            r for r in records if r["record_type"] == "pass_adjudication" and not r.get("genuine_evidence", True)
        ],
        "tool_error_ordinary": [
            r for r in records if r["record_type"] == "tool_error" and not r.get("attributable_to_unsupported_input")
        ],
        "unresolved_disagreement": [r for r in presentations if _has_unresolved_disagreement(r)],
    }


# ---------------------------------------------------------------------
# Full metrics (item 11).
# ---------------------------------------------------------------------
def build_metrics(records: list[dict]) -> dict:
    pr_summaries = [r for r in records if r["record_type"] == "pr_summary"]
    a_summaries = [r for r in pr_summaries if r["cohort"] == "A"]
    b_summaries = [r for r in pr_summaries if r["cohort"] == "B"]
    applicable = [r for r in pr_summaries if r.get("applicable")]

    a_correct, a_total = cohort_precision(records, "A")
    b_correct, b_total = cohort_precision(records, "B")

    effort_dist: dict[str, int] = {"<2": 0, "2-10": 0, ">10": 0, "unknown": 0}
    for r in pr_summaries:
        b = r.get("manual_effort_bucket")
        effort_dist[b if b in effort_dist else "unknown"] += 1

    gap_counts = {"known_gap": 0, "new_gap": 0}
    for r in pr_summaries:
        g = r.get("gap_classification")
        if g in gap_counts:
            gap_counts[g] += 1

    findings_per_pr: dict[int, int] = {}
    for r in records:
        if r["record_type"] == "actionable_presentation":
            findings_per_pr[r["pr"]] = findings_per_pr.get(r["pr"], 0) + 1

    # INCONCLUSIVE rate/root-cause breakdown and origin_unclear
    # frequency: read from pr_summary's own OPTIONAL
    # `inconclusive_count`/`inconclusive_root_causes`/
    # `origin_unclear_count` fields (not part of the required schema,
    # since not every pr_summary has an applicable oba target at all --
    # default to 0/empty when absent, never an error).
    total_inconclusive = 0
    root_cause_counts: dict[str, int] = {}
    total_origin_unclear = 0
    for r in pr_summaries:
        total_inconclusive += r.get("inconclusive_count", 0) or 0
        total_origin_unclear += r.get("origin_unclear_count", 0) or 0
        for cause in r.get("inconclusive_root_causes", []) or []:
            root_cause_counts[cause] = root_cause_counts.get(cause, 0) + 1

    # Reviewer-disagreement log: EVERY actionable presentation's own
    # agree/disagree/resolved status, not just the still-unresolved
    # ones (build_inventories' own `unresolved_disagreement` bucket is
    # the subset of this that still blocks the gate).
    disagreement_log = []
    for r in records:
        if r["record_type"] != "actionable_presentation":
            continue
        reviews = r.get("reviews") or []
        disagree = len(reviews) == 2 and any(
            reviews[0].get(f) != reviews[1].get(f)
            for f in (
                "scanner_substance_correct",
                "pr_relevant",
                "causal_framing_correct",
                "rendered_presentation_correct",
            )
        )
        disagreement_log.append(
            {
                "presentation_id": r.get("presentation_id"),
                "pr": r.get("pr"),
                "cohort": r.get("cohort"),
                "disagreed": disagree,
                "resolved": disagree and r.get("coordinator_resolution") is not None,
            }
        )

    return {
        "processed_a": len(a_summaries),
        "processed_b": len(b_summaries),
        "applicability_rate": (len(applicable) / len(pr_summaries)) if pr_summaries else None,
        "actionable_presentation_rate": (
            (a_total + b_total) / len(pr_summaries) if pr_summaries else None
        ),
        "a_precision": (a_correct, a_total),
        "b_precision": (b_correct, b_total),
        "pooled_lower_bound": clopper_pearson_lower_bound(a_correct + b_correct, a_total + b_total),
        "manual_effort_distribution": effort_dist,
        "findings_per_pr_distribution": findings_per_pr,
        "structural_non_applicability": len(pr_summaries) - len(applicable),
        "known_gap_count": gap_counts["known_gap"],
        "new_gap_count": gap_counts["new_gap"],
        "inconclusive_count": total_inconclusive,
        "inconclusive_rate": (total_inconclusive / len(applicable)) if applicable else None,
        "inconclusive_root_cause_breakdown": root_cause_counts,
        "origin_unclear_count": total_origin_unclear,
        "origin_unclear_frequency": (total_origin_unclear / len(applicable)) if applicable else None,
        "reviewer_disagreement_log": disagreement_log,
    }


def render_report(records: list[dict]) -> str:
    gate = evaluate_gate(records)
    metrics = build_metrics(records)
    inv = build_inventories(records)
    lines = [
        "# S4 report (generated, not hand-typed)",
        "",
        f"- Processed: S4-A {metrics['processed_a']}, S4-B {metrics['processed_b']}",
        f"- S4-A actionable precision: {metrics['a_precision'][0]}/{metrics['a_precision'][1]}",
        f"- S4-B actionable precision: {metrics['b_precision'][0]}/{metrics['b_precision'][1]}",
        f"- Pooled one-sided 95% exact lower bound: {metrics['pooled_lower_bound']:.4f}",
        f"- False PASS: {len(inv['false_pass'])}",
        f"- False finding: {len(inv['false_finding'])}",
        f"- PR-relevance errors: {len(inv['pr_relevance_error'])}",
        f"- Causal-framing errors: {len(inv['causal_framing_error'])}",
        f"- Rendered-presentation errors: {len(inv['rendered_presentation_error'])}",
        f"- TOOL_ERROR (ordinary input): {len(inv['tool_error_ordinary'])}",
        f"- Unresolved disagreements: {len(inv['unresolved_disagreement'])}",
        f"- Deployment gate: **{gate.verdict}**",
        "- Reasons: " + "; ".join(gate.reasons),
    ]
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------
def load_ledger(path: Path) -> list[dict]:
    records = []
    with path.open() as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            records.append(json.loads(line))
    return records


def load_frozen_order(path: Path) -> list[int]:
    return [r["number"] for r in json.loads(path.read_text())]


def run_report(ledger_path: Path, s4a_path: Path | None, s4b_path: Path | None) -> int:
    records = load_ledger(ledger_path)
    s4a_order = load_frozen_order(s4a_path) if s4a_path else None
    s4b_order = load_frozen_order(s4b_path) if s4b_path else None

    schema_errors = validate_ledger(records, s4a_order, s4b_order)
    if schema_errors:
        print("SCHEMA VALIDATION FAILED -- this is a report-generation TOOL_ERROR, not an analytical result:", file=sys.stderr)
        for e in schema_errors:
            print(f"  - {e}", file=sys.stderr)
        return 3

    if s4a_order is not None and s4b_order is not None:
        stopping_errors = check_stopping_rule(records, s4a_order, s4b_order)
        if stopping_errors:
            print("STOPPING-RULE VALIDATION FAILED -- this is a report-generation TOOL_ERROR:", file=sys.stderr)
            for e in stopping_errors:
                print(f"  - {e}", file=sys.stderr)
            return 3

    print(render_report(records))
    return 0


# ---------------------------------------------------------------------
# Hostile self-test suite (item 13), self-contained, run against the
# real CLI validation+gate path (run_report), not just internal
# helper functions -- exercised BEFORE any real adjudication.
# ---------------------------------------------------------------------
def _fake_sha40(tag: str) -> str:
    return hashlib.sha1(tag.encode()).hexdigest()


def _fake_sha256(tag: str) -> str:
    return hashlib.sha256(tag.encode()).hexdigest()


def _review(reviewer_id: str, correct: bool = True, **overrides) -> dict:
    r = {
        "reviewer_id": reviewer_id,
        "scanner_substance_correct": correct,
        "pr_relevant": correct,
        "causal_framing_correct": correct,
        "rendered_presentation_correct": correct,
        "notes": "selftest",
    }
    r.update(overrides)
    return r


def _pr_summary(cohort: str, pr: int, position: int, *, actionable_count=0, pass_verdict_count=0, tool_error_count=0, applicable=True) -> dict:
    return {
        "record_type": "pr_summary",
        "cohort": cohort,
        "position": position,
        "pr": pr,
        "base_sha": _fake_sha40(f"base{pr}"),
        "head_sha": _fake_sha40(f"head{pr}"),
        "applicable": applicable,
        "module_paths": [f"nixos/modules/services/x/{pr}.nix"],
        "test_paths": [f"nixos/tests/{pr}.nix"],
        "oba_binary_version": "0.4.4",
        "oba_artifact_sha256": _fake_sha256("oba-binary"),
        "command": f"oba audit-diff --base-root ... --head-root ... (pr {pr})",
        "raw_json_ref": f"artifacts/{pr}.json",
        "raw_json_sha256": _fake_sha256(f"raw{pr}"),
        "rendered_summary_ref": f"artifacts/{pr}.md",
        "rendered_summary_sha256": _fake_sha256(f"summary{pr}"),
        "actionable_count": actionable_count,
        "pass_verdict_count": pass_verdict_count,
        "tool_error_count": tool_error_count,
        "manual_effort_bucket": "<2",
        "gap_classification": None,
    }


def _presentation(cohort: str, pr: int, subject: str, correct: bool = True, idx: int = 0, **review_overrides) -> dict:
    reviews = [
        _review("reviewer-1", correct, **review_overrides),
        _review("reviewer-2", correct, **review_overrides),
    ]
    return {
        "record_type": "actionable_presentation",
        "cohort": cohort,
        "pr": pr,
        "presentation_id": f"{pr}:oba:{subject}:new_finding:{idx}",
        "engine": "oba",
        "subject": subject,
        "bucket": "new_finding",
        "raw_output_index": idx,
        "reviews": reviews,
        "coordinator_resolution": None,
    }


def _clean_pool(a_count: int, b_count: int) -> tuple[list[dict], list[int], list[int]]:
    """A minimal, fully valid, all-correct 30-actionable ledger (a_count
    from A, b_count from B, one presentation per PR, a_count+b_count
    total PRs each with exactly 1 actionable presentation and 0 PASS
    verdicts) -- the common base every hostile test perturbs."""
    records = []
    s4a_order = list(range(1, 1 + max(a_count, 60)))
    s4b_order = list(range(10000, 10000 + max(b_count, 40)))
    for i in range(a_count):
        pr = s4a_order[i]
        records.append(_pr_summary("A", pr, i + 1, actionable_count=1))
        records.append(_presentation("A", pr, f"opt{i}"))
    # pad remaining A positions (non-actionable) up to the 60 minimum
    for i in range(a_count, min(60, len(s4a_order))):
        pr = s4a_order[i]
        records.append(_pr_summary("A", pr, i + 1, actionable_count=0))
    for i in range(b_count):
        pr = s4b_order[i]
        records.append(_pr_summary("B", pr, i + 1, actionable_count=1))
        records.append(_presentation("B", pr, f"bopt{i}"))
    for i in range(b_count, min(40, len(s4b_order))):
        pr = s4b_order[i]
        records.append(_pr_summary("B", pr, i + 1, actionable_count=0))
    return records, s4a_order, s4b_order


def run_selftest() -> None:
    failures: list[str] = []

    def check(name: str, condition: bool) -> None:
        status = "ok" if condition else "FAILED"
        print(f"[selftest] {name}: {status}")
        if not condition:
            failures.append(name)

    def gate_of(records, s4a_order, s4b_order):
        schema_errors = validate_ledger(records, s4a_order, s4b_order)
        if schema_errors:
            return "SCHEMA_ERROR", schema_errors
        stopping_errors = check_stopping_rule(records, s4a_order, s4b_order)
        if stopping_errors:
            return "STOPPING_ERROR", stopping_errors
        return evaluate_gate(records).verdict, []

    # 1. Clean 30/30 (12 from A, 18 from B), all structural
    # requirements met -> PASS. (a_count=12 >=10 from A, total=30.)
    records, s4a, s4b = _clean_pool(12, 18)
    verdict, _ = gate_of(records, s4a, s4b)
    check("clean 30/30, all structural requirements met -> PASS", verdict == "PASS")

    # 2. Exact 30/30 one-sided 95% CP lower bound ~= 0.905.
    lb = clopper_pearson_lower_bound(30, 30)
    check(f"CP lower bound(30/30) ~= 0.905 (got {lb:.4f})", abs(lb - 0.9050) < 0.001)

    # 3. Insufficient-count ledger + false PASS -> FAIL (hard failure
    # takes precedence over the insufficient-count tier).
    records, s4a, s4b = _clean_pool(5, 3)  # well below thresholds
    pr = s4a[0]
    records.append({
        "record_type": "pass_adjudication", "cohort": "A", "pr": pr,
        "pass_id": f"{pr}:oba:enable", "genuine_evidence": False,
        "reviewer_id": "reviewer-1", "notes": "not genuine",
    })
    for r in records:
        if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == pr:
            r["pass_verdict_count"] = 1
    verdict, reasons = gate_of(records, s4a, s4b)
    check(
        "insufficient-count ledger + false PASS -> FAIL (hard failure precedence)",
        verdict == "FAIL",
    )

    # 4. Insufficient-count ledger + false finding -> FAIL.
    records, s4a, s4b = _clean_pool(5, 3)
    records.append(_presentation("A", s4a[0], "brokenopt", correct=False, idx=99))
    verdict, _ = gate_of(records, s4a, s4b)
    check("insufficient-count ledger + false finding -> FAIL", verdict == "FAIL")

    # 5. Insufficient-count ledger + framing/relevance error -> FAIL.
    records, s4a, s4b = _clean_pool(5, 3)
    p = _presentation("A", s4a[0], "framingopt", correct=True, idx=98)
    p["reviews"] = [_review("reviewer-1", True), _review("reviewer-2", True)]
    p["reviews"][0]["causal_framing_correct"] = False
    p["reviews"][1]["causal_framing_correct"] = False
    records.append(p)
    verdict, _ = gate_of(records, s4a, s4b)
    check("insufficient-count ledger + framing error -> FAIL", verdict == "FAIL")

    # 6. 30 duplicate copies of one presentation -> schema rejection,
    # never PASS.
    records, s4a, s4b = _clean_pool(12, 18)
    dup = _presentation("A", s4a[0], "opt0", idx=0)  # same presentation_id as an existing one
    records.append(dup)
    verdict, _ = gate_of(records, s4a, s4b)
    check("duplicate presentation_id -> schema rejection, not PASS", verdict == "SCHEMA_ERROR")

    # 7. PRs not forming a frozen-order prefix -> rejection.
    records, s4a, s4b = _clean_pool(12, 18)
    for r in records:
        if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["position"] == 1:
            r["pr"] = 999999  # not the real frozen-order entry at position 1
    verdict, _ = gate_of(records, s4a, s4b)
    check("PR not matching frozen order at its position -> rejection", verdict in ("SCHEMA_ERROR", "STOPPING_ERROR"))

    # 8. Fewer than 60 A processed despite enough actionable results -> rejection.
    records = []
    s4a_order = list(range(1, 121))
    s4b_order = list(range(10000, 10080))
    for i in range(15):  # only 15 processed, but all actionable -> 15 actionable, way past 10, way before 60
        pr = s4a_order[i]
        records.append(_pr_summary("A", pr, i + 1, actionable_count=1))
        records.append(_presentation("A", pr, f"opt{i}"))
    for i in range(18):
        pr = s4b_order[i]
        records.append(_pr_summary("B", pr, i + 1, actionable_count=1))
        records.append(_presentation("B", pr, f"bopt{i}"))
    verdict, _ = gate_of(records, s4a_order, s4b_order)
    check("fewer than 60 A processed despite enough actionable -> rejection", verdict == "STOPPING_ERROR")

    # 9. Fewer than 40 B processed despite enough actionable results -> rejection.
    records, s4a, s4b = _clean_pool(12, 0)
    records2 = [r for r in records if not (r.get("record_type") == "pr_summary" and r["cohort"] == "B")]
    for i in range(15):
        pr = s4b[i]
        records2.append(_pr_summary("B", pr, i + 1, actionable_count=1))
        records2.append(_presentation("B", pr, f"bopt{i}"))
    verdict, _ = gate_of(records2, s4a, s4b)
    check("fewer than 40 B processed despite enough actionable -> rejection", verdict == "STOPPING_ERROR")

    # 10. Skipped frozen-order PR -> rejection.
    records, s4a, s4b = _clean_pool(12, 18)
    records = [r for r in records if not (r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["position"] == 30)]
    verdict, _ = gate_of(records, s4a, s4b)
    check("skipped frozen-order position -> rejection", verdict in ("SCHEMA_ERROR", "STOPPING_ERROR"))

    # 11. Duplicate PR summary -> rejection.
    records, s4a, s4b = _clean_pool(12, 18)
    records.append(_pr_summary("A", s4a[0], 1, actionable_count=1))
    verdict, _ = gate_of(records, s4a, s4b)
    check("duplicate pr_summary -> rejection", verdict == "SCHEMA_ERROR")

    # 12. Duplicate PASS adjudication -> rejection.
    records, s4a, s4b = _clean_pool(12, 18)
    pr = s4a[0]
    for r in records:
        if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == pr:
            r["pass_verdict_count"] = 1
    records.append({
        "record_type": "pass_adjudication", "cohort": "A", "pr": pr,
        "pass_id": f"{pr}:oba:enable", "genuine_evidence": True,
        "reviewer_id": "reviewer-1", "notes": "ok",
    })
    records.append({
        "record_type": "pass_adjudication", "cohort": "A", "pr": pr,
        "pass_id": f"{pr}:oba:enable", "genuine_evidence": True,
        "reviewer_id": "reviewer-2", "notes": "ok, duplicate id",
    })
    verdict, _ = gate_of(records, s4a, s4b)
    check("duplicate pass_id -> rejection", verdict == "SCHEMA_ERROR")

    # 13. Missing required PASS adjudication -> incomplete/rejection.
    records, s4a, s4b = _clean_pool(12, 18)
    for r in records:
        if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == s4a[0]:
            r["pass_verdict_count"] = 1  # claims 1 PASS needing review, but none provided
    verdict, _ = gate_of(records, s4a, s4b)
    check("missing required PASS adjudication -> rejection (incomplete coverage)", verdict == "SCHEMA_ERROR")

    # 14. Actionable presentation with only one reviewer -> incomplete/rejection.
    records, s4a, s4b = _clean_pool(12, 18)
    for r in records:
        if r.get("record_type") == "actionable_presentation" and r["pr"] == s4a[0]:
            r["reviews"] = [r["reviews"][0]]
    verdict, _ = gate_of(records, s4a, s4b)
    check("only 1 reviewer on a presentation -> rejection", verdict == "SCHEMA_ERROR")

    # 15. Two reviewers with unresolved disagreement -> INSUFFICIENT
    # EVIDENCE unless another hard failure already makes it FAIL.
    records, s4a, s4b = _clean_pool(12, 18)
    for r in records:
        if r.get("record_type") == "actionable_presentation" and r["pr"] == s4a[0]:
            r["reviews"][1]["causal_framing_correct"] = False  # disagree
            r["coordinator_resolution"] = None  # unresolved
    verdict, _ = gate_of(records, s4a, s4b)
    check(
        "unresolved disagreement, no other hard failure -> INSUFFICIENT EVIDENCE",
        verdict == "INSUFFICIENT EVIDENCE",
    )

    # 16. Isolated pr_relevant=false -> explicit PR-relevance FAIL reason.
    records, s4a, s4b = _clean_pool(12, 18)
    p = _presentation("A", s4a[0], "irrelevantopt", correct=True, idx=97)
    for rv in p["reviews"]:
        rv["pr_relevant"] = False
    records.append(p)
    schema_errors = validate_ledger(records, s4a, s4b)
    if schema_errors:
        check("isolated pr_relevant=false -> PR-relevance FAIL reason", False)
    else:
        gate = evaluate_gate(records)
        check(
            "isolated pr_relevant=false -> PR-relevance FAIL reason",
            gate.verdict == "FAIL" and any("PR-relevance" in r for r in gate.reasons),
        )

    # 17. S4-B with zero actionable presentations -> INSUFFICIENT EVIDENCE.
    records, s4a, s4b = _clean_pool(12, 0)
    verdict, _ = gate_of(records, s4a, s4b)
    check("S4-B zero actionable presentations -> INSUFFICIENT EVIDENCE", verdict == "INSUFFICIENT EVIDENCE")

    if failures:
        print(f"\n{len(failures)} selftest check(s) FAILED: {failures}", file=sys.stderr)
        sys.exit(1)
    print(f"\nAll 17 selftest checks passed.")


def main() -> None:
    if len(sys.argv) > 1 and sys.argv[1] == "--selftest":
        run_selftest()
        return
    if len(sys.argv) > 1 and sys.argv[1] == "--ledger":
        ledger_path = Path(sys.argv[2])
        s4a_path = None
        s4b_path = None
        if "--s4a" in sys.argv:
            s4a_path = Path(sys.argv[sys.argv.index("--s4a") + 1])
        if "--s4b" in sys.argv:
            s4b_path = Path(sys.argv[sys.argv.index("--s4b") + 1])
        sys.exit(run_report(ledger_path, s4a_path, s4b_path))
    print("usage: generate-report.py --selftest | --ledger <path.jsonl> [--s4a <orders.json>] [--s4b <orders.json>]", file=sys.stderr)
    sys.exit(2)


if __name__ == "__main__":
    main()
