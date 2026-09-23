# S5 final report

Generated mechanically from the committed adjudication ledger
(`adjudication-ledger.jsonl`, 393 records: 369 `pr_summary` + 18
`actionable_presentation` + 6 `pass_adjudication`) by `generate-report.py`,
whose own numeric output is `s5-final-report.json` — this document is
prose around those numbers, never a hand-typed restatement of them.

**Frozen protocol**: `fixtures/s5-design/s5-protocol-final.md`.
**Analyzer measured**: `v0.4.5`, commit `8f1701a289ddab8bc23f23a25cc0937863fa0356`,
artifact SHA-256 `c07bed6c37fa3e0f1885099bb3dfc7a7b741531e8a156dc4fa8a7a160bd45800`
— exactly the frozen baseline, no other build was used anywhere in this round.

**S5-R0 report-integrity correction**: the initially-committed report
undercounted `s5a.pass_verdict_count` (0, corrected to 1) and
`s5b.actionable_presentation_count` (17, corrected to 18).
`generate-report.py` had summed `pr_summary.actionable_count`/
`pass_verdict_count` — fields that record what the tool's own
rendering flagged as needing review *at evidence-gathering time* — for
two presentations the coordinator's independent adjudication added
*after* evidence-gathering (PR `#471312`'s false "Unchanged" was never
flagged by the tool's own rendering as anything needing review; PR
`#461261`'s PASS was deliberately left unclassified by the
evidence-gathering pass pending coordinator judgment). Fixed to count
directly from the ledger's own `actionable_presentation`/
`pass_adjudication` records — the true source of truth — never from
`pr_summary`'s own counters. **This changed no adjudication, no
corpus membership, no gate verdict, and no confirmed-defect finding**
— `gate`, `pr_level_precision`, `reconciliation`, and
`path_overlap_diagnostics` are byte-identical before and after. See
`s5-confirmed-defects.md` for the full, unambiguous provenance record
of each of the 3 confirmed defects.

## Reconciliation

Both cohorts processed as an unbroken 1..N prefix of their frozen
order, byte-verified against `fixtures/s5-design/s5a-frozen-order.json`
/ `s5b-frozen-order.json` position-by-position: **0 mismatches, 0
duplicates**, both cohorts complete (S5-A 150/150, S5-B 219/219).

## S5-A — representative cohort (Estimand A)

Processed the complete frozen sample, no actionable-count stopping
condition, as required.

| | |
|---|---:|
| Positions processed | 150 / 150 |
| Applicable | 31 (20.7%) |
| `TOOL_ERROR` | 0 |
| Actionable presentations | 0 |
| PASS verdicts surfaced for review | 1 (`wakapi` #461261, `passwordSalt` — confirmed correct) |
| Manual effort `<2` / `2-10` | 119 / 31 |

## S5-B — enriched precision cohort (Estimand B)

Processed the complete frozen R4-selected order up to the cap; the
30-distinct-actionable-PR target was not reached before the cap
exhausted (see Gate, below — this is moot regardless, since Tier 1
already governs the verdict).

| | |
|---|---:|
| Positions processed | 219 / 219 (cap reached) |
| Applicable | 149 (68.0%) — vs. S5-A's 20.7%, confirming the frozen `R4_mkoption_line_edit_v1` selector's enrichment worked as designed |
| `TOOL_ERROR` | 0 |
| Actionable presentations surfaced | 18 (every actionable PR this round happened to contribute exactly 1 presentation, so this number equals the distinct-PR count below — not a general property, just how this round landed) |
| Distinct actionable PRs | **18** (target 30, not reached) |
| PR-level correct / total | **15 / 18** |
| PASS verdicts surfaced for review | 5 (all confirmed correct: `tayga`, `yggdrasil`, `vmalert`, `ncps`, `paretosecurity`) |
| False PASS | 0 |
| Unresolved reviewer disagreements | 0 |
| Manual effort `<2` / `2-10` | 152 / 67 |

Every actionable presentation and every materially-relevant PASS
received exactly two independent, blind reviewers who fetched and
verified against real `NixOS/nixpkgs` source themselves, dispatched in
parallel, never shown each other's judgment before submitting; any
disagreement was resolved by the coordinator against the raw evidence
and both original reviews preserved verbatim in the ledger.

## Gate

Evaluated mechanically by `fixtures/s5-design/gate.py::evaluate_gate`,
never overridden by hand.

```json
{
  "verdict": "FAIL",
  "reasons": [
    "PR #462487: incorrect presentation(s) ['B:462487:oba:logbackXml:finding_became_inconclusive:1']",
    "PR #471312: incorrect presentation(s) ['B:471312:oba:period:unchanged:1']",
    "PR #475112: incorrect presentation(s) ['B:475112:oba:gitHttpBackend.enable:new_finding:1']"
  ],
  "round_complete": true
}
```

**Tier 1 — FAIL.** Three confirmed defects, each independently
verified by two blind reviewers against real source, none disputed:

1. **`#475112` cgit — false finding.** The tool claimed
   `gitHttpBackend.enable`'s disabled branch was never test-covered
   (`witnessed: false`). The PR's own real test explicitly sets
   `gitHttpBackend.enable = false` on a dedicated third vhost and
   behaviorally exercises exactly that branch (a `git clone`
   succeeds/fails around a marker file, under a test section literally
   titled "Disabling the git-http-backend-works"). The cleanest,
   simplest false-finding instance found this round — a direct, wrong
   claim about test coverage that plainly exists.

2. **`#471312` angrr — false "Unchanged."** The most severe finding of
   the round: a real, deliberate breaking change (3 options —
   `period`, `removeRoot`, `ownedOnly` — removed via
   `mkRemovedOptionModule` with an explicit migration message) was
   rendered as **"Unchanged: 1"**, hiding it completely. Root cause,
   confirmed by both reviewers independently against real source: the
   tool's option-identity matching operates on bare leaf attribute
   names within a shared `option_prefix` scope, without composing the
   full structural path through submodule/`attrsOf` nesting — a
   coincidentally-named, structurally unrelated option nested 2+ levels
   deeper (`services.angrr.settings.temporary-root-policies.<name>.period`)
   was conflated with the real, removed top-level
   `services.angrr.period`. A false negative, not merely an imprecise
   positive — the single most consequential change in that PR is
   exactly the one the tool hid.

3. **`#462487` guacamole — materially wrong rendered presentation.** A
   real option relocation (`logbackXml`/`userMappingXml` moved from
   `guacamole-server.nix` to `guacamole-client.nix` via a genuine
   `mkRenamedOptionModule`, value auto-forwarded, fully preserved and
   functional) rendered as "declaration: not found" — both reviewers
   independently called this actively misleading, since a maintainer
   reading only the summary would reasonably conclude the setting was
   dropped or broken, when it is in fact alive one file over and the
   tool already has the rename construct in hand within the very
   module it parses.

Per the frozen protocol, a Tier-1 event does not stop sampling early —
the round continued to its pre-registered stop point (the 219 cap)
regardless, exactly as designed, and would have regardless of how many
more or fewer Tier-1 events had been found along the way.

**Contrast — five related findings that were reviewed just as
carefully and resolved as correct, not defects**, worth naming
explicitly since the line between "real defect" and "real but
disclosed limitation" matters for how these should be triaged
afterward:

- `#509507` sshd `banner`, `#466806` gollum `local-time` — genuine hard
  removals (`mkRemovedOptionModule`, no auto-forwarding) correctly
  rendered as "not found." Gollum additionally has an undocumented
  manual successor path (`extraConfig`) the renderer doesn't surface —
  a real, disclosed quality-of-service gap, but not a false belief
  (unlike guacamole, gollum's underlying option really is gone;
  gollum's gap is *incompleteness*, guacamole's is *misdirection*).
- `#397967` fedimintd `api_ws.openFirewall` — a renamed pre-existing
  gap (`api.openFirewall` → `api_ws.openFirewall`, identical shape),
  correctly detected as unwitnessed at the new name but framed as
  "newly observable" without crediting the rename lineage — a real,
  disclosed limitation (no rename-tracking), not a false claim.
- `#260551` prosody `checkConfig` and 7 other `origin_unclear` cases —
  the tool declines to assert causal PR-attribution rather than guess;
  established as honest, conservative non-claiming, consistent with
  identical S4 precedent (9/9 of S4's own reviewed presentations).
- `#490920` traefik `staticConfigFile` — reviewed with real
  disagreement (one reviewer initially argued the default/null branch
  was effectively exercised via live service behavior); resolved
  correct because `OBA001`'s own stated claim is specifically about
  *deviation-from-default* coverage, not code-execution coverage, and
  no test anywhere sets this option to a non-default value.

## Known rendering/detection gap catalogue (carried forward, not fixed by S5)

Beyond the 3 confirmed Tier-1 defects above, this round's raw evidence
surfaced a large, disclosed catalogue of non-blocking observations
(none individually adjudicated as Tier-1 since none were presented as
an actionable finding or PASS a maintainer would see — they showed up
as `OptionNotFound`/`PredicateNotFound` on both sides, i.e. no
transition, no presentation):

- **`*_not_found ↔ *_not_found` rendering blackout**: ~40+ instances
  across the round — a real transition the raw JSON captures correctly
  (`changed: 1`, a real `oba_verdict_transitions` entry) falls through
  every bucket in the rendered `summary.md`, `notable: []`. Distinct
  from the 3 confirmed defects above: this is a coverage gap in the
  *renderer's own bucket taxonomy* for this specific transition shape,
  not a false claim about any specific PR.
- **`mkPackageOption` non-discovery**: 5 confirmed instances (gonic,
  dex, n8n, traccar, dnscrypt-proxy2), including 2 clean
  before/after-transition pairs in opposite directions (olivetin,
  postgresql), plus graylog's controlled natural experiment. A
  `mkPackageOption`-declared option is never discovered; a plain
  `mkOption` at the identical site is.
- **Dotted-path total non-discovery**: `options.services.X = {...}` —
  4+ confirmed instances (wastebin, paretosecurity, sanoid, keycloak),
  though NOT a fully general rule (several working modules use the
  identical idiom — n8n, olivetin, photoprism, hedgedoc) — raw,
  unresolved asymmetry, no confirmed root cause.
- **`attrsOf(submodule)` wildcard-instance gaps**: confirmed recurring
  (kanidm, wstunnel), but NOT universal — several wildcard-instance
  modules resolve fine (drupal, frp, fedimintd) — same raw,
  unresolved asymmetry.
- **`options = let ... in {...}` / non-literal options-value wrapper**:
  4 confirmed instances (lanraragi, nvidia-container-toolkit ×3 across
  separate PRs, evcc, frigate), with a striking, reproducible symptom:
  the walker sometimes substitutes fields from an unrelated
  locally-scoped helper submodule instead of returning nothing.
- **`genAttrs`-computed option sets** (pgbackrest): a whole
  dynamically-constructed option branch invisible at both base and
  head.
- **Second-level submodule blind spot** (postgresql `ensureClauses`):
  the walker enumerates one level of nesting past a
  `listOf(submodule)` wrapper but not two.
- Carried forward unchanged from S4: Pixelfed nested/dotted
  declaration gap, Cloudlog declaration-vs-predicate asymmetry,
  wildcard `attrsOf(submodule)` discovery gap (both confirmed and
  recurring in S5), plain `oba diff`'s own missing-target behavior.

None of these were fixed during S5, per the frozen protocol's explicit
"no `src/` changes" boundary — they are disclosed evidence about the
measured v0.4.5 system, not defects introduced or resolved by this
round.

## Path-overlap diagnostics (§10 of the frozen protocol)

Of the 18 distinct actionable PRs: 3 touch a service-module path S4
had already examined, 15 are path-novel. Both groups have comparable
correctness (2/3 overlap-correct, 13/15 novel-correct) — no visible
clustering of the 3 confirmed defects by path-overlap status (all 3
incorrect PRs — cgit, angrr, guacamole — are path-novel, for what
that's worth on a sample this small; not a claim of statistical
significance).

## Evidence boundaries

This is a real, complete, mechanically-gated S5 result: **FAIL**,
driven by 3 independently-confirmed real defects in the measured
v0.4.5 binary, found through genuinely adversarial two-reviewer
verification against live source, not through synthetic or
constructed test cases. It does not constitute a fix, a patch, or a
regression test for any of the 3 defects — per the frozen protocol,
that is explicitly out of scope for S5 itself and would require a
separate S5-F* round with its own explicit authorization. The 149
`OptionNotFound`/`PredicateNotFound`-class gaps catalogued above are
disclosed, non-blocking observations about analyzer coverage, not
independently adjudicated Tier-1 events — they did not reach the
maintainer-facing bar (no transition, no rendered presentation) that
would have triggered the same two-reviewer process.
