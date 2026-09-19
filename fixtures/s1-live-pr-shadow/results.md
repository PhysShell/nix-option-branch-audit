# S1: live nixpkgs PR shadow evaluation -- results

All 30 PRs from `sample.md`, shadow-audited against the frozen `v0.4.0`
binary (`67bb2e1`), manually adjudicated. Zero `src/` changes, zero new
CDC registry entries, zero GitHub writes anywhere in this round.

## Headline number, stated plainly

**Across all 30 real PRs: `new_findings=0`, `new_inconclusives=0`,
`evidence_only_changes=0` everywhere. The bounded `notable` list --
the entire thing a `audit-diff/action.yml` step summary would ever
show a maintainer -- was EMPTY on every single one of the 30 runs.**

This means `correctness precision` and `actionable precision`
(correct/actionable findings ÷ inspected findings) are **undefined,
0/0** on this sample -- there was nothing to inspect. That is itself
the real result, not a gap in the writeup: on an unbiased, mechanically
drawn slice of the live nixpkgs PR stream, the current frozen product
essentially never had anything new to say. This is a genuinely
different, and more sobering, answer than "precision was low" would
have been -- see **Verdict** below for what it does and doesn't mean.

## Full per-PR table

| PR | title (short) | oba | result | notable |
|---|---|---|---|---|
| [347823](https://github.com/NixOS/nixpkgs/pull/347823) | immich: listen-on-all-interfaces test | yes | persistent OBA001 (`openFirewall`) | 0 |
| [492803](https://github.com/NixOS/nixpkgs/pull/492803) | ntfy: remove redundant user creation | yes | **resolved_finding=1** -- see caveat below | 0 |
| [496303](https://github.com/NixOS/nixpkgs/pull/496303) | opensearch-dashboards: init | no | new module, no base state | -- |
| [519494](https://github.com/NixOS/nixpkgs/pull/519494) | rosec: init | no | new module, no base state | -- |
| [525702](https://github.com/NixOS/nixpkgs/pull/525702) | fleet-orbit: init | no | new module, no base state | -- |
| [527821](https://github.com/NixOS/nixpkgs/pull/527821) | iocaine: init module | no | new module, no base state | -- |
| [539076](https://github.com/NixOS/nixpkgs/pull/539076) | lego: version bump | yes | persistent `PredicateNotFound` (lambda-param shadowing) | 0 |
| [543675](https://github.com/NixOS/nixpkgs/pull/543675) | openvswitch: transient ports | yes | persistent OBA001 (`resetOnStart`) | 0 |
| [549506](https://github.com/NixOS/nixpkgs/pull/549506) | kener: init module | yes* | **TOOL_ERROR (exit 3)** | -- |
| [550960](https://github.com/NixOS/nixpkgs/pull/550960) | wivrn: debug build flags | no | no NixOS test exists at all | -- |
| [551955](https://github.com/NixOS/nixpkgs/pull/551955) | yace exporter: init | yes* | **TOOL_ERROR (exit 3)** | -- |
| [553349](https://github.com/NixOS/nixpkgs/pull/553349) | cassandra: structuredAttrs | yes | persistent `TestConfigUnresolved` (function-composed config) | 0 |
| [553770](https://github.com/NixOS/nixpkgs/pull/553770) | portunus: dex redirectURIs | yes | persistent OBA001 (`dex.enable`) | 0 |
| [554779](https://github.com/NixOS/nixpkgs/pull/554779) | os-prober: version bump | yes | persistent `TestConfigUnresolved` | 0 |
| [555805](https://github.com/NixOS/nixpkgs/pull/555805) | silverbullet: version bump | yes | persistent OBA001 (`openFirewall`) | 0 |
| [556710](https://github.com/NixOS/nixpkgs/pull/556710) | fail2ban: socket permissions | yes | persistent clean PASS | 0 |
| [556729](https://github.com/NixOS/nixpkgs/pull/556729) | sstorytime: version bump | yes | persistent OBA001 (default==test value, never flips) | 0 |
| [557545](https://github.com/NixOS/nixpkgs/pull/557545) | mediamtx: yaml version | yes | persistent OBA001 (`allowVideoAccess`) | 0 |
| [558121](https://github.com/NixOS/nixpkgs/pull/558121) | systemd-networkd-vrf test fix | no | pure test-timing fix, no module | -- |
| [558854](https://github.com/NixOS/nixpkgs/pull/558854) | dawarich: version bump | yes | persistent `PredicateNotFound` (`lib.hasPrefix`) | 0 |
| [559055](https://github.com/NixOS/nixpkgs/pull/559055) | opentelemetry-collector: config validation | yes | persistent OBA001 unrelated; **PR's own default-expr rewrite invisible** -- see below | 0 |
| [559588](https://github.com/NixOS/nixpkgs/pull/559588) | netbird-relay: init module | no | new module, no base state | -- |
| [559627](https://github.com/NixOS/nixpkgs/pull/559627) | btrfs autoScrub refactor | yes | real `option_not_found->pass`, **unbucketed** -- see below | 0 |
| [560647](https://github.com/NixOS/nixpkgs/pull/560647) | libp11: version bump | no | plain library, no service module | -- |
| [561557](https://github.com/NixOS/nixpkgs/pull/561557) | modules/image: enable noop fix | yes | real `option_not_found->pass`, **unbucketed** -- see below | 0 |
| [561669](https://github.com/NixOS/nixpkgs/pull/561669) | librespeed: structuredAttrs | no | no NixOS test exists at all | -- |
| [561845](https://github.com/NixOS/nixpkgs/pull/561845) | komodo-periphery: path fixes | yes | persistent clean PASS | 0 |
| [563958](https://github.com/NixOS/nixpkgs/pull/563958) | noctalia-greeter: new option | no | no NixOS test exists at all | -- |
| [564021](https://github.com/NixOS/nixpkgs/pull/564021) | freshrss: extensions test fix | yes | 3 persistent OBA001, correctly not attributed to PR | 0 |
| [564357](https://github.com/NixOS/nixpkgs/pull/564357) | coredns: version bump | yes | persistent clean PASS | 0 |

`*` = a real comparison was genuinely attempted and hard-failed; not
the same as "no" (not attempted at all because nothing to point at).

`applicable_engines.cdc = no` for all 30 -- structurally guaranteed by
the protocol's own exclusion rule (every `CDC_CANDIDATE_NAMES` entry is
already-used and therefore excluded from the population), confirmed
directly per PR by every group, not merely assumed.

## Applicability

```
oba = yes, real comparison produced         18 / 30  (60%)
oba = yes-in-principle, but TOOL_ERROR        2 / 30  ( 7%)
oba = no (genuinely nothing to point at)     10 / 30  (33%)
cdc = yes                                     0 / 30  ( 0%, by construction)
```

The 10 "no" cases split into three real, distinct, honestly-reported
sub-shapes, not one blob: (a) brand-new module init with no test yet
touched in THIS PR either (496303, 519494, 525702, 527821, 559588) --
overlaps with the TOOL_ERROR category below; (b) a real module exists
but genuinely has no NixOS test anywhere in the tree (550960, 561669,
563958); (c) the PR touches neither a real module nor decision-bearing
test logic at all (558121, 560647).

## The most important single finding: TOOL_ERROR on module birth

**2 of 30 PRs (549506 `kener`, 551955 `yace` exporter -- both literally
titled "init module") produced a real `TOOL_ERROR` (CLI exit 3) when
run through `audit-diff`**, not a graceful "not applicable yet" like
group A's four similar-shaped PRs got (which stopped short of
attempting the run once they saw the base-side module was absent).
Confirmed independently by two different forks against two different
real PRs -- not a fluke.

**Why this matters more than an ordinary inconclusive**: per
`audit-diff/action.yml`'s own advisory exit semantics (P3c), CLI exit 2
(a real `INCONCLUSIVE`) makes the Action step *succeed*. CLI exit 3
*fails* the step. A brand-new NixOS service module -- one of the most
common, unremarkable, entirely-legitimate shapes of PR nixpkgs
receives constantly -- would make an installed `audit-diff` Action
**hard-fail the CI check**, reading to a maintainer as "this tool is
broken on my PR," not as the true story ("there is nothing to compare
yet, ask again after this module has a second commit"). At this
sample's own real rate (2/30 ≈ 7%, restricted to the population this
protocol's own eligibility screen already selected for), that is a
real, disqualifying UX defect for shipping an advisory beta as-is.

Root cause (structural, not a bug in `classify_transition_bucket` or
`compare_cdc`/`compare()` themselves): `analyze()` requires the named
`module` file to exist under `--base-root` at all; when it's genuinely
absent (a brand-new module), the whole `audit()` call for that root
errors out rather than the OBA half degrading to a per-target
`AddedSubject`-shaped result the way CDC's own half already does. This
is the concrete, scoped, named follow-up this round recommends --
**not fixed here**, per S1's own freeze.

## Two real, PR-relevant improvements that structurally never reach a maintainer

**559627** (`btrfs autoScrub` refactor) and **561557** (`modules/image`
enable-noop fix) each produced a real `option_not_found -> pass`
transition -- the PR genuinely made a previously-unanalyzable option
into a cleanly-covered one. `classify_transition_bucket`'s own
pre-registered truth table (P3c) deliberately leaves
`Inconclusive -> Pass` unbucketed (not one of the four named headline
buckets), so neither transition ever appears in `notable`, the bounded
step summary, or any of the four headline counters -- only visible by
reading the full JSON artifact directly. This is exactly what P3c's
own protocol said it would do, and the reasoning for it (the four
buckets were chosen to highlight what needs ATTENTION, not everything
that improved) still holds -- but S1 is the first time it was observed
costing something concrete: a real "this PR made things better and
more analyzable" signal that a maintainer skimming only the Action
summary would never see. Worth a real product conversation before P3c
is reopened, not a code change made here.

## One real "resolved_finding" that means something different than it sounds

**492803** (`ntfy: remove redundant user creation`) shows
`resolved_findings=1` -- but the PR doesn't make the test start
covering the `user` option's branch; it *deletes the option entirely*
(`mkRemovedOptionModule`, ntfy-sh moved to `DynamicUser`). The
`Oba001 -> OptionNotFound` transition is exactly what
`classify_transition_bucket` is supposed to report as
`resolved_finding` (a Finding-class kind moving to a non-Finding
class) -- the CLIENT-SIDE arithmetic is correct. But a maintainer
reading "1 resolved finding" in a step summary would reasonably assume
"someone fixed the coverage gap," when what actually happened is "the
option, and the gap along with it, stopped existing." This is a real,
disclosed semantic imprecision in what `resolved_finding` communicates
-- not a bug in the pre-registered classification itself.

## One real detection gap: a PR-introduced default rewrite, invisible

**559055** (`opentelemetry-collector: validate configs built from
settings`) genuinely rewrites the module's own default expression for
`configFile` (`isStorePath cfg.configFile` ->
`cfg.configFile == null || isStorePath cfg.configFile`) -- a real,
PR-introduced semantic change to the option's own default-outcome
logic. `audit-diff` reports the watched option as fully `Unchanged`,
because BOTH the old and new default expressions are non-literal
(`DefaultUnresolved` on both sides, the identical `VerdictKind`).
This is the real, concrete cost of this project's own already-
documented v1 scope limit (`ChangeKind::EvidenceChanged` is never
populated for OBA, only `VerdictChanged` -- see P3b's own doc comment
on `compare()`) actually being paid on a real PR, not a theoretical
gap.

## Recurring inconclusive causes (small, not diffuse, not dominant)

- `PredicateNotFound` via a lambda-parameter-shadowed or
  function-wrapped predicate (`data.listenHTTP` in lego's
  `certToConfig`, `lib.hasPrefix cfg.redis.host` in dawarich): 2
  instances.
- `TestConfigUnresolved` via a function-composed test node config
  (cassandra's `cassandraCfg pkgs ipAddress // extra`, os-prober's
  `let simpleConfig = {...}`): 2 instances.

Both are already-documented, already-understood H1.2-class opacity
limitations (see `README.md`'s own H1 sections) -- S1 found real,
fresh instances of each in the wild, not a new mechanism. Two each, in
a 30-PR sample, is real signal but not yet "many of one type" by the
protocol's own decision-rule bar -- named as debt, not urgent.

## Decision rule, applied

```
any dangerous false PASS?                    NONE found.
several repeating false FINDING (one class)?  NONE -- zero findings
                                               were produced at all, so
                                               trivially none were false.
mostly-correct-but-irrelevant findings?       The 10 persistent findings
                                               ARE this shape by
                                               construction -- but the
                                               tool already correctly
                                               excludes them from
                                               new_findings/notable.
                                               Working as designed, not
                                               a new problem.
many INCONCLUSIVE of one type?                Two small (2-instance)
                                               recurring clusters, named
                                               above -- not "many",
                                               noted as future-adapter
                                               debt, not urgent.
diffuse INCONCLUSIVE?                         Reasonably close --
                                               small, distinct, already-
                                               understood causes. Stay
                                               fail-closed.
```

**A real, repeating, disclosed bug was found that the pre-registered
rule didn't literally name but squarely meets in spirit**: `TOOL_ERROR`
(not a graceful inapplicability) on new-module-init PRs, confirmed
twice independently, with a real, concrete, understood root cause and
a real cost (it would fail CI on a common, legitimate PR shape). Per
the rule's own "a real bug -> fix before beta" clause, this is a
concrete blocker.

## Verdict

**Not ready to ship the advisory beta yet.** One concrete, scoped, real
bug (`TOOL_ERROR` instead of graceful inapplicability on module-birth
PRs) needs a fix first -- a named, authorized-separately follow-up,
not done in this round.

Separately, and just as honestly: **this specific 30-PR sample was too
quiet to test the actionability hypothesis at all.** Zero notable
findings means `correctness precision`/`actionable precision` are
undefined here, not "good" or "bad" -- the real, current answer to
"how often are findings actionable" remains genuinely unknown and
needs either a larger sample or another mechanically-fair draw once
the TOOL_ERROR bug is fixed. What IS now known, with real evidence:
applicability is real (60% of an unbiased, non-cherry-picked eligible
population produced a genuine comparison), runtime is trivially
cheap (single-digit-to-double-digit milliseconds per PR, no real
runtime concern for a PR workflow), the tool stayed conservative
rather than noisy (zero false positives observed anywhere, including
on PRs deliberately picked without any prior tuning), and the
already-known fail-closed inconclusive causes generalize cleanly to
genuinely new real-world instances rather than multiplying into new
ones.

**Next, NOT authorized here, awaiting explicit go**: fix the
`TOOL_ERROR`-on-module-birth bug (treat a base-side-absent module the
same `AddedSubject`-shaped way CDC's own half already does, rather
than erroring the whole `audit()` call), then re-run S1 (or a fresh,
equally mechanical draw) to get an actual, non-degenerate read on
correctness/actionable precision -- that second run is the one that
will finally be able to answer the actionability question this one
could not.
