# GO-H: evaluation protocol redesign (no new scoring)

**Verdict: PARTIAL.** The R/E/D/V model, the eligibility rubric, the target-validity rubric, the no-test behaviour, and the side rule are specified and tested on synthetic fixtures and on the frozen P1 cohort. Three things are still open: the product premises U1-U3 need user confirmation, the validator is a text heuristic that a parser-based check should replace, and the blind rubric agreement (section 9) is recorded below as it came back.

Scope kept to the directive: no oba change, no `mkPackageOption` fix, no new scoring sample, no GO-F replay, no re-verdicts, no P2 (configuration values), no thresholds fitted to historical results.

## 0. Provenance (H0)

- Base commit: `c9879b5` (branch `claude/go-g-nc2-yield-decomposition`, pushed, CI success).
- Frozen oba: `work/go-f/oba-candidate-9ff7c04`, sha256 `406a27f54378c3a5b5b14c08fb9bec4d12bb23313fea73233d7154bb01834988`, v0.5.0. Untracked, not committed, not rebuilt.
- Documentation inconsistency, recorded and not edited: `reports/go-f-f2-1-target-construction.md` (frozen) still lists the pre-correction `option_prefix` for `519655` settings and `557977` settings (`["services","bazarr","settings"]`, `["services","elk","settings"]`). The on-disk manifests under `work/go-f/trees/` and `reports/go-f-evaluation-final.md` use the corrected prefixes (`["services","bazarr"]`, `["services","elk"]`). Historical artifacts were not changed.

## 1. Model: R, E, D, V, A, W

| Layer | Meaning | Decided by | Measured with |
|---|---|---|---|
| R | PR touches `nixos/modules/**` or `nixos/tests/**` | file-path rule | census |
| E | PR changes option semantics (default, type, apply/readOnly, rename, branch predicate, new or removed declaration) | `work/go-h/eligibility-rubric.json` | source reading, blind-checked (section 9) |
| D | at least one mechanically derivable target by frozen steps 1-7 | `fixtures/s6-r0/target-construction-protocol.md` | deterministic construction |
| V | derived target is contract-valid (module exists on the relevant side, `option_prefix` is a walkable root, `watch` paths are declared) | `work/go-h/target-validity-rubric.json`, `work/go-h/validate_targets.py` | text checks before any oba call, plus reviewer S1 |
| A | oba's analyzer result on V targets (PASS / OBA001 / OptionNotFound / ...) | oba | only after V |
| W | the wired test witnesses the option (a test assignment on the opposite branch) | oba `check` / `diff` witness semantics | separate axis, not a validity criterion |

Order is strict: R, then E, then D, then V, then A. A layer is never computed from a later layer's output. Failures are attributed to one owner: CENSUS, ELIGIBILITY RUBRIC, TARGET CONSTRUCTOR, TARGET VALIDATOR, OBA DISCOVERY, OBA ANALYSIS, WITNESS INFRASTRUCTURE, KNOWN PRODUCT LIMITATION.

## 2. Eligibility rule (E)

The rubric is `work/go-h/eligibility-rubric.json`, status CANDIDATE. Decisive points:

- Product premises: P1 the unit is an option's semantics (default, type, apply/readOnly, predicates reading the option). Documentation attributes (description, example, defaultText) are outside the unit. P2 configuration values (`config.x = ...`) are a different unit, out of scope here. P3 eligibility is not reduced by oba's coverage; analyzer limits are counted after V.
- A PR is E iff at least one hunk is ELIGIBLE after CONDITIONAL resolution. CONDITIONAL hunks need a written reason; unresolved ones count as NOT_RESOLVED, never ELIGIBLE.
- Classes EC01-EC19 cover new/removed/changed declarations, rename/move, helper-generated declarations (`mkPackageOption`, `mkEnableOption`) as ELIGIBLE with a note that D may fail, and test-only or documentation-only changes as INELIGIBLE.
- Open product decisions: U1 documentation-only in scope (working position: no); U2 configuration-value mode (working position: separate mode); U3 value-only behavioural options in E although oba cannot analyze them (working position: yes, under P3); U4 predicate-only changes that read an existing option (EC12) in E (working position: yes, under P1; this is the 3-PR disagreement in section 9). **All four need user confirmation before preregistration.**

## 3. Declaration and configuration semantics

Option semantics (declaration, default, type, predicates) are the product unit. Configuration values are not: a PR that only changes `config.services.x.y = ...` is INELIGIBLE under EC11 and needs a separate mode with its own R/E/D/V. This is P2 and was not built.

## 4. No-test behaviour (H5 evidence)

Synthetic fixture `work/go-h/fixtures/h5/` (demo module with `openFirewall` default changed false to true; tests `demo-set`, `demo-noset`, `demo-empty`; head-only test variant in `h5-headonly/`). Raw outputs in `work/go-h/raw/h5/`.

| Probe | Result |
|---|---|
| `check`, all targets' tests missing | exit 3 (TOOL_ERROR), stdout empty, error on stderr |
| `check`, test present, option not witnessed | exit 2, inconclusive, targets reported as inconclusive |
| `diff`, test missing in both roots | exit 3 (TOOL_ERROR); `diff` does not partition unavailable targets |
| `diff`, test sets `openFirewall = true` (witness) | `pass->oba001` transition, exit 2 |
| `diff`, test does not set the option (no witness) | `unchanged` 3, exit 2 (see below) |
| one-sided test (present only in head) | `diff` tolerates it |
| `h5-mixed` (some targets with missing tests) | `check` lists an unavailable entry, exit 2 (from the H5 run; raw not re-checked in this pass) |

Consequences for the protocol:

- A test that does not touch the option yields OBA001 (witness false), not PASS. PASS requires a witness on the opposite branch.
- `diff` compares only the verdict kind (`src/main.rs` ~4580-4600, `if b.kind() != h.kind()`). A real default change that the test does not witness therefore appears as `unchanged`. A change that leaves the verdict kind the same is invisible to `diff`. This is an analyzer limitation that must appear in the metric family as its own failure owner, not as "no behaviour change".
- A missing test is a tool error for `diff`. The protocol should treat a missing test before derivation as a derivation or contract failure (TARGET CONSTRUCTOR), not as an analyzer result.

## 5. Derivation semantics (D)

D uses the frozen steps 1-7 (`fixtures/s6-r0/target-construction-protocol.md`), unchanged. Two points from the P1 cohort:

- Step 3 creates one target per touched declaration. In a brand-new module every declaration counts as touched, so #443747 (10 targets) and #568048 (11 targets) produce 21 targets from two PRs. This is a multiplicity effect of the rule, not a curation choice.
- The side rule: a target is validated against the side on which the option exists. Added options are validated against head, removed options against base, and changed-default options against both. Validating a removed option against head gave a false INVALID in the validator run (`listenPort_removed`: INVALID on head, VALID on base).

## 6. Validation procedure (V)

`work/go-h/validate_targets.py` (text-only, deterministic, no oba call). Checks:

- V1: the module file exists on the named side.
- V2: `option_prefix`'s last segment is not declared with `mk*Option` (interior option container, not a walkable root). This is the GO-F defect.
- V3: each watch segment is declared as a container key or dotted key (intermediate), and the leaf as an option or a key.
- V4: the last `option_prefix` segment is found as a container key. Otherwise AMBIGUOUS, which does not count as V.
- S1 (reviewer, not mechanical): the watch paths are the options the PR changes, not unrelated siblings. Decided from the eligibility record's decisive hunk.

Aggregation (decided before any run): per target, VALID needs V1-V3 passing, V4 not AMBIGUOUS, and S1 passing. Per PR, V holds iff at least one target is VALID; PARTIAL_V counts only VALID targets and reports the rest.

Reuse investigation (CLAUDE.md): the closest existing solution is a Nix parser that can produce the attribute tree of each module (for example `nix-instantiate --parse` or a tree-sitter Nix grammar). It was not used. The text validator is a stopgap: its regexes are unreliable for multi-line Nix strings and dotted keys. The concrete project-specific gap is the mapping from parsed attributes to `option_prefix`/`watch`. A parser-based replacement is proposed as GO-I (section 11).

Validator evidence:

- Corrected GO-F head targets (`work/go-f/trees/*/targets.toml`): all VALID, except `listenPort_removed` (VALID on base, as expected under the side rule).
- As-recorded F2.1 manifests (`work/go-h/manifests/as-recorded/`): 5/5 pre-correction prefix defects flagged INVALID by V2.
- Known limitation: `557977` `package` is declared with `mkPackageOption`; the validator reports VALID, oba reports OptionNotFound. This separates validity (V) from analyzer discovery (A). Not fixed here (GO-G1).

## 7. Metric family

Denominators are fixed per layer:

- M1 = E / R
- M2 = D / E
- M3 = V / D
- M4 = A-substantive / V
- M5 = correct / adjudicated substantive
- M6 = witnessed / witness-eligible (W)

Each layer is reported with its own failure-owner ledger. No layer's rate is computed on another layer's exclusions. Thresholds are not set from historical numbers.

## 8. Historical dry run (H12, diagnostic only)

Applied to the frozen P1 cohort (5 derived PRs: #508090, #568429, #563823, #443747, #568048), using only existing target manifests. Module and test sources fetched read-only at base and head SHAs. Script: `work/go-h/h12_dry_run.py`; raw: `work/go-h/h12/h12-results.json`.

**As-frozen manifest (28 targets, `targets/s6-r1-p1.toml`), validated on head:**
- 24 VALID: 21 new-module targets (#443747 x10, #568048 x11), plus `568429 settings`, `568429 openFirewall`, `563823 aclPolicies`.
- 4 INVALID (V2, prefix ends in an interior option container): `508090 SKIP_GPU` and `GPU_COLLECTOR` (prefix ends in `environment`), `568429 settings.server.port` and `settings.oauth.auth-dir` (prefix ends in `settings`).
- On base, the 21 new-module targets are INVALID only because the module does not exist there (V1), as the side rule predicts.

**Corrected manifest (7 targets, `targets/s6-r1-p1-corrected.toml`):**
- Head: 7/7 VALID.
- Base: 3 VALID, 4 INVALID (V3). These four watch options that exist only in head. The side rule says validate on head, so this is not a validity failure.

What this shows:

1. V2 catches the GO-F/P1 prefix defect class, and the corrected manifest passes it.
2. The 21 new-module targets pass V but are not analyzable. P1 recorded them as `NEW_MODULE_NO_BASE` and `NOT_EVALUABLE_BY_CURRENT_PROTOCOL`. Under this protocol they are V-valid and their non-evaluation is an analyzer-scope outcome (KNOWN PRODUCT LIMITATION), not a target error.
3. V does not capture multiplicity or semantic noise. The 21 targets pass V1-V3; their usefulness is a question for S1 and for the step-3 rule. This is a known gap.

No historical verdict was used as a validity criterion or threshold.

## 9. Independent rubric agreement (H13)

Blind reading: a fresh general-purpose agent applied `eligibility-rubric.json` to the 15 PRs of the GO-F sample (`fixtures/s6-r1/P1-sample.md`: 565943, 566007, 569867, 508090, 568429, 563823, 566696, 443747, 556752, 564688, 568782, 567915, 568245, 561242, 568048). It was forbidden to read `work/go-g/`, `reports/go-g*`, `reports/go-f-*`, `fixtures/`, `work/go-f/`, `work/go-h/` (except the rubric), or git history. It used read-only `gh api` only.

The blind agent's report was fixed before the comparison. The coordinator then read `work/go-g/pr-classification.json` and compared it to the blind E values. Isolation was by instruction only; the agent's tool-call log was not audited line by line, so the "blind" claim rests on the instructions given and the agent's report.

### 9.1 Result

Blind E on 15 PRs: 9 ELIGIBLE, 6 INELIGIBLE. GO-G reference, used here only as a proxy: "declaration touched in step 3" (`step3_declaration_touched`), which is a declaration-only criterion and not a ground truth for E.

| PR | Blind E | Decisive class | GO-G proxy (declaration touched) | Agree |
|---|---|---|---|---|
| 565943 | INELIGIBLE | EC15 | no | yes |
| 566007 | ELIGIBLE | EC12 (zfs.enabled predicate on new optional package; sandbox booleans, judgment) | no | **no** |
| 569867 | ELIGIBLE | EC13 (ELF branch removed from system.checks; judgment) | no | **no** |
| 508090 | ELIGIBLE | EC01 (environment.SKIP_GPU, GPU_COLLECTOR) | yes | yes |
| 568429 | ELIGIBLE | EC04 (settings type to submodule) | yes | yes |
| 563823 | ELIGIBLE | EC03 (aclPolicies default) | yes | yes |
| 566696 | ELIGIBLE | EC13 (indirect, via corePkgs feeding a warning; judgment) | no | **no** |
| 443747 | ELIGIBLE | EC01 (new module declarations) | yes | yes |
| 556752 | INELIGIBLE | EC11 (mkDefault config values, P2) | no | yes |
| 564688 | INELIGIBLE | EC09 (equivalent helper refactor) | no | yes |
| 568782 | ELIGIBLE | EC17 (dnscache package via mkPackageOption) | yes | yes |
| 567915 | INELIGIBLE | EC14 (test only) | no | yes |
| 568245 | INELIGIBLE | EC14 (test only) | no | yes |
| 561242 | INELIGIBLE | EC14 (test only) | no | yes |
| 568048 | ELIGIBLE | EC01 (new module declarations) | yes | yes |

Agreement with the proxy: 12/15. The three disagreements (566007, 569867, 566696) are all predicate-only or indirect changes: no declaration is touched, but the rubric counts a branch predicate that reads an option (EC12) or an assertion-like check (EC13). That is a rule-scope difference between the declaration-only proxy and P1, not a reading error on either side. The product decision it forces is U4 below.

Blind-reviewer gaps (reported by the agent, rubric v1 unchanged):

1. No class for pure equivalence refactors of let-helpers (564688). The agent used EC09/EC10 with an equivalence argument.
2. Config-level `mkDefault` values (556752) were classed EC11 under P2, not EC03.
3. The rubric does not say how to class whole new module files or `module-list.nix` registration. The agent used EC01 for declarations and registration, EC17 for `mkPackageOption`, and EC11 for config blocks of new modules.
4. Two ELIGIBLE verdicts (569867, 566696) rest on EC13 judgment calls. The agent flagged 566007's sandbox-boolean EC12 as a judgment call too.

Consequence for the protocol: E as a single boolean is not stable enough. Judgment-dependent ELIGIBLE verdicts (EC13, EC10, EC12 on booleans) need a separate tag, and the census should report E with and without them.

## 10. Ambiguities and gaps

- U1-U3 (section 2) need user confirmation before any preregistration.
- Text validator: regex-based, not a parser; multi-line strings and dotted keys are unreliable; S1 is judgment, not mechanical.
- Multiplicity: a brand-new module yields one target per declaration (21 targets from two PRs). Whether that is in scope is an open rule question.
- `diff` compares verdict kinds only, so unwitnessed default changes can look unchanged (section 4).
- `mkPackageOption` declarations: oba reports OptionNotFound on `557977 package` and on the synthetic fixture; the validator reports VALID. The discovery owner is oba. Not fixed (GO-G1).
- Attribute attribution by text is unreliable inside `''...''` strings.
- Documentation inconsistency in the frozen F2.1 table (section 0).

## 11. Proposed next GOs (not started)

- **GO-G1**: fix oba discovery for `mkPackageOption` declarations (engineering change, separate from any evaluation). Gate: positive and negative fixtures, plus the synthetic H5 package case.
- **GO-I**: replace the text V-check with a parser-based one (Nix parser for attribute trees; reuse investigation recorded per CLAUDE.md). Gate: validator reproduces section 8 counts without regexes.
- **GO-J**: after U1-U3 are confirmed, preregister the new protocol on a new population and a fresh sample. Requires explicit user GO. No GO-F replay.
- A separate configuration-value mode (P2) only if the user wants that product unit; not part of GO-H.

## 12. Commit and CI

Branch `claude/go-h-protocol-redesign` from `c9879b5`. Named files only; the frozen binary `work/go-f/oba-candidate-9ff7c04` and the fetched upstream sources under `work/go-h/h12/src/` are excluded. The commit SHA and CI result are reported in the GO-H final message, not in this file, because a file cannot contain its own commit SHA.
