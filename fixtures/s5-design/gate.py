#!/usr/bin/env python3
"""S5 protocol machinery: pure functions implementing the frozen
protocol's own aggregation/stopping/gate rules. No `oba` invocation, no
network, no file I/O beyond what's explicitly passed in -- these are
the RULES themselves, unit-tested against synthetic data in
`test_s5_protocol_machinery.py` since no real S5-B adjudication exists
yet. `s5-protocol-final.md` points at this module as its own
machine-readable definition of the gate.
"""
from dataclasses import dataclass, field


def is_unbroken_prefix(positions: list[int]) -> bool:
    """A cohort's processed positions must be exactly {1, ..., N} for
    some N >= 0 -- an unbroken 1..N prefix of its frozen order, never a
    gap or an out-of-order skip (S4's own frozen-order PREFIX property,
    carried forward unchanged).
    """
    if not positions:
        return True
    s = sorted(positions)
    return s == list(range(1, len(s) + 1))


def two_reviewers_ok(reviewer_ids: list[str]) -> bool:
    """Exactly 2 DISTINCT reviewers per actionable presentation or
    materially-relevant PASS -- not merely 'at least 2 entries'."""
    return len(reviewer_ids) == 2 and len(set(reviewer_ids)) == 2


@dataclass
class Presentation:
    presentation_id: str
    correct: bool  # the RESOLVED judgment, never a raw single review


@dataclass
class ActionablePR:
    pr: int
    presentations: list[Presentation] = field(default_factory=list)

    def is_correct(self) -> bool:
        """An actionable PR counts as ONE correct PR-level observation
        only if EVERY one of its actionable presentations is
        independently adjudicated correct. A PR with zero presentations
        is not an actionable PR at all -- calling this on one is a
        caller bug, not a valid 'vacuously correct' case.
        """
        if not self.presentations:
            raise ValueError(f"PR #{self.pr} has no actionable presentations -- not an actionable PR")
        return all(p.correct for p in self.presentations)


def pr_level_precision_counts(actionable_prs: list[ActionablePR]) -> tuple[int, int]:
    """Returns (correct_pr_count, total_pr_count) -- the PRIMARY
    statistical-unit counts for the S5-B confidence bound. Every
    presentation still individually adjudicated; only this aggregation
    changes the denominator from presentations to distinct PRs.
    """
    total = len(actionable_prs)
    correct = sum(1 for pr in actionable_prs if pr.is_correct())
    return correct, total


def should_continue_sampling(distinct_actionable_prs_so_far: int, target: int, selected_prs_processed: int, cap: int) -> bool:
    """S5-B's stopping rule is VOLUME-ONLY -- reaching the target or
    exhausting the cap, whichever comes first. This function takes no
    correctness/Tier-1 signal as input AT ALL, by construction: it is
    impossible for a Tier-1 FAIL to influence this decision, proving
    'no optional stopping' at the type level, not just by convention.
    """
    if distinct_actionable_prs_so_far >= target:
        return False
    if selected_prs_processed >= cap:
        return False
    return True


TIER1_FAIL = "FAIL"
TIER2_INSUFFICIENT = "INSUFFICIENT_EVIDENCE"
TIER3_PASS = "PASS"


def evaluate_gate(
    *,
    round_complete: bool,
    tier1_events: list[str],
    distinct_actionable_prs_correct: int,
    distinct_actionable_prs_total: int,
    target: int,
    lower_bound_threshold: float,
    lower_bound_fn,
    false_pass_count: int,
    unresolved_disagreement_count: int,
    s5a_complete: bool,
) -> dict:
    """Tier precedence: Tier 1 (hard FAIL) > Tier 2 (INSUFFICIENT) >
    Tier 3 (the statistical PASS gate), evaluated only once the round
    has reached its pre-registered stop point (`round_complete`) --
    this function does not decide WHEN to stop (see
    `should_continue_sampling`, whose inputs never include any of
    tier1_events/false_pass_count/unresolved_disagreement_count); it
    only classifies the OUTCOME once sampling has already stopped.

    A Tier-1 FAIL is returned even if evaluated on an incomplete round
    (a hard failure is a hard failure regardless of when it's
    discovered) -- but this function does not itself decide to STOP
    the round early because of it; `should_continue_sampling` alone
    governs stopping, called independently, every iteration.
    """
    all_tier1 = list(tier1_events)
    if false_pass_count > 0:
        all_tier1.append(f"{false_pass_count} false PASS")
    if unresolved_disagreement_count > 0:
        all_tier1.append(f"{unresolved_disagreement_count} unresolved disagreement(s)")

    if all_tier1:
        return {"verdict": TIER1_FAIL, "reasons": all_tier1, "round_complete": round_complete}

    if not round_complete:
        return {"verdict": None, "reasons": ["round not yet at its pre-registered stop point"], "round_complete": False}

    if not s5a_complete:
        return {"verdict": TIER2_INSUFFICIENT, "reasons": ["S5-A incomplete"], "round_complete": round_complete}

    if distinct_actionable_prs_total < target:
        return {
            "verdict": TIER2_INSUFFICIENT,
            "reasons": [f"S5-B cap exhausted before reaching {target} distinct actionable PRs "
                        f"(reached {distinct_actionable_prs_total})"],
            "round_complete": round_complete,
        }

    if distinct_actionable_prs_correct != distinct_actionable_prs_total:
        return {
            "verdict": TIER1_FAIL,
            "reasons": [f"{distinct_actionable_prs_total - distinct_actionable_prs_correct} "
                        "actionable PR(s) not all-correct -- should already be in tier1_events, this is a consistency backstop"],
            "round_complete": round_complete,
        }

    lower_bound = lower_bound_fn(distinct_actionable_prs_correct, distinct_actionable_prs_total)
    if lower_bound < lower_bound_threshold:
        return {
            "verdict": TIER2_INSUFFICIENT,
            "reasons": [f"one-sided 95% lower bound {lower_bound:.4f} < threshold {lower_bound_threshold}"],
            "round_complete": round_complete,
            "lower_bound": lower_bound,
        }

    return {"verdict": TIER3_PASS, "reasons": [], "round_complete": round_complete, "lower_bound": lower_bound}
