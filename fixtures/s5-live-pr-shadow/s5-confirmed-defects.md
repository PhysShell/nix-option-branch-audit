# S5 confirmed defects — provenance record (S5-R0)

This document is a report-integrity artifact only. It does not
reinterpret, extend, weaken, or supersede the frozen S5 result. The
underlying evidence, adjudication, and gate verdict are unchanged and
immutable — see `s5-final-report.md`/`s5-final-report.json` for the
result this record documents, and `adjudication-ledger.jsonl` for the
raw source of truth every field below is drawn from verbatim.

**S5 result this record belongs to** (immutable, unchanged by this document):
- Analyzer under test: `v0.4.5`, commit `8f1701a289ddab8bc23f23a25cc0937863fa0356`,
  artifact SHA-256 `c07bed6c37fa3e0f1885099bb3dfc7a7b741531e8a156dc4fa8a7a160bd45800`.
- Corpus: 369/369 (S5-A 150/150 + S5-B 219/219), full two-reviewer adjudication.
- Verdict: **`FAIL`** (`s5-final-report.json` → `gate.verdict`).
- These are the exact 3 Tier-1 events that produced that verdict —
  no more, no fewer; `gate.reasons` has exactly 3 entries.

No implementation change, no fix, no new replay, no version bump, and
no release accompanies this document. Each defect remains exactly as
adjudicated; this record only makes that adjudication's provenance
unambiguous and independently reproducible from committed artifacts
alone.

---

## Defect 1 — cgit: false finding

**PR / reproducer identity**
- PR: `NixOS/nixpkgs#475112` ("nixos/cgit: add gitHttpBackend options")
- Base commit: `308f82252459d32a68cfbbec3af672e5867ff994`
- Head commit: `a722a999d1af37d9f3760680dc50fc598fce01a6`
- Module: `nixos/modules/services/networking/cgit.nix`
- Test: `nixos/tests/cgit.nix`
- S5-B position: 170 (`s5b-frozen-order.json`, batch `s5b-batch7`)
- Ledger presentation id: `B:475112:oba:gitHttpBackend.enable:new_finding:1`

**Expected behaviour**: `oba`'s `check`/`audit-diff` should report a
predicate as `witnessed` when a real test in the corpus sets the
watched option to a value that exercises that predicate branch. Here,
`nixos/tests/cgit.nix` (at the head commit) explicitly sets
`services.cgit."no-git-http-backend.localhost".gitHttpBackend.enable = false;`
(line 58) and the `testScript` (lines 138-146, the section titled "Disabling
the git-http-backend-works") drives a real `git clone` against that
vhost both before and after removing a `git-daemon-export-ok` marker
file, asserting different success/failure behaviour each time — a
genuine, deliberate behavioural exercise of the disabled branch. The
expected verdict for this predicate is therefore `PASS` (witnessed),
not `OBA001`.

**Observed v0.4.5 behaviour**: `check --root head-root` returns
`{"verdict": "OBA001", "option": "gitHttpBackend.enable", ...,
"predicate_attempts": [{"witnessed": false}, {"witnessed": false}]}`
for the predicate at `cgit.nix` line 308
(`lib.optionalAttrs cfg.gitHttpBackend.enable {`), rendered in
`summary.md` as "NEWLY OBSERVABLE FINDING (ORIGIN UNCLEAR)" /
`uncovered option branch: no test assignment provably flips this
predicate away from its default`.

**Adjudicated defect class**: false finding in a maintainer-facing
actionable presentation (frozen gate Tier 1 category: "a false finding
in a maintainer-facing actionable presentation").

**Evidence / provenance sufficient for reproduction**:
- Raw analyzer output: `adjudication/s5b-batch7/475112/{raw.json,check-base.json,check-head.json,summary.md,targets.toml,command.txt,meta.json}`
- Real source confirming the test coverage: `nixos/tests/cgit.nix` at
  `a722a999d1af37d9f3760680dc50fc598fce01a6`, lines 42-59 (the 3-vhost
  test config) and 128-146 (the testScript sections exercising both
  the enabled and disabled paths) — independently re-fetched and
  quoted verbatim by both reviewers below, and independently
  re-verified by the coordinator directly (line 58 confirmed present)
  before resolution.
- Two independent blind reviews, both confirming the same defect
  (`adjudication-ledger.jsonl`, `actionable_presentation` record for
  `B:475112:oba:gitHttpBackend.enable:new_finding:1`, `reviews[0]` and
  `reviews[1]`), plus the coordinator's `coordinator_resolution` field
  on that same record explaining the resolution.

---

## Defect 2 — angrr: false "Unchanged" on a real breaking removal

**PR / reproducer identity**
- PR: `NixOS/nixpkgs#471312` ("angrr: 0.1.5 -> 0.2.0")
- Base commit: `ca696276378b844d888872c9045d89e9b58cc85a`
- Head commit: `62ea5b9ae7d4329475089f60232ee1426d9547a9`
- Module: `nixos/modules/services/misc/angrr.nix`
- Test: `nixos/tests/angrr.nix`
- S5-B position: 144 (`s5b-frozen-order.json`, batch `s5b-batch6`)
- Ledger presentation id: `B:471312:oba:period:unchanged:1`

**Expected behaviour**: the diff between base and head genuinely
removes 3 real, standalone top-level options —
`services.angrr.period` (a duration string, default `"7d"`),
`services.angrr.removeRoot`, and `services.angrr.ownedOnly` — each via
its own `lib.mkRemovedOptionModule [...]` entry with an explicit
migration message ("This option has been removed since angrr 0.2.0.
Please use `services.angrr.settings`..."). A tool comparing base and
head for the watched option `period` should report this as a real
removal/transition (e.g. `removed_subjects_with_finding` or an
equivalent "this option is gone" classification), not "no change."

**Observed v0.4.5 behaviour**: `check --root base-root` and
`check --root head-root` both return `{"verdict": "PredicateNotFound",
"option": "period"}` for the watched option, and `audit-diff` renders
`summary.md`'s table as `Unchanged: 1` with every other row at 0 —
i.e. the tool reports **no change at all**. Mechanism, confirmed
against `discovered_options` in both `check-base.json` and
`check-head.json`: base's `period` resolves to a real declaration at
`angrr.nix` line 18 (`default_source: "7d"`); head's `period` resolves
to a structurally and semantically unrelated declaration at line 113
(`default_source: null`), nested several levels deeper inside
`services.angrr.settings.temporary-root-policies.<name>.period` — a
brand-new, per-policy field in a freeform `attrsOf(submodule)`
introduced by this same PR, which happens to share the bare leaf name
"period" with the real, removed top-level option. The tool's own
option-identity matching operates on that bare leaf name within the
shared `option_prefix` scope, without composing the full structural
path through the submodule nesting boundary, so it treats the two
unrelated declarations as "the same option, continuously present."

**Adjudicated defect class**: materially wrong rendered presentation —
specifically a **false negative** (the frozen gate's "materially wrong
rendered presentation" category; this is the single most severe
confirmed defect of the round, since it is silence where a maintainer
should have seen a clear, unambiguous breaking-change signal, not an
imprecise or misleading positive).

**Evidence / provenance sufficient for reproduction**:
- Raw analyzer output: `adjudication/s5b-batch6/471312/{raw.json,check-base.json,check-head.json,summary.md,targets.toml,command.txt,meta.json}`
- The exact `discovered_options` entries establishing the two
  structurally distinct declarations are reproducible directly from
  the committed `check-base.json`/`check-head.json` (search for
  `"path": ["period"]` in each — no live fetch required to see the
  base/head disagreement itself).
- Real source confirming both the removal (`mkRemovedOptionModule`
  import block, `angrr.nix` head lines 239-241, also covering
  `removeRoot`/`ownedOnly`) and the unrelated new declaration
  (`temporaryRootPolicyOptions`, head lines 103-156) — independently
  re-fetched and quoted verbatim by both reviewers.
- Two independent blind reviews, both confirming the same defect and
  the same causal mechanism independently (`actionable_presentation`
  record for `B:471312:oba:period:unchanged:1`), plus the
  coordinator's `coordinator_resolution`.
- **Provenance note** (the specific fact this S5-R0 round exists to
  make unambiguous): this presentation was **not** flagged by the
  original evidence-gathering pass as needing review — `oba`'s own
  rendering gave no signal that anything was wrong (the tool's own
  bucket vocabulary has no category for a false "Unchanged," so
  nothing in its output ever pointed at this PR). It was found only
  because the evidence-gathering agent reported the raw
  `discovered_options` asymmetry to the coordinator as a suspicious
  raw fact worth checking, independent of what the tool itself
  claimed. The corresponding `pr_summary` ledger record for this PR
  (written at evidence-gathering time) still shows
  `actionable_count: 0`, reflecting that the tool's own rendering
  never flagged it — this pr_summary record is left untouched as
  historical evidence of exactly that fact; `generate-report.py` was
  corrected (S5-R0) to count actionable presentations from the
  ledger's own `actionable_presentation` records rather than from this
  now-demonstrably-incomplete counter.

---

## Defect 3 — guacamole: materially wrong rendered presentation

**PR / reproducer identity**
- PR: `NixOS/nixpkgs#462487` ("nixos/guacamole: client option defined in server module")
- Base commit: `3db8153243db1455a18da92119581d0580dd70b8`
- Head commit: `598f6dd4fc046410bb2719227d5d48e927e7d39b`
- Modules: `nixos/modules/services/web-apps/guacamole-server.nix` (watched target),
  `nixos/modules/services/web-apps/guacamole-client.nix` (relocation target)
- Test: `nixos/tests/guacamole-server.nix`
- S5-B position: 12 (`s5b-frozen-order.json`, batch `s5b-batch1`)
- Ledger presentation id: `B:462487:oba:logbackXml:finding_became_inconclusive:1`

**Expected behaviour**: the diff genuinely relocates `logbackXml`
(and `userMappingXml`) from `guacamole-server.nix` to
`guacamole-client.nix` via a real `lib.mkRenamedOptionModule
["services" "guacamole-server" "logbackXml"]
["services" "guacamole-client" "logbackXml"]` — a construct that
auto-forwards any existing configuration value to the new location, so
the option's underlying capability is fully preserved and functional,
not dropped. A rendered presentation describing this transition should
not be indistinguishable from a genuine, unexplained disappearance.

**Observed v0.4.5 behaviour**: `check --root base-root` (watching
`guacamole-server`, prefix `[services, guacamole-server]`) returns a
real `finding` for `logbackXml` at base (never witnessed by the real
test); `check --root head-root` returns `{"verdict": "OptionNotFound",
"option": "logbackXml"}`. `audit-diff`'s `summary.md` renders this
transition as: `### FINDING BECAME INCONCLUSIVE` / `no mkOption
declaration found for this watched option` / `declaration: not
found`. Nothing in the rendered text indicates a relocation occurred,
despite the `mkRenamedOptionModule` construct being present in the
very module file `oba` already parses.

**Adjudicated defect class**: materially wrong rendered presentation
(frozen gate Tier 1 category). Distinguished explicitly, during
adjudication, from two structurally similar but non-defective cases
found the same round (`#509507` sshd `banner`, `#466806` gollum
`local-time`) where a real `mkRemovedOptionModule` hard-removal, with
no automatic value-forwarding, was correctly rendered as "not found" —
the defect here is specific to the *auto-forwarding rename* case,
where the tool has direct, in-hand visibility into the exact
replacement location and does not surface it.

**Evidence / provenance sufficient for reproduction**:
- Raw analyzer output: `adjudication/s5b-batch1/462487/{raw.json,check-base.json,check-head.json,summary.md,targets.toml,command.txt,meta.json}`
- Real source confirming the relocation: `guacamole-server.nix` at
  base (real `mkOption` declarations, lines 43/52) and at head (the
  `mkRenamedOptionModule` imports, replacing the declarations
  entirely); `guacamole-client.nix` at head (real, newly-added
  `mkOption` declarations for the same two options, confirmed absent
  from this file at base) — independently re-fetched and quoted
  verbatim by both reviewers.
- Two independent blind reviews, both confirming the same relocation
  facts and independently characterizing the rendering as materially
  misleading (`actionable_presentation` record for
  `B:462487:oba:logbackXml:finding_became_inconclusive:1`), plus the
  coordinator's `coordinator_resolution`.

---

## Machine-readable cross-reference

All three `presentation_id`s above resolve directly to their full
`actionable_presentation` ledger records — including both raw
independent reviews and the coordinator's resolution — via:

```
grep -F '"pr": 475112' adjudication-ledger.jsonl   # cgit
grep -F '"pr": 471312' adjudication-ledger.jsonl   # angrr
grep -F '"pr": 462487' adjudication-ledger.jsonl   # guacamole
```

Each of the 3 `gate.reasons` entries in `s5-final-report.json` names
its PR number explicitly and corresponds 1:1 to one section above.
