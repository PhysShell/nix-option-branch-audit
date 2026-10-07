# GO-F evaluation status: blocked at Phase F2 by real-calendar-time requirement, not by design or integrity failure

**Parent**: `reports/go-f-prereg.md` (`22ff47d`).

## What completed

**Phase F0 (candidate freeze)**: complete. Candidate commit
`9ff7c048c500fc5939b5cf1da358383759e96676` (verified as `HEAD`, not
assumed; CI 6/6 green). Binary built fresh from this exact commit,
sha256 `406a27f54378c3a5b5b14c08fb9bec4d12bb23313fea73233d7154bb01834988`
at `work/go-f/oba-candidate-9ff7c04` (not committed to git -- a 2.5MB
binary blob, reproducible from the recorded commit+build command;
its sha256 is the frozen record). Stale-PATH binary explicitly
checked and distinguished (`~/.cargo/bin/oba`, sha256
`01732d91bf2e3a5d60a5caf3d9ee7fa36f7276a27f1ca9069135c9831957a232` --
different). Functional sanity against a GO-E kill-test fixture
confirms the asymmetric HEAD-only discovery feature is present.

**Phase F1 (preregistration)**: complete, committed (`22ff47d`), CI
green. Population, sampling rule (S6-R1's recalibrated
`CENSUS_CAP=750`/`NC1_THRESHOLD=20` mechanism plus an exclusion
filter), NC2/NC3/NC4 restated verbatim from `pilot-accounting.py`'s
real functions, `KNOWN-LIMITATION-1` (rename/move) with its
pre-registered classification token, the GO-D/GO-B-corrected
target-construction contract, and the blind-adjudication protocol
with its escalation triggers are all frozen as of that commit.
Mechanically-built exclusion set (`work/go-f/exclusion-set.json`,
1351 unique PR numbers with provenance) is committed alongside it.

## Phase F2 status: genuinely blocked, not worked around

The preregistration (Phase F1.1/F1.2) anchored the fresh population's
`LOWER_BOUND` at the preregistration commit's own instant
(`2026-10-07T11:22:03Z`) and reused S6-R1's own frozen
`WINDOW_INCREMENT_DAYS=5` (a number substantively motivated by raw
nixpkgs merge volume: ~180-230 PRs/day means 5 days is needed just to
exceed `CENSUS_CAP=750`'s own raw-population floor, not an arbitrary
choice). `census()`'s own maturity guard (inherited unchanged from
S6-R0C/S6-R1) requires `now_utc >= upper` before any query -- by
construction, before any candidate PR is ever looked at.

**This session cannot wait 5 real days.** At the time of writing this
document, only ~7 minutes have elapsed since the `LOWER_BOUND` anchor
(`date -u` = `2026-10-07T11:28:56Z`). No amount of continued execution
within this one continuous agent turn advances real wall-clock time by
5 days.

**What was NOT done, to stay honest rather than quietly route around
this**:
- Did **not** shrink `WINDOW_INCREMENT_DAYS` after seeing this
  problem -- that would be changing a frozen rule after freeze, the
  exact thing Phase F1 exit forbids.
- Did **not** reuse S6-R1's own "query `[release, now)` instead of
  waiting" substitution from this same lineage, because that
  specific substitution was made via an explicit, separate,
  real-time conversation with the project owner at the time (a
  documented mid-stream decision, not a standing rule this round can
  invoke unilaterally) -- applying it here would be this agent
  deciding, alone, to deviate from a rule it itself just froze an
  hour of wall-clock time ago. That decision belongs to the project
  owner, not to this round silently making it for them.
- Did **not** fabricate, simulate, or guess a census result.
- `work/go-f/run_census.py` (the real driver, reusing
  `fetch_merged_prs_in_window`/`systematic_sample`/the changed-file
  classifier exactly as frozen, with the exclusion filter spliced in
  before sampling) is committed as a specification, explicitly marked
  "NOT EXECUTED TO COMPLETION" in its own header -- the same
  discipline S6-R0/R1 used for their own population-and-sampling.py
  before real execution.

## Honest primary verdict for this attempt

None of `FRESH-EVAL-PASS-EXPANSION-GATES` / `FRESH-EVAL-KILL` /
`FRESH-EVAL-INCONCLUSIVE` actually fits what happened here --
`INCONCLUSIVE` is reserved (per the preregistration's own Phase
F1/F6 language, mirroring the user's own instruction) for a
genuine integrity failure (contaminated corpus, broken adjudication,
evidence loss) -- none of which occurred. This is instead an
**unexecuted precondition**: Phase F2 cannot start before
`2026-10-12T11:22:03Z` (`LOWER_BOUND + 5 days`), the same honest
"not yet matured" finding S6-R0C/S6-R0D/S6-R1 already established
as its own distinct category, never conflated with a real result.

No scoring run occurred. No target was constructed. No PR was
sampled. No adjudication occurred. **No verdict token from the
preregistered Final-verdict list is issued, because none of them
describe "the preregistered precondition has not yet elapsed."**

## What happens next (not decided here)

This requires the project owner's own explicit decision, options
being (not a recommendation, since this round is not authorized to
pick one for itself):
1. Wait the real 5 days (mirroring S6-R0/R1's own cron-reminder
   pattern) and resume Phase F2 once `2026-10-12T11:22:03Z` has
   actually elapsed.
2. Explicitly authorize, as a separate, documented, real-time
   decision -- the same kind S6-R1 itself made mid-stream for its own
   P0 window -- substituting an already-matured window (e.g.
   `[LOWER_BOUND, now)` at whatever later real instant this is
   revisited) for the as-yet-immature fixed-increment window, with
   the same "decided before seeing this specific window's own
   content" justification S6-R1's own substitution relied on.
3. Something else the project owner prefers.

## What did not happen (explicit)

No implementation change to `oba` after the F0 freeze. No change to
any frozen gate, threshold, or protocol rule after Phase F1 exit. No
corpus PR inspected. No P1/P2 work. No GO-E replay used as evidence.

**`S6_GOF_F2_BLOCKED_PENDING_WINDOW_MATURITY`**

**STOP.**
