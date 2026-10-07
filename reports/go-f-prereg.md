# GO-F preregistration: fresh evaluation of post-GO-E `oba`

Written and committed BEFORE any GO-F candidate PR is inspected. Own
lineage, own population, own sample -- not a continuation, replay, or
recomputation of P1/GO-B/GO-E/P2.

## Phase F0: candidate freeze

- Branch: `main`. Tree: clean at freeze time.
- **Candidate commit**: `9ff7c048c500fc5939b5cf1da358383759e96676`
  (verified as current `HEAD`, not assumed -- `git rev-parse HEAD`).
  `git merge-base --is-ancestor db4d44c HEAD` and `...9ff7c04 HEAD`
  both confirm ancestry.
- CI on `9ff7c04`: 6/6 green (`Dogfood`, `Dogfood diff`, `Dogfood
  audit-diff`, `Kani proofs`, `K1`, `Test suite (offline)`), verified
  via `gh run list`.
- **Binary built fresh from this exact commit**, not reused from any
  prior round's binary:
  - Build command: `cargo build --release --bin oba` via the rustup
    `nightly-2026-08-21-x86_64-unknown-linux-gnu` toolchain (`cargo
    1.100.0-nightly (514c56dd7 2026-08-19)`), `CARGO_TARGET_DIR=
    /home/tandem/cargo-target`.
  - Output: `/home/tandem/cargo-target/release/oba`, copied to the
    frozen record path `work/go-f/oba-candidate-9ff7c04`.
  - **sha256**: `406a27f54378c3a5b5b14c08fb9bec4d12bb23313fea73233d7154bb01834988`.
  - **Explicit stale-PATH check performed**: `which oba` resolves to
    `/home/tandem/.cargo/bin/oba`, sha256
    `01732d91bf2e3a5d60a5caf3d9ee7fa36f7276a27f1ca9069135c9831957a232`
    -- a **different, stale** binary (this is the one already found
    stale earlier in this lineage, during the defect-confirmation
    round). All GO-F invocations use the explicit path
    `work/go-f/oba-candidate-9ff7c04`, never bare `oba`.
  - Functional sanity: run against `fixtures/synthetic/
    diff-new-module-nested-options/{before-empty,after-with-module}`
    -- `{"mode":"diff","summary":{"added":3,...}}`, no `TOOL_ERROR`,
    confirming the GO-E feature is present in this exact binary.
- No implementation change after this point is permitted for the
  remainder of GO-F. If a candidate bug is found that would require a
  code change, this round stops with `STOP-GO-F-CANDIDATE-BROKEN`
  rather than fixing and continuing.

## Phase F1.1: population

**Repository**: `NixOS/nixpkgs`, same as every prior S6 round.

**Date/time window**: `[2026-10-07T11:22:03Z, upper)`, anchored at
this document's own commit instant (not reusing any prior round's
`LOWER_BOUND`) -- so this population has zero temporal overlap with
anything S6-R0/S6-R1's own census ever inspected (S6-R1's own census
ended at `2026-10-07T02:40:18Z`, ~8.7 hours before this anchor).
`upper` is determined the same way S6-R0/R1 determined it: extended in
fixed increments until the window matures AND clears the density gate
below, per F1.2's own sampling rule -- not fixed in advance as a
single number, to avoid re-deriving S6-R0's own low-density futility
problem by picking an arbitrarily small window.

**Merged/open status**: `is:pr is:merged base:master`, same as every
prior round -- open/draft PRs are not eligible (no final diff to
score against a stable head).

**Relevant file/path criteria**: identical to every prior S6 round's
own cheap relevance filter, reused verbatim, not reinvented: a PR is
`potentially_relevant` iff at least one changed file starts with
`nixos/modules/` or `nixos/tests/` (metadata-only, no diff-content
read at this stage).

**Option-changing criteria**: applied only at target-construction time
(F1.5), not at population/sampling time -- same separation of concerns
as every prior round (density-screening is metadata-only; "does the
diff touch a real option declaration" is a target-construction
question, never a population-filter question).

**Exclusions**: the full union of every real PR number this lineage
has already inspected at any level (census metadata, P1 sample pool,
GO-E replay), built mechanically from the actual committed artifacts,
not hand-typed:

| Source | Count contributed |
|---|---|
| `fixtures/s6-r0/excluded-pr-identities.json` (S5's own 369-PR ledger) | 369 |
| `fixtures/s6-r0/P0-window{1,2,3}-raw-log.json` (S6-R0's 3 real census checks) | up to 300 (with internal overlap -- windows 2/3 re-include window 1's own redo) |
| `fixtures/s6-r1/P0-window1-raw-log.json` (S6-R1's 750-record census) | 750 |
| `fixtures/s6-r1/P1-sample.md` (26-PR relevant pool: 15 selected + 11 unused) | 26 |
| GO-E replay (`#443747`, `#568048`) | 2 |

Built by `work/go-f/build_exclusion_set.py`, output
`work/go-f/exclusion-set.json`: **1351 unique excluded PR numbers**,
each with its own provenance (which source file(s) it came from).
Any eligible-population PR number appearing in this set is dropped
before sampling, unconditionally.

**Merge commits**: a PR's own `merge_commit_sha` is used as "head"
(same convention as every prior round); a PR that is itself a merge
of multiple feature commits is not specially excluded -- `oba` only
ever sees the resulting file content at base/head SHAs, not the commit
graph shape.

**Reverted PRs**: if a sampled PR was later reverted by a *separate*,
later PR, that is irrelevant here -- GO-F scores the sampled PR's own
merge against its own immediate parent, a historical fact unaffected
by anything that happened after it merged. Not specially excluded.

**Generated/vendor code**: no nixpkgs module/test file under
`nixos/modules/**`/`nixos/tests/**` is generated/vendored in the sense
that would need special handling (unlike, say, a vendored
`node_modules`); no special rule needed beyond the existing path
filter.

**PRs already seen in P1/GO-B/GO-E**: covered by the exclusion set
above.

## Phase F1.2: sampling

Reuses the exact frozen mechanism already in `fixtures/s6-r1/
population-and-sampling.py` (R1's own recalibrated `CENSUS_CAP=750`,
`NC1_THRESHOLD=20` -- restated, not re-derived, since GO-F's research
question is downstream of the same density-screening problem S6-R1
already solved) with exactly one addition: **the exclusion set above
is subtracted from the candidate population before `systematic_sample`
ever runs**, so no already-seen PR can be drawn into the census itself,
not merely filtered out after the fact.

- `fetch_merged_prs_in_window(lower, upper)` -- deterministic,
  whole-UTC-day-sharded, exactly as frozen.
- Candidates with `number` in `exclusion-set.json` are removed from
  the merged list before `systematic_sample(merged, CENSUS_CAP=750)`.
- Changed-file metadata classifies `potentially_relevant` vs
  `obviously_irrelevant`, exactly as frozen.
- **NC1 gate, restated from S6-R1 unchanged**: `NC1_THRESHOLD=20`
  relevant PRs required among the (now exclusion-filtered) 750-record
  census sample before freezing a relevant pool. If `<20`, the window
  extends by `WINDOW_INCREMENT_DAYS=5` (S6-R1's own frozen increment),
  exactly like every prior round -- including the possibility of
  another honest `STOP_LOW_YIELD` if density is low again. No frozen
  number is changed here.
- **P1-style sample**: `select_sample(relevant_pool, SEED, 15)`,
  `SEED = int("0a6f192", 16)` -- same seed convention, same release
  commit identity (the SUBJECT under test's own release artifact
  identity does not change just because the analyzer binary was
  patched by GO-E; `oba`'s own Cargo.toml version string is still
  `0.5.0` and no new tag has been cut).
- Full eligible-population manifest, the exclusion-filtered census
  log, the relevant pool, and the drawn sample are all committed as
  their own artifacts under `work/go-f/` once computed (Phase F2, not
  this document) -- this document fixes the RULE, not yet the result.
- **No replacement after seeing results**: if a sampled PR later turns
  out invalid under a rule that already existed in this document
  before sampling (e.g. it is discovered to have been merged to a
  branch other than `master` due to a metadata inconsistency), it is
  recorded in the trail and only a rule that was ALREADY written above
  may justify dropping/replacing it -- never a rule invented after
  seeing why it's inconvenient.

## Phase F1.3: sample size and gate definitions (restated verbatim from the frozen source, not reconstructed from memory)

`N = 15` (`P1_SAMPLE_SIZE`/`SAMPLED_PR_COUNT`, unchanged).

Restated directly from `fixtures/s6-r0/pilot-accounting.py` (the real,
tested implementation -- GO-F calls these functions, never
hand-copies the arithmetic):

```python
SAMPLED_PR_COUNT = 15
NC2_MIN_DERIVED_PRS = 9          # of SAMPLED_PR_COUNT
NC3_MIN_FRACTION = 0.7           # of derived_target_count
NC4_MIN_FRACTION = 0.7           # of substantive_target_count

def nc2_pr_level(derived_pr_count, sampled_pr_count=15, min_derived=9):
    return {"numerator": derived_pr_count, "denominator": sampled_pr_count,
            "passed": derived_pr_count >= min_derived}

def nc3_target_level(substantive_target_count, derived_target_count, min_fraction=0.7):
    if derived_target_count == 0:
        return {"numerator": 0, "denominator": 0, "threshold": 0, "passed": False}
    threshold = math.ceil(min_fraction * derived_target_count)
    return {"numerator": substantive_target_count, "denominator": derived_target_count,
            "threshold": threshold, "passed": substantive_target_count >= threshold}

def nc4_target_level(adjudicable_target_count, substantive_target_count, min_fraction=0.7):
    if substantive_target_count == 0:
        return {"numerator": 0, "denominator": 0, "threshold": 0, "passed": False}
    threshold = math.ceil(min_fraction * substantive_target_count)
    return {"numerator": adjudicable_target_count, "denominator": substantive_target_count,
            "threshold": threshold, "passed": adjudicable_target_count >= threshold}
```

**NC2**: numerator `derived_pr_count` (of the 15 sampled PRs, how many
reach target-construction step 6 for >=1 target); denominator 15;
PASS iff `>= 9`.

**NC3**: numerator `substantive_target_count` (derived targets reaching
a non-`INPUT_OR_HARNESS_FAILURE` result); denominator
`derived_target_count` (ALL derived targets, after the `MAX_TARGETS_
TOTAL=30` budget cap); PASS iff `>= ceil(0.7 * derived_target_count)`.

**NC4**: numerator `adjudicated_target_count` (substantive targets
reaching a non-`ORACLE_AMBIGUOUS` adjudication); denominator
`substantive_target_count`; PASS iff `>= ceil(0.7 * substantive_target_count)`.

**Effect of GO-E on these definitions, addressed explicitly per this
round's own instruction**: GO-E does **not** require any redefinition
or new gate name. It only changes which targets are CAPABLE of
reaching a substantive result (new-module-file targets, which used to
be forced into `INPUT_OR_HARNESS_FAILURE` by a `TOOL_ERROR`, can now
reach a real verdict) -- this widens NC3's own numerator's potential
population without touching NC3's own formula, denominator
definition, or threshold. No old semantics are broken; nothing is
silently redefined. **New-module-file PRs are therefore STANDARD,
first-class derivable cases in GO-F** (per F1.5 below), not a special
exception the way they were in P1.

## Phase F1.4: known limitations, registered before sampling

**`KNOWN-LIMITATION-1`**: rename/move is not representable as one
semantic fact by the current CLI/target model (`oba diff` can only see
"this path exists in root A" / "this path exists in root B"
independently per root; a file renamed between base and head is seen
as an independent deletion-shaped absence on one side and an
addition-shaped presence on the other, never linked). **Pre-registered
classification for any such case encountered in GO-F's own sample**:
`KNOWN_UNSUPPORTED_RENAME_MOVE` -- applied whenever a sampled PR's own
relevant file is detected, from the real diff's own `previous_filename`
field (`gh api .../pulls/<n>/files`, `status: "renamed"`), to be a
rename/move rather than a pure add/modify/delete. This target still
counts toward `derived_pr_count`/`derived_target_count` bookkeeping as
a mechanically-identified case, but its own substantive/adjudication
outcome is frozen as `KNOWN_UNSUPPORTED_RENAME_MOVE`, not
`INPUT_OR_HARNESS_FAILURE` and not a defect -- decided now, before any
such case has been seen.

## Phase F1.5: target construction protocol (frozen, corrected per GO-D/GO-B)

Reuses `fixtures/s6-r0/target-construction-protocol.md`'s steps 1-7
verbatim, with the GO-D/GO-B correction substituted for the original
(flawed) step 4 text:

- `option_prefix` must name a point where a real `options={...}` root
  actually exists -- the module's own true top-level root, or a point
  `find_nested_options_block` itself recurses into (a GAP-4-style
  interior `mkOption`-wrapped submodule container's own name is
  **never** a valid terminal `option_prefix`).
- `watch` is the FULL remaining relative path from that root to the
  leaf -- one dotted multi-segment string when more than one hop is
  involved, never a bare final segment that assumes the walker will
  resume from an interior container.
- Mechanically auditable chain, recorded per target: `PR -> changed
  source -> base/head files -> declaration/change -> canonical option
  path -> option_prefix -> watch -> predicate/test config`.
- **New-module-file PRs are now standard, first-class cases** (per
  F1.3's own point above) -- target construction proceeds for them
  exactly as for any other PR; GO-E's own feature is exercised, not
  worked around.
- Deletion-only PRs (an option declaration removed, nothing added):
  constructible as a target the same way as any other declaration
  change -- `oba diff`'s own `removed`/`added` classification (now
  present for asymmetric cases per GO-E) handles this without special
  casing.
- Rename/move: per `KNOWN-LIMITATION-1` above.
- `freeformType`, `submodule`, plain nested attrs, bare `mkOption`:
  all follow the single corrected rule above uniformly -- no
  per-shape special case, exactly the lesson GO-D's own audit
  established (the shape never mattered; only where `option_prefix`
  terminated did).

## Phase F1.6: adjudication protocol (frozen, reusing S6-R0's rubric)

Reuses `fixtures/s6-r0/adjudication-rubric.md`'s own oracle procedure,
blinded question, and 8-label error taxonomy verbatim, with the blind
packet for GO-F explicitly defined as containing ONLY:

- the real base/head module source for the target's own declaration
  and its directly-associated predicate/config-generation site;
- the real base/head test source for the frozen target's own named
  test file;
- the frozen target record itself (`option_prefix`, `watch`,
  `cfg_ident`, base/head SHAs);
- any statically-literal migration directive the reviewer's own
  reading of the source surfaces (not told in advance).

**Explicitly withheld from the blind packet**: `oba`'s own verdict;
`discovered_options`; any predicate-attempt result; any P1 verdict for
any PR (none should overlap, per the exclusion set, but stated as a
hard rule regardless); any GO-E replay result.

**Ordering discipline**: for every adjudicated target, the reviewer's
written answer is saved as its own immutable artifact (file) BEFORE
the real `oba` output for that same target is ever generated or
opened, with the file's own mtime (and, where feasible, a SHA-256
digest recorded in a log) serving as ordering evidence -- the same
check this session has independently verified for every prior blind
round in this lineage (`r2-blind-answers.md`'s own mtime vs. its
verdict JSONs).

## Phase F1.7: second-reviewer escalation (frozen trigger list, reused from S6-R0's rubric item 12)

A second, independent, ALSO-blind reviewer is required, and a defect
is never declared on a single reviewer's say-so, when any of:
- the primary reviewer's own post-reveal classification is
  `FALSE_FINDING`, `MISSED_FINDING`, or `MISLEADING_PRESENTATION`;
- `ORACLE_AMBIGUOUS` on the first pass;
- the case's own structural shape does not clearly match any pattern
  already documented in `fixtures/s5-remediation-closeout/closeout.md`
  or this lineage's own existing hostile-test corpus;
- the primary reviewer self-flags low confidence.

Per this lineage's own explicit, hard-won lesson (GO-A's
`CONFIRMED_NEW_DEFECT`, later retracted by GO-D): escalation to a
second blind reviewer is necessary but **not sufficient** -- before
any defect is finalized, target-construction VALIDITY itself (is
`option_prefix`/`watch` actually a contract-correct request, per F1.5
above) must be separately checked, exactly as GO-D had to do
retroactively for P1. GO-F builds this check into target construction
itself (F1.5), rather than leaving it to be discovered after the fact
a second time.

## Phase F1 exit

This document is the preregistration. Once committed and CI-green (if
applicable -- this document alone changes no code), the rules above
are frozen: no population, sampling, target-construction, or
adjudication rule may change for the remainder of GO-F.

**`S6_GOF_F1_PREREGISTERED`**
