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
  position: int (>=1, the real 1-indexed position in that cohort's frozen order)
  pr: int (>0)
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
  actionable_count: int (>=0) -- MUST equal the real number of
    actionable_presentation records for this PR
  pass_verdict_count: int (>=0) -- MUST equal the real number of
    pass_adjudication records for this PR
  tool_error_count: int (>=0) -- MUST equal the real number of
    tool_error records for this PR
  manual_effort_bucket: "<2" | "2-10" | ">10" | null
  gap_classification: "known_gap" | "new_gap" | null
  # optional, default 0/empty when absent (e.g. oba=no PRs never carry these):
  inconclusive_count: int (>=0, optional)
  inconclusive_root_causes: list[str] (optional, one entry per inconclusive instance)
  origin_unclear_count: int (>=0, optional)

actionable_presentation -- one per actionable presentation, with a
GENUINELY DERIVED presentation_id and TWO independent reviewer
judgments:
  record_type: "actionable_presentation"
  cohort: "A" | "B"
  pr: int
  presentation_id: str -- MUST equal presentation_id(cohort, pr,
    engine, subject, bucket, raw_output_index); validated, not trusted
  engine: str
  subject: str
  bucket: str
  raw_output_index: int (>=0)
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

pass_adjudication -- one per PASS verdict materially relevant to a PR
transition. Mirrors actionable_presentation's own structure exactly --
protocol.md requires "independent verification" for a materially
relevant PASS, the same standard as an actionable presentation, so
this is not a lighter-weight record:
  record_type: "pass_adjudication"
  cohort: "A" | "B"
  pr: int
  pass_id: str -- MUST equal pass_id(cohort, pr, engine, subject);
    validated, not trusted
  engine: str
  subject: str
  reviews: [review, review]  -- EXACTLY 2, distinct reviewer_id
    review = {
      reviewer_id: str
      genuine_evidence: bool
      notes: str
    }
  coordinator_resolution: null | {
      resolved_by: str
      genuine_evidence: bool
      rationale: str
    }

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
carried as a separate, potentially-inconsistent flag. Likewise there
is no separate `false_pass` flag -- a PASS's false-PASS status is
ALWAYS derived from `genuine_evidence == False` on ITS resolved
judgment.
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from dataclasses import dataclass, field
from math import comb
from pathlib import Path

ROOT = Path(__file__).resolve().parent

SHA40_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


# ---------------------------------------------------------------------
# Genuinely derived stable identities (item 4) -- these are the single
# source of truth; the validator checks every SUPPLIED id against them,
# it never just checks "is this string unique".
# ---------------------------------------------------------------------
def presentation_id(cohort: str, pr: int, engine: str, subject: str, bucket: str, raw_output_index: int) -> str:
    return f"{cohort}:{pr}:{engine}:{subject}:{bucket}:{raw_output_index}"


def pass_id(cohort: str, pr: int, engine: str, subject: str) -> str:
    return f"{cohort}:{pr}:{engine}:{subject}:PASS"


# ---------------------------------------------------------------------
# Exact one-sided Clopper-Pearson lower confidence bound. No scipy
# available in this environment -- implemented directly via the exact
# relationship between the regularized incomplete beta function and
# the binomial survival function (both integer-parameter forms),
# inverted by bisection. 30/30 successes at 95% one-sided confidence
# must give ~90.5% (the protocol's own worked example).
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

FINDING_REVIEW_REQUIRED = {
    "reviewer_id": str,
    "scanner_substance_correct": bool,
    "pr_relevant": bool,
    "causal_framing_correct": bool,
    "rendered_presentation_correct": bool,
    "notes": str,
}

PASS_REVIEW_REQUIRED = {
    "reviewer_id": str,
    "genuine_evidence": bool,
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

PASS_RECORD_REQUIRED = {
    "cohort": str,
    "pr": int,
    "pass_id": str,
    "engine": str,
    "subject": str,
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


def _check_reviews(rec: dict, required: dict, ctx: str, errors: list[str]) -> tuple[bool, dict | None]:
    """Validates a `reviews: [r1, r2]` + `coordinator_resolution` pair,
    shared logic between actionable_presentation and pass_adjudication.
    Returns (disagree, coordinator_resolution-or-None)."""
    reviews = rec.get("reviews")
    if not isinstance(reviews, list) or len(reviews) != 2:
        errors.append(
            f"{ctx}: must have EXACTLY 2 reviews, got {len(reviews) if isinstance(reviews, list) else type(reviews).__name__}"
        )
        return False, None
    for j, rv in enumerate(reviews):
        if not isinstance(rv, dict):
            errors.append(f"{ctx}.reviews[{j}]: must be an object")
            continue
        _check_fields(rv, required, f"{ctx}.reviews[{j}]", errors)
    ids = [rv.get("reviewer_id") for rv in reviews if isinstance(rv, dict)]
    if len(ids) == 2 and ids[0] == ids[1]:
        errors.append(f"{ctx}: the two reviews must have DISTINCT reviewer_id, both are {ids[0]!r}")
    disagree = False
    if len(ids) == 2 and all(isinstance(rv, dict) and all(k in rv for k in required) for rv in reviews):
        judgment_fields = [k for k in required if k != "reviewer_id" and k != "notes"]
        disagree = any(reviews[0].get(f) != reviews[1].get(f) for f in judgment_fields)
    cr = rec.get("coordinator_resolution")
    if not disagree and cr is not None:
        errors.append(f"{ctx}: reviewers agree but a coordinator_resolution is present (must be null when there is no disagreement)")
    if cr is not None:
        cr_required = {k: v for k, v in required.items() if k != "reviewer_id"}
        cr_required = {**cr_required, "resolved_by": str, "rationale": str}
        _check_fields(cr, cr_required, f"{ctx}.coordinator_resolution", errors)
    return disagree, cr


def validate_ledger(
    records: list[dict],
    s4a_order: list[int] | None = None,
    s4b_order: list[int] | None = None,
) -> list[str]:
    """Returns a list of validation errors. Empty list == valid."""
    errors: list[str] = []

    pr_summaries: dict[tuple[str, int], dict] = {}
    presentation_ids_seen: set[str] = set()
    pass_ids_seen: set[str] = set()
    presentation_count_by_pr: dict[tuple[str, int], int] = {}
    pass_count_by_pr: dict[tuple[str, int], int] = {}
    tool_error_count_by_pr: dict[tuple[str, int], int] = {}
    seen_pr_summary_key: set[tuple[str, int]] = set()
    all_pr_summary_keys: set[tuple[str, int]] = {
        (r.get("cohort"), r.get("pr")) for r in records if r.get("record_type") == "pr_summary"
    }

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
            pr = rec.get("pr")
            if isinstance(pr, int) and pr <= 0:
                errors.append(f"{ctx}: pr must be > 0, got {pr}")
            pos = rec.get("position")
            if isinstance(pos, int) and pos < 1:
                errors.append(f"{ctx}: position must be >= 1, got {pos}")
            for count_field in ("actionable_count", "pass_verdict_count", "tool_error_count"):
                v = rec.get(count_field)
                if isinstance(v, int) and v < 0:
                    errors.append(f"{ctx}: {count_field} must be >= 0, got {v}")
            for opt_count_field in ("inconclusive_count", "origin_unclear_count"):
                v = rec.get(opt_count_field)
                if v is not None and (not isinstance(v, int) or v < 0):
                    errors.append(f"{ctx}: optional field {opt_count_field} must be an int >= 0 when present, got {v!r}")
            v = rec.get("inconclusive_root_causes")
            if v is not None:
                if not isinstance(v, list) or not all(isinstance(x, str) for x in v):
                    errors.append(f"{ctx}: inconclusive_root_causes must be a list of str when present")
            for path_field in ("module_paths", "test_paths"):
                v = rec.get(path_field)
                if isinstance(v, list) and not all(isinstance(x, str) for x in v):
                    errors.append(f"{ctx}: {path_field} elements must all be str")

            if "base_sha" in rec and isinstance(rec["base_sha"], str) and not SHA40_RE.match(rec["base_sha"]):
                errors.append(f"{ctx}: base_sha is not a real 40-hex-char SHA: {rec['base_sha']!r}")
            if "head_sha" in rec and isinstance(rec["head_sha"], str) and not SHA40_RE.match(rec["head_sha"]):
                errors.append(f"{ctx}: head_sha is not a real 40-hex-char SHA: {rec['head_sha']!r}")
            for shafield in ("oba_artifact_sha256", "raw_json_sha256", "rendered_summary_sha256"):
                v = rec.get(shafield)
                if isinstance(v, str) and not SHA256_RE.match(v):
                    errors.append(f"{ctx}: {shafield} is not a real 64-hex-char SHA256: {v!r}")

            key = (cohort, pr)
            if key in seen_pr_summary_key:
                errors.append(f"{ctx}: duplicate pr_summary for cohort={cohort} pr={pr}")
            seen_pr_summary_key.add(key)
            pr_summaries[key] = rec
            if rec.get("manual_effort_bucket") not in ("<2", "2-10", ">10", None):
                errors.append(f"{ctx}: manual_effort_bucket has an invalid value {rec.get('manual_effort_bucket')!r}")
            if rec.get("gap_classification") not in ("known_gap", "new_gap", None):
                errors.append(f"{ctx}: gap_classification has an invalid value {rec.get('gap_classification')!r}")
            order = s4a_order if cohort == "A" else s4b_order
            if order is not None and isinstance(pr, int):
                if pr not in order:
                    errors.append(f"{ctx}: pr {pr} is not a member of the frozen S4-{cohort} order")
                elif isinstance(pos, int):
                    expected_pos = order.index(pr) + 1
                    if pos != expected_pos:
                        errors.append(
                            f"{ctx}: pr {pr} has position {pos}, but its real frozen-order position is {expected_pos}"
                        )

        elif rt == "actionable_presentation":
            _check_fields(rec, PRESENTATION_REQUIRED, f"{ctx} (actionable_presentation)", errors)
            pr = rec.get("pr")
            roi = rec.get("raw_output_index")
            if isinstance(roi, int) and roi < 0:
                errors.append(f"{ctx}: raw_output_index must be >= 0, got {roi}")
            if isinstance(pr, int) and pr <= 0:
                errors.append(f"{ctx}: pr must be > 0, got {pr}")
            pid = rec.get("presentation_id")
            if all(isinstance(rec.get(k), (str, int)) for k in ("cohort", "pr", "engine", "subject", "bucket", "raw_output_index")) and isinstance(pid, str):
                expected = presentation_id(rec["cohort"], rec["pr"], rec["engine"], rec["subject"], rec["bucket"], rec["raw_output_index"])
                if pid != expected:
                    errors.append(f"{ctx}: presentation_id {pid!r} does not match the derived value {expected!r}")
            if isinstance(pid, str):
                if pid in presentation_ids_seen:
                    errors.append(f"{ctx}: duplicate presentation_id {pid!r}")
                presentation_ids_seen.add(pid)
            key = (cohort, pr)
            if key not in all_pr_summary_keys:
                errors.append(f"{ctx}: presentation for pr={pr} cohort={cohort} has no matching pr_summary (unknown/non-processed PR)")
            else:
                presentation_count_by_pr[key] = presentation_count_by_pr.get(key, 0) + 1
            _check_reviews(rec, FINDING_REVIEW_REQUIRED, ctx, errors)

        elif rt == "pass_adjudication":
            _check_fields(rec, PASS_RECORD_REQUIRED, f"{ctx} (pass_adjudication)", errors)
            pr = rec.get("pr")
            if isinstance(pr, int) and pr <= 0:
                errors.append(f"{ctx}: pr must be > 0, got {pr}")
            pid = rec.get("pass_id")
            if all(isinstance(rec.get(k), (str, int)) for k in ("cohort", "pr", "engine", "subject")) and isinstance(pid, str):
                expected = pass_id(rec["cohort"], rec["pr"], rec["engine"], rec["subject"])
                if pid != expected:
                    errors.append(f"{ctx}: pass_id {pid!r} does not match the derived value {expected!r}")
            if isinstance(pid, str):
                if pid in pass_ids_seen:
                    errors.append(f"{ctx}: duplicate pass_id {pid!r}")
                pass_ids_seen.add(pid)
            key = (cohort, pr)
            if key not in all_pr_summary_keys:
                errors.append(f"{ctx}: pass_adjudication for pr={pr} cohort={cohort} has no matching pr_summary (unknown/non-processed PR)")
            else:
                pass_count_by_pr[key] = pass_count_by_pr.get(key, 0) + 1
            _check_reviews(rec, PASS_REVIEW_REQUIRED, ctx, errors)

        elif rt == "tool_error":
            _check_fields(rec, TOOL_ERROR_REQUIRED, f"{ctx} (tool_error)", errors)
            pr = rec.get("pr")
            if isinstance(pr, int) and pr <= 0:
                errors.append(f"{ctx}: pr must be > 0, got {pr}")
            key = (cohort, pr)
            if key not in all_pr_summary_keys:
                errors.append(f"{ctx}: tool_error for pr={pr} cohort={cohort} has no matching pr_summary (unknown/non-processed PR)")
            else:
                tool_error_count_by_pr[key] = tool_error_count_by_pr.get(key, 0) + 1

        elif rt == "protocol_note":
            if "reason" not in rec or not isinstance(rec.get("reason"), str) or not rec["reason"]:
                errors.append(f"{ctx} (protocol_note): missing non-empty 'reason'")

    # Cross-record: the three symmetric count invariants -- every
    # pr_summary's own claimed counts must equal the real number of
    # matching detail records, in EITHER direction (missing OR excess).
    for key, summary in pr_summaries.items():
        for count_field, actual_map, label in (
            ("actionable_count", presentation_count_by_pr, "actionable_presentation"),
            ("pass_verdict_count", pass_count_by_pr, "pass_adjudication"),
            ("tool_error_count", tool_error_count_by_pr, "tool_error"),
        ):
            expected = summary.get(count_field)
            actual = actual_map.get(key, 0)
            if isinstance(expected, int) and expected != actual:
                errors.append(
                    f"pr_summary cohort={key[0]} pr={key[1]}: {count_field}={expected} but {actual} real {label} record(s) present"
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
# Mechanical stopping-rule check (item 6 of the earlier round).
# ---------------------------------------------------------------------
def check_stopping_rule(records: list[dict], s4a_order: list[int], s4b_order: list[int]) -> list[str]:
    errors: list[str] = []
    notes = {r["cohort"] for r in records if r.get("record_type") == "protocol_note"}

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
        errors.append(f"S4-A stopped at {len(a_summaries)}, before the required stopping point {required_stop_a}")
    if len(a_summaries) > required_stop_a and "A" not in notes:
        errors.append(
            f"S4-A continued to {len(a_summaries)}, past the first valid stopping point {required_stop_a}, with no documented protocol_note reason"
        )

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
        errors.append(f"S4-B stopped at {len(b_summaries)}, before the required stopping point {required_stop_b}")
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
# Resolved judgments.
# ---------------------------------------------------------------------
def _disagrees(rec: dict, judgment_fields: list[str]) -> bool:
    reviews = rec.get("reviews") or []
    if len(reviews) != 2:
        return False
    return any(reviews[0].get(f) != reviews[1].get(f) for f in judgment_fields)


_FINDING_JUDGMENT_FIELDS = [
    "scanner_substance_correct",
    "pr_relevant",
    "causal_framing_correct",
    "rendered_presentation_correct",
]
_PASS_JUDGMENT_FIELDS = ["genuine_evidence"]


def _has_unresolved_disagreement(rec: dict) -> bool:
    return _disagrees(rec, _FINDING_JUDGMENT_FIELDS) and rec.get("coordinator_resolution") is None


def _has_unresolved_pass_disagreement(rec: dict) -> bool:
    return _disagrees(rec, _PASS_JUDGMENT_FIELDS) and rec.get("coordinator_resolution") is None


def resolved_judgment(rec: dict) -> dict | None:
    """The presentation's own final judgment: the coordinator
    resolution if one is present; either (agreeing) review when both
    reviewers agree; **None** when the two reviewers genuinely
    disagree and no coordinator_resolution has been recorded yet --
    UNDETERMINED, never defaulted to either reviewer's own opinion."""
    if rec.get("coordinator_resolution") is not None:
        return rec["coordinator_resolution"]
    if _has_unresolved_disagreement(rec):
        return None
    return rec["reviews"][0]


def resolved_pass_judgment(rec: dict) -> dict | None:
    """Same principle as resolved_judgment, for a pass_adjudication's
    own genuine_evidence question."""
    if rec.get("coordinator_resolution") is not None:
        return rec["coordinator_resolution"]
    if _has_unresolved_pass_disagreement(rec):
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
    disagreement -- their correctness is undetermined, not "0"."""
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


def evaluate_gate(records: list[dict]) -> GateResult:
    """3-tier precedence, frozen in protocol.md:
    1. any hard correctness failure -> FAIL, unconditionally.
    2. else insufficient volume/undefined precision/unresolved
       disagreement/exhausted cap -> INSUFFICIENT EVIDENCE.
    3. else evaluate precision/CI-bound conditions -> PASS or FAIL.
    """
    presentations = [r for r in records if r["record_type"] == "actionable_presentation"]
    pass_records = [r for r in records if r["record_type"] == "pass_adjudication"]
    a_correct, a_total = cohort_precision(records, "A")
    b_correct, b_total = cohort_precision(records, "B")
    total_correct = a_correct + b_correct
    total_presentations = a_total + b_total

    determined = [r for r in presentations if resolved_judgment(r) is not None]
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

    pass_determined = [r for r in pass_records if resolved_pass_judgment(r) is not None]
    false_pass = [r for r in pass_determined if not resolved_pass_judgment(r).get("genuine_evidence")]
    unresolved_pass_disagreements = [r for r in pass_records if _has_unresolved_pass_disagreement(r)]

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
        insufficient_reasons.append(f"{len(unresolved_disagreements)} unresolved reviewer disagreement(s) on actionable presentations")
    if unresolved_pass_disagreements:
        insufficient_reasons.append(f"{len(unresolved_pass_disagreements)} unresolved PASS-adjudication disagreement(s)")
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
# Error inventories -- each its own machine-derived list.
# ---------------------------------------------------------------------
def build_inventories(records: list[dict]) -> dict[str, list[dict]]:
    presentations = [r for r in records if r["record_type"] == "actionable_presentation"]
    pass_records = [r for r in records if r["record_type"] == "pass_adjudication"]
    determined = [r for r in presentations if resolved_judgment(r) is not None]
    pass_determined = [r for r in pass_records if resolved_pass_judgment(r) is not None]
    return {
        "false_finding": [r for r in determined if not resolved_judgment(r).get("scanner_substance_correct")],
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
        "false_pass": [r for r in pass_determined if not resolved_pass_judgment(r).get("genuine_evidence")],
        "tool_error_ordinary": [
            r for r in records if r["record_type"] == "tool_error" and not r.get("attributable_to_unsupported_input")
        ],
        "unresolved_disagreement": [r for r in presentations if _has_unresolved_disagreement(r)],
        "unresolved_pass_disagreement": [r for r in pass_records if _has_unresolved_pass_disagreement(r)],
    }


# ---------------------------------------------------------------------
# Full metrics.
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

    total_inconclusive = 0
    root_cause_counts: dict[str, int] = {}
    total_origin_unclear = 0
    for r in pr_summaries:
        total_inconclusive += r.get("inconclusive_count", 0) or 0
        total_origin_unclear += r.get("origin_unclear_count", 0) or 0
        for cause in r.get("inconclusive_root_causes", []) or []:
            root_cause_counts[cause] = root_cause_counts.get(cause, 0) + 1

    disagreement_log = []
    for r in records:
        if r["record_type"] == "actionable_presentation":
            disagree = _disagrees(r, _FINDING_JUDGMENT_FIELDS)
            disagreement_log.append({
                "kind": "actionable_presentation",
                "id": r.get("presentation_id"),
                "pr": r.get("pr"),
                "cohort": r.get("cohort"),
                "disagreed": disagree,
                "resolved": disagree and r.get("coordinator_resolution") is not None,
            })
        elif r["record_type"] == "pass_adjudication":
            disagree = _disagrees(r, _PASS_JUDGMENT_FIELDS)
            disagreement_log.append({
                "kind": "pass_adjudication",
                "id": r.get("pass_id"),
                "pr": r.get("pr"),
                "cohort": r.get("cohort"),
                "disagreed": disagree,
                "resolved": disagree and r.get("coordinator_resolution") is not None,
            })

    return {
        "processed_a": len(a_summaries),
        "processed_b": len(b_summaries),
        "applicability_rate": (len(applicable) / len(pr_summaries)) if pr_summaries else None,
        "actionable_presentation_rate": ((a_total + b_total) / len(pr_summaries)) if pr_summaries else None,
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


# ---------------------------------------------------------------------
# The single production pipeline -- used by --ledger, --json, AND the
# self-test suite. This is the one path that really runs, so every
# hostile self-test exercises it directly (item 5).
# ---------------------------------------------------------------------
def run_pipeline(ledger_path: Path, s4a_path: Path | None, s4b_path: Path | None) -> dict:
    records = load_ledger(ledger_path)
    s4a_order = load_frozen_order(s4a_path) if s4a_path else None
    s4b_order = load_frozen_order(s4b_path) if s4b_path else None

    schema_errors = validate_ledger(records, s4a_order, s4b_order)
    if schema_errors:
        return {"status": "schema_error", "errors": schema_errors}

    if s4a_order is not None and s4b_order is not None:
        stopping_errors = check_stopping_rule(records, s4a_order, s4b_order)
        if stopping_errors:
            return {"status": "stopping_error", "errors": stopping_errors}

    gate = evaluate_gate(records)
    metrics = build_metrics(records)
    inventories = build_inventories(records)
    return {
        "status": "ok",
        "gate": {"verdict": gate.verdict, "reasons": gate.reasons},
        "metrics": metrics,
        "inventories": {k: [r for r in v] for k, v in inventories.items()},
        "inventory_counts": {k: len(v) for k, v in inventories.items()},
    }


def render_text_report(result: dict) -> str:
    if result["status"] != "ok":
        lines = [f"VALIDATION FAILED ({result['status']}):"]
        lines += [f"  - {e}" for e in result["errors"]]
        return "\n".join(lines) + "\n"
    m = result["metrics"]
    inv = result["inventory_counts"]
    gate = result["gate"]
    lines = [
        "# S4 report (generated, not hand-typed)",
        "",
        f"- Processed: S4-A {m['processed_a']}, S4-B {m['processed_b']}",
        f"- S4-A actionable precision: {m['a_precision'][0]}/{m['a_precision'][1]}",
        f"- S4-B actionable precision: {m['b_precision'][0]}/{m['b_precision'][1]}",
        f"- Pooled one-sided 95% exact lower bound: {m['pooled_lower_bound']:.4f}",
        f"- False PASS: {inv['false_pass']}",
        f"- False finding: {inv['false_finding']}",
        f"- PR-relevance errors: {inv['pr_relevance_error']}",
        f"- Causal-framing errors: {inv['causal_framing_error']}",
        f"- Rendered-presentation errors: {inv['rendered_presentation_error']}",
        f"- TOOL_ERROR (ordinary input): {inv['tool_error_ordinary']}",
        f"- Unresolved disagreements: {inv['unresolved_disagreement']}",
        f"- Unresolved PASS disagreements: {inv['unresolved_pass_disagreement']}",
        f"- Deployment gate: **{gate['verdict']}**",
        "- Reasons: " + "; ".join(gate["reasons"]),
    ]
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------
# CLI plumbing.
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


def run_report(ledger_path: Path, s4a_path: Path | None, s4b_path: Path | None, as_json: bool) -> int:
    result = run_pipeline(ledger_path, s4a_path, s4b_path)
    if as_json:
        print(json.dumps(result, indent=2, default=str))
    else:
        print(render_text_report(result))
    if result["status"] == "schema_error":
        print("SCHEMA VALIDATION FAILED -- a report-generation TOOL_ERROR, not an analytical result.", file=sys.stderr)
        return 3
    if result["status"] == "stopping_error":
        print("STOPPING-RULE VALIDATION FAILED -- a report-generation TOOL_ERROR.", file=sys.stderr)
        return 3
    return 0


# =======================================================================
# Hostile self-test suite -- exercised through run_pipeline() (the SAME
# production path --ledger/--json use), by materializing each hostile
# case to a real temp JSONL file and real temp frozen-order JSON files,
# never a private gate-only helper.
# =======================================================================
def _fake_sha40(tag: str) -> str:
    return hashlib.sha1(tag.encode()).hexdigest()


def _fake_sha256(tag: str) -> str:
    return hashlib.sha256(tag.encode()).hexdigest()


def _freview(reviewer_id: str, correct: bool = True, **overrides) -> dict:
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


def _preview(reviewer_id: str, genuine: bool = True, **overrides) -> dict:
    r = {"reviewer_id": reviewer_id, "genuine_evidence": genuine, "notes": "selftest"}
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
        "command": f"oba audit-diff ... (pr {pr})",
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


def _presentation(cohort: str, pr: int, subject: str, correct: bool = True, idx: int = 0) -> dict:
    return {
        "record_type": "actionable_presentation",
        "cohort": cohort,
        "pr": pr,
        "presentation_id": presentation_id(cohort, pr, "oba", subject, "new_finding", idx),
        "engine": "oba",
        "subject": subject,
        "bucket": "new_finding",
        "raw_output_index": idx,
        "reviews": [_freview("reviewer-1", correct), _freview("reviewer-2", correct)],
        "coordinator_resolution": None,
    }


def _pass_record(cohort: str, pr: int, subject: str, genuine: bool = True) -> dict:
    return {
        "record_type": "pass_adjudication",
        "cohort": cohort,
        "pr": pr,
        "pass_id": pass_id(cohort, pr, "oba", subject),
        "engine": "oba",
        "subject": subject,
        "reviews": [_preview("reviewer-1", genuine), _preview("reviewer-2", genuine)],
        "coordinator_resolution": None,
    }


def _clean_pool(a_count: int, b_count: int) -> tuple[list[dict], list[int], list[int]]:
    records = []
    s4a_order = list(range(1, 1 + max(a_count, 60)))
    s4b_order = list(range(10000, 10000 + max(b_count, 40)))
    for i in range(a_count):
        pr = s4a_order[i]
        records.append(_pr_summary("A", pr, i + 1, actionable_count=1))
        records.append(_presentation("A", pr, f"opt{i}"))
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


def _run_via_pipeline(tmpdir: Path, tag: str, records: list[dict], s4a_order: list[int], s4b_order: list[int]) -> dict:
    """Materializes a hostile case to real temp files and runs it
    through the exact same run_pipeline() the --ledger/--json CLI uses."""
    ledger_path = tmpdir / f"{tag}.jsonl"
    with ledger_path.open("w") as f:
        for r in records:
            f.write(json.dumps(r) + "\n")
    s4a_path = tmpdir / f"{tag}-s4a.json"
    s4b_path = tmpdir / f"{tag}-s4b.json"
    s4a_path.write_text(json.dumps([{"number": n} for n in s4a_order]))
    s4b_path.write_text(json.dumps([{"number": n} for n in s4b_order]))
    return run_pipeline(ledger_path, s4a_path, s4b_path)


def run_selftest() -> None:
    import tempfile

    failures: list[str] = []

    def check(name: str, condition: bool) -> None:
        status = "ok" if condition else "FAILED"
        print(f"[selftest] {name}: {status}")
        if not condition:
            failures.append(name)

    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)

        def run(tag, records, s4a, s4b):
            return _run_via_pipeline(tmpdir, tag, records, s4a, s4b)

        # 1. Clean 30/30 (12 from A, 18 from B) -> PASS.
        records, s4a, s4b = _clean_pool(12, 18)
        result = run("t01", records, s4a, s4b)
        check("clean 30/30, all structural requirements met -> PASS", result["gate"]["verdict"] == "PASS" if result["status"] == "ok" else False)

        # 2. CP lower bound worked example.
        lb = clopper_pearson_lower_bound(30, 30)
        check(f"CP lower bound(30/30) ~= 0.905 (got {lb:.4f})", abs(lb - 0.9050) < 0.001)

        # 3-5. insufficient-count ledger + {false PASS, false finding, framing error} -> FAIL
        records, s4a, s4b = _clean_pool(5, 3)
        pr = s4a[0]
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == pr:
                r["pass_verdict_count"] = 1
        records.append(_pass_record("A", pr, "enable", genuine=False))
        result = run("t03", records, s4a, s4b)
        check("insufficient-count ledger + false PASS -> FAIL (hard failure precedence)", result["status"] == "ok" and result["gate"]["verdict"] == "FAIL")

        records, s4a, s4b = _clean_pool(5, 3)
        records.append(_presentation("A", s4a[0], "brokenopt", correct=False, idx=99))
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == s4a[0]:
                r["actionable_count"] = 2
        result = run("t04", records, s4a, s4b)
        check("insufficient-count ledger + false finding -> FAIL", result["status"] == "ok" and result["gate"]["verdict"] == "FAIL")

        records, s4a, s4b = _clean_pool(5, 3)
        p = _presentation("A", s4a[0], "framingopt", correct=True, idx=98)
        p["reviews"][0]["causal_framing_correct"] = False
        p["reviews"][1]["causal_framing_correct"] = False
        records.append(p)
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == s4a[0]:
                r["actionable_count"] = 2
        result = run("t05", records, s4a, s4b)
        check("insufficient-count ledger + framing error -> FAIL", result["status"] == "ok" and result["gate"]["verdict"] == "FAIL")

        # 6. 30 duplicate copies of one presentation -> schema rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        dup = _presentation("A", s4a[0], "opt0", idx=0)
        records.append(dup)
        result = run("t06", records, s4a, s4b)
        check("duplicate presentation_id -> schema rejection, not PASS", result["status"] == "schema_error")

        # 7. PR not matching frozen order at its position -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["position"] == 1:
                r["pr"] = 999999
        result = run("t07", records, s4a, s4b)
        check("PR not matching frozen order at its position -> rejection", result["status"] in ("schema_error", "stopping_error"))

        # 8. Fewer than 60 A processed despite enough actionable -> rejection.
        s4a_full = list(range(1, 121))
        s4b_full = list(range(10000, 10080))
        recs = []
        for i in range(15):
            pr = s4a_full[i]
            recs.append(_pr_summary("A", pr, i + 1, actionable_count=1))
            recs.append(_presentation("A", pr, f"opt{i}"))
        for i in range(18):
            pr = s4b_full[i]
            recs.append(_pr_summary("B", pr, i + 1, actionable_count=1))
            recs.append(_presentation("B", pr, f"bopt{i}"))
        result = run("t08", recs, s4a_full, s4b_full)
        check("fewer than 60 A processed despite enough actionable -> rejection", result["status"] == "stopping_error")

        # 9. Fewer than 40 B processed despite enough actionable -> rejection.
        records, s4a, s4b = _clean_pool(12, 0)
        records2 = [r for r in records if not (r.get("record_type") == "pr_summary" and r["cohort"] == "B")]
        for i in range(15):
            pr = s4b[i]
            records2.append(_pr_summary("B", pr, i + 1, actionable_count=1))
            records2.append(_presentation("B", pr, f"bopt{i}"))
        result = run("t09", records2, s4a, s4b)
        check("fewer than 40 B processed despite enough actionable -> rejection", result["status"] == "stopping_error")

        # 10. Skipped frozen-order PR -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        records = [r for r in records if not (r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["position"] == 30)]
        result = run("t10", records, s4a, s4b)
        check("skipped frozen-order position -> rejection", result["status"] in ("schema_error", "stopping_error"))

        # 11. Duplicate PR summary -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        records.append(_pr_summary("A", s4a[0], 1, actionable_count=1))
        result = run("t11", records, s4a, s4b)
        check("duplicate pr_summary -> rejection", result["status"] == "schema_error")

        # 12. Duplicate pass_id -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        pr = s4a[0]
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == pr:
                r["pass_verdict_count"] = 2
        records.append(_pass_record("A", pr, "enable", genuine=True))
        records.append(_pass_record("A", pr, "enable", genuine=True))
        result = run("t12", records, s4a, s4b)
        check("duplicate pass_id -> rejection", result["status"] == "schema_error")

        # 13. Missing required PASS adjudication -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == s4a[0]:
                r["pass_verdict_count"] = 1
        result = run("t13", records, s4a, s4b)
        check("missing required PASS adjudication -> rejection", result["status"] == "schema_error")

        # 14. Actionable presentation with only one reviewer -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        for r in records:
            if r.get("record_type") == "actionable_presentation" and r["pr"] == s4a[0]:
                r["reviews"] = [r["reviews"][0]]
        result = run("t14", records, s4a, s4b)
        check("only 1 reviewer on a presentation -> rejection", result["status"] == "schema_error")

        # 15. Unresolved disagreement, no other hard failure -> INSUFFICIENT EVIDENCE.
        records, s4a, s4b = _clean_pool(12, 18)
        for r in records:
            if r.get("record_type") == "actionable_presentation" and r["pr"] == s4a[0]:
                r["reviews"][1]["causal_framing_correct"] = False
        result = run("t15", records, s4a, s4b)
        check("unresolved disagreement, no other hard failure -> INSUFFICIENT EVIDENCE", result["status"] == "ok" and result["gate"]["verdict"] == "INSUFFICIENT EVIDENCE")

        # 16. Isolated pr_relevant=false -> explicit PR-relevance FAIL reason.
        records, s4a, s4b = _clean_pool(12, 18)
        p = _presentation("A", s4a[0], "irrelevantopt", correct=True, idx=97)
        for rv in p["reviews"]:
            rv["pr_relevant"] = False
        records.append(p)
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == s4a[0]:
                r["actionable_count"] = 2
        result = run("t16", records, s4a, s4b)
        check(
            "isolated pr_relevant=false -> PR-relevance FAIL reason",
            result["status"] == "ok" and result["gate"]["verdict"] == "FAIL" and any("PR-relevance" in r for r in result["gate"]["reasons"]),
        )

        # 17. S4-B zero actionable -> INSUFFICIENT EVIDENCE.
        records, s4a, s4b = _clean_pool(12, 0)
        result = run("t17", records, s4a, s4b)
        check("S4-B zero actionable presentations -> INSUFFICIENT EVIDENCE", result["status"] == "ok" and result["gate"]["verdict"] == "INSUFFICIENT EVIDENCE")

        # --- New PASS-review hostile cases (item 1) ---

        # 18. One-reviewer PASS -> schema rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        pr = s4a[0]
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == pr:
                r["pass_verdict_count"] = 1
        bad_pass = _pass_record("A", pr, "enable", genuine=True)
        bad_pass["reviews"] = [bad_pass["reviews"][0]]
        records.append(bad_pass)
        result = run("t18", records, s4a, s4b)
        check("one-reviewer PASS -> schema rejection", result["status"] == "schema_error")

        # 19. Two identical reviewer IDs on a PASS -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        pr = s4a[0]
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == pr:
                r["pass_verdict_count"] = 1
        bad_pass = _pass_record("A", pr, "enable", genuine=True)
        bad_pass["reviews"] = [_preview("same-reviewer", True), _preview("same-reviewer", True)]
        records.append(bad_pass)
        result = run("t19", records, s4a, s4b)
        check("two identical reviewer IDs on a PASS -> rejection", result["status"] == "schema_error")

        # 20. Two agreeing genuine PASS reviews -> accepted (clean gate stays PASS).
        records, s4a, s4b = _clean_pool(12, 18)
        pr = s4a[0]
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == pr:
                r["pass_verdict_count"] = 1
        records.append(_pass_record("A", pr, "enable", genuine=True))
        result = run("t20", records, s4a, s4b)
        check("two agreeing genuine PASS reviews -> accepted, gate stays PASS", result["status"] == "ok" and result["gate"]["verdict"] == "PASS")

        # 21. Two agreeing false-PASS reviews -> hard FAIL.
        records, s4a, s4b = _clean_pool(12, 18)
        pr = s4a[0]
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == pr:
                r["pass_verdict_count"] = 1
        records.append(_pass_record("A", pr, "enable", genuine=False))
        result = run("t21", records, s4a, s4b)
        check(
            "two agreeing false-PASS reviews -> hard FAIL",
            result["status"] == "ok" and result["gate"]["verdict"] == "FAIL" and any("false PASS" in r for r in result["gate"]["reasons"]),
        )

        # 22. Unresolved PASS disagreement -> INSUFFICIENT EVIDENCE.
        records, s4a, s4b = _clean_pool(12, 18)
        pr = s4a[0]
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == pr:
                r["pass_verdict_count"] = 1
        pass_rec = _pass_record("A", pr, "enable", genuine=True)
        pass_rec["reviews"][1]["genuine_evidence"] = False
        records.append(pass_rec)
        result = run("t22", records, s4a, s4b)
        check("unresolved PASS disagreement -> INSUFFICIENT EVIDENCE", result["status"] == "ok" and result["gate"]["verdict"] == "INSUFFICIENT EVIDENCE")

        # 23. Resolved PASS disagreement -> uses coordinator resolution.
        records, s4a, s4b = _clean_pool(12, 18)
        pr = s4a[0]
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == pr:
                r["pass_verdict_count"] = 1
        pass_rec = _pass_record("A", pr, "enable", genuine=True)
        pass_rec["reviews"][1]["genuine_evidence"] = False
        pass_rec["coordinator_resolution"] = {"resolved_by": "coordinator", "genuine_evidence": False, "notes": "n/a", "rationale": "reviewer-2 was right, re-checked raw evidence"}
        records.append(pass_rec)
        result = run("t23", records, s4a, s4b)
        check(
            "resolved PASS disagreement -> uses coordinator resolution (false) -> hard FAIL",
            result["status"] == "ok" and result["gate"]["verdict"] == "FAIL" and any("false PASS" in r for r in result["gate"]["reasons"]),
        )

        # --- New cross-record count invariant hostile cases (item 2) ---

        # 24. actionable_count mismatch (claims 2, only 1 present) -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == s4a[0]:
                r["actionable_count"] = 2  # only 1 actionable_presentation actually exists for this PR
        result = run("t24", records, s4a, s4b)
        check("actionable_count mismatch -> rejection", result["status"] == "schema_error")

        # 25. tool_error_count mismatch (claims 1, none present) -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == s4a[0]:
                r["tool_error_count"] = 1
        result = run("t25", records, s4a, s4b)
        check("tool_error_count mismatch -> rejection", result["status"] == "schema_error")

        # 26. pass_verdict_count mismatch (claims 0, one pass_adjudication present) -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        pr = s4a[0]
        records.append(_pass_record("A", pr, "enable", genuine=True))  # pass_verdict_count on pr's summary stays 0
        result = run("t26", records, s4a, s4b)
        check("pass_verdict_count mismatch (excess record) -> rejection", result["status"] == "schema_error")

        # --- New numeric/domain validation hostile cases (item 3) ---

        # 27. Negative position -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["position"] == 1:
                r["position"] = -1
        result = run("t27", records, s4a, s4b)
        check("negative position -> rejection", result["status"] == "schema_error")

        # 28. Negative raw_output_index -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        for r in records:
            if r.get("record_type") == "actionable_presentation" and r["pr"] == s4a[0]:
                r["raw_output_index"] = -1
        result = run("t28", records, s4a, s4b)
        check("negative raw_output_index -> rejection", result["status"] == "schema_error")

        # 29. PR <= 0 -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["position"] == 1:
                r["pr"] = 0
        result = run("t29", records, s4a, s4b)
        check("pr <= 0 -> rejection", result["status"] == "schema_error")

        # --- presentation_id / pass_id genuine derivation (item 4) ---

        # 30. presentation_id not matching the derived value -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        for r in records:
            if r.get("record_type") == "actionable_presentation" and r["pr"] == s4a[0]:
                r["presentation_id"] = "not-the-real-derived-id"
        result = run("t30", records, s4a, s4b)
        check("presentation_id not matching derived value -> rejection", result["status"] == "schema_error")

        # 31. pass_id not matching the derived value -> rejection.
        records, s4a, s4b = _clean_pool(12, 18)
        pr = s4a[0]
        for r in records:
            if r.get("record_type") == "pr_summary" and r["cohort"] == "A" and r["pr"] == pr:
                r["pass_verdict_count"] = 1
        bad_pass = _pass_record("A", pr, "enable", genuine=True)
        bad_pass["pass_id"] = "not-the-real-derived-id"
        records.append(bad_pass)
        result = run("t31", records, s4a, s4b)
        check("pass_id not matching derived value -> rejection", result["status"] == "schema_error")

    if failures:
        print(f"\n{len(failures)} selftest check(s) FAILED: {failures}", file=sys.stderr)
        sys.exit(1)
    print(f"\nAll 31 selftest checks passed, all exercised through run_pipeline() (the real --ledger/--json production path).")


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
        as_json = "--json" in sys.argv
        sys.exit(run_report(ledger_path, s4a_path, s4b_path, as_json))
    print("usage: generate-report.py --selftest | --ledger <path.jsonl> [--s4a <orders.json>] [--s4b <orders.json>] [--json]", file=sys.stderr)
    sys.exit(2)


if __name__ == "__main__":
    main()
