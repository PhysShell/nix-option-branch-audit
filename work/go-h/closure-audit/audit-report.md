# GO-H closure audit report

Audited commit: 5d13bda (verdict there: PROTOCOL-REDESIGN-READY).
Reviewer report: reviewer-report.txt (verbatim, not edited).
Reviewer input: packet/ as built by build_packet.py at the audited commit.

## Verdict

**PROTOCOL-REDESIGN-PARTIAL.** Construct: the validity state (V) of a derived target whose only changed term is an option-semantic default change of W, with no changed predicate and C4 not satisfied (reviewer case K04a).

Second contradictory construct, same verdict: a default change on an ancestor option that reaches a freeform-only key, where the freeform declaration is not an independent option (reviewer cases K04b and K17b).

GO-I handoff: not written (READY not confirmed). GO-G1 and GO-J: not started. oba: not modified.

## Why PARTIAL

**1. K04a: V state is contradictory in target-validity-rubric.v2.json.**
- `states.V_INVALID` (line 11): V_INVALID when "the derived target has no DATA_DEPENDENT changed predicate on W and no option-semantic change under C4".
- `aggregation.per_target` (line 40): "V_INVALID iff a reason applies. V_AMBIGUOUS otherwise."
- K04a: W = services.speechd.modules is derived by its default change (derivation (a)). C4 is not satisfied (no DATA_DEPENDENT predicate reads W). No changed predicate exists, so no CONTROL_DEPENDENT_ONLY, FOREIGN_ONLY or UNRESOLVED path exists. No invalid_reason fits.
- Reading A ("no option-semantic change C4 makes eligible"): states says V_INVALID, aggregation says V_AMBIGUOUS. Contradiction.
- Reading B ("no option-semantic change C4 governs"): V_INVALID does not fire, V_VALID does not fire, and the aggregation default V_AMBIGUOUS requires an unresolved path that states.V_AMBIGUOUS also requires. No state fits.
- Either reading leaves K04a without a determinate V state. The E category (E_NO via C4) is determinate; the V state is not.

**2. K04b / K17b: freeform-only key under an ancestor default.**
- eligibility `h1d_freeform.F2`: "A watch path naming such a key is V_INVALID with reason FREEFORM_KEY_NOT_INDEPENDENT_DECLARATION."
- eligibility `h1d_validity_states` derivation (c) and `ambiguity_states`: an undecided ancestor reach is PARENT_DEFAULT_REACH, "which then gives V_AMBIGUOUS".
- target-validity v2 `precedence` 1 (mechanical failure first) would give INVALID, because v2 labels FREEFORM as mechanical. But the eligibility text names no tie-break between F2 and PARENT_DEFAULT_REACH. Both are explicit for the same case with different results.

## Reviewer conflicts: controller disposition

- **C1 (K03, K06a, K06b): not a protocol conflict.** The reviewer's C1 compares the case convention in 06-cases.md ("evaluate V for the target W names") with derivation_precondition ("a changed predicate INDEPENDENT of W does not derive W"). The rubric is explicit, and the case convention is controller text. Under the rubric, K03 is determinate: W is not derived; no V state; no category for W. K06a and K06b: W is not derived; the iterated option services.foo.items has E_AMBIGUOUS. The case convention text is defective and is recorded here, not changed.
- **C2 (K04b, K17b): the reviewer's stated basis is wrong; the conflict is real.** The reviewer said v2 does not list PARENT_DEFAULT_REACH. The committed v2 does list it (line 31). The packet does not, because the sanitizer deleted it (see packet defect 1). The substantive conflict is the F2 vs PARENT_DEFAULT_REACH conflict under Verdict section 2.
- **C3 (K04a E category): not blocking.** A4 lists non-consumer classes excluded "by scope.out". scope.out names U2 "separate mode, not Layer A". Wording gap only. K04a's E category is E_NO through C4 regardless of the absence result (the reviewer's "independent E_NO route").

## Reviewer results, summarized

Determinate by the reviewer (class and outcome): K01 V_INVALID NO_WATCHED; K02 AMBIGUOUS (E) with V_VALID conditional on S1; K03 not derived; K05 E_DIRECT, V conditional on S1 and G2; K06a/K06b E_AMBIGUOUS; K07a AMBIGUOUS; K07b E_DIRECT; K08 AMBIGUOUS; K09 E_NO; K10 and K11 V_AMBIGUOUS; K12 E_DIRECT; K13a E_INDIRECT; K13b E_NO; K14 REVIEWER_OWNED (EQ3, interim E_AMBIGUOUS); K15a AMBIGUOUS; K15b E_DIRECT; K16a T_NONE E_NO; K16b T_UNKNOWN E_AMBIGUOUS; K17a V_INVALID WATCH_PATH_NOT_DECLARED.

Not determinate by the reviewer: K04a (V state, Verdict section 1); K04b and K17b (Verdict section 2).

Reviewer's global gaps:
- **G1 (option_prefix not stated in any case):** case-authoring gap. V2 and V4 are not run in any case. The V states assume they pass. Caveat for any re-audit; not blocking on its own.
- **G2 (V3 "container" undefined for submodule-typed mk*Option segments; D1 and K05):** see C5a below.
- **G3 (EQ3 decision has no post-decision category; K14, D2):** see C7c below.

**C5a (D1, mpd settings.music_directory):** REVIEWER_OWNED. Basis: eligibility `h1d_verdict.conditions` assigns "C5a (V3 submodule container)" to a reviewer. The reviewer's own result is that the rules do not decide it (G2). Status: open, awaiting a recorded reviewer decision with evidence. C5a is not V_VALID yet.

**C7c (D2, boolToYesNo EQ3):** REVIEWER_OWNED, per the same `h1d_verdict.conditions` line. Interim E_AMBIGUOUS. The post-decision category is not stated (G3).

## Controller source verifications (exact SHAs)

- K16a: `exact_tree_search.sh e4c7d977 nixos/ \bzzgohprobe\b` returned ZERO_COMPLETE, 0 matches, 4637 files. Matches the reviewer.
- K16b: `\bport\b` over nixos/ returned MATCH, 5318 matches with no exclusion. The reviewer's 5309 count excluded movim.nix, which the case text does not name. The result (MATCH, not absence) is unchanged.
- K04a owner-key matches for `\bspeechd\b` outside speechd.nix at E: module-list.nix:440 (import), profiles/installation-device.nix:136 (comment), orca.nix:25 and graphical-desktop.nix:54 (config-value assignments). Matches the reviewer's four.
- speechd.nix E:36-38: `modules = mkOption { type = ...submodule { freeformType = attrsOf lines; }; default = { }; }`. Matches.
- lib/default.nix E:266-280: `inherit (self.lists) ... filter`. lib/lists.nix E:16-26: `inherit (builtins) ... filter`. Reviewer did not re-read these at E; confirmed here.
- mpd.nix E:178-180: `music_directory = lib.mkOption { type = ...; default = "${cfg.dataDir}/music"; ...}`. Default is at line 180, not 178 as the case text says (case wording only; the packet is not changed).
- beszel-agent.nix F:121 environment with lib.mkOption: not re-read in this controller pass; reviewer's G2 reference for K05.

## Packet defects (recorded; packet not changed, so the reviewer's inputs stay as the reviewer saw them)

1. **build_packet.py `sanitize_v2` removed `PARENT_DEFAULT_REACH` from `ambiguous_states`.** The committed v2 lists it. The intended change was only to remove the "(eligibility-rubric h1d_closure gap 9)" label. This is a substantive deletion. It is the basis of the reviewer's C2 wording error.
2. **The packet withheld `h1d_verdict.conditions`** (the REVIEWER_OWNED designation of C5a and C7c). This was intentional as verdict text, but it is a rule-bearing assignment. The reviewer's C5a and C7c results were therefore produced without it. Any re-audit should decide whether to include that sentence as a rule.
3. **06-cases.md case convention** ("evaluate V for the target W names") contradicts derivation_precondition for synthetic cases with no W path (K03, K06a, K06b).
4. **K16b** does not name a declaring file; the reviewer's exclusion of movim.nix has no basis in the case text.
5. **Sanitizer-only scrub** left the reviewer packet otherwise intact; scrub-scan.txt (controller artifact) is not committed.

## Controller notes

- The reviewer report is preserved verbatim (reviewer-report.txt, including the harness frame line). Its conclusions are not adopted where the controller verification above differs.
- The rubric was not altered to improve agreement.
- The verdict is decided by the two contradictory constructs above, not by the reviewer's C1-C3 labels.
- Any next round must fix packet defects 1-4 before a re-audit. A re-audit is a new round and needs its own authorization.
