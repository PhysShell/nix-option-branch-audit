# S5-F1-R: full-corpus regression replay of S5-F1 (candidate `cdfb4ca`)

This document is a validation artifact only. It does not rewrite,
supersede, or reinterpret the historical S5 result. Historical S5
(verdict **FAIL**, corpus 369/369, S5-R0 commit `28a1d58`) remains
completely unchanged — this replay reads its committed evidence
read-only and writes ONLY new files under this directory.

**Baseline**: `v0.4.5`, commit `8f1701a289ddab8bc23f23a25cc0937863fa0356`.
**Candidate**: commit `cdfb4cac31b7346d925a28eb2c4e5c280aeac95d` (S5-F1's own
fix), built from a genuinely clean checkout (`candidate-provenance.json`),
binary SHA-256 `10563c648967ce68e7b03f81817d176d92aa3312be2cd4432fb334fa6f091462`.
**Targeted defect**: PR `#471312` (angrr) false `"Unchanged"`.

## Replay coverage: 369/369

- 189 PRs historically `applicable: false` — not re-executed (applicability
  is a frozen, content-based fact independent of the candidate binary;
  "same applicable execution semantics" per the mandate). Trivially
  unchanged by definition.
- 180 PRs historically `applicable: true` — fully replayed: real content
  re-fetched at the exact frozen `base_sha`/`head_sha`, the exact
  originally-committed `targets.toml` reused verbatim, the exact 3
  commands re-run with the candidate binary.
- **0 `window_evaluation_incomplete`** — every one of the 180 replayed
  cleanly, no fetch failures, no infrastructure problems. `gh api
  rate_limit` confirmed healthy throughout.

## Comparison result

- **158 / 180 byte-identical** to the historical S5 output (`raw.json`,
  `check-base.json`, `check-head.json` all parse-equal; both binaries
  self-report `"tool.version": "0.4.5"` identically, so no field needed
  filtering).
- **22 / 180 changed.**

## Delta classification (every changed PR individually traced)

| Classification | Count | PRs |
|---|---:|---|
| **expected target correction** | 1 | `#471312` |
| **same-root-cause correction** | **0** | — |
| **unexpected semantic delta** | 21 | see below |

### `#471312` — expected target correction, confirmed

Re-verified from the frozen S5 replay itself (not merely the synthetic
regression fixture): head's watched `period` correctly changes from a
false `PredicateNotFound` match to the correct `OptionNotFound`,
`predicate_not_found->option_not_found`, base unchanged at
`PredicateNotFound`. Exactly the intended F1 fix.

### 21 PRs — unexpected semantic delta: a genuine over-exclusion bug, distinct from F1's own collision-fix mechanism

**This is the headline finding of this replay.** All 21 remaining
deltas trace to the SAME modified code path
(`is_nested_inside_named_let_or_rec_binding`, gated on
`prefix_is_concrete`) but the OPPOSITE, HARMFUL direction: the fix
excludes a named `let`/`rec`-bound submodule's declarations whenever
it isn't textually nested inside a tracked `mkOption {...}` call and
the target's `option_prefix` is concrete — but this check cannot
distinguish angrr's real defect (a named submodule genuinely
UNRELATED to `option_prefix`, reached from several structural levels
away) from a named submodule that IS the legitimate, real way a
concrete `option_prefix`'s own content is declared, just referenced by
name (`types.attrsOf/listOf/nullOr (types.submodule <name>)`) rather
than written inline. Confirmed against real, live-fetched nixpkgs
source for all 3 affected module families (`classification-notes.json`
has the full per-PR trace):

- **prosody.nix** (`mucOpts`, `httpFileShareOpts`, ...): `muc = mkOption
  { type = types.listOf (types.submodule mucOpts); ... };` declared
  exactly at `option_prefix=[services,prosody,muc]`'s own concrete
  path.
- **drupal.nix** (`siteOpts`): `sites = mkOption { type =
  types.attrsOf (types.submodule siteOpts); ... };` declared exactly
  at `option_prefix=[services,drupal,sites]`'s own concrete path.
- **fedimintd.nix** (`fedimintdOpts`): the whole per-instance type is
  `types.attrsOf (types.submodule fedimintdOpts)`, `option_prefix =
  [services,fedimintd]`.

**7 of these 21 produce a real watched-verdict regression** (the
reported outcome itself changes, not just internal bookkeeping):
`431289`, `506644`, `508427`, `440660`, `428153`, `397967`, `427260`.
In every case a real, historically-correct `VerdictChanged` (typically
`option_not_found -> predicate_not_found`, i.e. "we correctly found
this declaration, though no test exercises it") collapses into a false
`Unchanged` (`OptionNotFound` on both sides) — the SAME failure shape
angrr itself had, now reproduced by the fix in the opposite,
unintended place. `#397967` is the most consequential instance: it is
`fedimintd`'s `api_ws.openFirewall`, a **real, already-adjudicated
actionable finding from S5 itself** (two independent blind reviewers
confirmed it correct) — the candidate binary would make this real,
confirmed-correct finding **invisible**.

**14 of these 21 are cosmetic-only**: `discovered_options`'s own
list differs (fewer entries recorded, from the same over-exclusion),
but the specific watched leaf's own verdict happens not to be among
the excluded declarations, so the reported outcome is unaffected.
Two of these fourteen are worth naming explicitly since they are
themselves real, already-adjudicated S5 findings: `#260551` (prosody
`checkConfig`, a real actionable finding) and `#432528` (tayga
`wkpfStrict`, a real confirmed PASS) — both independently re-verified
here as **verdict-identical** to their historical record (only
unrelated `discovered_options` entries elsewhere in the same files
changed), so neither of S5's own already-reviewed findings is put at
risk by this bug, even though the underlying mechanism touched their
files too.

## Anchor checks (all confirmed)

1. **`#471312` corrected as intended** — confirmed above, from the real
   frozen replay.
2. **`#475112` (cgit) and `#462487` (guacamole) remain byte-identical**
   — `comparison-report.jsonl` confirms `identical: true` for both;
   neither of the two known, deliberately-unfixed defects was silently
   touched.
3. **Fatal/error controls intact** — 0 `parse_errors`/`tool_error`-shaped
   results anywhere across all 540 replayed output files (180 PRs × 3
   files), matching S5's own historical 0-tool-error record exactly.

## Gate

Per the frozen mandate: S5-F1-R may PASS only if every delta is either
the expected target correction or a demonstrated same-root-cause
correction, with zero unexpected semantic regressions. That condition
is **not met**: 21 unexpected semantic deltas were found, 7 of them
real watched-verdict regressions, one of which (`#397967`) would hide
an already-adjudicated real finding.

```json
"gate_verdict": "FAIL"
```

Per the mandate's own explicit instruction for this outcome: this delta
is recorded fully (this document + `classification-notes.json` +
`delta-classification.json` + the complete raw replay under
`replay/`), it is **not patched during F1-R**, and no rerun follows an
ad-hoc fix. S5-F1's own commit (`cdfb4ca`) is not reverted or altered
by this replay — it remains exactly as committed, now with this
replay's evidence attached showing it needs a narrower, more precise
fix (distinguishing "unrelated auxiliary submodule" from "the
legitimate declaration site for this option_prefix, referenced by
name") before it can be trusted as a clean, isolated correction.

## Integrity / reproducibility

- `compare-replay.py` and `classify-deltas.py` are both pure,
  deterministic functions of the committed replay output + the
  committed historical S5 evidence + `classification-notes.json`'s
  own hand-verified causal notes — re-running them against the
  committed `replay/` directory reproduces `comparison-report.jsonl`
  and `delta-classification.json` exactly.
- `candidate-provenance.json` records the exact clean-checkout build
  process and binary SHA-256 used for every one of the 180 replay
  runs.

## Files in this commit

- `applicable-manifest.json` — the 180 applicable PRs and their frozen
  identities (input to the replay).
- `candidate-provenance.json` — candidate binary build/identity record.
- `replay/<cohort>/<pr>/{raw.json,check-base.json,check-head.json,summary.md,command.txt}` —
  full raw candidate output for all 180 applicable PRs.
- `comparison-report.jsonl` — mechanical identical/changed comparison
  for all 180, including full before/after JSON for every changed PR.
- `classification-notes.json` — hand-verified, real-source-traced
  causal classification for every one of the 22 changed PRs.
- `delta-classification.json` — the mechanical join of the above.
- `f1-r-summary.json` — machine-readable summary of this whole report.
- `compare-replay.py`, `classify-deltas.py` — the reproducible
  comparison/classification scripts themselves.
