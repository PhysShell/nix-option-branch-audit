# Independent closure audit: reviewer task

You are an independent reviewer. You are read-only. Do not create, modify, stage or commit any file. Do not run any command that writes to a repository.

## What you may read
1. The files in this packet directory: 00-product-definition.md, 01-eligibility-rubric.json, 02-target-validity-rubric-v1.json, 03-target-validity-rubric-v2.json, 04-exact-tree-search.sh, 05-source-excerpts.txt, 06-cases.md, and this file.
2. Source evidence only at the pinned SHAs in 06-cases.md, using `git -C /home/tandem/.cache/go-h/nixpkgs-exact show <sha>:<path>`, `git -C ... grep -n <pattern> <sha> -- <path>`, or `git -C ... rev-parse --verify <sha>^{commit}`. Working-tree reads are not evidence.
3. The exact-tree search procedure in 04-exact-tree-search.sh, run against the nixpkgs cache with a pinned SHA, when a case needs a T_NONE or T_UNKNOWN absence decision.

Do not read any other file in the repository, including other files in work/ or reports/, and do not read any earlier review record. The packet and the pinned sources are the only inputs.

## What the question is
Given the frozen product scope in 00-product-definition.md, does the specification tell an implementer or reviewer what to do for each case in 06-cases.md?

You are not asked to decide whether the product should behave in some other way. Do not substitute your own preferred semantics. Derive each answer from the rules and the evidence.

An outcome of AMBIGUOUS, REVIEWER_OWNED, or D_CAPABILITY_LIMITATION is a valid determinate disposition when the specification assigns the case to that class. Report it as such. A case is not undetermined merely because the answer is not mechanical.

Only report a case as missing a disposition when the specification gives no rule that assigns it, or when two mandatory rules give incompatible dispositions. Name the rule and say what it leaves open.

## Disposition classes
- MECHANICAL: a named rule decides the outcome and a validator could implement it from the rule and exact-SHA source.
- OWNER_RULED: an owner ruling in the rubric decides the outcome.
- V_INVALID: target validity is V_INVALID under a named reason.
- AMBIGUOUS: the rules assign an explicit ambiguity state (for example V_AMBIGUOUS, E_AMBIGUOUS, or P_UNKNOWN). This is a determinate disposition.
- REVIEWER_OWNED: the rules assign the decision to a reviewer.
- D_CAPABILITY_LIMITATION: the construct is outside what the bounded D stage can represent, and the rules say so.

## Report format (plain text, at most 400 lines)
One block per case or sub-case:

CASE: <id>
protocol disposition: <one class above>
outcome detail: <dependency state and role if relevant; E category or "none of the four"; V state and reason if V_INVALID>
rule/evidence used: <clause names and any SHA/path/line evidence you read>
operationally unambiguous: YES or NO
missing rule: <"none", or the rule that is missing and what it leaves open>

Then a section "Mandatory-rule conflicts": "none", or each conflict with the rule names.

Then a section "Source discrepancies": any place where the excerpts in 05-source-excerpts.txt or the case text disagrees with the pinned source. Say what you read at the SHA. "none" if none.

Do not add a summary, a verdict, or any recommendation about READY or PARTIAL. The controller decides the verdict.
