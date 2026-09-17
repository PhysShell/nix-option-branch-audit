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
3. Walks the test file as a small explicit state machine, not one function
   accumulating special cases (H1.3 review) — `TestSpecRoot` (finds
   `nodes`/`containers`, both the flat `nodes.foo = ...;` and nested
   `nodes = { foo = ...; };` forms, and recognizes the
   `import ./make-test-python.nix (...)` wrapper) → `ModuleRoot(instance)`
   (where `imports` and `config = {...}` are meaningfully special,
   *because* it's a module root — the same keys one level deeper are
   ordinary option data) → `ConfigTree(instance)` (ordinary recursive
   descent, reached identically from module-root shorthand or an explicit
   `config` block, so both normalize to the same option-path namespace).
   Every point the walker can't see into — `imports`, an instance/`config`
   value that isn't a literal attrset, `inherit`, a dynamic `${...}` key,
   an unrecognized root wrapper, a spec root with no recognized
   `nodes`/`containers` at all — leaves an explicit `Opacity` record
   rather than silently producing nothing.
4. For each explicitly `watch`ed option, structurally matches test
   assignments against `option_prefix ++ predicate_path` (prefix supports
   a `*` wildcard for `attrsOf`-submodule instance names) and computes the
   predicate's *boolean outcome* for both the default and every matching
   assignment — evidence means a matching assignment's outcome is
   *provably the opposite* of the default's, not merely "a different
   value" (see `c6a`/`c6b` below for why that distinction is load-bearing).

Verdicts come from a 6-step gate chain, each one a hard prerequisite for
the next — `PASS` is unreachable unless it resolves first:

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
4. any known opposite-outcome         yes → PASS, with the opposite-outcome assignment(s) as evidence
   test assignment?                       (wins even if 5/6 below would also fire -- see c17)
   ▼ no
5. does any part of the test config     yes → TestConfigUnresolved (imports/alias/function call
   that could structurally contain            hid a scope that might have set this option)
   this option remain unseen by
   the walker (imports, an alias,
   a function call)?
   ▼ no
6. any matching assignment's          yes → TestValueUnresolved (might be the opposite -- can't
   outcome Unknown?                        tell, don't guess)
   ▼ no
   → OBA001 (every matching assignment's outcome is known, and none of them differ from the default)
```

Every verdict is one of `OptionNotFound` / `PredicateNotFound` /
`DefaultUnresolved` / `TestConfigUnresolved` / `TestValueUnresolved` (all
**inconclusive** — the tool couldn't establish an opinion, distinct from
and just as loud as a finding) / `OBA001` (a real finding) / `PASS` (proof
of a branch-outcome transition). Process exit code is 4-state, not 3 —
`0` = every watched option resolved to `PASS`; `1` = `FINDING`, at least
one `OBA001` and nothing inconclusive; `2` = `INCONCLUSIVE`, takes
precedence over `FINDING` (a run that couldn't fully evaluate everything
has no business reporting itself as merely "found some bugs, otherwise
clean"); `3` = `TOOL_ERROR` — the tool itself didn't run (bad manifest, a
target's module/test file doesn't exist, malformed TOML, a bad CLI flag).
`2` and `3` look the same from a shell ("something's wrong") but are
different claims to a machine consumer: "I analyzed this and couldn't
prove anything" is not "I never got to analyze it" — see
`h1_1_missing_file_is_tool_error_not_finding` and
`h1_2_cli_parse_failure_is_tool_error_not_inconclusive`, which exist
because the pre-H1.1/pre-H1.2 code let exactly this distinction collapse
twice, in two different places (a bubbled I/O error, and `clap`'s own
`Error::exit()` running before this tool's exit-code contract even
started applying).

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

## H1.2 review fixes

A third review found the sharpest gap yet -- the kind the whole project
exists to catch -- plus a CLI-level exit-code leak and a `verify-upstream.sh`
bug:

- **P0 -- `OBA001` didn't actually prove absence of activation evidence,
  only absence of *observed* evidence.** The test-file walker only ever
  descends into literal `NODE_ATTR_SET` values. A perfectly ordinary
  nixosTest pattern --
  ```nix
  nodes.machine = { ... }: { imports = [ ./common.nix ]; };
  # or:
  nodes.machine = machineConfig;  # a local `let`-bound alias
  ```
  -- makes the relevant assignment live somewhere the walker structurally
  cannot see, and the old code just silently found nothing and reported
  `OBA001`: "branch never exercised", when the honest claim is "not
  observed in the part of the config I can read". Those are different
  facts, and conflating them was exactly the failure mode this tool exists
  to name in *other* tools. Fixed with a new `Opacity` record: every point
  the walker gives up (an `imports` key at any nesting level; an instance
  whose config isn't a literal attrset; any non-attrset value that could
  structurally contain more nested options -- classified by AST kind, same
  approach as `ValueClass`) is tracked with its scope. Gate 4 (now the
  full 6-step chain above) checks, after opposite-outcome evidence and
  before per-value ambiguity: does any opacity site's scope structurally
  contain the watched option's absolute path? If so: `TestConfigUnresolved`,
  not `OBA001`. Crucially, **existential positive evidence still wins**:
  an explicit, provable transition found anywhere in the file outranks an
  *unrelated* opacity site elsewhere (`c17` — a real transition in one
  node, an unrelated `imports` in a different node, must still be `PASS`).
  Without this fix, a census over real modules wouldn't find "uncovered
  branches" -- it would find a mix of genuinely uncovered branches and
  "config arrived via `imports` and the walker never saw it", indistinguishable
  in the output. `c15-imports`, `c16-instance-alias`, `c17-opposite-wins-over-import`.

  Building `c16` caught a second, independent bug live: `let machineConfig
  = { ... }; in { nodes.machine = machineConfig; }` is a completely
  ordinary nixosTest idiom, and the walker never even started for it --
  `unwrap_lambda_chain` unwrapped `NODE_ROOT` and `NODE_LAMBDA` but not
  `NODE_LET_IN`, so a file whose top level is `let ... in { ... }` (rather
  than a bare lambda-to-attrset) silently produced zero assignments *and*
  zero opacity records, not because the fixture was well-covered but
  because the scanner never ran on it at all. Fixed by teaching
  `unwrap_lambda_chain` the same "last child is the continuation" pattern
  for `NODE_LET_IN` that it already used for `NODE_LAMBDA`.

- **P1 -- a bad CLI invocation bypassed the exit-code contract entirely.**
  `Cli::parse()` calls `clap`'s own `Error::exit()` internally on a bad
  flag or a missing required argument, which prints and terminates the
  process with *clap's* code (2 for a genuine usage error) before this
  tool's `run()` -- and its whole 0/1/2/3 contract -- ever starts. `oba
  --bogus-flag` exited 2, indistinguishable from a real `INCONCLUSIVE`
  analysis result, despite no analysis ever running. Fixed with
  `Cli::try_parse()`, mapping a genuine parse error onto `TOOL_ERROR` (3)
  explicitly, while still letting `--help`/`--version` exit 0 as clap
  intends. `h1_2_cli_parse_failure_is_tool_error_not_inconclusive`.

- **P2 -- `verify-upstream.sh` rejected valid git worktrees.** It checked
  `[ -d "$REPO_CHECKOUT/.git" ]`; a linked worktree's `.git` is a *file*
  containing `gitdir: ...`, not a directory, so a perfectly valid worktree
  checkout would be wrongly rejected. Fixed by asking git itself (`git -C
  "$REPO_CHECKOUT" rev-parse --git-dir`) instead of guessing about
  on-disk layout. Verified directly: the old check rejects a real `git
  worktree add` output, the new one accepts it. Also softened the header
  comment's claim -- the script proves the commit/path/hash triple, but
  never independently validates that the passed-in checkout actually *is*
  the named `repo` (any local checkout with the right commit reachable
  will do); not a problem for this corpus, but `repo` is informational
  metadata, not part of the proof, and the comment now says so.

Two smaller items from the same review, folded into `validate_manifest`:
`option_prefix`/`watch` entries with an empty segment (`option_prefix =
["services", "", "foo"]`, `watch = ["database..socket"]`) and a
non-identifier `cfg_ident` used to reach analysis and surface as a
confusing `OptionNotFound`/`PredicateNotFound` instead of the manifest
error they actually are -- now rejected before any target runs, same as
every other manifest-shape problem.

## H1.3 review fixes + syntax-visibility census

A fourth review found the sharpest gap across all four rounds, of exactly
the kind this project exists to catch -- and, for the first time, backed
it with a real corpus rather than a plausible scenario: **`nodes = {
machine = ...; };` (the nested form) made the real assignment invisible to
the walker**, which only ever recognized the flat `nodes.machine = ...;`
form. Not an edge case: 559 files under `nixos/tests` use `nodes = {`.
`nixos/tests/ifm.nix` is one of them -- a real, unmodified, currently
upstream nixosTest, now vendored at `fixtures/real/ifm-test.nix` with a
scanner-level unit test (`tests::scanner_reads_real_ifm_test_correctly`)
proving the walker finds its real `services.ifm.{enable,port,dataDir}`
assignments and flags its dynamic `${config.services.ifm.dataDir}.d` key
as opacity, not silence.

This forced the walker's actual redesign (not another `if` bolted onto
the old one): three explicit contexts --`TestSpecRoot` /
`ModuleRoot(instance)` / `ConfigTree(instance)` -- described in "What it
does" above and in the comment above `scan_test_assignments`. Each fixed
gap is pinned to its own `fixtures/synthetic/` case:

| gap | fixture | must produce |
|---|---|---|
| nested `nodes = { machine = ...; };` form, opposite evidence | `c18` | `PASS` |
| nested form, default-matching only | `c19` | `OBA001` |
| entirely unrecognized root wrapper | `c20` | `TestConfigUnresolved` |
| `import ./make-test-python.nix (...)` wrapper | `c21` | `PASS` (unwrapped, real evidence found) |
| module-root `config = { ... };` | `c22` | `PASS` (path normalized, not `config.*`-prefixed) |
| module-root `config = <opaque expr>;` | `c23` | `TestConfigUnresolved` |
| `inherit` / a dynamic `${...}` attribute name | `c24` | `TestConfigUnresolved`, both recorded |

`config` is intercepted only in `ModuleRoot` context specifically because
the same key one level deeper (e.g. a genuine `containers.peer.config`
*option*) is ordinary option data, not module syntax -- conflating them
with a blanket "key named config" check anywhere in the tree would have
been wrong in the other direction. Two bugs were caught live while
building these fixtures, not by the reviewer: `c16`'s `let machineConfig =
{...}; in { nodes.machine = machineConfig; }` never started walking at all
(`unwrap_lambda_chain` didn't handle `NODE_LET_IN` as a root form), and
several pre-H1.3 synthetic fixtures (`c6a`/`c6b`/`c11`) put their
assignments directly at the test file's top level with no `nodes`/
`containers` wrapper at all -- a fixture-authoring shortcut that real
nixosTest files never take, and which the new `TestSpecRoot` context
correctly stopped recognizing; fixed by making the fixtures realistic
instead of loosening the walker back down to accept them.

### Syntax-visibility census

Per the review's own proposed closing move: not another round of "invent
five synthetic cases, find a sixth on review," but a cheap, rnix-only pass
over a real corpus -- no VM, no Nix evaluation -- checking one invariant:
**every syntactic form either becomes a known `TestAssignment` or leaves
an explicit `Opacity`; there is no third "silently continue" state for
anything that could be option-relevant.** Built as `oba --census <dir>`
(runs the test-file walker standalone, no module/option/predicate matching
at all) and run against `nixos/tests` in the same nixpkgs checkout used
for the golden corpus:

```
files scanned:             1609
parse errors:               0
root: direct attrset:      1389
root: import-wrapper:        70
root: opaque/unrecognized:  150
total assignments found:  11351
total opacity sites:       2488
files with any opacity:    1156
files with NEITHER:          14
```

"Files with NEITHER" (no assignment *and* no opacity -- the bucket that
would mean the invariant is actually broken) started at **149**. Spot
checks of the first several -- `atop.nix` (`{ justThePackage = runTest {
nodes.machine = ...; ...}; defaults = runTest { ... }; ... }`, multiple
independent scenarios keyed by name) and `agnos.nix` (a single `<name> =
makeTest { nodes = ...; };` wrapper) -- surfaced a real, previously
unanticipated convention: **test files with no top-level `nodes`/
`containers` at all**, because the whole spec lives one level down inside
one or more named `runTest`/`makeTest` calls. Rather than chase every
test-builder-function name (open-ended, out of scope for this pass),
`walk_test_spec_root` now records one whole-file opacity whenever it finds
zero recognized `nodes`/`containers` bindings anywhere at the spec root,
instead of silence. That took the bucket from 149 to **14** -- and every
one of the remaining 14 was manually confirmed to be a real, legitimate
case: `nodes.machine = { };` (or `{ ... }: { };`), an intentionally empty
node config with nothing to find, not a walker gap (`simple-vm.nix`,
`systemd-no-tainted.nix`, `kbd-setfont-decompress.nix`, and 11 others,
all the same shape).

The remaining 150 `root: opaque/unrecognized` files and the long tail of
per-file opacity reasons (`imports`: 307, non-literal instance/value: 203,
`inherit`: 126+2, dynamic keys: 67+35+5) are exactly what the invariant
promises: known, named, non-silent gaps -- not claimed to be resolved by
this pass, and not required to be. "They don't have to be supported. They
have to not stay silent" was the explicit bar for closing this round, not
"support 100% of nixpkgs's test syntax."

## H1.3a review fixes

A fifth review pass, against `ef96244` (H1.3 + the first census run). The
walker redesign itself held up; the gap was in `TestSpecRoot`'s own
closing assumption. `walk_test_spec_root` treated "not `nodes`/
`containers`" as proof of "harmless harness metadata" — true for `name`/
`meta`/`testScript`, but never actually checked against anything, so a real
second scenario sitting at the same level (`hiddenScenario = runTest {
nodes.other = { ... }: { services.synth.foo = null; }; };`, right next to
an ordinary, fully-visible `nodes.machine`) was silently discarded exactly
like metadata — and because `nodes.machine` *was* found, the old
whole-file fallback (which only fired when literally nothing was
recognized) never fired either. Result: a real opposite-outcome assignment
existed in the file, entirely invisible to the walker, with nothing in the
report — no opacity, no assignment — hinting it was ever there. `sddm.nix`
(named `default = runTest {...}; autoLogin = runTest {...};` scenarios,
no root-level `nodes` at all) confirmed the shape is real, not contrived.

Fixed by turning the assumption into an actual check. Every top-level
test-spec-root key is now classified against `KNOWN_HARNESS_METADATA_KEYS`
— a small allowlist built by reading the real schema
(`nixos/lib/testing/{driver,meta,testScript,name,run}.nix` in nixpkgs),
not guessed: only keys structurally incapable of carrying NixOS module
config (`name`, `meta`, `testScript`, `enableOCR`, `skipLint`,
`skipTypeCheck`, `globalTimeout`, `hostPkgs`, `passthru`, ...) are on it.
Reading the schema turned up two keys that looked like plausible metadata
and are not: `defaults`/`nodeDefaults`/`containerDefaults`/
`extraBaseModules`/`extraBaseNodeModules` are documented as "NixOS
configuration applied to all nodes" (real option data, deliberately kept
off the allowlist), and `interactive` is documented to accept
`interactive.nodes.<x> = { ... }` overrides (`nixos/lib/testing/
interactive.nix`'s own doc example uses exactly that shape) — also kept
off. Anything not on the allowlist, including the legacy `machine = { ...
}: { ... };` single-node shorthand (a real, separate key from `nodes`,
confirmed in `nixos/lib/testing/legacy.nix`), now gets its own `Opacity`
record (`REASON_UNCLASSIFIED_ROOT_ENTRY`) instead of silent disposal.
Pinned by `c25-partial-root-opacity`: a normal `nodes.machine` with no
opposite evidence, next to a `hiddenScenario` the walker can't see into —
must be `TestConfigUnresolved`, not `OBA001`.

The census gained the metric this round's whole complaint was actually
about. `files_with_neither` (no assignment *and* no opacity) is a weaker
claim than it looks — a file can have 200 correctly-found assignments and
one fully-invisible sibling scenario and still land in neither bucket. The
census now also reports `unclassified_root_entries`
(`unclassified_root_entries >= 1` for the c25 pattern specifically), and
distinguishes "genuinely can't read a file" from "read fine, nothing
there": `unreadable_files` is now tracked and reported separately, and the
census's own exit code stopped being a hardcoded `0` regardless of what it
found — `unreadable_files > 0` is now `TOOL_ERROR` (3, the corpus wasn't
fully covered at all) and `parse_errors > 0` is `2` (covered, but not all
analyzable), matching the same fail-closed discipline `run()` already
applied to a real analysis run. `--targets` and `--census` are now also
genuinely mutually exclusive (`conflicts_with` in clap) — previously
passing both silently ran the census and ignored `--targets` instead of
rejecting the combination.

Re-run against the same `nixos/tests` checkout (1609 files, `--census`):

```
files scanned:             1609
unreadable files:             0
parse errors:                 0
root: direct attrset:      1389
root: import-wrapper:        70
root: opaque/unrecognized:  150
total assignments found:  11351
total opacity sites:       4277
files with any opacity:    1181
files with NEITHER (weaker bucket): 12
unclassified root entries (option-relevant syntax that could have been silently discarded): 1903
```

`files_with_neither` moved from 14 to 12 (two of the previous 14 turned
out to have an unrecognized root entry alongside their empty `nodes`, so
they now correctly land in "any opacity" instead). The number that
actually matters is the new one: **1903 unclassified root entries** across
a corpus that previously reported this bucket as invisible — the
`unrecognized test-spec-root entry` reason alone accounts for all 1903 of
them; a rough independent check (`grep -rlE '^\s*machine\s*='`) finds 119
files using the `machine =` shorthand alone, confirming this is a real,
sizeable, previously-silent bucket and not a rounding artifact. This pass
does not resolve any of it — `machine`, `interactive.nodes.*`, and
whatever else lives in the other ~1780 entries are exactly the deferred
"they don't have to be supported, they have to not stay silent" cases the
H1.3 review closed on; they're now visible and counted for the first time,
which was the actual bar for this round, not resolving them.

## H1.3b review fixes

A sixth review pass, against `bf48d25` (H1.3a's root-entry allowlist +
census fixes). Same class of bug, one level down: the NESTED `nodes = {
...
};` form's own inner loop (over each instance binding inside the
attrset) only ever handled `inst_segs.len() == 1` (a plain instance name,
`enter_instance`'d) and a dynamic `${...}` instance name — anything else,
including a multi-segment attrpath directly under `nodes = { ... };`
(`hidden.services.synth.foo = null;`, sugar for a second, malformed
"instance") or an `inherit`, fell straight through the loop with neither
an assignment nor an opacity record. Worse than the H1.3a root-level
version of the same bug: because the normal sibling instance (`machine`)
*was* found and recognized, this was invisible to every metric that
existed at the time, including the brand-new `unclassified_root_entries`
— the unrecognized entry is already inside a value H1.3a's allowlist
check correctly accepted as `nodes = { ... };`, so it never even reaches
that check. Fixed the same way as H1.3a: `NODE_INHERIT` under nested
`nodes`/`containers` now produces opacity, and an `inst_segs.len() != 1`
entry produces opacity instead of silently continuing past the loop
iteration. Pinned by two new fixtures, `c26-nested-instance-multisegment`
and `c27-nested-instance-inherit`, both a normal `machine` (no opposite
evidence) next to the unrecognized entry — must be `TestConfigUnresolved`.
Census gained the matching counter, `unclassified_instance_entries`,
reported and JSON-tested the same way as its root-level sibling.

As a closing mechanical check (not a new architectural review), every
`continue` in all four walker contexts (`TestSpecRoot`'s two loops,
`ModuleRoot`, `ConfigTree`/`walk_config_entry`) was re-read against one
question: does this path skip an AST node kind that could carry NixOS
module config without leaving an `Opacity`? Every remaining silent
`continue` is now either a `NODE_ATTRPATH_VALUE`/`NODE_INHERIT` kind guard
(no third child kind is reachable from a cleanly-parsed `NODE_ATTR_SET`)
or a defensive `children().next()` `None` case that's structurally
unreachable once rnix has parsed the node without errors (a
`NODE_ATTRPATH_VALUE` always has both an attrpath and a value child by
grammar) — not a live gap.

Re-run against the same 1609-file `nixos/tests` checkout:

```
files with NEITHER (weaker bucket): 10   (was 12)
unclassified root entries:        1903   (unchanged -- different bug)
unclassified instance entries:       4
```

Small in absolute count (4 multi-segment-attrpath sites, plus 7 more
caught by the same fix's `inherit`-under-nested-`nodes` case, both new
`opacity_reason_counts` entries), but real, and exactly the shape that
matters most: each one sits next to a normal, already-recognized sibling
instance, which is precisely the condition under which every *other*
gap-detection mechanism in this tool (both `files_with_neither` and
`unclassified_root_entries`) stays blind.

## H1: FROZEN at `fa953d6`

Six review rounds (H1 → H1.3b), each finding a real bug, closed the walker's
one governing invariant on the supported syntactic subset: every construct
either resolves to a known `TestAssignment` or leaves an explicit
`Opacity` — never silence. `AGENTS.md`'s closing line applies here
directly: *"A frozen layer is reopened only by a concrete counterexample,
invalidated assumption, or downstream requirement, not for naming cleanup
or speculative completeness."* The ~1900 unsupported syntactic forms the
census counts (`machine =` shorthand, `interactive.nodes.*`, ...) are not
a debt against this freeze — they were never required to be understood,
only required to never silently disappear, and the census proves they
don't.

## H2 (in progress) — commit 1: reuse survey, gate-1 fix, pure Predicate IR

H1's declaration/predicate model has one governing assumption baked in:
a predicate is a unary check of exactly one option's value
(`ValueClass` → `bool`). Real modules break this. `davis.nix`'s actual
MySQL branch is gated by `mysqlLocal = db.createLocally && db.driver ==
"mysql";` — a `let`-bound alias to a *compound* boolean expression over
*two* options, referenced elsewhere as a bare condition
(`if mysqlLocal then ...`). No amount of hardening the H1 walker touches
this; it needs a different predicate model. H2 builds that model, staged
across two commits per an explicit reviewer request to keep failures
attributable rather than landing one large change.

**Reuse survey** (required by `AGENTS.md` before writing any of the code
below; the full writeup is a doc comment directly above the IR in
`src/main.rs`, not duplicated here). Checked and independently confirmed
this session: rnix 0.11's typed `ast` module
(`rnix::ast::{BinOp, UnaryOp, BinOpKind, UnaryOpKind}`) already classifies
`&&`/`||`/`==`/`!=`/`!` into a clean enum via `.operator()`, read directly
from the vendored crate source rather than assumed — used going forward
in the new lowering code, zero new dependencies. Checked and kept as
design reference only (not vendored): `oxalica/nil`'s name-resolution
(full lexical scoping, but embedded in its own Salsa/IDE architecture —
importing the algorithm would mean importing most of the crate graph
around it), `nixd`/`libnixf` (a C++ stack over the real Nix evaluator, not
a Rust dependency at all), and Tvix (a real evaluator with its own scope
tracking — noted as a possible future *differential oracle*, not
something to embed in the analysis core). The actual project-specific gap
no reused artifact fills: a small, non-evaluating, fail-closed resolver
for exactly the alias shapes real nixpkgs modules use — landing in the
next commit, not this one.

**Gate-1 fix.** `scan_options` only recognized the nested `options = {
...};` form (kimai's shape); davis's real, flat `options.services.davis =
{ ...};` root made `database.driver` permanently undiscoverable, so
`davis-before`/`davis-after` failed at gate 1 (`OptionNotFound`) before
ever reaching the alias gap. Fixed generically against the *target's own*
`option_prefix` from the manifest — never hardcoded to davis or any other
module: if the segments after a leading `options` exactly match
`option_prefix` (only possible when `option_prefix` is fully concrete; a
wildcard `"*"` can never appear literally in a module's own static
declaration path, so wildcarded targets like kimai's simply fall through
to the unchanged nested-form branch). `davis-before`/`davis-after` now
correctly reach gate 2 and fail there instead
(`PredicateNotFound` — the honest, next gap, not a false pass).

**Pure Predicate IR.** A `Pred`/`ValueExpr`/`Scalar` type, independent of
H1's `PredicateKind` (left untouched — H1 is frozen): `Pred::{Eq, Not,
And, Or}` over `ValueExpr::{Ref(OptionPath), Literal(Scalar)}`. Two pure
functions — `lower_pred`/`lower_value_expr` (AST → `Option<Pred>` /
`Option<ValueExpr>`, `None` for anything unrepresentable, never a guess)
and `eval_pred`/`eval_value_expr` (`Pred` + a concrete environment →
`Option<bool>`/`Option<Scalar>`, `And`/`Or` deliberately do **not**
short-circuit on a known operand — an unresolved operand means this
static pass genuinely doesn't know whether real Nix evaluation would have
forced it, so it can't claim to know the combined outcome either). Not
yet wired into `run_target`'s actual gate 2–4 logic; that wiring, together
with alias resolution, is the next commit.

Verified with 10 unit tests: direct-form lowering (`!=`, `==`, bare
truthy, `!`, `&&`, `||`) matches H1's shapes exactly; compound forms
(`db.createLocally && db.driver == "mysql"`, without alias resolution yet)
lower correctly; unsupported shapes (`builtins.elem ...`, a bare alias
ident used as a condition, `<`) are `None`, not a guess; two example-based
compatibility tests reproduce H1's entire `predicate_outcome()` table for
`NullNeq`/`NullEq`/`Truthy`/`NegTruthy` exactly via the new evaluator
(one theoretical corner — `Truthy` combined with `ValueClass::
DefinitelyNonNull` — is explicitly *not* reproduced and documented why: no
real predicate + value combination in the golden suite ever reaches it,
since a `Truthy` predicate's operand must be a real Nix bool for
evaluation to succeed at all); and four `proptest` property tests —
`Not(Not(p)) == p`, `And(p, true) == p`, `Or(p, false) == p`, and the
one that matters most going into the next commit: deleting any single
entry from an environment that fully resolves `p` never flips a known
result to a *different* known result, only ever weakens it to `None`.
`proptest` added as a dev-dependency per `AGENTS.md`'s reuse-first rule
applied to the test infrastructure itself — the maintained, standard Rust
property-testing crate, not a hand-rolled generator loop mislabeled as
one.

44 tests total (33 in `tests/golden.rs`, 1 in `tests/fixture_integrity.rs`,
10 in `main.rs`'s own unit-test module) — was 34 at the H1.3b freeze: +1
golden (`h2_gate1_davis_flat_option_root_is_discovered`) and +9 new unit
tests for the pure IR (`scanner_reads_real_ifm_test_correctly` was already
counted in H1.3's tally).

## H2 — Kleene refinement + lexical alias resolver (not yet wired into run_target)

Two follow-up steps on commit 1, requested on review before the
counterfactual gate-4 rewrite:

**`eval_pred`'s `And`/`Or` refined to strong (Kleene) three-valued
logic.** The original version required *both* operands to resolve,
reasoning that was the safer fail-closed choice — that reasoning was
wrong, not just cautious. A known-`false` operand pins `And`'s result to
`false` regardless of the *other* operand's resolution: real Nix's `a &&
b` either short-circuits on a `false` `a` without forcing `b` at all, or
forces `b` and gets exactly the value already known statically — both
paths land on `false`. Symmetrically for a known-`true` operand under
`Or`. Verified with both operand orderings (the known value can be on
either side) and by re-running all four existing `proptest` properties
unchanged afterward — including the monotonicity property this refinement
could plausibly have threatened.

**A narrow lexical alias resolver**, reference-resolution based (walks
the existing tree, never rewrites or re-parses text): `resolve_ident_binding`
walks outward from a use site through enclosing `let`/lambda scopes,
nearest-binding-wins, stopping at the first scope that defines the name.
`lower_value_expr`/`lower_pred` switched from `Option<_>` to
`Result<_, ResolveFailure>` (`Unbound`/`Cycle`/`UnsupportedScope`/
`UnsupportedExpression`) — a bare `None` was fine while the IR was only
unit-tested in isolation, but once this feeds the real verdict chain, "no
predicate" and "a real alias this resolver can't trace" are different
facts a report reader needs to tell apart (the same reason `Opacity`
carries a `reason` string instead of being a bare marker). Select aliases
(`db = cfg.database;`) splice the remaining path onto the resolved base;
expression aliases used bare as a condition (`if mysqlLocal then ...`)
resolve and lower the *bound* expression as a `Pred` directly, not as a
value reference — the two alias shapes are distinguished by what they
resolve to, never guessed from spelling. `with`, `inherit`, function
parameters, and a `rec` attrset's internal mutual visibility are each
explicitly `UnsupportedScope`, not silently skipped.

Also added, per explicit review request: a safety gate on the gate-1 flat-
option-root fix. Before this, a manifest could claim `cfg_ident =
"otherCfg"` with `option_prefix = ["services","davis"]` and have
declarations from one scope silently correlated with predicates from a
completely different one — a manifest-induced false correlation, not a
tool bug exactly, but not something this tool should be able to produce
either. `cfg_ident_binds_to_prefix` checks that the module's own
`cfg_ident = config.<path>;` binding actually matches `option_prefix`
before the flat-root walk proceeds.

Eight new adversarial resolver tests, not just the one happy Davis
path: multi-hop select alias (`a = cfg.database; b = a;` resolving
through both hops), a cyclic alias (`a = b; b = a;`, must fail as `Cycle`
not a stack overflow), nested `let` shadowing (nearest binding wins),
lambda-parameter shadowing, a `@`-pattern-bind shadowing, a
function-produced alias (`db = someFunction cfg.database;`, must stay
unresolved, never assumed to equal its argument), and both directions of
the `cfg_ident`/`option_prefix` safety gate (matches → declaration found;
mismatch → correlation refused). **One of these caught a real bug before
it ever reached a golden fixture**: the lambda-shadowing test initially
failed — `resolve_ident_binding` checked for a bare `NODE_IDENT` lambda
parameter, but rnix's actual AST wraps a simple `x:` parameter in
`NODE_IDENT_PARAM` (confirmed with a throwaway AST-dump probe test, not
assumed from the type name), so `x` inside a lambda body was silently
resolving *past* the lambda's own parameter to an outer alias of the same
name — exactly the kind of wrong-shadowing bug this resolver exists to
prevent, on the simplest possible lambda form. Fixed, and the same probe
caught that `NODE_PAT_BIND` (the `@args` binding) needed the identical
fix, pinned by its own test.

54 tests total (was 44): +3 for `eval_pred`'s Kleene refinement, +6
adversarial resolver tests, +1 for the `cfg_ident`/`option_prefix` safety
gate. Still not wired into `run_target` -- the dead_code warnings on the
IR/resolver are expected until the counterfactual gate-4 rewrite (next)
actually calls into this.

## Running

```
cargo test    # the full acceptance suite (tests/golden.rs, tests/fixture_integrity.rs,
              # plus a real-world unit test against fixtures/real/ifm-test.nix)
cargo run -- --targets targets/golden.toml [--json]

# syntax-visibility census against a real corpus, not option-branch
# analysis -- no manifest, no module/option matching:
cargo run -- --census /path/to/nixpkgs-checkout/nixos/tests [--json]

# on-demand, needs a local nixpkgs checkout, not run by cargo test:
scripts/verify-upstream.sh /path/to/nixpkgs-checkout
```

## Acceptance criteria (as specified, all met)

- [x] A. reproduces the historical finding on the exact parent commit
- [x] B. clears it on the exact fix commit, with evidence attached
- [x] C. survives 26 adversarial/synthetic/real-world cases: `c2`–`c5`
      (original 4 kimai mutations), `c6a`/`c6b` (non-null-default
      false-positive + its positive control), `c7` (unresolvable boolean
      default), `c8` (missing declaration), `c9` (parse-error fail-closed),
      `c10` (non-literal null-predicate default, AST- vs text-classified),
      `c11` (unresolvable test value), `c12`/`c13` (empty watch / missing
      file, both tool errors), `c15`/`c16` (test config hidden behind
      `imports` / an instance-level alias — both `TestConfigUnresolved`),
      `c17` (explicit opposite evidence still wins over an unrelated
      import elsewhere in the file), `c18`/`c19` (nested `nodes = {`
      form), `c20` (unrecognized root wrapper), `c21`
      (`import ./make-test-python.nix (...)` wrapper unwrapped), `c22`/`c23`
      (module-root `config`, literal and opaque), `c24` (`inherit` +
      dynamic attrpath), `c25` (unrecognized root entry next to a normal
      node — `TestConfigUnresolved`, not silently dropped as metadata),
      `c26`/`c27` (multi-segment attrpath / `inherit` under nested
      `nodes = { ... };`, the same bug one level down), plus a
      scanner-level unit test against real, unmodified `nixos/tests/
      ifm.nix`
- [x] D. produces source spans + evidence (file:line:col, matched assignment)
- [x] E. does not invoke VM tests
- [x] F. does not know anything about Doctrine

`cargo test` — 54 tests, all passing (34 at the H1.3b/H1-freeze point
below, +1 golden and +19 unit tests from H2's gate-1 fix, pure Predicate
IR, `eval_pred` refinement, and lexical alias resolver — see "H2" above;
H2 hasn't added new `cN`-numbered fixtures yet, so criterion C's case
list stays as of the freeze): 5 from the original spike, 5 from H1
(exit codes, parse-errors-fail-closed, outcome-transition +
positive-control, unresolvable-default, mandatory-declaration-gate), 4
from H1.1 (unresolved-test-value, AST-classified null-predicate default,
empty-watch-is-tool-error, missing-file-is-tool-error), 6 from H1.2
(imports/alias → `TestConfigUnresolved`, opposite-evidence-wins positive
control, its own opacity-detector positive assertion, CLI parse failure →
`TOOL_ERROR`), 7 from H1.3 (nested nodes form ×2, unknown root, the
make-test-python.nix wrapper, module-root config ×2, inherit+dynamic
attrpath), 3 from H1.3a (unclassified root entry → `TestConfigUnresolved`,
`--targets`/`--census` CLI mutual exclusion, the census's own
`unclassified_root_entries` metric), 3 from H1.3b (multi-segment attrpath
and `inherit` under nested `nodes`, the census's own
`unclassified_instance_entries` metric), 1 fixture integrity lock, 1
real-world scanner unit test against `fixtures/real/ifm-test.nix`.
`scripts/verify-upstream.sh`'s worktree fix, the CLI-level clap tests, and
the `--census` run against 1609 real files are exercised outside
`cargo test` (a shell script, raw process exit codes, and a full-corpus
pass, respectively) but verified the same way as everything else in this
project: by reproducing the actual bug first, then confirming the fix
against it, not just reading the diff and hoping.

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
