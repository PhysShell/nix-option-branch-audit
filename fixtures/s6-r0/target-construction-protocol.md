# S6 target-construction protocol (frozen before any P1 PR is examined)

**Amended by S6-R0A** (`R0A-protocol-errata.md` items 4-5): the unit
model, the NC2/NC3/NC4 denominators, and the multi-target budget cap
below are the CORRECTED versions — the original S6-R0 text conflated
"PR" and "target" as one unit and let a PR's own non-derivation leak
into the target-level NC3/NC4 denominators. See the errata for the
full rationale; this file states only the corrected rule.

Purpose: make NC2 ("target derivability without large amounts of
manual invention") a checkable fact, not a post-hoc impression. For
each of the 15 P1-sampled PRs, apply these exact steps, in order, and
record which step the process stopped at. No step below permits a
judgment call beyond what it itself specifies.

**Ordering guarantee this protocol exists to enforce**: steps 1-6 (diff
inspection, target freezing) happen BEFORE step 7 (running `oba`).
Nothing in steps 1-6 ever runs the analyzer or looks at its output.
This is the mechanical enforcement of S6-R0's own item 9 ("never: run
OBA broadly, find an interesting output, construct a target around
it").

## Steps

1. **Fetch the real PR diff** (`gh pr view <n> --repo NixOS/nixpkgs
   --json files,baseRefOid,headRefOid` or equivalent), restricted to
   files under `nixos/modules/**` or `nixos/tests/**` (the same two
   path prefixes the cheap relevance filter already used to select this
   PR).

2. **Identify candidate module file(s)**: every changed file under
   `nixos/modules/**` that contains at least one `mkOption`/
   `mkEnableOption`/`mkPackageOption` call in either its base or head
   version (a syntactic grep, not a semantic judgment). If none, record
   `target_not_mechanically_derivable: no option-declaring file
   changed` and STOP for this PR. **(S6-R0A correction)**: this PR
   contributes ZERO derived targets and counts as a non-derivation
   toward NC2's own PR-level denominator ONLY — it contributes NOTHING
   to NC3/NC4 (both now target-level; a PR with no derived target has
   no target to be substantive or adjudicable about, so it is simply
   absent from those two counts, not present as a placeholder "no
   substantive target" entry the way the original S6-R0 text
   incorrectly implied).

3. **Identify the candidate watched option(s)**: among the option
   declarations in the changed module file(s), select the one(s)
   the diff itself directly touches (the declaration's own source
   lines appear in the diff — added, removed, or modified). If the
   diff touches more than one independent option declaration in the
   same file, construct ONE target per touched declaration, in the
   order those declarations appear in the file (top to bottom by
   source position) — do not pick "the most interesting one," and do
   not reorder by any judgment. If the diff touches zero option
   declarations directly (e.g., only touches `config =` logic, not
   `options =`), record `target_not_mechanically_derivable: no option
   declaration in the diff itself` and STOP (same NC2-only accounting
   as step 2, above).

   **Target-multiplicity budget cap (S6-R0A item 4)**: process the 15
   sampled PRs in their own frozen sample order (the order
   `select_p1_sample` returned them in); within each PR, process its
   own touched declarations in the source-position order just
   described. Accept targets into the pilot until the pilot-wide
   `MAX_TARGETS_TOTAL = 30` cap (`pilot-accounting.py`) is reached;
   every target beyond the cap is recorded as
   `target_construction_incomplete_due_to_budget_cap` — never silently
   dropped, never adjudicated, and excluded from every NC count. This
   cap is a pre-committed COST bound (roughly two independently-touched
   declarations per sampled PR, on average) decided now, in R0A,
   before any real PR is examined — never adjusted after seeing how
   many targets a real batch actually produces. See
   `pilot-accounting.apply_target_cap` for the exact, tested
   implementation.

4. **Derive `option_prefix`/`watch`**: `option_prefix` is the
   declaration's own path up to but not including its final segment;
   `watch` is that final segment (dotted, if the declaration is nested
   under further sub-attributes within the same `options = {...}`
   block, mirroring this project's own existing `watch` convention).
   `cfg_ident`: the local `let`-bound alias for `config.<option_prefix>`
   if one exists in the module (grep for `= config\.<option_prefix
   joined by "\.">` ), else the literal string `"cfg"` if the module
   itself never introduces its own alias (matching this project's own
   default-fallback convention used throughout S1-S5).

5. **Identify the wired test file**: the corresponding
   `nixos/tests/**` file changed by the SAME PR, if any; else, grep
   `nixos/tests/all-tests.nix` at the PR's own head SHA for a reference
   to a test file whose own name matches the module's own service name
   (same heuristic S5's own protocol used, e.g. `gonic = runTest
   ./gonic.nix;`). If neither exists, record
   `target_not_mechanically_derivable: no wired test file found` and
   STOP.

6. **Freeze the target**: write `{name, module, test, cfg_ident,
   option_prefix, watch}` to this PR's own frozen target record,
   together with the exact base/head SHAs the PR diff was read from.
   This record is written BEFORE step 7 and is never edited after
   step 7 (S6-R0 item 9's own ordering guarantee).

7. **Only now**, run `oba check`/`oba audit-diff` against the real,
   live-fetched base/head source at the frozen SHAs, using the frozen
   target record from step 6, exactly as every prior S-round's own
   replay tooling already does.

## Unit model and NC2/NC3/NC4, restated precisely (S6-R0A correction)

**Sampling unit**: PR (exactly 15, fixed). **Analysis/adjudication
unit**: target (zero, one, or several per PR, per step 3's own
multi-target rule, subject to the pilot-wide budget cap).

Five counters, kept explicitly distinct (never conflated):

- `sampled_pr_count` = 15 (fixed).
- `derived_pr_count` = number of the 15 PRs that reach step 6 for at
  least one target (regardless of how many targets that PR itself
  produced).
- `derived_target_count` = total number of targets accepted into the
  pilot across all 15 PRs, after the budget cap (`pilot-accounting.
  apply_target_cap`'s own `accepted` list length).
- `substantive_target_count` = of `derived_target_count`, how many
  reach a substantive `oba` result (not `INPUT_OR_HARNESS_FAILURE`).
- `adjudicated_target_count` (a.k.a. "adjudicable" in NC4) = of
  `substantive_target_count`, how many receive a trustworthy,
  non-`ORACLE_AMBIGUOUS` adjudication.

**NC2** (`pilot_accounting.nc2_pr_level`): numerator `derived_pr_count`,
denominator `sampled_pr_count` (= 15, always). A PR that stops at step
2, 3, or 5 is a mechanical non-derivation — it affects NC2's own
denominator (it was sampled) but contributes NOTHING to NC3/NC4 (it
produced no target to be substantive or adjudicable about). KILL:
`derived_pr_count < 9`.

**NC3** (`pilot_accounting.nc3_target_level`): numerator
`substantive_target_count`, denominator `derived_target_count`
(TARGETS, never PRs, and never inflated by a PR's own non-derivation).
KILL: `substantive_target_count < ceil(0.7 * derived_target_count)`.

**NC4** (`pilot_accounting.nc4_target_level`): numerator
`adjudicated_target_count`, denominator `substantive_target_count`.
KILL: `adjudicated_target_count < ceil(0.7 * substantive_target_count)`.

A target cut by the budget cap (`target_construction_incomplete_due_to_
budget_cap`) is excluded from ALL FIVE counters above — it was never
attempted, so it can be neither a derivation, a substantive result, nor
an adjudication.

## What this protocol deliberately does NOT attempt

- Judging whether the changed option is "interesting" or "likely to
  reveal a defect" — every option touched by the diff becomes a
  target, mechanically, per step 3.
- Resolving `imports=`, wildcards, or any other already-disclosed
  residual limitation specially — this protocol is blind to which
  known-limitation class (if any) a given PR might touch; that
  determination happens only during adjudication (see
  `adjudication-rubric.md`), never during target construction.
- Guaranteeing every PR yields a target — a PR that stops at step 2/3/5
  is real, honest information about NC2's own success rate, not a
  protocol failure to paper over.
