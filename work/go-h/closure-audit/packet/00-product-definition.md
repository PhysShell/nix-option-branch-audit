# Product definition (README.md at the audited commit, verbatim)

   1  # oba — Option Branch Activation evidence
   2  
   3  Layer 1 only, of the three explicitly separated layers agreed on before
   4  writing any code:
   5  
   6  ```
   7  A. option branch activation    — did a test drive this branch's predicate
   8     (OBA, this tool)              to a different outcome than its default?
   9  B. consumer contract drift      — does the emitted key name match what
  10     (CDC, not built)               the pinned consumer actually recognizes?
  11  C. runtime observability        — does the value actually change observed
  12     (ROB, not built)               behavior, or is it masked by a fallback?
  13  ```
  14  
  15  `B PASS` does not imply `C PASS`, and this tool does not attempt either —
  16  it answers exactly one question: **did a test ever drive this branch's
  17  predicate to the *opposite* boolean outcome from what the option's own
  18  default produces.** Not "was some non-default value assigned" — a
  19  non-null value that still evaluates the same predicate the same way as the
  20  default (see `c6a` below) proves nothing, and a `PASS` verdict is never
  21  reachable from one. A `PASS` verdict from this tool is *activation
  22  evidence*, not a correctness proof — see the `kimai-after` golden result
  23  below for a real illustration of exactly why that distinction matters.
  24  

...

 163  ## Non-goals (deliberate)
 164  
 165  - **No `let`-bound alias resolution.** `foo = cfg.x; if foo != null then
 166    ...` is invisible to this MVP. Reported as `PredicateNotFound`, never
 167    silently absorbed into a `PASS`.
 168  - **No VM tests, no consumer knowledge.** The tool doesn't run anything
 169    and doesn't know what Doctrine, or any other consumer, is.
 170  - **No claim about runtime behavior.** `PASS` means "a test drove this
 171    predicate to the opposite outcome from its default", full stop — never
 172    "the fix works", never "this is safe".
 173  - **No Nix evaluation.** `ValueClass` is a syntactic classifier, not an
 174    interpreter. `builtins.elem "x" [ "x" "y" ]` is `true` at runtime and
 175    `Unknown` to this tool, on purpose (see `c11` below) — evaluating
 176    arbitrary Nix expressions is a different, much bigger tool than this
 177    one, and pretending otherwise is exactly the kind of overclaiming this
 178    whole layered design exists to avoid.
 179  
 180  ## Golden corpus

# Scope object (eligibility-rubric scope, verbatim)

{
  "in": "Changes to an option's semantics (declaration, default, type, apply, readOnly, path) that are traceable to an option-dependent branch predicate; changes to a boolean predicate that directly or indirectly consumes option state and controls selection, emission, enabling, rejection or branch choice.",
  "out": [
    "Documentation-only changes (U1).",
    "Configuration-value assignments config.<path> = ... (U2): separate mode, not Layer A.",
    "Changes to warning or assertion message text, or to data that a condition reads, when the condition expression itself is unchanged.",
    "Shell or external-command tests (including inside derivation strings); their operands are command output, not Nix option state.",
    "Test-only changes (witness only; counts for R, never for E). Test files are not Layer-A consumers.",
    "Import-list registration, package-only changes, and runtime behavior or observability (README non-goals; layer C). Layer A claims only the selection decision, never downstream runtime behavior of the selected element."
  ]
}
