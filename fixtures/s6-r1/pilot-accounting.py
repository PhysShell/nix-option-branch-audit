#!/usr/bin/env python3
"""S6-R0A (amends S6-R0, commit 218bf47 -- see R0A-protocol-errata.md
items 3-6): the PR-vs-target unit model, the corrected NC2/NC3/NC4
denominators, the target-multiplicity budget cap, and the corrected
per-batch expansion-gate/stopping logic.

Unit model (R0A item 4):
  - sampling unit: PR.
  - analysis/adjudication unit: target.
  - a PR may yield zero, one, or several mechanically-derived targets
    (per target-construction-protocol.md step 3); every one is
    retained UNLESS the pilot-wide MAX_TARGETS_TOTAL budget cap is
    already exhausted, in which case the excess is recorded, never
    silently dropped, and excluded from every NC count below.

Pure, deterministic, side-effect-free -- no network call, no `oba`
execution, no dependency on real S6 data. See
`test_population_and_sampling.py` for synthetic coverage.
"""
import math

# Frozen thresholds -- must stay byte-identical to
# fixtures/s6-r0/preregistration.md (as amended by R0A-protocol-errata.md).
SAMPLED_PR_COUNT = 15
NC2_MIN_DERIVED_PRS = 9          # of SAMPLED_PR_COUNT
NC3_MIN_FRACTION = 0.7           # of derived_target_count
NC4_MIN_FRACTION = 0.7           # of substantive_target_count
MAX_TARGETS_TOTAL = 30           # pilot-wide budget cap, cost-justified (see
                                 # population-and-sampling.py's own doc comment),
                                 # NOT derived from observed S6 data.
CUMULATIVE_SUBSTANTIVE_TARGET_STOP = 30
CONSECUTIVE_PASSING_BATCHES_STOP = 3


def apply_target_cap(per_pr_targets, cap=MAX_TARGETS_TOTAL):
    """`per_pr_targets`: ordered list of `(pr_number, [target_id, ...])`
    in frozen sample order; each PR's own target list already in
    frozen within-PR order (source position in the diff, per
    target-construction-protocol.md step 3 -- never "most interesting
    first"). Returns `(accepted, cut)`: `accepted` is the flat
    `(pr_number, target_id)` list kept within the cap; `cut` is
    whatever was excluded once the cap was reached -- always returned,
    never silently discarded, so a caller can record
    `target_construction_incomplete_due_to_budget_cap` for each one.
    """
    accepted, cut = [], []
    for pr_number, targets in per_pr_targets:
        for t in targets:
            (accepted if len(accepted) < cap else cut).append((pr_number, t))
    return accepted, cut


def nc2_pr_level(derived_pr_count, sampled_pr_count=SAMPLED_PR_COUNT, min_derived=NC2_MIN_DERIVED_PRS):
    """R0A item 5 -- PR-level. numerator: sampled PRs yielding >=1
    mechanically frozen target. denominator: the fixed sampled-PR
    count. Never affected by target COUNT (a PR with 3 targets counts
    once here, same as a PR with 1)."""
    return {
        "numerator": derived_pr_count,
        "denominator": sampled_pr_count,
        "passed": derived_pr_count >= min_derived,
    }


def nc3_target_level(substantive_target_count, derived_target_count, min_fraction=NC3_MIN_FRACTION):
    """R0A item 5 -- TARGET-level. numerator: derived targets reaching
    a substantive oba result. denominator: ALL derived targets actually
    attempted (never PRs, never mechanical-non-derivations -- those
    affect NC2 only, per R0A's own explicit correction)."""
    if derived_target_count == 0:
        return {"numerator": 0, "denominator": 0, "threshold": 0, "passed": False}
    threshold = math.ceil(min_fraction * derived_target_count)
    return {
        "numerator": substantive_target_count,
        "denominator": derived_target_count,
        "threshold": threshold,
        "passed": substantive_target_count >= threshold,
    }


def nc4_target_level(adjudicable_target_count, substantive_target_count, min_fraction=NC4_MIN_FRACTION):
    """R0A item 5 -- TARGET-level. numerator: substantive targets
    receiving a trustworthy (non-ORACLE_AMBIGUOUS) adjudication.
    denominator: substantive targets (== NC3's own numerator)."""
    if substantive_target_count == 0:
        return {"numerator": 0, "denominator": 0, "threshold": 0, "passed": False}
    threshold = math.ceil(min_fraction * substantive_target_count)
    return {
        "numerator": adjudicable_target_count,
        "denominator": substantive_target_count,
        "threshold": threshold,
        "passed": adjudicable_target_count >= threshold,
    }


def batch_passes_expansion_gates(derived_pr_count, substantive_target_count,
                                  derived_target_count, adjudicable_target_count,
                                  sampled_pr_count=SAMPLED_PR_COUNT):
    """R0A item 3 -- the corrected PER-BATCH expansion gate. Deliberately
    does NOT reference NC1 (a window/census-level condition about raw
    PR density among <=100 unfiltered records) -- every expansion
    batch is drawn from a population that already passed NC1 once, at
    window-establishment time; re-testing NC1's own literal threshold
    (>=20 relevant among <=100 records) against a batch of 15
    ALREADY-relevant PRs is the exact incoherence R0A corrects.
    """
    nc2 = nc2_pr_level(derived_pr_count, sampled_pr_count)
    nc3 = nc3_target_level(substantive_target_count, derived_target_count)
    nc4 = nc4_target_level(adjudicable_target_count, substantive_target_count)
    return (nc2["passed"] and nc3["passed"] and nc4["passed"]), {"nc2": nc2, "nc3": nc3, "nc4": nc4}


def expansion_stopping_decision(batch_history):
    """R0A item 3 -- the corrected larger-S6 stopping rule. `batch_history`:
    ordered list of dicts, each with `derived_pr_count`,
    `substantive_target_count`, `derived_target_count`,
    `adjudicable_target_count`, and optionally `sampled_pr_count`
    (defaults to SAMPLED_PR_COUNT). Returns one of "CONTINUE",
    "STOP_CUMULATIVE_TARGET_COUNT", "STOP_CONSECUTIVE_PASSING_BATCHES".
    The stop is on CUMULATIVE SUBSTANTIVE TARGETS (the NC3/NC4 unit),
    never PRs -- matching S6-R0A's own corrected unit model.
    """
    cumulative_targets = sum(b["substantive_target_count"] for b in batch_history)
    if cumulative_targets >= CUMULATIVE_SUBSTANTIVE_TARGET_STOP:
        return "STOP_CUMULATIVE_TARGET_COUNT"
    if len(batch_history) >= CONSECUTIVE_PASSING_BATCHES_STOP:
        last_n = batch_history[-CONSECUTIVE_PASSING_BATCHES_STOP:]
        all_passed = all(
            batch_passes_expansion_gates(
                b["derived_pr_count"], b["substantive_target_count"],
                b["derived_target_count"], b["adjudicable_target_count"],
                b.get("sampled_pr_count", SAMPLED_PR_COUNT),
            )[0]
            for b in last_n
        )
        if all_passed:
            return "STOP_CONSECUTIVE_PASSING_BATCHES"
    return "CONTINUE"


if __name__ == "__main__":
    raise SystemExit(
        "This script is a frozen SPECIFICATION for S6-R0A's own corrected "
        "accounting model. It is committed but not executed against real "
        "S6 pilot data by this round."
    )
