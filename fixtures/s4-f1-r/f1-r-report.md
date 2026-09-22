# S4-F1-R: regression report for the S4-F1 fix

**This is a regression-only exercise over already-seen S4 data. It is
NOT a fresh evaluation, NOT a replay of S4's deployment gate, and makes
no new generalization claim. S4's own gate result (`FAIL`,
`fixtures/s4-live-pr-shadow/s4-final-report.md`, commit `cebb3a6`)
remains immutable and is not superseded by anything in this report.**

Every count/table below is generated from `fixtures/s4-f1-r/ledger.jsonl`
and `fixtures/s4-f1-r/negative-controls.json`, mechanically, by the
scripts committed alongside it (`derive-corpus.py`,
`build-manifests.py`, `fetch-content.py`, `run-comparison.py`,
`run-negative-controls.py`) — never hand-typed.

## Frozen identities

**Baseline**: published `oba` v0.4.4, commit
`81e1131c55be103431075ec3e82e58eb45fd92d5`. The same independently
verified published binary S4 itself used:
`/tmp/verify-v044-scratch/oba-x86_64-unknown-linux-musl/oba`, sha256
`388ea5ab8147c1bc5d71948ff63afe0c8ee3fec3ac4abb9fbc53ca14293d9225`,
re-verified present and re-checksummed (matching S4's own recorded
value exactly) at the start of this exercise.

**Candidate**: exact source commit
`940e388d625bd9ed09138f816bfacdccdd187bb1` (the S4-F1 fix), built once
from a genuinely clean checkout (`git clone` to a fresh directory, `git
checkout 940e388...`, `cargo build --release`, `CARGO_TARGET_DIR`
pointed outside the working tree) — not the working tree's own
incrementally-built `target/`. Resulting binary sha256:
`2348d513da4fa153c52299ae6752e54103446bbe8003658b1c8926d78cf0b580`.

`src/` was not modified at any point during F1-R: `git log -1 --
src/main.rs` still resolves to `940e388` at report time, and `git
status`/`git diff --stat src/` are both empty.

## Corpus: mechanically derived from the S4 ledger

`derive-corpus.py` reads `fixtures/s4-live-pr-shadow/adjudication-ledger.jsonl`
(untouched by this exercise) and filters `record_type == "pr_summary"
and applicable == true`:

```
total pr_summary records: 187
applicable (S4-F1-R corpus): 94
  S4-A: 69
  S4-B: 25
```

94 matches the expected S4 applicable population exactly — derived
mechanically, not copied by hand, per the F1-R authorization's own
requirement.

## Target manifests: exact reconstruction

`build-manifests.py` produced the exact manifest used for each of the
94 corpus PRs:

- **57 PRs** (S4-A batches 6-10, every S4-B batch): the real
  `targets.toml` S4 itself committed, copied byte-for-byte
  (`manifest_provenance: copied_verbatim_from_s4_artifact`). This
  includes `#543492` — its F1-R manifest is confirmed byte-identical
  (`diff` clean) to `fixtures/s4-live-pr-shadow/adjudication/batch10/543492/targets.toml`,
  the ORIGINAL combined manifest that caused the S4 failure. No
  hand-constructed supplemental single-target manifest is used
  anywhere in this exercise's primary comparison.
- **37 PRs** (S4-A batches 1-5, positions 1-60, which only committed
  `raw.json`/`summary.md`, not `targets.toml`): mechanically
  reconstructed from that PR's own `raw.json`'s `oba[].identity`
  entries (`module`, `test`, `cfg_ident`, `option_prefix`, grouped;
  every `watched_path` collected into `watch`) — the exact real
  target/watch set that PRODUCED that raw.json, not a fresh
  re-derivation. Sanity-checked before trusting it for these 37: the
  same technique, applied to a PR that DOES have a committed
  `targets.toml`, reproduces an equivalent target/watch set (verified
  directly, `#531455` netbox — same module/test/cfg_ident/option_prefix,
  same 5-option watch set, differing only in cosmetic array order and
  the synthesized `name` field, which is never semantically compared).

## Content: real, frozen-SHA nixpkgs source

`fetch-content.py` fetched every `module`/`test` path each PR's
manifest references, at the PR's own real, frozen `base_sha`/`head_sha`
(from the S4 ledger — never a moving branch or current PR head), via
`gh api repos/NixOS/nixpkgs/contents/<path>?ref=<sha>`. All 94 PRs
fetched cleanly. Exactly **one** genuine absence was recorded across
the entire corpus:
`nixos/modules/services/x11/display-managers/lightdm-greeters/enso-os.nix`
at `#543492`'s head SHA (`aa970620a51824a5361565d16a18882fcf1a3759`) —
confirmed, before running the fetch, by an independent mechanical check
of every corpus PR's own `raw.json` summary
(`added`/`removed` subject counts): `#543492` is the only PR with
`removed > 0`; all other 93 show `added=0, removed=0`, meaning no other
PR's manifest references a module/test file that is genuinely absent on
either side. This was verified empirically, not assumed.

## Comparison contract

Defined and committed (`run-comparison.py`'s own
`normalize_check_json`) BEFORE examining any corpus differences: for
`check` JSON, `summary.unavailable`/`unavailable_targets` are stripped
ONLY when they carry the default "nothing unavailable" value (`0`/`[]`)
before comparing baseline against candidate. No other field is ever
normalized away. `audit-diff` output is compared with **no**
normalization at all — nothing additive was ever added to its schema by
S4-F1.

## Re-run matrix and results

For every one of the 94 applicable PRs: `check --root base-root`,
`check --root head-root`, and `audit-diff --base-root --head-root`
(the negative control for the subcommand S4-F1 never touched), each
run under BOTH the baseline and candidate binary, against the same
real content and the same manifest.

```
total ledger rows: 282  (188 check + 94 audit-diff)
  identical_after_additive_normalization: 281
  expected_missing_target_recovery:         1
  unexpected_difference:                    0
```

Exit-code cross-tab, `check` rows only (188 total):

| baseline exit | candidate exit | count |
|---:|---:|---:|
| 0 | 0 | 78 |
| 1 | 1 | 7 |
| 2 | 2 | 102 |
| 3 | 2 | 1 |

Every `audit-diff` row (94/94) shows byte-identical baseline/candidate
stdout, stderr, and exit code — the negative control holds with zero
exceptions: **F1 caused zero semantic delta in `audit-diff`.**

### `#543492`: the real reproducer, using its real original manifest

```
check  base:  baseline exit=2  candidate exit=2   identical_after_additive_normalization
check  head:  baseline exit=3  candidate exit=2   expected_missing_target_recovery
audit-diff:   baseline exit=2  candidate exit=2   identical_after_additive_normalization
```

- **Base side**: `enso-os.nix` exists on both sides at base — both
  binaries analyze it normally, byte-identical after normalization.
- **Head side, the real historical failure**: baseline exit 3, empty
  stdout (the original incident, reproduced byte-for-byte against the
  real historical content and the real original combined manifest —
  independently confirmed via `git stash` against `src/main.rs` before
  this exercise, in the S4-F1 commit itself). Candidate exit 2: the
  unaffected `lightdm` target's own real `PASS` verdict is present in
  `targets`, `lightdm-enso-greeter` appears under `unavailable_targets`
  with `summary.unavailable == 1`. No hand-constructed supplemental
  manifest was needed or used to recover the valid sibling result —
  the ORIGINAL combined manifest alone produces the fixed behavior.
- **`audit-diff`**: byte-identical baseline vs candidate, confirming
  the subcommand that already handled this PR's real module-removal
  gracefully (S4's own original finding) is completely unaffected by
  the `check` fix.

This is the sole occurrence of `expected_missing_target_recovery` in
the entire 282-row ledger — exactly matching the pre-registered
expectation (verified, not assumed, via the empirical
`added`/`removed` check above) that no OTHER corpus PR exhibits this
shape. No "additional expected benefit" cases exist to report
separately.

## Fatal negative controls

`run-negative-controls.py`, both binaries, same real fixture content
at HEAD (`940e388`) for every case (the S4-F1 hostile fixtures did not
exist at v0.4.4's own commit; the baseline BINARY is run against them
anyway, from the current working tree, to prove each condition was
ALREADY fatal before S4-F1, not something the fix happened to start
protecting):

| case | baseline exit | candidate exit | fatal-status match | stderr text |
|---|---:|---:|---|---|
| absolute module path | 3 | 3 | yes | identical |
| `../` escape | 3 | 3 | yes | identical |
| symlink escape | 3 | 3 | yes | identical |
| missing `--root` | 3 | 3 | yes | identical |
| absolute path alongside a deleted target | 3 | 3 | yes | **differs (disclosed below)** |
| malformed manifest (bad TOML) | 3 | 3 | yes | identical |
| every OBA target unavailable (multi-target) | 3 | 3 | yes | **differs (disclosed below)** |

7/7 pass the literal acceptance criterion (#8: "previously-fatal
non-absence conditions remain fatal" — exit code and empty stdout).
Zero fatal-status regressions.

**Disclosed, non-blocking observation** (raised mid-exercise, resolved
by explicit user direction to record and continue rather than treat as
blocking): two adversarial multi-target manifests show a real
difference in stderr TEXT, though not in exit code or fatal status.

- `targets/s4f1-negative-control-escape.toml` lists a soft-missing
  target (`deleted-target`) BEFORE a hard-error target
  (`absolute-escape`). Baseline (which has no soft/hard distinction —
  ANY missing file aborts immediately) reports `deleted-target`'s
  error, since it's scanned first, and never even reaches
  `absolute-escape`. Candidate (which no longer aborts on a
  soft-missing target) scans past `deleted-target`, reaches
  `absolute-escape`, and reports THAT error instead — arguably more
  accurate, since it names the actually security-relevant target
  rather than an incidental unrelated one.
- `targets/s4f1-all-deleted.toml` (every target unavailable): baseline
  names only the first missing target it happens to scan; candidate's
  own "no target in this manifest is analyzable" message names every
  unavailable target, not just one.

Analysis: this is a genuine, understood, DIRECT consequence of the
fix's own core design (a soft-missing target no longer short-circuits
the manifest scan) — not a detection-coverage regression. In every
case checked, a real hard error anywhere in a manifest still
propagates via `?` and still aborts the whole invocation (candidate's
own `partition_targets_by_availability` scans every target's
module+test unconditionally before returning); no scenario was found,
or is possible by construction, where a real hard error goes
unreported. Only WHICH specific problem's text is surfaced first can
differ when a manifest contains more than one distinct kind of
problem — a shape none of the 94 real S4 corpus PRs actually exhibit
(confirmed: zero `unexpected_difference` rows in the corpus ledger).
Recorded here in full per the explicit instruction not to silently
normalize away a real difference; not reclassified as part of the
pre-registered "expected changed" class, since it is a genuinely
distinct shape (stderr attribution under a multi-problem manifest, not
missing-target recovery). Does not affect any acceptance criterion,
all of which are about exit code / fatal status / JSON semantics /
`audit-diff` parity, none about stderr text content in an adversarial
multi-problem case outside the real corpus.

## Test suite and CI

Full `cargo test` at `940e388` (325 tests, 0 failed, run before F1-R
began, as part of the S4-F1 commit itself) and real CI (all 6
workflows: Dogfood, Dogfood audit-diff, Dogfood diff, Test suite
(offline), Kani proofs, K1 (CDC real-eval proof)) confirmed green on
that exact commit before this exercise started. No `src/` change
occurred during F1-R (confirmed above), so neither needed re-running;
this report's own closing commit re-confirms CI green on the F1-R
commit itself.

## Acceptance criteria

1. `#543492` reproduces the old failure under the frozen v0.4.4
   binary — **met** (exit 3, empty stdout, confirmed against the real
   historical content and the real original manifest).
2. `#543492` is fixed under `940e388` exactly as F1 specifies — **met**
   (exit 2, `lightdm`'s real PASS preserved, `lightdm-enso-greeter`
   correctly unavailable, no supplemental manifest needed).
3. Every other successful `check` observation is semantically identical
   after ONLY the pre-declared additive-field normalization, unless it
   independently belongs to the missing-target recovery class —
   **met** (281/282 rows `identical_after_additive_normalization`, the
   1 exception is exactly the pre-registered `#543492` case).
4. Zero unexpected verdict changes — **met** (0
   `unexpected_difference` rows in the 282-row corpus ledger).
5. Zero unexpected exit-code changes — **met** (same evidence; the one
   exit-code change, 3→2, is the pre-registered expected one).
6. Zero unexpected lost targets — **met** (every surviving target's
   full verdict/evidence/span set is identical after normalization in
   every row, including `#543492`'s own `lightdm`).
7. Zero new TOOL_ERRORs — **met** (candidate never produces a
   TOOL_ERROR the baseline didn't already produce; the one delta runs
   the opposite direction, baseline TOOL_ERROR → candidate success).
8. Fatal negative controls remain fatal — **met** (7/7, see above; one
   disclosed, non-blocking stderr-text-only observation, explicitly
   not a fatal-status regression).
9. `audit-diff` negative controls show zero F1-caused semantic
   regressions — **met** (94/94 byte-identical).
10. Full current test suite and CI remain green — **met** (325/325
    local tests, 6/6 CI workflows, both confirmed on `940e388` itself).
11. `src/` remains exactly at `940e388` — **met** (confirmed via `git
    log`/`git status`/`git diff --stat` immediately before writing this
    report).

**All eleven criteria hold. Zero unexpected differences were found
across 282 corpus comparison rows and 7 negative controls; the one
disclosed stderr-text-only observation was raised transparently,
resolved by explicit user direction to record rather than treat as
blocking, and does not affect any criterion above.**

## Conclusion

F1's real reproducer (`#543492`) is fixed, exactly as specified. The
fix produced zero regressions on the already-seen S4 regression
corpus: every one of the 94 applicable PRs' `check` and `audit-diff`
behavior is identical to baseline after only the two pre-declared
additive fields are normalized away, with the single exception being
the one pre-registered missing-target-recovery case the fix exists to
produce.

**This report does NOT say, and none of its evidence supports saying:**
S4 now passes; deployment readiness is established; precision or
generalization improved; the corpus is fresh; or the old S4 gate
result is superseded. **S4 remains immutable `FAIL`**
(`fixtures/s4-live-pr-shadow/s4-final-report.md`, commit `cebb3a6`).
F1-R is an answer-key regression exercise on already-seen data, nothing
more.

## What this report deliberately does not authorize

Per the explicit F1-R authorization: no version bump, no release/tag,
no S5, no capability-gap fixes (pixelfed/cloudlog/wildcard
`attrsOf(submodule)`, S4 report item 17), no fix to plain `oba diff`'s
own analogous all-or-nothing gap (noted as a known, disclosed,
out-of-scope limitation in the S4-F1 commit message, confirmed
untouched here). Each requires its own separate, explicit
authorization.
