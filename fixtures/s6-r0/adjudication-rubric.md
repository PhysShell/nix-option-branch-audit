# S6 adjudication rubric (frozen before any P1 result is examined)

**Unit note (S6-R0A)**: every "P1 case" below is a **target**, not a
PR — one sampled PR may yield several targets (see
`target-construction-protocol.md`'s own S6-R0A correction), and every
one of them is adjudicated independently, up to the pilot-wide
`MAX_TARGETS_TOTAL` budget cap.

## Oracle procedure (item 10)

For every P1 target that reaches a substantive `oba` result (not
`INPUT_OR_HARNESS_FAILURE`), the reviewer is handed an **evidence
packet** containing only:

- the real base/head module source for the target's own declaration
  and its own directly-associated predicate/config-generation site
  (the same span-level evidence this project's own investigation
  rounds have always used — not a summary, the actual source lines);
- the real base/head test source for whatever file the frozen target
  record names;
- the frozen target record itself (`option_prefix`, `watch`,
  `cfg_ident`, base/head SHAs) — the reviewer already knows this,
  since they are the same person who performed target construction
  (see "What cannot be blinded" below);
- any statically-literal migration directive
  (`mkRenamedOptionModule`/`mkRenamedOptionModuleWith`/
  `mkRemovedOptionModule`) touching the target's own complete path, if
  the reviewer's own reading of the module source surfaces one —
  reviewers are NOT told this is relevant in advance; they find it (or
  don't) from the real source the same way `oba` itself would have to.

The evidence packet does **not** include `oba`'s own verdict, evidence
list, predicate-attempt results, or rendered summary text at this
stage.

## The blinded question (item 11)

The reviewer answers, in writing, before `oba`'s own verdict is
revealed:

> "Given this base/head module and test source alone, what does the
> evidence establish about this watched option's own declaration and
> predicate-outcome transition between base and head? State one of:
> a provable transition exists (describe it); no transition is
> provable from this evidence (describe why — no test evidence, no
> predicate, unresolvable default, etc.); the option was renamed/
> removed/relocated (describe the real directive found, if any); or
> the evidence is insufficient to decide (describe what's missing)."

Only after this written answer is recorded does the reviewer see
`oba`'s own actual verdict/evidence for the same target, and classify
the pair into exactly one taxonomy label (see below).

**What cannot practically be blinded** (stated explicitly, per item
11's own instruction, not silently ignored): the reviewer already
knows which option/predicate was selected as the watched target, since
target construction (frozen before running `oba`, per
`target-construction-protocol.md`) and adjudication are performed by
the same process in this pilot's own cheap-escalation design (see
below). This means the reviewer is never blind to "which option
matters here" — only to `oba`'s own conclusion about it. This is an
accepted, disclosed limitation of a cheap single-reviewer-by-default
design, not something this rubric pretends is fully blinded.

## Reviewer escalation rule (item 12)

One primary reviewer decides every P1 target via the blinded procedure
above. A second, INDEPENDENT reviewer (a fresh process/agent instance
with no memory of the primary reviewer's own written answer, given
only the same evidence packet and the same blinded question) is
invoked **only** when the primary reviewer's own post-reveal
classification is one of:

- `FALSE_FINDING`, `MISSED_FINDING`, or `MISLEADING_PRESENTATION`
  (suspected analyzer defect);
- `ORACLE_AMBIGUOUS` on the primary reviewer's own first pass;
- the primary reviewer cannot confidently match the case's own
  structural shape to any pattern already documented in
  `fixtures/s5-remediation-closeout/closeout.md` or the existing
  hostile-test corpus (candidate "novel semantic class");
- the primary reviewer self-flags low confidence in their own written
  answer, independent of the other three triggers.

No other case receives a second reviewer. This is a deliberate,
frozen, cheap-by-default policy — not S5's own two-reviewer-for-
everything convention, per S6-R0's own explicit instruction not to
reproduce that cost automatically.

## Error taxonomy (item 13, restated verbatim as the frozen classification)

- **CORRECT** — analyzer conclusion materially agrees with the
  reviewer's own pre-reveal, evidence-based answer.
- **CONSERVATIVE_INCONCLUSIVE** — analyzer refuses/fails closed where
  real evidence exists but unsupported semantics prevent a firm
  result. Not automatically a correctness defect.
- **FALSE_FINDING** — analyzer reports a substantive problem the
  evidence contradicts.
- **MISSED_FINDING** — analyzer returns PASS/unchanged/inconclusive
  where a relevant defect should have been surfaced under the
  analyzer's own claimed contract.
- **MISLEADING_PRESENTATION** — structured/core evidence may be
  technically defensible, but the maintainer-facing interpretation is
  materially wrong.
- **INPUT_OR_HARNESS_FAILURE** — cannot evaluate analyzer correctness
  at all (fetch failure, parse crash, missing file, etc.).
- **ORACLE_AMBIGUOUS** — the reviewer's own evidence is insufficient
  for a trustworthy correctness judgment.
- **KNOWN_LIMITATION_REOBSERVED** — matches an already-disclosed
  pre-S6 limitation (the register in
  `fixtures/s5-remediation-closeout/closeout.md`), producing exactly
  its already-documented behavior. Kept distinct from a newly
  discovered defect.

## Definition of "new defect" (item 14, restated)

A case is a new S6 defect only if ALL of:

1. an adjudicated materially wrong output (one of `FALSE_FINDING`,
   `MISSED_FINDING`, `MISLEADING_PRESENTATION`);
2. reproducible evidence (the real base/head source, retained);
3. a clearly identified, violated analyzer invariant or claimed
   behavior (named, not merely "this feels wrong");
4. not merely an already-documented, intentional fail-closed boundary
   producing its own documented behavior (that's
   `KNOWN_LIMITATION_REOBSERVED`, not a new defect).

Do not inflate defect counts by counting `CONSERVATIVE_INCONCLUSIVE`
or `KNOWN_LIMITATION_REOBSERVED` cases as defects.
