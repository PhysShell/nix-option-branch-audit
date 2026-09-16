# oba — Option Branch Activation evidence

Layer 1 only, of the three explicitly separated layers agreed on before
writing any code:

```
A. option branch activation    — did a test drive this branch's predicate
   (OBA, this tool)              to a different outcome than its default?
B. consumer contract drift      — does the emitted key name match what
   (CDC, not built)               the pinned consumer actually recognizes?
C. runtime observability        — does the value actually change observed
   (ROB, not built)               behavior, or is it masked by a fallback?
```

`B PASS` does not imply `C PASS`, and this tool does not attempt either —
it answers exactly one question: **did a test ever drive this branch's
predicate to the *opposite* boolean outcome from what the option's own
default produces.** Not "was some non-default value assigned" — a
non-null value that still evaluates the same predicate the same way as the
default (see `c6a` below) proves nothing, and a `PASS` verdict is never
reachable from one. A `PASS` verdict from this tool is *activation
evidence*, not a correctness proof — see the `kimai-after` golden result
below for a real illustration of exactly why that distinction matters.

## What it does

Given a NixOS module and a test file:

1. Finds `mkOption { default = ...; }` declarations nested under any
   `options = { ... };` block anywhere in the module (handles both a
   module's own top-level options and a separately-defined submodule's,
   e.g. kimai's `siteOpts`), and classifies each default's `ValueClass`
   from its actual AST node kind: `Null` (the literal `null`), `Bool(b)`
   (the literal `true`/`false`), `DefinitelyNonNull` (a string, number,
   list, attrset, or path literal — can never be null regardless of
   *what* it contains), or `Unknown` (anything else: an `if`, a select, a
   call, a reference to another binding — genuinely undecidable from
   syntax alone, never guessed at).
2. Finds branch predicates that reference `cfg.<path>` directly (`cfg_ident`
   is configurable) via a fixed, narrow grammar: `!= null`, `== null`,
   bare truthy, `!cfg.foo`, and `lib.{mkIf,optional,optionals,
   optionalString,optionalAttrs} cfg.foo`.
3. Walks the test file's attrset tree (transparently unwrapping
   `containers.<name>` / `nodes.<name>` lambda wrappers, which are
   nixosTest scaffolding, not part of the option namespace) and finds
   every leaf assignment, classifying each value's `ValueClass` the same
   way as the defaults.
4. For each explicitly `watch`ed option, structurally matches test
   assignments against `option_prefix ++ predicate_path` (prefix supports
   a `*` wildcard for `attrsOf`-submodule instance names) and computes the
   predicate's *boolean outcome* for both the default and every matching
   assignment — evidence means a matching assignment's outcome is
   *provably the opposite* of the default's, not merely "a different
   value" (see `c6a`/`c6b` below for why that distinction is load-bearing).

Verdicts come from a 4-gate chain, each one a hard prerequisite for the next
— `PASS` is unreachable unless all four resolved:

```
watch "database.socket"
   │
   ▼
1. mkOption declaration found?          no → OptionNotFound
   ▼ yes
2. direct cfg.<path> predicate found?   no → PredicateNotFound (often a let-alias, out of scope)
   ▼ yes
3. default's ValueClass resolves        no → DefaultUnresolved (default is Unknown under this predicate)
   to a predicate outcome?
   ▼ yes
4. among structurally-matching test assignments:
   any known opposite-outcome value?   yes → PASS, with the opposite-outcome assignment(s) as evidence
   ▼ no
   any assignment's outcome Unknown?   yes → TestValueUnresolved (might be the opposite -- can't tell, don't guess)
   ▼ no
   → OBA001 (every matching assignment's outcome is known, and none of them differ from the default)
```

Every verdict is one of `OptionNotFound` / `PredicateNotFound` /
`DefaultUnresolved` / `TestValueUnresolved` (all **inconclusive** — the
tool couldn't establish an opinion, distinct from and just as loud as a
finding) / `OBA001` (a real finding) / `PASS` (proof of a branch-outcome
transition). Process exit code is 4-state, not 3 — `0` = every watched
option resolved to `PASS`; `1` = `FINDING`, at least one `OBA001` and
nothing inconclusive; `2` = `INCONCLUSIVE`, takes precedence over
`FINDING` (a run that couldn't fully evaluate everything has no business
reporting itself as merely "found some bugs, otherwise clean"); `3` =
`TOOL_ERROR` — the tool itself didn't run (bad manifest, a target's module/
test file doesn't exist, malformed TOML). `2` and `3` look the same from a
shell ("something's wrong") but are different claims to a machine
consumer: "I analyzed this and couldn't prove anything" is not "I never
got to analyze it" — see `h1_1_missing_file_is_tool_error_not_finding`,
which exists because the pre-H1.1 code let exactly this distinction
collapse (any `?`-propagated I/O error fell through to Rust's default
error exit, indistinguishable from a real `FINDING`).

## Non-goals (deliberate)

- **No `let`-bound alias resolution.** `foo = cfg.x; if foo != null then
  ...` is invisible to this MVP. Reported as `PredicateNotFound`, never
  silently absorbed into a `PASS`.
- **No VM tests, no consumer knowledge.** The tool doesn't run anything
  and doesn't know what Doctrine, or any other consumer, is.
- **No claim about runtime behavior.** `PASS` means "a test drove this
  predicate to the opposite outcome from its default", full stop — never
  "the fix works", never "this is safe".
- **No Nix evaluation.** `ValueClass` is a syntactic classifier, not an
  interpreter. `builtins.elem "x" [ "x" "y" ]` is `true` at runtime and
  `Unknown` to this tool, on purpose (see `c11` below) — evaluating
  arbitrary Nix expressions is a different, much bigger tool than this
  one, and pretending otherwise is exactly the kind of overclaiming this
  whole layered design exists to avoid.

## Golden corpus

Real commits, not synthetic fixtures: `PhysShell/nixpkgs` branch
`fix/doctrine-unix-socket-param-name`.

| commit | what it is |
|---|---|
| `5530e24f2` | parent of the fix — the actual, historical nixpkgs bug |
| `37f81efa4` | first fix commit (module fix + v1 regression test) |

```
$ cargo run -- --targets targets/golden.toml
```

| target | expected | actual |
|---|---|---|
| `kimai-before` | `OBA001` on `database.socket` | ✅ |
| `kimai-after` | `PASS` on `database.socket`, evidence from `socketMachine` | ✅ |
| `davis-before` | `OptionNotFound` on `database.driver` | ✅ |
| `davis-after` | `OptionNotFound` on `database.driver` | ✅ |

davis fails at gate 1: it declares options via the flat
`options.services.davis = { ... };` attrpath form, which the (deliberately
narrow) declaration scanner doesn't recognize — see "Real bugs" below. Even
if it did, davis's branch is gated by `mysqlLocal = db.createLocally &&
db.driver == "mysql";`, a `let`-bound alias, not a direct `cfg.foo` select,
so it would fail at gate 2 instead. Either way, `INCONCLUSIVE`, never a
guessed `PASS`: this MVP is honestly telling you it didn't look hard
enough to have an opinion, rather than quietly passing something it never
actually checked.

**Why `kimai-after` matters as a worked example of the OBA/ROB split**:
the v1 regression test added in `37f81efa4` already satisfies OBA (it does
assign a non-default `database.socket`) — but it took two more rounds
(`v2`, `v3`, documented in `nixos/tests/kimai.nix`'s comments on that
branch) to actually *prove* the fix worked end-to-end, because
`pdo_mysql.default_socket` happened to coincidentally match the real
socket path, masking the bug at the ROB layer even with OBA satisfied.
If this tool's `PASS` were read as "the fix is proven", that would have
been wrong for three more commits. It was never claiming that.

## Adversarial mutations

All run against the *fixed* kimai module + a mutated test file, all
expected (and confirmed) to stay `OBA001` — i.e. none of them trick the
matcher into a false `PASS`:

| mutation | what it tests | result |
|---|---|---|
| `c2-remove-scenario` | delete the regression scenario entirely | ✅ `OBA001` |
| `c3-explicit-null` | assign `database.socket = null;` (same as default) | ✅ `OBA001` |
| `c4-wrong-prefix` | assign under `services.notKimai.*` instead of `services.kimai.*` | ✅ `OBA001` |
| `c5-wrong-suffix` | assign a sibling option (`database.host`), never touch `database.socket` | ✅ `OBA001` |

`c3` and `c4`/`c5` specifically rule out "naive grep in an expensive coat":
`c3` proves the matcher checks *value class*, not just *key presence*; `c4`
proves it's structurally path-bound, not a suffix grep; `c5` proves
evidence doesn't leak across sibling options under the same submodule.

`c6` onward (a synthetic module independent of kimai/davis) each pin a
specific fix from the H1/H1.1 review passes below rather than mutating the
kimai corpus further — documented alongside the bug each one demonstrates,
not duplicated here.

## H1 hardening pass

A review of `56bfed8` before extending the corpus found three real
correctness issues — two of them "correct by accident on this specific
corpus", not correct in general. All three fixed, each pinned to a fixture
that specifically demonstrates it (`fixtures/synthetic/`, a minimal module
independent of kimai/davis, isolating the fix from the rest of the corpus):

1. **`PredicateNotFound` (and friends) exited 0.** A detector that
   couldn't evaluate a watched option looked identical to a clean sweep to
   any CI gate keyed off exit code alone — precisely the "detector died,
   green light stayed on" failure mode `watch` was added to prevent in the
   first place. Fixed: three-state exit code (`0` clean / `1` finding /
   `2` inconclusive, inconclusive takes precedence), verified in
   `h1_exit_codes_distinguish_clean_finding_and_inconclusive` against three
   dedicated manifests (`targets/clean.toml`, `targets/findings-only.toml`)
   plus the mixed `golden.toml`.

2. **Default-class membership was textual equality, not predicate-outcome
   transition.** `is_default_class()` compared a test value's raw source
   text against the default's raw source text. For a `!= null` predicate
   with a `null` default (kimai's actual case) that's accidentally
   equivalent to the right answer. For a `!= null` predicate with a
   **non-null** default, it's wrong: assigning the option's own non-null
   default read as "not literally `null`, therefore non-default evidence"
   — a real `PASS` that proves no branch transition happened at all.
   Fixed with `predicate_outcome()`: compute the predicate's boolean
   outcome for both the default and the test value, and only count
   evidence where they *differ*. `c6a-same-as-nonnull-default` /
   `c6b-transitions-from-nonnull-default` are the isolated repro + positive
   control (`h1_outcome_transition_not_textual_equality`). A default whose
   outcome isn't a literal `null`/`true`/`false` — i.e. genuinely
   unclassifiable, not silently guessed either way — now reports
   `DefaultUnresolved` (`c7-unresolved-default`,
   `h1_unresolvable_default_is_inconclusive_not_guessed`).

3. **`PASS` was reachable without the declaration scanner having found
   anything.** `default_source` was an `Option`, quietly `None`-able, and
   nothing gated on it existing. Fixed: declaration lookup is now gate 1,
   hard-required before gate 2 (predicate) is even attempted —
   `c8-option-not-found` watches a path with no `mkOption` at all and
   confirms `OptionNotFound`, not a silent `PASS`
   (`h1_declaration_is_a_mandatory_gate`).

Two smaller items from the same review, also done:

- **Parse errors now fail closed.** rnix is error-tolerant and returns a
  partial tree even on malformed input; previously that partial tree was
  scanned anyway. A target with any parse error now short-circuits to
  `parse_errors` + `INCONCLUSIVE` before touching the (unreliable) tree at
  all — `c9-parse-error`, `h1_parse_errors_fail_closed`.
- **Fixture provenance is now locked, not asserted in a comment.**
  `fixtures/integrity-lock.toml` pins each vendored kimai/davis fixture
  file's sha256; `tests/fixture_integrity.rs` recomputes and compares on
  every `cargo test` run. A fixture silently "cleaned up for convenience"
  six months from now fails loudly instead of the suite staying green
  while "real historical commit" quietly becomes fiction — verified to
  actually catch drift, not just pass trivially, by tampering with a
  fixture and confirming the test fails before reverting. (Renamed from
  "provenance" to "integrity" in H1.1 — see below for why, and for the
  actual provenance check this one doesn't do.)

## H1.1 review fixes

A second review, this time of `1b79cad`, found four more correctness edge
cases (two of them, again, "correct by accident on the corpus tested so
far") plus two smaller holes. All four fixed, each pinned to a dedicated
fixture, all in `fixtures/synthetic/` alongside the H1 ones:

1. **An unresolvable *test* value silently fell through to `OBA001`.**
   Gate 4's filter kept only assignments whose outcome was provably the
   opposite of the default's; an assignment whose outcome was `None`
   (statically unknown) just failed that filter and vanished — read as "no
   evidence" when the honest answer is "can't tell, and this one might
   well have been the evidence". `c11-unresolved-test-value`: a boolean
   option with a *known* `false` default (isolating this from the
   default-side bug below) and a test assignment of
   `builtins.elem "x" [ "x" "y" ]` — `true` at runtime, syntactically
   opaque. Fixed with a proper three-way split per matching assignment
   (known-opposite / known-same / unresolvable) instead of a binary
   filter, with a known opposite taking precedence if *any* matching
   assignment provides one. New verdict: `TestValueUnresolved`.

2. **`predicate_outcome()` for `!= null`/`== null` was still a textual
   check wearing an AST-shaped comment.** The doc comment promised "only
   literal `null`/`true`/`false` are understood"; the implementation did
   `value_text != "null"` and called anything else *definitely* non-null.
   `default = if cond then null else "/run/foo";` or
   `lib.mkDefault null` have no text equal to `"null"` even when their
   runtime value definitely is. `c10-null-expression-default`: an
   `if`-expression default under a `!= null` predicate. Fixed by replacing
   the text check with `ValueClass`, an actual AST-node-kind classifier
   (see "What it does" above) shared between defaults and test values —
   `NODE_STRING`/`NODE_LITERAL`/`NODE_ATTR_SET`/`NODE_LIST`/`NODE_PATH` are
   `DefinitelyNonNull` regardless of content, everything else syntactically
   undecidable is honestly `Unknown`.

3. **An empty `watch = []` silently produced a clean `PASS`.** A target
   that watches nothing checks nothing, and previously exited 0 with zero
   findings and zero inconclusive — the same "detector died, green light
   stayed on" failure mode `watch` exists to prevent in the first place,
   one level up the stack. `targets/tool-error-empty-watch.toml`. Fixed
   with `validate_manifest()`: `watch` (and `target.name` uniqueness,
   `cfg_ident`, `option_prefix`) are now required and validated non-empty
   before any target runs; failure is a tool error (exit 3), not a
   passing analysis.

4. **A missing module/test file, or a malformed manifest, exited as if it
   were a `FINDING`.** `main()` returned `anyhow::Result<()>`, and every
   `?`-propagated I/O or TOML-parse error fell through to Rust's default
   error exit code — typically `1`, the same code this tool defines as
   "genuine `OBA001` finding, nothing inconclusive". A CI script branching
   on exit code alone couldn't tell "found a real bug" from "the manifest
   named a file that doesn't exist". `targets/tool-error-missing-file.toml`.
   Fixed: the exit-code scheme is now 4-state, not 3 (`0`/`1`/`2` as
   before, `3` = `TOOL_ERROR`), with `main()` split into a thin wrapper and
   a `run()` that returns the intended exit code on success and only ever
   reaches `main()`'s `Err` arm for genuine tool failures.

Two smaller items from the same review:

- **`davis-before`/`davis-after`'s golden assertion accepted either
  `OptionNotFound` or `PredicateNotFound`.** Weaker than what H1 actually
  established (exactly `OptionNotFound`, gate 1) — a regression that
  shifted which gate davis fails at would have passed silently. Tightened
  to the exact verdict.
- **The README's opening definition still said "assigned a non-default
  value"**, contradicting its own more careful language two paragraphs
  later. A non-null value that evaluates a `!= null` predicate to the same
  outcome as the default (`c6a`) proves nothing and was never a reachable
  `PASS` — the top-line definition just hadn't caught up. Fixed to state
  the outcome-transition definition consistently throughout.
- **Fixture "provenance" was actually just an integrity lock.** The old
  `tests/provenance.rs` recomputed a local file's sha256 and compared it
  to a neighboring TOML's sha256 field — it never independently checked
  that the claimed `repo`/`commit`/`upstream_path` were true, so a fixture
  and its lock hash could both be edited together and the suite would stay
  green. Renamed honestly to `fixtures/integrity-lock.toml` /
  `tests/fixture_integrity.rs` (still useful, still kept, just named for
  what it actually checks), and added `scripts/verify-upstream.sh` — an
  on-demand script (not part of `cargo test`, no network by default) that
  re-derives each fixture's hash directly from a local nixpkgs checkout via
  `git show <commit>:<path> | sha256sum` and compares against both the
  lock file and the local fixture, closing the actual provenance loop.
  Verified working both directions against the real fork checkout used to
  build this corpus (all 8 fixtures OK; then deliberately corrupted one
  locked hash and confirmed it fails, before reverting).

## Running

```
cargo test    # the full acceptance suite (tests/golden.rs, tests/fixture_integrity.rs)
cargo run -- --targets targets/golden.toml [--json]

# on-demand, needs a local nixpkgs checkout, not run by cargo test:
scripts/verify-upstream.sh /path/to/nixpkgs-checkout
```

## Acceptance criteria (as specified, all met)

- [x] A. reproduces the historical finding on the exact parent commit
- [x] B. clears it on the exact fix commit, with evidence attached
- [x] C. survives 12 adversarial/synthetic cases: `c2`–`c5` (original 4
      kimai mutations), `c6a`/`c6b` (non-null-default false-positive + its
      positive control), `c7` (unresolvable boolean default), `c8`
      (missing declaration), `c9` (parse-error fail-closed), `c10`
      (non-literal null-predicate default, AST- vs text-classified), `c11`
      (unresolvable test value), `c12`/`c13` (empty watch / missing file,
      both tool errors)
- [x] D. produces source spans + evidence (file:line:col, matched assignment)
- [x] E. does not invoke VM tests
- [x] F. does not know anything about Doctrine

`cargo test` — 15 tests, all passing: 5 from the original spike, 5 from H1
(exit codes, parse-errors-fail-closed, outcome-transition +
positive-control, unresolvable-default, mandatory-declaration-gate), 4
from H1.1 (unresolved-test-value, AST-classified null-predicate default,
empty-watch-is-tool-error, missing-file-is-tool-error), 1 fixture
integrity lock.

## Real bugs this spike itself found in its own implementation

Left in as evidence the golden-test harness is a real backstop, not
theater — both were caught by `kimai-after` unexpectedly showing `OBA001`
instead of the expected `PASS` during development, not by manual review:

1. `NODE_ROOT` vs its child `NODE_LAMBDA`: `test_root.syntax()` returns the
   root wrapper node, not the lambda directly — `unwrap_lambda_chain`
   silently did nothing until it handled `NODE_ROOT` explicitly.
2. `containers.<name>` / `nodes.<name>` segments were being folded into the
   matched dotted path, so every test assignment's path carried a
   `containers.X.` prefix that could never match a real option's absolute
   path — zero test assignments ever matched anything, regardless of the
   fixture.

Documented (not fixed, on purpose) rather than papered over: davis.nix's
`options.services.davis = { ... };` flat-dotted-attrpath form isn't
recognized by the option-declaration scanner (see the comment at its call
site) — fixing it naively would produce option paths inconsistent with how
the predicate scanner reports paths for a `cfg` bound below the top level,
which is the same class of problem as the `mysqlLocal` alias gap.

## Explicit follow-ups (deliberately not this pass)

Per the stated order: harden Layer 1 to a place it's actually trustworthy
*before* extending it, not alongside. In particular, **alias resolution is
next but wasn't touched in H1 on purpose** — expanding `foo = cfg.x; if
foo then ...` before the outcome-transition semantics above were nailed
down would have meant building on the wrong foundation. It also reveals
that `Option → Predicate` is too simple a model the moment it's attempted
for real: davis's actual gate is `mysqlLocal = db.createLocally &&
db.driver == "mysql"`, a conjunction over *two* options, which means
evidence has to be proven as a conjunction inside a single node/container
instance, not as two independently-satisfied option assignments that
happen to appear anywhere in the test file. `TestAssignment.instance`
already exists for diagnostics; it becomes semantically load-bearing the
moment conjunctions are supported.

- **H2**: a small predicate IR (`Null`, `Bool`, `Eq literal`, `Not`, `And`
  / `Or`) plus local immutable `let`-alias expansion — unblocks davis's
  real predicate.
- **Davis golden**: `mysqlLocal` resolves, and evidence is required to
  satisfy the full conjunction within one `nodes.machine3`-style instance,
  not scattered across unrelated nodes.
- flat-dotted `options.a.b.c = { ... };` recognition, done consistently
  with `cfg_ident` scope resolution (see the H1-era comment at
  `scan_options`'s call site for why a naive fix here was reverted rather
  than shipped half-right).
- CDC (consumer contract drift) and ROB (runtime observability) as
  separate tools/layers, per the explicit non-conflation this spec insisted
  on from the start.
- Only *after* the davis golden lands: a small real census (20–30 web-app
  modules), not before — numbers without knowing what they mean yet aren't
  worth collecting.
