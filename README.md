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

## H2 — safety gate made scope-aware (own review round)

One more correction before the counterfactual gate-4 wiring, this time
against the safety gate itself: `cfg_ident_binds_to_prefix` (the gate-1
flat-root check above) did a flat, scope-blind `root.descendants()`
search for the first `NODE_ATTRPATH_VALUE` named `cfg_ident` anywhere in
the whole module — correct for the simple cases, wrong the moment an
unrelated helper function has its own, differently-named-the-same `cfg`
in a completely different scope:

```nix
let
  helper = x:
    let cfg = config.services.other;
    in cfg.foo;

  cfg = config.services.davis;   # the REAL top-level one
in
{ options.services.davis = { ... }; }
```

`helper`'s own `cfg` appears *earlier* in the source than the real one, so
a flat first-match search finds the wrong binding first and wrongly
rejects a legitimate flat root. Pointed out directly: this was ironic — a
scope-aware lexical resolver had just been built for exactly this problem
one section above, and the safety gate right next to it was still doing
the scope-blind thing it was supposed to replace. Two independent,
disagreeing ways of answering "what does `cfg_ident` mean here" is exactly
the failure mode "one semantics, one implementation" exists to prevent.

Fixed by building `resolve_cfg_root(use_site, cfg_ident) ->
Result<OptionPath, ResolveFailure>`: resolves `cfg_ident` *as seen from a
specific tree position*, reusing the same `resolve_ident_binding`/
`resolve_alias_recursively` machinery every other alias lookup in H2 already
uses (chasing through further aliases via `resolve_config_rooted_path` if
`cfg_ident` isn't directly `config`-rooted), rather than a second,
independent search. `scan_options` now calls it once per candidate flat
root, from that declaration's own position — so an unrelated helper's
shadowed `cfg` is never even visited (it isn't an ancestor of the real
declaration), regardless of document order. `cfg_ident_binds_to_prefix`
deleted outright, not kept alongside as a fallback.

Two new regression tests, not just re-asserting the original happy/sad
pair: `resolve_cfg_root_ignores_an_unrelated_earlier_shadow_in_document_order`
(the exact shape above, end-to-end through `scan_options`) and
`resolve_cfg_root_is_scope_aware_not_a_flat_grep` (unit-level proof that
querying from the real declaration resolves to `config.services.davis`
while querying from *inside* the shadowed `helper` resolves to
`config.services.other` — the foundation a future predicate-site check
would need, even though predicate-site wiring itself is still gate-4's
job, not this pass's). Also fixed in the same pass: the H2 IR's top doc
comment still said lowering returns `Option<_>` with `None` meaning
unsupported, stale since the `Result<_, ResolveFailure>` switch two
sections above — corrected.

56 tests total (was 54).

## H2 — counterfactual gate 4, wired in: the davis acceptance case

The point of all of H2 so far. `run_target`'s gate 2/4 now has a real H2
path. (This paragraph originally said the H2 path only ran when H1's own
unary predicate scan found nothing for the watched option — that was the
shape as first wired, and it was wrong: see H2.2 Finding 2 below, which
replaced it with unconditional aggregation. H1's byte-for-byte behavior on
every unary-only target is still completely unchanged — verified by
re-running the full suite after every change in this section, not just
argued.)

1. **`KnownValue`, not a fabricated placeholder.** Reviewed before
   wiring: the environment domain needed to distinguish "known non-null,
   but exact value withheld" (`KnownValue::DefinitelyNonNull`, only ever
   proves a null-comparison) from "known to be exactly this scalar"
   (`KnownValue::Exact`, proves any equality) — collapsing them (e.g. into
   a fake `Scalar::Str("placeholder")`, as the earlier compatibility
   tests did) would let the evaluator draw equality conclusions
   (`"placeholder" == "mysql"`) it has no basis for. `classify_known_value`
   mirrors `classify_value`/`ValueClass` but preserves exact literal
   content; `TestAssignment`/`OptionDecl` each gained a `known_value`/
   `default_known_value` field alongside their existing H1 ones.
2. **Every predicate referencing the watched option, not just the first
   one found.** `scan_resolved_predicates` finds every branch-condition
   site (`if`/`mkIf`/`optional`/...) and lowers each through `lower_pred`
   — for a *concrete* `option_prefix`, each site is also verified with
   `resolve_cfg_root(condition_site, cfg_ident) == option_prefix`, the
   same declaration-side safety check applied to the predicate side of
   the correlation (wildcarded targets like kimai skip this, unchanged
   from before). An earlier version of the counterfactual gate picked
   only the *first* resolved predicate referencing the watched option --
   reviewed and generalized: davis's real `database.driver` is referenced
   by both a simple `db.driver == "sqlite"` check *and* the compound
   `mysqlLocal` alias, and "the predicate" stopped being a well-defined
   singular concept the moment more than one could exist. Every candidate
   is now evaluated; a witness through *any* of them is existential
   evidence, and what happened with every candidate (not just the winning
   one) survives into a new `predicate_attempts` field, so a `PASS`
   never silently hides that a different predicate on the same option was
   inconclusive.
3. **Per-instance, never cross-instance.** For each real test instance
   that explicitly assigns the watched option `x`, two hypothetical
   environments — everything *else* the predicate references held at
   whatever *that instance* actually did (or the option's own declared
   default, if untouched) — differing only in whether `x` is at its
   declared default or this instance's test value. Deliberately never
   merges assignments from different `nodes`/`containers` instances into
   one synthetic environment, which would prove something about two
   independent machines' combined state that no real Nix evaluation ever
   produces.

**The davis acceptance case, on real code, both directions.** Watching
`database.driver`: both `davis-before` and `davis-after` correctly `PASS`
— but *not* via `mysqlLocal`. `machine1` sets `database.driver =
"postgresql"` in both fixtures (default is `"sqlite"`), which is real,
legitimate evidence against the simple `db.driver == "sqlite"` predicate
regardless of whether any mysql scenario exists at all. Caught during
this pass, before assuming the acceptance case was satisfied: numerically
matching "both PASS" isn't the same claim as "the compound alias resolver
works", and the two were conflated at first. `predicate_attempts` makes
the actual, separate claim directly assertable:
`davis_after_witnesses_the_mysql_local_compound_alias` checks that the
`mysqlLocal`-sourced attempt (an `And(...)` in the reported IR, not just
a source string that happens to mention the name) is `witnessed: true`
(`machine3`, added alongside the real fix, sets `driver = "mysql"`, with
`createLocally` at its declared-default `true`: `false -> true`).
`davis_before_does_not_witness_the_mysql_local_compound_alias` is the
negative control on the exact same predicate (`mysqlLocal`'s own
definition is byte-for-byte identical between before/after) — real
evidence exists (`machine1`/`machine2` are both consistently
non-mysql), but never a transition, so `witnessed: false`, never
silently omitted or silently counted as a win.

**Four adversarial cases beyond davis**, in a dedicated synthetic module
(`fixtures/synthetic/h2-compound/`) isolating the same
`createLocally && driver == "mysql"` shape from any real-corpus noise,
each verified to produce exactly its predicted verdict:

| case | scenario | verdict |
|---|---|---|
| 2: absorbed watched change | `createLocally=false`, `driver`: sqlite→mysql | `OBA001` — the other operand already pins the predicate false; the watched option's own change never flips anything |
| 3: symmetric watch | same instance, watching `createLocally` instead: true→false | `PASS` — proves causation is attributed to whichever option is actually watched |
| 4: per-instance anti-cross-contamination | nodeA (`driver=mysql, createLocally=false`), nodeB (`createLocally=true`) | `OBA001` for `driver` — if nodeB's `createLocally=true` ever leaked into nodeA's evaluation this would wrongly `PASS` |
| 5: unknown context | `driver=mysql` known; the other operand's default is a non-literal expression, never assigned anywhere | `TestValueUnresolved` — never silently `OBA001` (false "no evidence") or `PASS` (fabricated transition) |

62 tests total (was 56): the old single-predicate davis golden test
replaced by 3 (direct-predicate PASS, mysqlLocal-witnessed positive
control, mysqlLocal-unwitnessed negative control) plus 4 new golden tests
for the synthetic adversarial cases — net +6 in `tests/golden.rs` (33 →
39).

This counterfactual wiring was deliberately *not* frozen as terminal H2 —
reviewed once more before mutation testing, below.

## H2.2 — corrective pass on the counterfactual wiring (three findings)

A close read of the wiring above (not just "tests pass") found three real
gaps, two capable of producing a false strong verdict. Fixed with a
dedicated regression fixture per finding, none deferred:

1. **An explicit-but-opaque co-operand must never silently become its own
   declared default.** `evaluate_predicate_witness`'s per-`r`-in-`refs`
   lookup used to be `assignments.find(...).and_then(|a|
   a.known_value.clone()).or_else(|| declared_defaults.get(r).cloned())`
   — `.and_then` collapses "explicitly assigned here, but to something
   unclassifiable" and "never assigned at all" into the same `None`, so
   *either* fell through to `r`'s own declared default. Concretely: an
   instance setting `createLocally = builtins.pathExists /etc/flag;` (an
   opaque expression) alongside `driver = "mysql"` would silently
   evaluate `p = true && driver == "mysql"` (`createLocally`'s declared
   default substituted for its real, unknown value) — a false `PASS` on
   `driver`'s clean sqlite→mysql transition. Fixed by looking up the
   instance's own assignment first and using its `known_value` (`None`
   included) directly, falling back to the declared default *only* when
   there is no assignment for this instance at all. Regression:
   `h2_case6_explicit_opaque_co_operand_is_never_treated_as_its_own_default`
   (`fixtures/synthetic/h2-compound/test-opaque-other-operand.nix`) —
   must be `TestValueUnresolved`, was silently `PASS` before the fix.
2. **H1's own predicate and every H2 `ResolvedPredicate` must be
   aggregated together, not "H1 first, H2 only as a fallback when H1
   finds nothing."** The original wiring's `h2_counterfactual_verdict`
   function was called only from the `else` branch of `predicates
   .iter().find(...)` — the moment H1's own unary walker found *any*
   predicate for the watched option (even one that never witnesses
   anything, e.g. an unrelated `cfg.foo != null` guard sitting next to a
   real compound predicate the option also participates in), H2's
   compound candidates were never even evaluated, silently downgrading a
   possible `PASS` to `OBA001`. The standalone function was deleted and
   `run_target`'s gate 2-4 block rewritten to collect H1's own predicate
   (if any) and every H2 `ResolvedPredicate` referencing the watched
   option into one `Vec<PredicateAttempt>`, evaluated unconditionally; a
   witness through *any* candidate wins. (This unification is also what
   the davis acceptance case above already exercised — the same
   machinery, just no longer gated behind "H1 found nothing".)
3. **A branch-condition site that fails to lower must not silently
   vanish — and relevance took three attempts, each proven wrong or
   incomplete empirically before the next one landed.** `scan_resolved_predicates`
   used to `continue` straight past whatever `lower_pred` couldn't
   handle — the identical "not found reads as genuinely absent" shape
   H1's frozen walker spent three review rounds closing, now reopened in
   H2's own scanner. Fixed with a new `UnresolvedPredicateSite { span,
   source, failure, refs }`, returned alongside the resolved predicates
   and surfaced on `TargetReport`. `ResolveFailure::TrivialConstant` (a
   bare `true`/`false` condition, `mkIf true {...}`) is excluded outright
   from the very first version — its outcome is fully known, so unlike
   every other failure it carries no risk of hiding evidence. Everything
   else needed real iteration on *relevance*:
   - **Attempt 1 — "inside the module's `config` block".** Wrong: kimai's
     own `config = mkIf (eachSite != { }) (mkMerge [ ... ]);` *is* (part
     of) its `config` value, so this wouldn't have excluded it at all.
   - **Attempt 2 — "the failed condition's own text literally mentions
     `cfg_ident`"**, target-wide (any qualifying site anywhere in the
     target weakens *every* watched option's verdict). This correctly
     excluded kimai's `eachSite != {}` (an existence check unrelated to
     any single option, whose unresolvable half is a bare `{}` literal)
     and the synthetic `if builtins.pathExists /etc/synth-baz ...` false
     positive (lives inside an option's own `default = ...;`, never
     mentions `cfg` either) — but was unsound the moment it shipped,
     caught in the very next review round: a condition site can be a
     bare alias identifier (`suspicious`) whose *binding*, not its own
     syntax, is what references `cfg` (`suspicious =
     someUnsupportedHelper cfg.database.driver;`) — exactly the "alias
     hides the real reference" shape H2's own resolver exists to see
     *through* for successful lowerings, silently blind to it on the
     failure path. The target-wide scope made this tolerable by
     accident (over-inclusion, not under-inclusion, was attempt 2's
     failure mode) — but under-inclusion directly produces a false
     `OBA001` on exactly the watched option this project exists to
     protect, so it couldn't stand once spotted.
   - **Attempt 3 (current) — collect every option path *reachable* from
     the failed condition, including through alias resolution, and match
     it against the specific watched option, not target-wide.**
     `collect_reachable_refs` reuses the exact same
     `lower_value_expr_chained`/`resolve_ident_binding`/`AliasChain`
     primitives the successful-lowering path already uses (no second
     independent resolver, per explicit review instruction — this
     project had already been "hit twice" for maintaining two
     implementations of the same scoping semantics, see the
     `resolve_cfg_root` history above), just with a relaxed success
     criterion: find any `Ref` anywhere reachable from the failed node,
     even inside an alias's binding, even when the whole expression
     doesn't lower. `run_target` then matches a site's `refs` against
     `watched_path` directly, replacing the target-wide boolean
     entirely. Both of attempt 2's real fixtures still resolve correctly
     under attempt 3, for the right reason this time: kimai's `eachSite
     != {}` now collects `refs = [["sites"]]` (genuinely cfg-rooted, via
     `eachSite = cfg.sites;`, but never matches an unrelated watched
     option like `database.socket`) — no longer excluded by an accident
     of spelling, excluded because it's genuinely about a different
     option. `someUnsupportedHelper cfg.database.driver` hidden behind
     `suspicious` now collects `refs = [["database","driver"]]`,
     correctly matching when that's the watched option.

   Three regressions, one retired: the 5 golden tests that regressed
   against attempt 2's naive predecessor stay green; the original
   `h2_case7_relevant_unresolved_site_blocks_a_false_oba001` fixture had
   to be *rebuilt*, not just re-verified — its unresolved site referenced
   an unrelated option (`items`), which was sufficient evidence under
   attempt 2's target-wide gate but is (correctly) no longer sufficient
   under attempt 3's per-option gate, so the fixture now has the
   unresolved site's failing argument be a direct `cfg.watched` select
   instead; new `h2_case8_alias_hidden_unresolved_relevance_blocks_a_false_oba001`
   (`fixtures/synthetic/h2-alias-hidden-unresolved/`) is the alias-hidden
   positive control that actually falsified attempt 2.

65 tests total (62 → 65): 3 new/rebuilt golden tests (39 → 42 in
`tests/golden.rs`); `collect_reachable_refs` itself is only exercised
through the golden suite, no dedicated unit test — its real fixtures
(kimai, the two synthetic alias cases) are the test vectors, not
synthesized inputs.

H2.2 landed as commit `cef12d7`, pushed. Before moving to mutation
testing, this commit was itself put through one hostile review checking
four invariants (the three findings above plus the alias-hidden
correction folded into Finding 3) against a fresh, independent read of
the actual committed code, not the commit message. Three held outright;
one didn't.

## H2.2 hostile review of `cef12d7` — a fourth real gap, found in the fix itself

Invariants 1 (explicit opaque override), 3 (unresolved-relevant-site
gating), and 4 (alias-surviving, per-option relevance) all held up against
independent code reading. Invariant 2 — "H1's own predicate must never
suppress evaluation of H2 candidates for the same option" — did not.

`run_target`'s gate 3 for H1's own predicate specifically still `continue`d
immediately whenever `decl.default_class.and_then(|c|
predicate_outcome(&pred.kind, c))` was `None`, before the H2 loop below it
ever ran — the exact same "H1's own gate short-circuits the whole option"
shape Finding 2 had just fixed for gate 4, reopened one gate earlier. The
code's own comment at the time called this "narrow, deliberate scope...
which no finding actually asked for" — true when written, but the reasoning
didn't survive contact with the fact that H1's `ValueClass` classifies a
default's outcome strictly more coarsely than H2's `KnownValue` does for
the *identical* AST node: a plain string-literal default is
`ValueClass::DefinitelyNonNull` (enough for a null-check predicate, not
enough for `Truthy`/`NegTruthy`, which need an actual `Bool`) but
`KnownValue::Exact(Scalar::Str(..))` under `classify_known_value` — fully
sufficient for a separate `Eq`-based H2 predicate on the same option. The
old `continue` meant that gap silently downgraded a resolvable `PASS` to a
false `DefaultUnresolved`.

Fixed by no longer treating H1's own gate-3 failure as an early exit: it
now just means H1 contributes no `PredicateAttempt` at all (there's no
default outcome to compare a test value against), tracked in a
`h1_default_unresolved` flag, and the H2 loop always runs regardless.
`h1_default_unresolved` is checked once every candidate (H1's own and every
H2 one) has had a chance — right after a real `winner`, at the same
near-top priority `DefaultUnresolved` always had (the original code
returned it before opacity or assignments were even looked at, so it
implicitly outranked everything else whenever it applied). Getting this
priority right took a live regression: an earlier version of the fix folded
`h1_default_unresolved` into the generic `has_unresolved` bucket alongside
`TestValueUnresolved`, which flipped the pre-existing
`h1_unresolvable_default_is_inconclusive_not_guessed` golden
(`c7-unresolved-default`) from `DefaultUnresolved` to `TestValueUnresolved`
— caught because that fixture's `bar` option turns out to *always* get a
redundant H2 attempt too (a bare `cfg.bar` select H1 recognizes as a direct
predicate is independently picked up by `scan_resolved_predicates`, so
`attempts` is essentially never empty whenever H1 found something — the
original fix's `attempts.is_empty()`-gated special case for
`DefaultUnresolved` was therefore dead code for the case it was written
for). Restructured as an explicit priority check instead of an
emptiness check.

New regression: `h2_case9_h2_candidate_rescues_a_false_default_unresolved`
(`fixtures/synthetic/h2-default-unresolved-rescue/`) — an option `flag`
defaulting to the plain string `"sqlite"`, with H1 finding a `Truthy`
predicate on it directly (`mkIf cfg.flag {...}`, unable to classify a
string default as a boolean outcome) *and* a separate `flag == "mysql"`
predicate elsewhere in the same module that H2 resolves cleanly. Asserts
`PASS`, not the false `DefaultUnresolved` the bug would have produced.
`h1_unresolvable_default_is_inconclusive_not_guessed` (`c7-unresolved-default`,
no competing H2 candidate exists there) is the negative control proving
the fix didn't just relocate the bug into weakening every `DefaultUnresolved`
case indiscriminately.

66 tests total (65 → 66; `tests/golden.rs` 42 → 43).

Deliberately not done in this pass, per explicit scope: mutation testing
against the new IR/evaluator/resolver/counterfactual-gate (next, now that
H2.2's correctness pass — including the hostile-review round on the H2.2
commit itself — is closed); Kani/bounded model checking (only after
mutation testing, only if small and useful); no Z3/SMT (`Eq`/`Not`/`And`/
`Or` over concrete finite values evaluates directly).

## Mutation testing (`cargo-mutants`), narrow semantic core

H2.2 closed, so this is the first time `cargo-mutants` actually runs
against real, wired-in logic rather than a dead-code pure core. Scoped
deliberately narrow, not the whole file: `eval_known_eq`, `eval_pred`,
`resolve_alias_recursively`, `lower_value_expr_chained`,
`lower_pred_chained`, `collect_reachable_refs`, `evaluate_predicate_witness`,
and `run_target`'s aggregation block — the functions that actually decide
a verdict, not scanning/plumbing code around them.

```
cargo mutants -F 'eval_known_eq|eval_pred|resolve_alias_recursively|lower_value_expr_chained|lower_pred_chained|collect_reachable_refs|evaluate_predicate_witness|run_target' -j 1
```

**First run: 64 mutants — 51 caught, 5 unviable, 8 survived.** The 5
unviable are all `replace FUNCTION with Default::default()` mutants
against types (`T`, `ValueExpr`, `Pred`, `PredicateWitnessOutcome`,
`TargetReport`) that don't implement `Default` — rejected by the type
system itself, not a test gap. Every survivor was individually read
against the actual code and classified — none dismissed on the strength
of "the suite is green":

- **`collect_reachable_refs`'s own alias-following cycle guard**
  (`!chain.iter().any(|n| n == &name)` → `!=`) survived because the only
  existing alias-hidden fixture (`h2-alias-hidden-unresolved`) starts its
  own alias-following with an EMPTY chain, where the mutated guard
  happens to coincide with the correct (permissive) answer — the bug only
  shows up two hops deep, where a *different* name already sits in the
  chain and the mutated logic wrongly reads "still resolving this
  particular name" as "block everything." New fixture
  `h2-alias-hidden-two-hop` (`hidden` → `mid` →
  `someUnsupportedHelper cfg.database.driver`) isolates exactly that
  depth. **Real gap, closed.**
- **`run_target`'s H1-gate-4 opposite-outcome match guard**
  (`o == !default_outcome` → constant `false`) survived against the
  ENTIRE existing golden suite. Root cause, verified by hand: every
  existing PASS-via-H1 fixture uses a bare `cfg.foo` select or a null
  comparison, and H2's OWN scanner independently rediscovers and
  re-evaluates the identical logical predicate as its own
  `ResolvedPredicate` — so H2's redundant witness silently masks H1's own
  detection logic being completely broken. In effect, H1's gate-4 code
  had become dead weight for every fixture that existed, and nothing
  would have caught it regressing. New fixture
  `h2-scope-shadowed-h1-only` isolates H1-only coverage: the sole
  predicate on `flag` sits behind a LOCALLY shadowed `cfg` binding that
  H2's scope-aware `resolve_cfg_root` correctly excludes from
  `resolved_predicates` (spelling matches `cfg_ident`, lexical scope
  doesn't) — so no H2 candidate exists anywhere in the module to
  redundantly confirm the transition. **Real gap, closed** — and a
  genuinely useful thing to have learned about the current architecture
  independent of the mutation itself: H1's own gate 4 is currently
  live/load-bearing only for spelling-shadowed edge cases, not the common
  path.
- **Five more survivors, all in purely-diagnostic fields never asserted
  anywhere**: `PredicateAttempt.witnessed` for the H1 (`Unary`) attempt
  specifically (only H2 `Resolved` attempts were ever checked before);
  `DefaultUnresolved`'s embedded `predicate` field (verdict *tag* was
  checked, not which predicate got attached to it); `diagnostic_default_outcome`'s
  two lookups (H1-side and H2-side) — asserting the VALUE on the two most
  obvious existing fixtures (kimai, `h2-compound`) turned out to be
  insufficient to kill either: kimai's H1 lookup mutation still fell
  through to a coincidentally-correct H2 `.or_else` fallback, and
  `h2-compound`'s H2 lookup mutation still matched via the compound
  predicate's OTHER ref (`refs` has 2 entries, `.any()` stays true either
  way). Both needed purpose-built single-source fixtures instead
  (`h2-case12`/`h2-case13`: zero-H2-candidates and single-ref-H2-only,
  respectively) before they actually killed anything — confirmed by
  re-running `cargo mutants --iterate` against the same output directory
  after each fix, not assumed. `ResolveFailure::TrivialConstant` vs
  `Unbound` for a bare `true`/`false` condition got a direct unit test
  instead of chasing it through a verdict (it never affects one — a bare
  `true`/`false` lowers as a `Literal`, contributing no reachable ref
  either way). **All five real (if low-severity) gaps, closed.**
- **`evaluate_predicate_witness`'s `instances_assigning_x` membership
  check** (`&&` → `||`) survived and stayed survived through every fix
  above. Confirmed **equivalent**, not a gap: broadening membership only
  adds instances that never actually assigned the watched option, and
  the very next line's `x_assignment` lookup (which still correctly
  filters on `path_matches_prefix`) silently drops every one of them via
  its own `else { continue; }` before any other computation runs — no
  output is ever observably different. Documented in place (a comment at
  the mutation site) so a future mutation-testing pass doesn't waste time
  rediscovering the same non-gap.

**Second and third `cargo mutants --iterate` runs** (resuming against the
same `-o` output directory, so only previously-uncaught mutants re-run)
confirmed each fix in turn: 8 → 3 → 1 survivor, the final 1 being the
confirmed-equivalent case above. 71 tests total (66 → 71: 2 new golden
fixtures for the two real semantic gaps, 2 more for the two diagnostic-
field gaps that needed dedicated isolation, 1 new unit test for
`TrivialConstant`, plus assertion strengthening on 3 already-existing
tests that didn't by themselves kill anything).

**One environment note, not a code issue**: `cargo-mutants` running in
the background left this VPS's shared `CARGO_TARGET_DIR` (used across
several unrelated projects on this machine) in a state where a
subsequently-run `cargo test` intermittently failed with `fixtures/
integrity-lock.toml must exist` — a stale test binary with a
compile-time-baked `CARGO_MANIFEST_DIR` pointing at a path that no
longer existed, not a real fixture problem. `cargo clean -p oba` before
trusting any `cargo test` run immediately after a `cargo-mutants`
invocation resolves it; `env -u CARGO_TARGET_DIR` on the `cargo mutants`
invocation itself reduces but doesn't eliminate the risk.

## KANI-0: bounded formal proofs of the semantic core

Mutation testing (`0afe187`) left a small, clean semantic core — the
condition the user set for deciding this determination was finally
meaningful rather than decorative. Final scope, after a real attempt at
a third layer was tried and dropped (below): **K0.1 + K1, 6 harnesses
total.**

- **K0.1 — `eval_known_eq` never fabricates knowledge.** It returns
  `Option<bool>`; the entire `PASS`/`OBA001` distinction rests on
  `Some(b)` meaning "we actually know this", not "this looks plausible".
  Proven directly against a small finite `Scalar`/`KnownValue` domain.
- **K1 — `aggregate`'s priority ordering is exhaustively correct.**
  Already pinned by brute-force enumeration
  (`aggregate_priority_is_exhaustively_correct_over_all_32_cases`, plain
  `cargo test`, 32 cases is cheap) — restated as 5 `#[kani::proof]`
  harnesses (K1.1–K1.5) because this exact priority ordering has broken
  twice for real (H2.2 Finding 2; the post-`cef12d7` hostile-review
  fixup), so it earns a formal artifact of its own. K1.5 in particular
  states the fail-closed guarantee as a postcondition on the verdict
  itself: if `aggregate` ever returns `Oba001`, none of the other four
  uncertainty facts held.

Both are cheap: all 6 harnesses verify in a few seconds combined, on
this project's own resource-constrained dev VPS (1 core, 1.9GB RAM) as
well as in CI.

**`eval_pred` soundness/monotonicity (originally-planned K0.2/K0.3, plus
a K0.4 sanity check) was attempted and DROPPED, not deferred — the
central finding of this whole KANI-0 round.** The harnesses were
well-formed and compiled correctly; what killed them was CBMC's cost for
symbolically modeling Rust `HashMap`'s SipHash hasher, present in any
use of the real `HashMap<Vec<String>, KnownValue>` environment type
`eval_pred` actually takes — essentially independent of harness size.
Five escalating scope reductions tried locally (free recursive `Pred`
generator over 3 paths/depth 2, then 2/1, then 1/1, then one FIXED tree
shape per `Pred` constructor with a single `Ref` lookup — a genuine
harness-design improvement, kept in spirit even after the layer itself
was dropped) each still pushed the VPS to the edge of OOM before
finishing. Moved to GitHub Actions next, on the theory that more RAM/CPU
would fix it — **it didn't**: on a real CI run, the single cheapest
harness (`k0_4_double_not_is_identity`, same fixed-shape design)
still hadn't finished after ~29 minutes of a 30-minute job timeout, and
had to be force-killed as an orphan `cbmc` process. That confirmed the
cost is fundamental to the (real `HashMap`, real `String` keys)
approach, not a resource ceiling any one machine happened to hit — the
risk a small finite-domain *mirror* type (proving an isomorphic copy of
`eval_pred`, not the production function itself) was originally raised
to avoid. Offered again after the CI failure and explicitly declined, in
favor of stopping at K0.1 + K1. `eval_pred`'s own Kleene-logic soundness
stays covered by the existing proptest properties
(`eval_pred_and_short_circuits_on_a_known_false_operand_either_side` and
siblings) and mutation testing, not by a bounded proof.

**Deliberately not attempted for the same "small self-contained
function" reason**: `rnix`/`rowan`, `resolve_ident_binding`,
`lower_pred_chained`, the test-file walker, or `run_target` as a whole.
A deep call graph over untyped syntax trees is exactly what Kani's own
guidance warns against. That surface already has goldens, adversarial
fixtures, proptest, mutation testing, and hostile review — formalizing
it now would be negative ROI.

`.github/workflows/kani.yml` runs `cargo kani` in CI on every push/PR to
`main` (`timeout-minutes: 10`, generous headroom for 6 harnesses that
take seconds). Reuse survey (AGENTS.md step 3/5): tried the official
`model-checking/kani-github-action` first — both published tags (`v1`,
`v1.1`) have a confirmed bug in their shared `install-kani.sh` (extracts
the installed version via `kani --version | awk '{print $2}'`, but the
real output is two lines and field 2 of line 1 is "Rust", not the
version number — reproduced locally, not guessed), so every real run
failed in ~25-30s before ever reaching Kani. The workflow composes
around it instead: same `kani-verifier` crate, same
`dtolnay/rust-toolchain@stable` the action itself uses internally, same
`cargo kani setup`/`cargo kani` commands — only the action's own broken
self-verification wrapper is skipped.

```
cargo kani list              # enumerate harnesses without running any (fast, local, safe)
cargo kani                   # run all 6 -- a few seconds, safe on any machine
```

## K1 — a spike testing OBA's original premise on a real production defect

Not H2, not D3. A deliberately narrow question, asked directly: **can this
approach automatically detect a real `unixSocket`/`socket` ↔
`unix_socket` defect on historical nixpkgs, with provenance and
fail-closed semantics, using real Nix evaluation and the exact pinned
Doctrine consumer contract?** OBA answers "did a test exercise this
option's branch"; K1 asks a genuinely different question — "does the
*external output contract* match a real consumer" — Contract Drift
Checking (CDC), not OBA. Kept as a separate module (`src/cdc.rs`), not
merged into OBA's `Verdict`/report types: two different kinds of
evidence, not forced into one enum before there's a reason to.

**Answer: yes.** All 8 falsification criteria from the task spec were
checked against a real run, not assumed:

| case | expected | actual |
|---|---|---|
| Kimai before (`5530e24f2`) | FINDING | FINDING |
| Kimai after (`d81d88f4354b`) | PASS | PASS |
| Davis before | FINDING | FINDING |
| Davis after | PASS | PASS |
| mutation `unix_socket`→`unixSocket` | FINDING | FINDING |
| mutation `unix_socket`→`socket` | FINDING | FINDING |
| mutation `unix_socket`→`unix-socket` | FINDING | FINDING |
| control `unix_socket`→`unix_socket` | PASS | PASS |

**Architecture actually implemented** (real Nix evaluation as semantic
oracle, `rnix`/source AST plays no role at all here):
```
option value (sentinel or, for davis, a fixed hardcoded string)
    ↓
real nixosSystem evaluation (nix eval --impure --raw, no build, no VM)
    ↓
rendered DATABASE_URL string (a plain Nix string attribute)
    ↓
Phase C: extract_key_for_value -- fail-closed DSN-query key extraction
    ↓
Phase D: extract_accepted_keys -- bounded isset($params['x']) scan over
         the exact pinned doctrine/dbal source (vendored fixture)
    ↓
Phase E: compare_contract -- boring: emitted_key ∈ accepted_keys ⇒ Pass
```

**Historical corpus provenance** (Phase A, established before any code,
per the task's own requirement): before = `PhysShell/nixpkgs`
`5530e24f2100f4c2ca766050a805a12d7541662f` (a real, but otherwise
unrelated, pre-existing upstream commit — the bug predates it); after =
`d81d88f4354b2c9d8a7492c9b72cd3add62a34e0`, current tip of
`fix/doctrine-unix-socket-param-name`. **Provenance correction made
during Phase A, not glossed over**: the "after" commit was originally
recorded as `37f81efa4abf623009e474fa563a094658c25be6` — that SHA still
resolves via the GitHub API but is no longer reachable from the branch's
current tip (amended once to add an `Assisted-by` trailer, then a
test-only follow-up landed on top). Verified before repointing
`fixtures/integrity-lock.toml`: `kimai.nix`/`davis.nix` are byte-identical
(same sha256) across the old and new "after" SHAs — only the commit
message and a later, separate test-file change differ, never the module
content this spike or the existing OBA golden corpus actually pin.
Doctrine DBAL: independently confirmed **for each app separately, not
assumed shared** — Kimai 2.66.0's own `composer.lock` and Davis v5.4.4's
own `composer.lock` both resolve `doctrine/dbal` to `3.10.6` at the same
git commit `c95589d775a0b2e543467d40f8c3ecccf586f2b4`. Accepted keys
(`host`, `port`, `dbname`, `unix_socket`, `charset`) extracted from that
exact commit's real
`src/Driver/PDO/MySQL/Driver.php`, vendored as
`fixtures/cdc/doctrine-dbal-3.10.6/PDO-MySQL-Driver.php` (sha256-locked,
same discipline as every other vendored fixture).

**A real mechanism finding, not just a result**: `nixosSystem`'s
`disabledModules` (used to substitute a locally-mutated `kimai.nix` for
mutation testing, through the *same* evaluation path as the historical
corpus, not a separate mutation-only harness) must be given as a Nix
**path value** built from the same `nixpkgsSrc`, not a string — 
`disabledModules = [ "nixos/modules/.../kimai.nix" ]` silently fails to
match nixpkgs's internal module key and the real module loads anyway
(`option already declared` collision); `disabledModules = [ (nixpkgsSrc +
"/nixos/modules/.../kimai.nix") ]` matches correctly. Confirmed by
actually trying the string form first, not assumed from documentation.

**Davis diverges from Kimai's probe, disclosed not hidden**: Kimai's
`database.socket` is a real option, so a unique sentinel is injected and
traced to the generated `kimai-init-<host>` systemd script (real evidence
that the value flows all the way to a production sink). Davis's socket
path is a hardcoded string literal (`/run/mysqld/mysqld.sock`) regardless
of configuration — there is no option to inject a sentinel into — so its
probe evaluates `config.services.davis.config.DATABASE_URL` directly.
Still real Nix evaluation of the real module through the real option
system (`mkIf`/`createLocally` branch selection genuinely happens), just
without a sentinel-driven trace to a further sink, because there's
nothing sentinel-shaped to trace.

**Fail-closed, exercised by dedicated tests, not just claimed**: every
extraction failure path returns `Inconclusive`, never a guessed key or a
silent `Pass` —  sentinel found zero or 2+ times, a value not immediately
preceded by `=`, a malformed key, `isset($params[...` truncated before
`'])`, or the `isset($params['<key>'])` pattern not found at all (the
extractor no longer understands the source shape). `probable_candidate`
(a similarity hint, e.g. `unixSocket`/`socket` → `unix_socket`) is
attached to a `Finding` that already exists on its own evidence — it
never itself decides Pass/Finding.

**Hardcoded corpus assumptions in K1 — the explicit "what K2 would need to
generalize" list**:
- The two nixpkgs revisions (`BEFORE_REV`/`AFTER_REV`) and the app names
  (kimai, davis) are Rust constants, not discovered from anywhere.
- The Doctrine DBAL version/commit pin (`3.10.6` @ `c95589d7...`) is a
  Rust constant, independently *confirmed* against each app's real
  `composer.lock` during Phase A, but not *derived automatically* at
  runtime from `package.nix` → `composer.lock` — K1 doesn't walk that
  chain itself.
- The Doctrine driver source is a vendored, sha256-locked fixture, not
  fetched live from `doctrine/dbal` at test time (avoids a third live
  network dependency during tests; `nixpkgs`'s `fetchTarball` is still
  live).
- Kimai's sink identity (`systemd.services."kimai-init-<host>".script`)
  and Davis's (`config.services.davis.config.DATABASE_URL`) are both
  hand-picked per app, not discovered by walking a generic "sink
  universe" — exactly per the task's own instruction not to build one
  without a corpus reason to.
- The DSN-query key-extraction model (Phase C) and the
  `isset($params['x'])` scan (Phase D) are narrow, MySQL-DSN- and
  Doctrine-PDO-shaped on purpose, not a general URI parser or PHP parser.

**Tests**: 11 new offline unit tests (Phase C/D/E logic, no `nix`/network
— run by every plain `cargo test`) + 8 `#[ignore]`-by-default tests (the
real golden and mutation proof above, needing a real `nix` binary and
network access — run explicitly with `cargo test --bin oba -- --ignored
cdc::`; **not currently run in CI at all**, since this project's existing
`kani.yml`/`dogfood.yml` workflows don't include a general `cargo test`
job — a real, disclosed gap, not swept under the rug). 58 tests in the
`oba` binary's own unit-test target (was 50), 107 total across the whole
project including `tests/*.rs` black-box suites, all green; `cargo
clippy --all-targets` clean (same 2 pre-existing warnings, neither
touched by K1).

**Stop condition honored**: this section, the code, and the hostile
self-check above are where K1 stops. Not continued into a generic CDC
architecture, D3, H2, or scanning further services — that's a decision
for whoever reads this hostile review next, not something to drift into.

**K1 FROZEN at `4e968a7`.** Reopened only by a concrete counterexample
(same standing rule as every other frozen layer in this project — see
`CLAUDE.md`), not for architectural cleanup, renaming, or generalizing
ahead of a real need. The hostile-review verdict: the proof chain is real
end-to-end (real `nix eval` → a really-rendered `DATABASE_URL` → an
extracted producer key → the exact pinned Doctrine 3.10.6 → an extracted
consumer contract → a boring comparison) — no `grep`-on-`kimai.nix`
shortcut, no hardcoded "correct answer" anywhere in the pipeline itself.

Reviewed next-step order, deliberately **not** generalizing the whole
surface at once:

1. **K1.1 — CI qualification for the real proof. Closed, `2a9032a`, real
   CI confirmed green** (`gh run view` on the actual run, not just "the
   job succeeded": the log shows all 8 `cdc::` tests genuinely executed
   — `50 filtered out` confirms it wasn't an empty/no-op filter match —
   and passed in 39.6s on a fresh GitHub-hosted runner). `.github/
   workflows/k1.yml` runs K1's 8 `#[ignore]`d historical/mutation tests
   (`cargo test --bin oba -- --ignored "cdc::"`) in a real Nix-equipped
   CI job (`cachix/install-nix-action`, the same action
   `PhysShell/nixpkgs`'s own `nixpkgs-vet` CI job already uses) — without
   it, "K1 works" was a claim backed by one specific dev machine, not CI.
2. **K2a — automatic consumer provenance. Closed.** Narrow goal, held to:
   remove the hand-verified `DOCTRINE_DBAL_VERSION`/`DOCTRINE_DBAL_REV`
   Rust constants and get the same provenance automatically from the
   analyzed nixpkgs revision — not arbitrary Composer dependency
   resolution, not a Rust package manager. `resolve_consumer_identity`
   (pure: parses an already-fetched `composer.lock`'s JSON, finds the
   entry named EXACTLY `package_name` — no fuzzy/similarity matching
   anywhere near this trust boundary, proven by a dedicated test with a
   `doctrine/dbal-foo` decoy sitting right next to the real entry) is
   kept strictly separate from `fetch_composer_lock` (real Nix eval:
   `pkgs.<kimai|davis>.src` is that app's own `fetchFromGitHub` result —
   asking Nix directly for it, not hand-interpreting `package.nix`'s
   `imports`/`callPackage`; realizing the fixed-output derivation is
   cheap and in practice already cache-hit on cache.nixos.org, ~2-3s
   each, confirmed for real, not assumed). Kimai and Davis resolve via
   two fully independent calls — never one lookup shared between them,
   the thing that would have quietly turned this into "a slightly
   better-disguised global constant." `verify_identity_matches_vendored_fixture`
   refuses to pair a resolved identity with a vendored source whose
   `source.reference` doesn't match — a tampered/updated `composer.lock`
   must never silently keep using the old vendored contract.

   **All 5 stop-condition items met, checked against a real run, not
   assumed**: Kimai provenance resolves automatically (✓, both
   revisions); Davis provenance resolves automatically, independently
   (✓, both revisions); the original K1 8/8 stayed unchanged through the
   swap (✓, re-ran for real after removing the constants); the
   `DOCTRINE_DBAL_VERSION`/`DOCTRINE_DBAL_REV` constants are gone from
   `src/cdc.rs` (✓ — the transition tests now compare the auto-resolved
   identity against `fixtures/integrity-lock.toml`'s own recorded
   provenance for the vendored fixture instead, reusing the *existing*
   hand-verified record rather than duplicating it under a new constant
   name); missing/ambiguous/mismatched provenance is fail-closed (✓ — 8
   new offline tests: missing `packages` array, a removed `doctrine/dbal`
   entry, 2+ matching entries, a missing `version`/`source.reference`
   field, a tampered `source.reference` failing the vendored-fixture
   check, plus the decoy-package test above); CI reproduces all of it
   (✓ — `k1.yml`'s existing `cdc::` filter picked up the 4 new real
   transition tests automatically, no workflow change needed, confirmed
   green on the actual push: `gh run view --log` shows `12 passed; 0
   failed; ... 58 filtered out` in 44.9s on a fresh runner). 12 real
   end-to-end tests now (was 8), 8 new offline tests — 70 total in the
   `oba` binary's own unit-test target counting both (was 58). Reasoning
   for doing this before K2b/K2c: a generalized CDC that finds contracts
   beautifully but still trusts a partially hand-verified pin for what a
   consumer *accepts* is a real, ugly trust boundary — worth
   closing before the surface grows at all.
3. **K2b — generalize producer evidence. Closed.** `ProducerEvidence`
   (`SentinelFlow { option, sentinel, sink, rendered_value, emitted_key }`
   | `EvaluatedLiteral { sink, rendered_value, emitted_key }`), exactly
   the sum type sketched in the design note. Kept scoped tightly —
   generalizes the *shape* of the proof only, not sink discovery: both
   variants still know their own concrete, hand-picked sink (Kimai's
   `kimai-init-<name>` script, Davis's `services.davis.config`), no
   generic "find all sinks" mechanism was built.

   Acquisition (`eval_kimai_script`/`eval_davis_database_url`, real
   Nix+network, unchanged from K1) is kept strictly separate from
   evidence construction (`build_sentinel_flow_evidence`/
   `build_evaluated_literal_evidence`, pure, offline-testable) — the
   same resolution/fetching split K2a already established, applied here
   to the producer side. **The four requested proofs, each checked
   against a real run**: (1) Kimai still acquires `SentinelFlow` and the
   original K1 8/8 stayed unchanged after the refactor (re-ran for real:
   12/12, unchanged from before K2b); (2) Davis is no longer a special
   `if app == davis` case at the verdict level — it acquires
   `EvaluatedLiteral` and goes through the exact same downstream
   function Kimai does, not a parallel code path (both golden tests now
   assert the concrete variant *and* call the identical
   `verdict_for_evidence`); (3) branching ends the moment
   `ProducerEvidence` exists — `verdict_for_evidence(&ProducerEvidence)
   -> CdcVerdict` takes the enum, not a variant, and internally only
   ever calls `compare_contract(evidence.emitted_key(), ...)`, so there
   is structurally nowhere left for variant-specific verdict logic to
   hide; (4) fail-closed producer-evidence mutations — 6 new offline
   tests: `SentinelFlow` with an absent sentinel / an ambiguous
   (2-occurrence) sentinel, `EvaluatedLiteral` with a malformed DSN
   (literal present but not `key=`-shaped) / an unextractable key
   (literal appears twice) — all `Inconclusive`, plus one success case
   per variant proving the full evidence (option/sentinel/sink for
   `SentinelFlow`, sink for `EvaluatedLiteral`) actually gets carried,
   not just the key.

   **The "don't make one variant stronger than the other" rule** is
   checked by construction, not merely stated: a dedicated test builds
   one real `SentinelFlow` and one real `EvaluatedLiteral`, both
   resolving the identical `emitted_key`, and asserts
   `compare_contract` returns the identical `Pass` verdict for both —
   there is no `StrongPass`/`WeakPass` anywhere in this codebase and the
   type system doesn't leave room to add one by accident, since
   `compare_contract`'s signature only ever sees `&str`, never the
   `ProducerEvidence` enum itself.

   77 tests total in the `oba` binary's own unit-test target (was 70):
   65 offline (was 58, +7) + 12 real/ignored (was 12, unchanged in count
   — all 12 refactored onto the new pipeline, not added to). Confirmed
   green in actual CI, not just locally: `gh run view --log` shows `12
   passed; ... 65 filtered out` in 40.1s on a fresh runner.
4. **K2c — corpus qualification census. Closed. Not "support more apps"
   on purpose** — a measurement of how far `SentinelFlow`/`EvaluatedLiteral`
   actually reach across a small real corpus, without changing core
   semantics to make them reach further. Full raw table, methodology, and
   the real (not just read-from-source) `strichliste` verification are in
   [`fixtures/cdc/k2c-census/census.md`](fixtures/cdc/k2c-census/census.md)
   — deliberately kept as an ugly table, not smoothed into prose, because
   the distribution is the actual deliverable, not a narrative about it.

   **Zero `src/cdc.rs` changes this round** — the one thing this census
   touches beyond reading source is reusing K2a's `fetch_composer_lock`/
   `resolve_consumer_identity` *as-is* against a brand-new app
   (`strichliste`) to confirm they generalize with no code changes at
   all, which they did. Explicitly resisted the temptation named in the
   design review — no new `ProducerEvidence` variant was added after the
   first (or fourth) awkward service; the whole 14-app corpus was
   surveyed first, reasons for non-fit collected, and only THEN checked
   for a repeating pattern.

   **Distribution across 14 real candidates** (every non-dev-tool
   `php.buildComposerProject2` web app in nixpkgs at `AFTER_REV`):
   supported as-is **3** (kimai, davis — frozen K1 baseline — plus
   `strichliste`, newly verified for real this round: a live `nix eval`
   shows a sentinel passed through `services.strichliste.environment.DATABASE_URL`
   byte-for-byte, `unix_socket=<sentinel>` intact); needs a new
   `ProducerEvidence` variant **4** (`agorakit`/`movim`/`snipe-it`: a flat,
   discrete `DB_HOST`/`DB_SOCKET`/... env-var shape, not a DSN query
   string — a real, *repeated* pattern (3 of 14), the one form of
   evidence worth actually generalizing to if this gets picked up again;
   `flarum`: a generated PHP config array, a one-off shape within this
   corpus, deliberately not promoted to a pattern on a single instance);
   consumer contract unsupported **9** (`part-db`: right shape, wrong DSN
   dialect — Postgres, not MySQL, our vendored driver doesn't apply; 8
   more apps with no `doctrine/dbal` in their dependency closure at all);
   ambiguous/inconclusive **0** — every candidate resolved to a definite
   bucket.

   Two real, recurring sub-findings kept in the census file rather than
   promoted to new buckets or new code: `composer.lock` sometimes lives
   IN nixpkgs itself (`composerLock = ./composer.lock;`, upstream ships
   none) rather than in the fetched app source — hits `flarum`/`baikal`/
   `postfixadmin`, a real "K2a's consumer-side fetch needs an alternate
   locator" case, not a producer concern; and `doctrine/dbal` merely
   *appearing* in `composer.lock` is necessary but not sufficient proof
   it's the actual runtime DB consumer — the three Laravel-family apps
   almost certainly use Laravel's own connector instead, a trust-boundary
   lesson worth remembering the next time "grep the lockfile for the
   package name" gets reused.

   **Stop condition, all 6 items met**: ≥5 real services (14, +3 real
   verifications beyond K1's frozen 2); each through exact pinned
   consumer provenance (resolved where `doctrine/dbal` existed, explicitly
   recorded absent where it didn't — never guessed); zero existing K1/
   K2a/K2b result changed; every unsupported case explicit, never a
   silent skip or a fallback `Pass`; special cases (the two sub-findings
   above) listed separately from the bucket table; coverage visible —
   3/14 (21%) clean, the dominant reason being `doctrine/dbal` simply
   absent from most of this particular corpus slice, a fact about the
   sample, not a verdict on the producer model.

   **Methodology correction (post-K2c, before K2d): the counts above
   don't sum to 14** (3 + 4 + 9 = 16) because "bucket" was overloading
   three independent questions — producer support, consumer support, and
   provenance location — into one column. Fixed in
   [`census.md`](fixtures/cdc/k2c-census/census.md) by splitting into
   three separate axes, each of which now sums to 14 on its own; no new
   data was collected, every entry's axis values are re-derived from the
   same findings recorded above. This is a documentation fix, not a
   reopening of K2c's substance — nothing about which services fit which
   producer shape changed.
5. **K2d — `FlatEnvVars`, a third `ProducerEvidence` variant. Closed.**
   The corpus-backed signal K2c produced, acted on: `agorakit`/`movim`/
   `snipe-it` share a flat, discrete `DB_HOST`/`DB_PORT`/`DB_SOCKET`/...
   env-var shape (rendered directly as a Nix attrset via
   `services.<app>.config`/`.settings`), not one value embedded in a DSN
   query string the way Kimai/Davis are. `ProducerEvidence::FlatEnvVars
   { sink, emitted: Vec<String> }` models it; all 3 apps go through the
   ONE `build_flat_env_vars_evidence` path, zero app-name branches
   anywhere in comparison.

   `ProducerEvidence::emitted_key()` changes from `&str` to `Option<&str>`
   — the minimal change Rust's match-exhaustiveness forces, nothing more:
   `SentinelFlow`/`EvaluatedLiteral` still return `Some` unchanged,
   `FlatEnvVars` returns `None` because there genuinely is no single
   comparable key for this shape (a DSN's `unix_socket=...` parameter and
   a bare `DB_SOCKET` env var are not the same kind of fact).
   `compare_contract` itself is untouched.

   **Deliberately does not** attempt to map `DB_HOST`/`DB_SOCKET` onto
   Doctrine's `host`/`unix_socket` parameter names for any of the three
   apps — that mapping crosses a framework layer (Laravel's own
   `Illuminate\Database` connector) this module doesn't model at all, the
   exact "presence isn't wiring" trust-boundary lesson K2c's census
   flagged. Consumer support for these three apps stays honestly
   `unsupported`, not guessed — acquiring producer evidence was the only
   job this round.

   **Real acquisition, all three verified for real via `nix eval --json`**
   (new `eval_nix_json`, parallel to `eval_nix_raw` but `--json` instead
   of `--raw` — the evidence here is a whole attrset, not one string):
   agorakit's `services.agorakit.config` (`DB_HOST`/`DB_PORT`/
   `DB_DATABASE`/`DB_USERNAME`/`DB_PASSWORD`, no socket-equivalent key);
   snipe-it's `services.snipe-it.config` (same keys, **plus** a dedicated
   `DB_SOCKET`); movim's `services.movim.settings`, evaluated with
   `database.type = "postgresql"` rather than `"mariadb"` — the `mariadb`
   path hits a real, independently confirmed nixpkgs bug at
   `movim.nix:628` (`config.services.${cfg.database.type}.settings.port`
   interpolates the enum value `"mariadb"` directly as a `services.<x>`
   attribute name, but the module's own real service registration a few
   lines later is `services.mysql`, not `services.mariadb` — the
   attribute lookup fails outright). Documented in `src/cdc.rs`'s doc
   comment on `acquire_movim_evidence`, not fixed (a different defect
   class from this module's own subject matter, out of scope for K2d) and
   not silently routed around — `postgresql` is a correctly-wired sibling
   path in the same module, used here only to prove the acquisition
   mechanism.

   **Stop condition, all 5 items met**: all 3 apps through ONE
   `FlatEnvVars` path (✓, one `build_flat_env_vars_evidence` function, no
   per-app branch); zero app-name branches in comparison (✓ —
   `compare_contract` wasn't touched at all, and nothing calls it with
   `FlatEnvVars` evidence); real env key/values via `nix eval` (✓, all
   three re-verified live, not read from cached module source); malformed/
   empty/non-object evidence is `Inconclusive` (✓ — 4 new offline tests:
   a JSON array, a JSON scalar, an empty object, plus one success case
   asserting the full sorted key set and `emitted_key() == None`);
   existing 12 real K1/K2a/K2b tests unchanged (✓ — re-ran for real,
   identical pass count and behavior). 84 tests total in the `oba`
   binary's own unit-test target (was 77): 69 offline (was 65, +4) + 15
   real/ignored (was 12, +3). Confirmed green in actual CI, not just
   locally: `gh run view --log` on the pushed commit (`3bc0bfa`) shows
   `15 passed; 0 failed; 0 ignored; ... 69 filtered out` in 42.8s on a
   fresh runner.

   **K2d FROZEN at `45d740d`** (the documented-completion commit — `3bc0bfa`
   is the implementation-point, the commit where the code and real tests
   actually landed). Same standing rule as every other frozen layer in
   this project: reopened only by a concrete counterexample, not for
   architectural cleanup or speculative completeness.
6. **K2a.1 — a second, explicit `composer.lock` provenance source.
   Closed, `dce5660`, confirmed green in real CI (`gh run view --log`:
   21 passed, 0 failed, 45.7s).** The nixpkgs-local locator gap K2c found
   recurring 3× (`flarum`/`baikal`/`postfixadmin`'s `pkgs.<attr>.src +
   "/composer.lock"` fails outright — upstream ships no lock file, so
   nixpkgs vendors its own). Deliberately NOT "learn to find any
   `composer.lock`" — a second, explicit provenance source alongside the
   already-working one, held to a tight model: `resolve_composer_lock`
   tries the package's own fetched source first (K2a's original path,
   `fetch_composer_lock`, completely UNCHANGED — Kimai/Davis's existing
   call sites untouched), and a `nixpkgs_local` candidate resolved
   through ONE explicit package-expression relationship —
   `pkgs.<attr>.composerVendor.composerLock` — never a directory walk for
   the nearest file named `composer.lock`, exactly the kind of "convenient
   false-confidence generator" ruled out up front.

   **The relationship is real, not assumed** — confirmed by reading
   `pkgs/build-support/php/builders/v2/build-composer-project.nix` itself:
   `composerVendor = args.composerVendor or (php.mkComposerVendor { ...;
   composerLock; ...})`, and `lib.extendMkDerivation`'s own merge
   semantics (documented in `lib/customisation.nix`: the overlay is
   applied *before* passing to `constructDrv`, on top of the original
   `args`, not instead of them) carry the caller's `composerLock`
   argument through onto the resulting derivation's own attribute set
   even though `mkComposerVendorOverride` never re-exports it explicitly.
   So `.composerVendor.composerLock` isn't nixpkgs convention being
   guessed at — it's asking Nix to hand back the exact same path value
   the package expression itself declared, through the one relationship
   that actually connects them.

   **Priority when both candidates exist, proven not assumed**: the spec
   sketch had source-first, local-as-fallback; reading the same builder
   file turned up a stronger fact — when `composerLock` is non-null, the
   real build feeds it straight to `composer install`, and never reads
   whatever `composer.lock` might also happen to sit inside `src`. So
   `NixpkgsLocal` wins whenever present, not by "first candidate wins"
   convention but because that is what the actual build does — checked
   by a dedicated offline test using two deliberately different
   synthetic contents, so the priority is exercised by construction, not
   merely asserted in a comment. No real corpus app currently has both
   candidates present, so this priority is enforced but not yet
   corpus-exercised — disclosed, not glossed over.

   **All 6 stop-condition items met**: `flarum`/`baikal`/`postfixadmin`
   each auto-resolve the correct local lock (✓, real `nix eval`, not
   read from cached source — `flarum` additionally resolves a genuine
   `doctrine/dbal` 2.13.9 entry via the unchanged `resolve_consumer_identity`;
   `baikal`/`postfixadmin` genuinely have none, now for the confirmed
   reason instead of a blocked locator); Kimai/Davis/strichliste's
   source-based path is completely unchanged (✓ — `fetch_composer_lock`
   untouched, plus a new regression test proving the unified resolver
   still lands on `PackageSource` for all three — `strichliste`'s first
   automated real test in this module, previously only verified manually
   in K2c's own census transcript); both-present resolves by proven
   priority, never "first" (✓, see above); the local lock is tied to a
   real package derivation/expression, never merely adjacent on disk (✓
   — `.composerVendor.composerLock`, not a path search); missing/
   ambiguous/mismatched identity is fail-closed (✓ — `Inconclusive` when
   neither candidate resolves, unchanged fail-closed behavior in
   `resolve_consumer_identity` downstream); `resolve_consumer_identity`
   never learns which source its input came from (✓ — it still takes a
   plain `&str`, unchanged signature; `ComposerLockOrigin` is carried
   only in the new `ResolvedComposerLock` wrapper, one layer up).

   `ComposerLockOrigin` (`PackageSource | NixpkgsLocal`) added for
   provenance, not architectural decoration — a Finding can now honestly
   say which source its consumer contract came from. Not wired into any
   report/Finding type yet; this module still has no CLI/report
   integration at all.

   **A real, incidental corpus finding, deliberately NOT fixed in this
   round**: `movim`'s `package.nix` calls `php.mkComposerVendor` directly
   with its own argument list (bypassing `buildComposerProject2`'s
   automatic `composerLock` forwarding entirely), so
   `pkgs.movim.composerVendor.composerLock` is not `null` but genuinely
   ABSENT as an attribute — a real, different shape from every other
   surveyed app (all of which have the attribute present, `null` or a
   path). Harmless for K2a.1 (`movim` isn't in its target scope — its
   consumer support stays unsupported for the unrelated K2d reason,
   Laravel's own DB connector), collapsed correctly by an `or null` guard
   regardless — recorded here as a corpus finding with its exact
   reproduction, same discipline as movim's `mariadb`-path bug from K2d.
   Not folded into "let's also fix movim while we're here" — the user's
   own explicit instruction this round, to keep the roadmap from
   re-branching on every incidental discovery.

   4 new offline tests + 6 new real tests. 94 tests total in the `oba`
   binary's own unit-test target (was 84): 73 offline (was 69, +4) + 21
   real/ignored (was 15, +6).

   **K2a.1 FROZEN at `acce174`** (the last documentation commit — `dce5660`
   is the implementation-point). The user's own framing for why the
   local-wins-when-present priority (a real deviation from the original
   spec sketch's source-first ordering) is the right call to keep, not a
   liberty taken: "Спецификация обязана проиграть фактам, когда
   выясняется, что была придумана неправильно" — a specification loses to
   facts once it turns out to have been designed on a wrong assumption;
   the alternative is treating the spec as dogma over the actual
   `buildComposerProject2` semantics that were read and verified for
   real. Same standing rule as every other frozen layer: reopened only by
   a concrete counterexample.

7. **K2e — consumer reachability census. Closed, census-only, zero
   `src/cdc.rs` changes.** Motivated directly by K2c's own unresolved
   finding: `doctrine/dbal`'s presence in `composer.lock` is necessary,
   not sufficient, evidence it's the actual runtime consumer of what Nix
   emits. Before building another `ProducerEvidence` variant or a new
   consumer adapter on a guess, this round traced the REAL chain — Nix-
   emitted key → app/framework config layer → actual runtime consumer —
   for every one of the 8 corpus apps where `doctrine/dbal` is present at
   all, from real pinned source at an exact tag per app, not framework
   reputation. New 4th independent axis in `census.md`: `consumer path`
   = `direct` / `mediated-known` / `mediated-unknown` / `not_applicable`.

   **Result: `direct` 2 (kimai/davis, already proven by K1) + `mediated-known`
   6 + `mediated-unknown` 0 + `not_applicable` 8** (`doctrine/dbal`
   absent, the question doesn't arise). Full chains with file:line
   citations are in `census.md`'s own K2e section; summary:
   - **`strichliste`/`part-db`**: `mediated-known`, but the terminal
     contract is unchanged from the direct case — Symfony's
     `doctrine/doctrine-bundle` eagerly calls Doctrine's OWN
     `Doctrine\DBAL\Tools\DsnParser` class one stack frame earlier than
     `DriverManager::getConnection()` would, with zero key renaming.
     These need no new adapter at all — K1's existing model already
     covers them once Phase D vendors `doctrine/dbal` 3.10.5 (already
     identified in K2c, still not done).
   - **`agorakit`/`movim`/`snipe-it`**: `mediated-known`, terminal
     library `Illuminate\Database`, `doctrine/dbal` CONFIRMED vestigial
     — Laravel 11 deleted every DBAL schema-bridge method
     (`Illuminate\Database\Schema\Grammars\MySqlGrammar` now generates
     raw `ALTER TABLE` SQL natively), and all three still carry the
     now-pointless direct `composer.json` requirement. `DB_HOST`/
     `DB_PORT`/`DB_DATABASE`/`DB_USERNAME`/`DB_PASSWORD` (+ `DB_SOCKET`
     → `unix_socket` for snipe-it, a direct 1:1 rename) map straight
     into `config/database.php`'s connection array.
   - **`flarum`**: `mediated-known`, same terminal library
     (`Illuminate\Database\Capsule\Manager`), but `doctrine/dbal` is
     real here, not vestigial — narrowly required to unlock Illuminate's
     optional `renameColumn()`/`dropColumn()`/`change()` schema
     operations during migrations, never touching the primary
     connection built from the Nix-generated `config.php` values.

   **The actionable signal, exactly as intended**: 4 of the 6
   `mediated-known` apps (agorakit, snipe-it, flarum, movim) converge on
   ONE chain shape — `DB_*`/config-array env keys → a Laravel-family
   `config/database.php`-equivalent → `Illuminate\Database`. That is
   real, repeated, corpus-backed support for a future
   Laravel-config-aware consumer path, not a guess dressed up as one.
   The other 2 (`strichliste`, `part-db`) need zero new machinery.

   Explicitly **not decided or built this round**: no new
   `ProducerEvidence` variant, no consumer-adapter class, no framework
   config parser. K2e's own job was only to make that decision
   mechanical for whoever picks it up next, per the user's own framing —
   the choice is now genuinely informed, not a hunch.
8. **K2f — Laravel/Illuminate consumer adapter. Closed, `9e03eca`,
   confirmed green in real CI (`gh run view --log`: 25 passed, 0 failed,
   35.5s).** The corpus-justified choice from K2e, picked explicitly over
   package-bump drift and the strichliste/part-db Phase D vendoring
   (both real, both cheaper, both deliberately deferred — user's own
   reasoning: this is "the only next step both corpus-confirmed AND
   expanding the checker's actual semantic power, not just closing a
   technical tail"). Held to a tight, fully-specified model: `Nix
   FlatEnvVars -> DB_* keys -> config/database.php -> Laravel config
   structure -> Illuminate\Database -> consumer contract`, with the
   explicit rule that the contract must come from Illuminate, never
   Doctrine, even though `doctrine/dbal` sits right there in
   `composer.lock` — confirmed vestigial for exactly this reason in K2e.

   **Reachability made explicit type, not hidden in an extractor** — the
   design review's own sketch, built close to literally:
   `ConsumerHop { layer, from_key, to_key }`, `ConsumerRoute::Direct |
   Mediated { hops }`, `ConsumerContractEvidence { route,
   consumer_library, accepted_keys }`. Phase E's `compare_contract`
   (frozen, K1) is reused completely UNCHANGED against
   `ConsumerRoute::resolved_key()` — no type-level room for a mediated
   route to become "stronger" or "weaker" than a direct one, same
   discipline K2b established for `ProducerEvidence`.

   **Two new pure extractors, both bounded literal scans** — Phase D's
   own reuse-first discipline applied a second time, not reinvented:
   `extract_config_key_for_env_var` (the exact `'<key>' => env('<ENV_VAR>'
   ...)` shape `config/database.php` uses, one simple cast tolerated;
   `Ok(None)` when the env var isn't present at all is a real, valid
   outcome — not every `FlatEnvVars` key is part of the DB config
   surface — never an error) and `illuminate_connector_accepts_key`
   (does the vendored Illuminate connector source reference a key at
   all, via array-keyed `$config['host']` (MySQL) or boundary-checked
   bare-variable `$host` (Postgres, after its own `extract($config,
   EXTR_SKIP)`) — deliberately NOT `isset(...)`-only, since `host`/
   `database` are read unconditionally by both drivers, confirmed by
   reading the real vendored source directly).

   **A real research finding that reshaped the design before any code
   was written**: MySQL's and Postgres's Illuminate connectors are
   structurally different, not just differently-named — MySQL's
   `MySqlConnector.php` uses array-keyed `isset($config['unix_socket'])`;
   Postgres's `PostgresConnector.php` uses `extract($config, EXTR_SKIP)`
   then bare-variable checks, and has **no unix-socket concept at all**
   (no `hasSocket()`-equivalent exists there, confirmed by reading the
   real pinned source). The socket-naming defect class this whole
   project is built around structurally cannot occur for a Postgres
   deployment via Illuminate — a real, disclosed finding, not a gap.
   Resolved by branching the adapter on `IlluminateDriver` (a property of
   how a deployment configures its connection), never on app identity —
   satisfies "no `if app == ...`" while still genuinely covering movim.
   Also caught for real during research: snipe-it's `config/database.php`
   maps `DB_SOCKET` -> `unix_socket` identically in BOTH its `mysql` and
   `mariadb` connection blocks — confirmed this is NOT ambiguity (same
   distinct key twice), which is why the extractor checks distinct
   resolved values rather than raw occurrence count.

   **Illuminate connector fixtures vendored and integrity-locked**
   (`fixtures/cdc/illuminate-database/`, same discipline as K1's Doctrine
   fixture) — `MySqlConnector.php`/`Connector.php` verified BYTE-IDENTICAL
   (diffed directly, not assumed) between agorakit's pinned
   `laravel/framework` v11.44.2 and snipe-it's pinned v12.59.0; one
   fixture serves both, citation covers both commits.
   `PostgresConnector.php` is movim's own pinned `illuminate/database`
   v12.69.2, a structurally different file, not shared with the MySQL
   pair.

   **All 5 stop-condition items met**: ONE shared adapter
   (`acquire_illuminate_consumer_contract`), zero app-name branches
   anywhere, works on all three (✓ — real tests below); mapping derived
   from exact pinned application/framework source, never a hardcoded
   env-var list (✓ — both extractors read real vendored/fetched bytes);
   missing/ambiguous mapping is `Inconclusive`, never guessed (✓ — 6 new
   offline tests covering absent/duplicate/conflicting/malformed shapes);
   `doctrine/dbal`'s presence never affects these three apps' verdict (✓
   — structurally true, nothing in this path references `doctrine/dbal`
   or `composer.lock` at all); Kimai/Davis/Symfony paths unchanged (✓ —
   zero K1/K2a/K2b/K2d/K2a.1 code touched, all 21 pre-existing real tests
   pass unchanged).

   **Real-verified for all three**: agorakit's `DB_HOST` -> `host`
   (MySQL) — Pass; snipe-it's `DB_SOCKET` -> `unix_socket` (MySQL) — Pass,
   the exact K1-defect-class key, via a fully mediated route, never
   Doctrine's contract; movim's `DB_HOST` -> `host` (Postgres, a
   genuinely different connector source) — Pass; movim's `DB_SOCKET` —
   `Inconclusive`, correctly, for the real reason above. A mutation-style
   positive control (`consumer_contract_evidence_mismatched_key_is_a_real_finding`)
   renames the mapped key and confirms the detector flips from `Pass` to
   a real `Finding`, not a vacuous always-pass.

   **`flarum` deliberately NOT required for closure**, per the design
   review: its own chain (same terminal library, `doctrine/dbal` real but
   migrations-only, per K2e) is a genuine multi-consumer case the review
   explicitly said to record as a future corpus case rather than distort
   v1 for "pretty 4/4" — not attempted this round.

   111 tests total in the `oba` binary's own unit-test target (was 94):
   86 offline (was 73, +13) + 25 real/ignored (was 21, +4).
   `tests/fixture_integrity.rs` passes with the 3 new vendored files'
   sha256 locks. **Real, disclosed CI gap found while verifying this
   round, pre-existing, not introduced here**: no workflow runs a plain,
   unscoped `cargo test` — `k1.yml` only ever runs `cargo test --bin oba
   -- --ignored "cdc::"`, so `tests/fixture_integrity.rs`/`tests/golden.rs`
   and the rest of the `tests/*.rs` black-box suites are exercised
   locally only, never in CI. Not fixed this round — flagged, not glossed
   over, same discipline as every other gap this project has found in
   itself.

   **K2f FROZEN at `9e03eca`.** The design review's own framing for why
   this round is particularly worth freezing rather than just closing:
   the new special case (`IlluminateDriver`, mysql vs postgres) landed at
   the level of a real interface, not an app name — exactly the
   universality test asked of every prior round. Same standing rule as
   every other frozen layer: reopened only by a concrete counterexample.

Reviewed next-step order, deliberately narrow (two small, cheap items
before any more intelligence gets added to the checker):
9. **CI qualification gap — infrastructure only, not analysis. Closed,
   `1e39345`, confirmed green in real CI — and confirmed by log, not
   just a checkmark.** Closes the gap K2f's own writeup disclosed:
   `tests/*.rs` (the integration suites this project treats as part of
   its own proof chain) had never run in CI, only locally.
   `.github/workflows/test.yml`: one small job, `dtolnay/rust-toolchain@stable`
   (reuse, same action `k1.yml`/`kani.yml` already use) + a plain
   `cargo test` — no `#[ignore]`d real-eval test runs here (that's
   `k1.yml`'s own job, untouched), no restructuring of any existing
   workflow. `gh run view --log` on the real run shows all 5 targets
   genuinely executing, each with its own `Running`/`test result: ok`
   pair, not inferred from a green checkmark: `unittests src/main.rs`
   (86 passed, 25 ignored), `tests/check_root.rs` (9 passed),
   `tests/diff_cli.rs` (11 passed), `tests/fixture_integrity.rs`
   (1 passed), `tests/golden.rs` (47 passed) — 154 tests total, 37s.
10. **K2g — Phase D vendoring for `strichliste`/`part-db`. Closed,
    `86c83e4`, confirmed green in real CI (`gh run view --log`: 27
    passed, 0 failed, 41.4s).** The user's own pick among the three
    K2e/K2f-opened options (over `flarum`'s multi-consumer case and
    package-bump drift) — two real corpus cases, zero new semantic model
    needed (K2e already qualified their consumer route as Symfony's
    bundle → Doctrine's own `DsnParser`, contract unchanged), a cheap
    way to grow real end-to-end coverage and confirm K2a's/K2e's
    conclusions actually compose into one pipeline rather than staying
    separately-proven facts. Reuses the ENTIRE existing K1 pipeline
    unchanged (Phase C/D/E, K2a's `resolve_consumer_identity`/
    `fetch_composer_lock`, K2b's `build_sentinel_flow_evidence`) — no new
    comparison semantics, only two new real acquisition functions and
    two new vendored consumer fixtures.

    **`strichliste`**: `environment.DATABASE_URL` is a real option,
    sentinel-injected exactly like Kimai's `database.socket`. Pins
    doctrine/dbal 3.10.5 — fetched that exact commit's own MySQL driver
    and diffed it directly against the already-vendored 3.10.6 fixture:
    BYTE-IDENTICAL. Reused rather than duplicated (same "verify before
    vendoring a duplicate" discipline K2a.1 already established for the
    Illuminate MySQL connector) — no new fixture file for this one.

    **`part-db`**: a genuinely different real finding. Its
    `settings.DATABASE_URL` renders into `envFile`, a real
    `pkgs.writeText` derivation — reading it requires REALIZING that
    (tiny, cheap) derivation, a real if minor difference from Kimai's
    purely-evaluated string. More importantly: part-db's real Postgres
    deployment has **no separate `unix_socket` DSN parameter at all** —
    confirmed by reading both `part-db.nix`'s own default
    (`host=/run/postgresql`) and the pinned doctrine/dbal 4.4.3 Postgres
    driver's `constructPdoDsn` directly (no `unix_socket` key anywhere
    in it). Postgres overloads `host=` for both a TCP hostname and a
    unix-socket directory path, so the sentinel goes into `host=`
    instead — `extract_key_for_value` (Phase C, completely unchanged)
    correctly resolves `emitted_key = "host"`, not `"unix_socket"`,
    proving Phase C was never hardcoded to one specific key name. First
    Postgres-dialect Doctrine fixture vendored in this project.

    `vendored_doctrine_source_reference` generalized into
    `vendored_fixture_source_reference(path)`, parameterized by vendored
    path — the original K1/K2a call site's own behavior is completely
    unchanged, that's the entire change to that function.

    2 new real tests (`strichliste_golden_is_pass`,
    `part_db_golden_is_pass`), both full end-to-end `Pass` verdicts on
    the CURRENT real deployment — no historical before/after pair exists
    for these two apps (unlike Kimai/Davis), so this is honest
    coverage-extension, not fabricated defect-reproduction. 27 real
    tests total (was 25, +2); zero offline tests added (no new pure
    logic); `fixture_integrity` passes with the new sha256 lock.

Explicitly **not next yet**, deliberately, not from lack of interest:
`flarum`'s multi-consumer shape (kept as a future *adversarial* corpus
case on purpose — one instance shouldn't force multi-consumer semantics
into the type system before a second one shows the shape actually
repeats) and package-bump differential drift (explicitly sequenced
AFTER strichliste/part-db close, so drift gets checked against a real,
multi-path consumer-reachability model — direct Doctrine, Symfony-
mediated-but-same-contract, Laravel/Illuminate MySQL, Laravel/Illuminate
PostgreSQL — instead of naive "dependency is present," which is exactly
the kind of fast false-alarm generator this whole project exists to
avoid). Also explicitly not touched: `ConsumerRoute`'s own types, even
though movim's Postgres `Inconclusive` result already hints at a future
distinction worth having (`Inconclusive` = "couldn't prove it" vs. a
possible future "provably no such contract exists for this
consumer/driver") — deliberately deferred until a second real corpus
case shows the same shape, not designed speculatively off one instance.

## Productization: from research phase to a usable CI product

The semantic core has now been through H1 freeze, H2.2, hostile review,
64-mutant mutation testing, and KANI-0 — deliberately declared "battle
enough" rather than kept growing indefinitely. The next phase is turning
`oba` from a spike into something a `nixpkgs` fork's CI can actually
depend on, not adding more analysis smartness.

**CI architecture decision: ratchet / base-vs-head comparison belongs to
`oba` itself, not to the GitHub Action.** The Action's job is limited to
acquiring both roots, installing a pinned `oba` binary, invoking it
*once*, and rendering the report. Reuse survey: `nixpkgs-vet` — the closest
real precedent, same ratchet philosophy (old violations grandfathered, new
ones blocked) — was checked directly against its live source rather than
assumed: `NixOS/nixpkgs`'s own `.github/workflows/lint.yml` invokes it as
`nix-build ... -A nixpkgs-vet --arg base ./nixpkgs/trusted --arg head
./nixpkgs/untrusted`, i.e. **one invocation, given both trees**, not two
independent runs whose output gets diffed by shell glue in the workflow.
Rationale for following the same shape, beyond precedent: ratchet
semantics (what counts as a regression / improvement / grandfathered /
new-target / target-definition-changed) is analysis logic, not CI
plumbing — it belongs somewhere testable without Git or GitHub in the
loop. A single invocation also guarantees base and head are analyzed by
the exact same `oba` version and the exact same comparison rules, and
keeps the Action from slowly growing a second, slightly-worse analyzer in
`jq`/bash as ratchet edge cases accumulate. (One correction to the
precedent as originally cited: `nixpkgs-vet`'s GitHub Releases shipped a
prebuilt `x86_64-linux.nar.gz` closure through `v0.3.0` but ship no
release assets at all as of the current `v0.3.4` — not treated as an
active alternate-delivery pattern to fall back on.)

Planned CLI shape (PR B): `oba check --root <dir> --targets <manifest>`
(today's single-tree mode, kept) alongside a new `oba diff --base-root
<dir> --head-root <dir> --targets <manifest>`, backed internally by
`analyze(root, targets) -> AnalysisReport` and `compare(base: &AnalysisReport,
head: &AnalysisReport) -> DiffReport` — Git/GitHub stays entirely outside
this boundary, which is also what makes it testable without either. The
versioned JSON envelope (`schema_version: 1`) is designed up front to
carry both modes (`"mode": "check"` vs `"mode": "diff"`, the latter with
`base`/`head`/`transition` per target) so `schema_version` doesn't have to
bump the week after v1 ships when the diff mode lands. The exact set of
`transition` values is deliberately NOT frozen until PR D actually
implements them.

Open trust question for PR D, deliberately not resolved in PR A/B: if
`targets.toml` lives in the audited repository itself, an untrusted PR can
edit the manifest to make an inconvenient target disappear. Needs an
explicit policy (existing targets keyed off the trusted/base manifest;
new targets from head allowed in but can't remove/weaken existing ones;
manifest changes are themselves part of the diff report) before ratchet
mode is CI-trustworthy — real ratchet-mode nuance, belongs inside `oba
diff`, not as ad hoc `jq` in the Action.

Planned sequencing, each an independently landable/provable PR, not one
mega-commit:

- **PR A — Distribution bootstrap. Closed: `v0.1.0` tagged, released, and
  smoke-tested for real.** `dist init -y --ci github -t x86_64-unknown-linux-musl
  -i shell` (`dist` v0.33.0, installed from its own prebuilt release
  binary rather than `cargo install`, on the same "don't compile it if a
  binary exists" principle this PR applies to `oba` itself) generated
  `dist-workspace.toml` + `.github/workflows/release.yml` + `[profile.dist]`
  in `Cargo.toml`; targets trimmed by hand from `dist`'s 6-platform
  default down to just the one actually needed (`ubuntu-latest`-hosted
  nixpkgs CI), `github-attestations = true` added explicitly (off by
  default). Validated with `dist plan` (no real build): one
  `oba-x86_64-unknown-linux-musl.tar.xz` + shell installer + per-artifact
  and global sha256 checksums, tag `v0.1.0`. Reuse survey: `cargo install`
  as the Action's install path was rejected outright — a consumer's CI
  shouldn't compile this project's analyzer on every PR; `dist` exists
  specifically to generate the release CI and ship prebuilt binaries
  instead. `taiki-e/install-action` (checksum verification +
  `cargo-binstall` fallback) is a plausible *additional* install channel
  later, once the release layout is stable — not a replacement for
  `oba`'s own composite Action, which also has to do root
  acquisition/orchestration `install-action` has no opinion on.
  License gap (no `LICENSE` file existed at all) closed first, dual
  `MIT OR Apache-2.0` — same choice Kani itself ships under, and this
  project's closest neighbor besides. Real release:
  <https://github.com/PhysShell/nix-option-branch-audit/releases/tag/v0.1.0>.
  Acceptance was deliberately NOT "the workflow went green" — a fresh
  scratch directory downloaded the actual published asset (never reusing
  the local build that produced it): `sha256sum -c` against the published
  checksum, `ldd` confirming a truly static binary, `oba --help` runs,
  `gh attestation verify --format json` returns a real signed bundle
  whose certificate SAN names this exact repo/workflow/tag
  (`.github/workflows/release.yml@refs/tags/v0.1.0`), and the published
  `oba-installer.sh` piped through `sh` into an isolated `CARGO_HOME`
  installs a working binary end to end.
- **PR B — CLI / report foundations. Closed.** `2b4b19c` / `a3de569` /
  (this commit). Deliberately does NOT include a working `oba diff` or
  `compare()` — a command that exists implies it guarantees something;
  shipping a decorative one would have quietly become half of PR D two
  PRs early. The actual split:
  ```
  PR B: analyze(root, manifest) -> AnalysisReport
        oba check
        versioned, extensible report envelope (schema_version: 1)
  PR D: compare(base, head) -> DiffReport
        oba diff
        ratchet semantics
  ```
  Three commits, each independently provable:
  1. `analyze(root: &Path, manifest: &TargetFile) -> AnalysisReport`
     extracted out of `run()`'s inline loop, called with `root = "."` —
     behavior-preserving by construction (proved by the full 72-test
     suite staying green unchanged, not just claimed).
  2. `oba check --root <dir> --targets <manifest>`, with `--root` as a
     real filesystem boundary: `module`/`test` paths are resolved through
     `resolve_within_root`, which canonicalizes and hard-rejects an
     absolute path, a `../` escape, or a symlink resolving outside
     `root` — TOOL_ERROR (exit 3), not a read from wherever it points.
     Matters because `--targets` can name a manifest living inside the
     very repository under analysis, editable by the same untrusted PR
     being audited. `--targets` itself stays resolved relative to cwd,
     not `--root` — deliberately the odd one out, so PR D can apply one
     manifest to two different roots without "relative to which root?"
     ever being ambiguous for the manifest path itself. Legacy flat `oba
     --targets <manifest>` still works, routed through the exact same
     `run_check()` as `check --root . --targets <manifest>` — proved
     equivalent (byte-identical JSON on the real golden manifest), not
     just similar. `--census` untouched.
  3. The versioned envelope: `{schema_version, tool: {name, version},
     mode, summary: {pass, finding, inconclusive}, targets}`. `mode` is
     reserved as a sum-type tag (`"check"` today) specifically so `"diff"`
     lands in PR D without a `schema_version: 2` bump — but no
     `transition`/`base`/`head` shape is guessed at before PR D actually
     needs one. No absolute `--root`, timestamps, hostname, or PID in the
     payload — `Span.file`/module/test strings were already the
     manifest's own root-relative text, never a canonicalized path, so
     this fell out of commit 2 rather than needing new work. Crate bumped
     to `0.2.0` (a real machine-contract change, not a patch release).

  New `tests/check_root.rs` (9 black-box subprocess tests): all three
  escape vectors → exit 3 (two needing no new fixtures at all — rejection
  is syntactic for an absolute path, and `../../Cargo.toml` already
  exists two levels above `fixtures/synthetic`; the symlink case uses one
  new checked-in relative symlink, `fixtures/synthetic/root-escape/root/
  evil -> ..`); missing `--root` → exit 3; legacy vs `check --root .`
  byte-identical JSON; running from a totally different cwd with an
  explicit `--root` reproduces the same report; two runs of the same
  invocation are byte-stable; `--census` unaffected; the envelope's own
  shape (`schema_version`/`mode`/`tool.name`/`tool.version`, no stray
  top-level keys, `pass + finding + inconclusive` actually sums to the
  real verdict count). Golden suite's own summary assertions updated for
  the renamed/reshaped fields (`findings` → `finding`, `status` string
  dropped — derivable from the counts, wasn't adding a distinct claim).
- **PR C — Thin GitHub Action. Closed.** `action.yml`, a composite action
  (not Docker, not Node): install a pinned `oba` release, run `oba check
  --root <root> --targets <targets>`, pass its exit code straight through
  as the step's own outcome, hand back the raw `schema_version: 1` JSON
  as both a step output (`report-json`) and an uploaded artifact.
  **Deliberately narrower than first planned**: no JSON parsing, no
  `::error`/`::warning` annotations, no `$GITHUB_STEP_SUMMARY` table —
  that would be this project's own analysis semantics leaking into CI
  glue, the exact thing the productization decision (single-invocation
  ratchet living inside `oba`, not the Action) was written to prevent one
  layer up. Rendering the JSON into annotations is a separate, later
  concern built *on top of* this output, not inside it. SARIF stays
  deferred for the same reason it always was.

  Self-dogfood, added in this same PR rather than left for later:
  `.github/workflows/dogfood.yml` runs this repo's own `action.yml`
  (`uses: ./`, pinned to a real released tag, not `latest`) against this
  repo's own `targets/clean.toml` (must exit 0) and `targets/golden.toml`
  (must exit 2, `continue-on-error: true` + an explicit
  `steps.*.outcome == 'failure'`/`exit-code == '2'` check — the same
  pattern `model-checking/kani-github-action`'s own `test-action.yml`
  uses for its invalid-version case, confirmed real-world precedent, not
  invented here). Not for coverage — `cargo test` already covers the
  analysis far more thoroughly — but to prove install/cwd/paths/exit-code
  passthrough/JSON output actually work together on a real runner, not
  just individually look correct. It earned that: a *local* bash
  simulation of the install+run steps (real `curl | sh` against the real
  `v0.2.0` release, `$GITHUB_OUTPUT`/`$GITHUB_PATH` faked as plain files)
  caught one bug before anything was pushed — the JSON report is
  pretty-printed (`"schema_version": 1`, with a space), so a first-draft
  `grep -q '"schema_version":1'` (no space) would have silently never
  matched; switched to `jq -e`. But three more bugs were only visible on
  a REAL GitHub-hosted runner, invisible to any local simulation because
  they're runner/JS-action/OS-level, not bash semantics:
  1. `github.action_path` for a local `uses: ./` reference resolves to a
     path ending in `/.` — `actions/upload-artifact@v4`'s glob matcher
     hard-rejects any `.`/`..` path segment outright, independent of
     whether the file exists. Fixed: write the report under
     `$RUNNER_TEMP` instead (a clean absolute path GitHub provides for
     exactly this, identical whether the action is referenced locally or
     as `owner/repo@ref`).
  2. Splicing `${{ steps.X.outputs.report-json }}` straight into a
     `run:` bash block is a real script-injection shape, not a style
     nit — the JSON embeds raw Nix source text (parens, quotes),
     GitHub's own substitution happens as literal text *before* bash
     parses anything, and it broke a single-quoted `echo` with a genuine
     syntax error. Fixed: pass values through `env:` instead. Promoted
     to a standing project rule in `AGENTS.md` — never splice an
     output through `${{ ... }}` into `run:`, even the tool's own,
     since its content (Nix source) is effectively untrusted from the
     templating engine's point of view; a later PR rendering
     annotations/a summary must do so by parsing the JSON in real code,
     never by `echo`-and-grep on a templated expression.
  3. **The architecturally significant one**: `env:` has its own real
     size ceiling, independent of GitHub's documented 1MB-per-output
     cap. `golden.toml`'s real report is ~390KB (28 targets) — passing
     that through `env:` hit the OS's own `execve()` argument+
     environment limit ("Argument list too long") well before 1MB. Not
     "fix the test cleverer" — this is a real signal about the
     `report-json` *output*'s own design, not a dogfood-workflow bug:
     fighting GitHub's and the OS's size limits further isn't a fight
     worth having. Contract decided (not yet re-implemented — no need
     to reopen this PR for it, the documented caveat below is enough for
     now): the **artifact** is the canonical full report; `report-json`
     stays a convenience output for genuinely small checks, with its
     size ceiling stated plainly in `action.yml` itself; a
     `report-path` output (the artifact's path on the runner's own
     filesystem, so a downstream step in the *same* job can `jq`/upload
     elsewhere/keep provenance without threading hundreds of KB through
     expression/env/output plumbing at all — the filesystem is still a
     perfectly good IPC mechanism) is the natural next small contract
     addition, tracked for whenever `action.yml` next changes, not
     urgent enough to reopen this PR on its own. Only small, genuinely
     bounded values (`exit-code`, and later `pass`/`finding`/
     `inconclusive`/`schema-version` mirrored as their own outputs, not
     just embedded in the JSON) belong as first-class outputs going
     forward — never another whole report.
- **PR D — Native differential/ratchet mode. Design note, not started —
  written before any diff code, on purpose.** The two-root model is the
  most dangerous single step left in this roadmap: get it wrong and
  "diff" quietly becomes "ran twice, compared two JSON blobs", which
  looks fine right up until a manifest edit or a renamed fixture makes it
  lie. Questions to answer explicitly before writing `compare()`, not
  discovered mid-implementation:
  - **`base_root` / `head_root`**: two independent filesystem roots,
    each analyzed through the *existing* `analyze()` unchanged — `oba
    diff`'s only new code is `compare(base: &AnalysisReport, head:
    &AnalysisReport) -> DiffReport`, a pure function over two already-
    complete reports, never touching Git/GitHub/filesystems itself (same
    "can this be tested without CLI/git/GitHub" bar as `analyze()`
    itself already meets).
  - **One manifest or two?** One, by construction: `oba diff --base-root
    --head-root --targets <manifest>` applies the *same* manifest to
    both roots (this is exactly why `--targets` was made cwd-relative,
    not root-relative, back in PR B commit 2 — so this question doesn't
    come up as an ambiguity here). Whether an untrusted PR can smuggle
    manifest changes past this is the separate trust-policy question
    below, not a second-manifest design.
  - **Target identity across revisions — decided, not open**: a
    positional/array index or the free-text `name` is not a stable key
    (reordering `targets.toml` must never itself read as "changes").
    Identity is the canonical tuple `(module, test, cfg_ident,
    option_prefix, watched_path)` — i.e. per *watched option*, not just
    per target block, since that's the actual unit a verdict is computed
    for. An explicit manifest-provided `id` field remains an option if
    the canonical tuple ever proves too brittle in practice, but isn't
    added speculatively before a real case demonstrates the need.
  - **Added / deleted / renamed**: a target/watched-option present in
    head only is `new`; present in base only is `removed`; a `module`/
    `test` path edit changes the identity tuple above, so it surfaces as
    a `removed` + `new` pair rather than a mis-detected "same target,
    different verdict" — correct per the identity rule, but worth
    stating explicitly so it isn't mistaken for a bug later.
  - **Does identity survive a rename/move — decided, not open**: no.
    Identity is *syntactic*, not semantic — `foo.nix` renamed to
    `bar.nix` (even with byte-identical content) changes the identity
    tuple, so v1 reports it as `Removed` + `Added`, never a detected
    "same logical target, different path". No rename tracking, no
    content-similarity heuristic, no cross-revision identity resolution
    — that's a fundamentally harder, separate problem (matching against
    a moving, possibly-edited target is underdetermined without extra
    signal this tool doesn't have), and not one this checker needs to
    solve to be useful. If a real corpus ever makes bare delete+add
    noise genuinely painful, that's a concrete counterexample worth
    reopening this decision for — not speculative completeness now.
  - **`compare()`'s result shape — an explicit change algebra, not a
    verdict pair.** Comparing two full `TargetReport`/`AnalysisReport`
    serializations field-by-field (or eyeballing "old verdict vs new
    verdict" as the only case) is how a diff checker quietly turns into
    an archaeology expedition through `span`/evidence noise. Decided
    shape instead:
    ```
    enum TargetDiff {
        Unchanged,
        Added { head: TargetOutcome },
        Removed { base: TargetOutcome },
        Changed { base: TargetOutcome, head: TargetOutcome, changes: Vec<ChangeKind> },
    }
    enum ChangeKind {
        VerdictChanged,
        EvidenceChanged,
        PredicateChanged,
        VisibilityChanged,
    }
    ```
    v1 only needs to populate `VerdictChanged` — but reserving the enum
    now means a later PR adding evidence/predicate/visibility-level
    diffing extends `ChangeKind`, it doesn't redesign `TargetDiff`
    itself or fall back to comparing serialized JSON as a stand-in for
    "did anything change".
  - **`compare()` reports transitions, never regression/improvement
    judgments — same boundary as `analyze()`/CLI, one layer further
    out.** `compare()`'s job stops at a neutral fact: `PASS ->
    INCONCLUSIVE`, `INCONCLUSIVE -> FINDING`, `FINDING -> PASS`, etc.
    Whether `PASS -> INCONCLUSIVE` should block a PR, or
    `INCONCLUSIVE -> PASS` counts as an improvement worth celebrating in
    a step summary, is CI *policy* — it belongs in the Action or a
    ratchet-policy layer built on top of `oba diff`'s output, never
    baked into `compare()` itself. Exactly the same reason `analyze()`
    doesn't know about exit codes: a pure function that only states what
    happened stays testable and reusable by a policy it can't predict in
    advance.
  - **What's actually compared**: verdict kind is the minimum
    (`VerdictChanged`) for ratchet classification; whether
    `evidence`/`predicate_attempts`/opacity also need their own
    `ChangeKind` variants populated (e.g. "still PASS, but via a
    different predicate") is left open until a real case motivates it —
    not guessed at now, just reserved a slot in the enum above.
  - **Manifest-tampering trust policy** (flagged in the roadmap
    intro above, restated here since it's really part of this same
    design question): if `targets.toml` lives in the audited repo, an
    untrusted head can edit it. Resolution belongs in `compare()`/`oba
    diff`, not the Action.

  Standing constraints while this gets built, not just for PR D: `oba
  diff`/`compare()` are built on the CURRENT verdict semantics, not
  bundled with any H2 predicate-IR change — if differential output ever
  looks wrong, this keeps "two roots" and "a new evaluator" from being
  tangled into one unfalsifiable bug hunt. (H2's own IR already matches
  the normal form this kind of guardrail usually has to ask for up
  front: `Pred::{Eq,Not,And,Or}` over `ValueExpr::{Ref,Literal}`, with
  alias resolution as its own pre-lowering step (`resolve_ident_binding`/
  `resolve_alias_recursively`, `lower_pred`/`lower_value_expr` returning
  `Result<_, ResolveFailure>`) rather than folded into each `Eq`/`And`
  case — this was already the H2 round 2 design, not new work triggered
  by this note. No ritual refactor to "match yesterday's plan" is
  warranted just because it happens to already fit — H2's next
  meaningful proof, whenever it's picked back up, should be a real,
  previously-unsupported case going green (davis's actual
  `mysqlLocal = db.createLocally && db.driver == "mysql"` correctly
  classified, where it was out of scope before) plus an adversarial
  mutation flipping one alias/predicate operand and forcing the expected
  result to change — not "we added `And`".) `schema_version: 1` itself
  stays untouched through this design phase; only *additive* fields
  (e.g. a `"diff"` mode payload) get added under it, per the envelope's
  own reserved `mode` tag from PR B commit 3 — a bump is for an
  incompatible change, not routine, or it becomes exactly the
  version-every-commit theater the schema was designed to avoid. Once
  `compare()` exists: `oba diff`, target correlation, the trusted-manifest
  policy, transition classification, one `DiffReport` — the Action's job
  shrinks to materializing both roots and invoking `oba` once, unchanged
  from the original plan.

  **Planned sequencing for this phase**: D design freeze (this note) →
  pure `compare()` (no CLI/Action yet, tested standalone) → an
  adversarial diff corpus (added/removed/changed/unchanged/rename-as-
  delete+add cases, same discipline as every H1.x/H2.x fixture before
  it) → `oba diff` CLI → Action diff integration → only *then* H2's
  real-world unlock (davis/`mysqlLocal`). Diff plumbing and H2 semantics
  are validated independently, in that order, specifically so that if
  differential output ever looks wrong, it's diagnosable as "the two-root
  plumbing" or "the evaluator", never both at once.

  **PR D1 — pure `compare()` + the adversarial diff corpus. Closed, still
  no CLI/Action anywhere near it.** `TargetIdentity` (the canonical
  `(module, test, cfg_ident, option_prefix, watched_path)` tuple),
  `TargetOutcome`/`ChangeKind`/`TargetDiff`/`VerdictTransition`/
  `ComparisonReport`, `index_by_identity`, and `compare(base: &AnalysisReport,
  head: &AnalysisReport) -> Result<ComparisonReport, CompareError>` itself
  — all in `src/main.rs`, `#[cfg(test)]`-invisible to the CLI/Action so
  far (same "dead code until wired up" state H2's IR was in after its own
  first commit). `TargetReport` gained `module`/`test`/`cfg_ident`/
  `option_prefix` fields (additive to `schema_version: 1`, populated from
  the manifest's own `Target` in `run_target`) so `compare()` can compute
  identity from an `AnalysisReport` alone, without needing the original
  manifest alongside it. `Verdict::option()`/`::kind()` added as thin
  accessors (`kind()` returns the new `VerdictKind` bare-discriminant enum
  — what `ChangeKind::VerdictChanged` actually compares, deliberately
  ignoring payload so a same-kind-different-evidence pair reads as
  `Unchanged`, per the type's own documented meaning).

  Every invariant from the design review, proven, not just asserted:
  `compare(A, A)` is all `Unchanged` (example + `proptest` property);
  target/verdict order never affects the result (property test reverses
  a generated report's verdict order and asserts an identical
  `ComparisonReport`); entries are always sorted by identity (property
  test, plus a hand-built out-of-order example); `compare(A, B)` and
  `compare(B, A)` are exact mirrors — `Added`↔`Removed`, `Changed`'s
  `base`/`head` swap, transitions reverse (property test asserted
  structurally per entry, not just "same length"); duplicate identity on
  either side is `Err(CompareError)`, never "pair with the first match"
  (example test using two *different* target blocks that happen to share
  one identity tuple — the subtle case, not a literal copy-pasted
  manifest entry); identity carries zero presentation fields (two
  dedicated examples: same verdict kind with different `evidence`, and
  with only a `Span` differing, both `Unchanged`); a `module` rename is
  `Removed`+`Added`, never a detected move (no heuristic exists to detect
  one). Plus the full adversarial corpus asked for: reorder-without-change,
  duplicate identity, bare `Added`, bare `Removed`, all 7×7 `VerdictKind`
  transition pairs (brute-forced exhaustively, same discipline as
  `aggregate`'s own 32-case check), same-verdict-different-evidence,
  same-verdict-different-span, move-as-delete+add, and multiple
  simultaneous changes with a stable, identity-sorted order. 15 new tests
  (11 example + 4 `proptest` properties), 96 total (was 81 after PR C).

  Two real clippy findings fixed, not suppressed: `TargetDiff`'s
  `Added`/`Removed`/`Changed` payloads are now `Box<TargetOutcome>` (an
  unboxed `Changed` made every `TargetDiff` — including the
  zero-payload, by-far-most-common `Unchanged` case — pay for the
  largest variant's ~472 bytes, `clippy::large_enum_variant`, a real
  signal at real report sizes, not noise); `CompareError`'s `identity`
  is `Box<TargetIdentity>` for the same reason on the error path
  (`clippy::result_large_err`, ~128 bytes unboxed). `ChangeKind`'s
  shared `Changed` postfix triggered `clippy::enum_variant_names` --
  `#[allow]`ed with a comment, since the naming is the design note's own
  and nothing here is ever glob-imported (the lint's actual concern).

  **PR D2 — `oba diff` CLI. Closed. Zero new comparison semantics, on
  purpose** — pure orchestration around D1's `analyze()`/`compare()`:
  ```
  oba diff --base-root <dir> --head-root <dir> --targets <manifest> [--json]
  ```
  Two independent `analyze()` calls (each the exact same fail-closed,
  `resolve_within_root`-guarded analysis `check` already has — the
  security boundary is reused, not re-implemented) feeding one
  `compare()`. `--targets` stays cwd-relative, deliberately not
  `--base-targets`/`--head-targets` — one manifest is the whole point:
  comparing two different *specifications* of what to watch is a
  different, messier question (did the target change, or did what's
  being looked at change) this tool isn't answering. D1 was already
  treated as the frozen spec: no identity/verdict/change definition was
  touched to make the CLI more convenient.

  **Exit codes are deliberately NOT `check`'s 0/1/2/3 with the same
  meanings** — `check` answers "what's HEAD's state", `diff` answers
  "what changed", and `compare()` is intentionally transition-neutral (no
  `Changed` is inherently bad, since `FINDING -> PASS` and `PASS ->
  FINDING` are both `Changed` and D1 explicitly refused to rank them).
  So: `0` = the comparison was produced, regardless of what it found — a
  real `Changed` entry is not a CLI failure; `2` = the comparison was
  produced, but at least one side's own analysis had an inconclusive
  verdict or a parse error — checked directly against `base`/`head`'s
  `AnalysisReport`s (mirroring `check`'s own `is_inconclusive()` check,
  applied to both sides), NOT reconstructed from the `ComparisonReport`,
  because `TargetDiff::Unchanged` deliberately carries no payload (see
  its own doc comment) — an inconclusive verdict identical on both sides
  wouldn't even be visible from the diff alone; `3` = a tool/input error
  (bad manifest, a missing or escaping file under either root,
  `CompareError` — duplicate identity is a manifest problem, same class
  as any other `check` TOOL_ERROR, not special diff handling). **Exit `1`
  never appears for `diff` at all** — treating "anything changed" as
  failure would be exactly the hidden policy judgment `compare()` itself
  refuses to make; that belongs to a future CI-policy layer (e.g. "block
  on `PASS -> FINDING`"), built on top of this, not baked in here.

  JSON is a **separate** envelope type (`DiffEnvelope`/`DiffSummary`),
  not `ReportEnvelope` stretched to cover both modes — `check`'s
  `targets: Vec<TargetReport>` and `diff`'s `targets: Vec<ComparisonEntry>`
  are structurally different, and forcing one shared Rust enum over both
  would be worse types for a JSON convenience that doesn't exist on the
  wire (`mode` is a field value both share by convention, not a common
  parent type): `{schema_version, tool, mode: "diff", summary:
  {unchanged, added, removed, changed, verdict_transitions}, targets:
  [{identity, diff}]}`. `verdict_transitions` (e.g. `"oba001->pass": 1`)
  uses D1's own fine-grained `VerdictKind` naming, not a new
  PASS/FINDING/INCONCLUSIVE bucketing invented for this summary — bare
  counts, explicitly not a regression/improvement classification.

  New `tests/diff_cli.rs`, 11 tests, deliberately NOT re-covering D1's
  own exhaustive corpus (the 7×7 `VerdictKind` matrix, mirror-under-swap,
  etc. stay in `#[cfg(test)]` only) — only the genuinely new CLI surface:
  a **real** end-to-end proof against the actual historical
  `fixtures/kimai/before` → `fixtures/kimai/after` OBA001 → PASS
  transition, reached this time through two independent `--*-root`
  analyses of one manifest instead of two static `[[target]]` entries;
  `--base-root` escape reuses the exact fixtures PR B/D1 already proved
  `check --root` against; a **dedicated new fixture pair**
  (`fixtures/synthetic/diff-root-escape/{base,head}`) isolates
  `--head-root`'s own boundary specifically — `base/` is real, valid
  files (so `--base-root` analysis succeeds cleanly first), `head/
  module.nix` is a checked-in relative symlink escaping `head/`'s own
  subtree, proving the guarantee isn't just "whichever root happens to
  be checked first"; missing file under either root is the same
  TOOL_ERROR class `check` already has; duplicate identity
  (`fixtures/synthetic/diff-duplicate/`, two different target blocks
  sharing one identity tuple) reaches the CLI as exit 3; `--targets`
  stays cwd-relative even with both `--*-root`s absolute and elsewhere;
  repeated runs are byte-stable; human and JSON output are cross-checked
  against the SAME invocation's counts (not independently recomputed);
  `diff` of identical roots is all `Unchanged`, exit 0; comparing
  `golden.toml` against itself (real OBA001 + davis's inconclusive
  `OptionNotFound` on both sides) is exit 2, never 1.

  107 tests total (was 96 after D1). No Git anywhere in `oba` itself —
  deliberately only two directories in, a comparison out. A future
  Action (PR D3) does whatever checkout/worktree dance it needs and
  hands `oba diff` two paths; `oba` stays usable identically from GitHub
  Actions, a local shell, a Nix derivation, another CI, or two unpacked
  tarballs — never a second, worse frontend to `git`.
- **PR E, only if dogfooding on `PhysShell/nixpkgs` shows it's actually
  needed** — changed-target selection (skip targets whose `module`/`test`
  didn't change), kept deliberately separate from and after D: proving
  differential semantics are correct on the full small corpus first,
  *then* optimizing which targets run, not before — otherwise this
  project acquires yet another way to get a green CI by quietly not
  running a check, a category of bug it already has a documented history
  of (see the H1.x/H2.x rounds above).

Nix-native packaging (`flake.nix`, `packages.default`/`apps.default`) is
planned but explicitly kept separate from the GitHub Action delivery path
— a static musl binary is cheaper and simpler for the CI consumer than
adopting `nixpkgs-vet`'s NAR-closure pattern, which isn't even current
practice upstream anymore (see above).

Kani stays entirely internal to `oba`'s own development CI (`cargo test` /
clippy / occasional mutation testing / `cargo kani`'s 6 harnesses) and
never becomes something a consumer's `nixpkgs` PR CI has to install —
nothing in a changed `davis.nix` should require a CBMC install just to
learn whether `database.socket` got exercised.

**Proof-audit tooling: deferred, not rejected.** A brief survey (theoremc's
structured-obligation RFC, os-checker/distributed-verification's proof
inventory + snapshot diffing, provable-contracts' obligation→Kani→Lean
ladder, cbmc-starter-kit/Litani for large CBMC proof suites) turned up
nothing that fits directly, and building a bespoke "does this harness's
actual GOTO/reachable-function boundary match its stated claim" checker
now would be infrastructure without an object to watch: K0.1 is bounded to
`eval_known_eq` over a finite scalar domain, K1.1–K1.5 touch only the
5-field `AggregateFacts` struct — both proof boundaries are already small
and structurally obvious by inspection. K0.2–K0.4, the one place where the
boundary actually ballooned (through the real `HashMap`/SipHash-backed
`Environment`), were removed for exactly that reason, not audited around.
Trigger for picking this back up: not a harness count (50 more K1-shaped
proofs wouldn't need it), but a *new* proof that stops being local and
starts reaching into the production dependency graph again — K2
(`counterfactual_once`), `Environment`/`HashMap`, rnix/parsing, the
filesystem, or any other nontrivial adapter/stub. First step then is still
not a framework: `kani::cover` for non-vacuity, inspecting reachable
functions, a small expected/forbidden-dependency check, and a negative
control that must break the proof. A YAML DSL or a standalone `xtask
proof-audit` only if that minimal check actually catches drift and the
need repeats.

## Running

```
cargo test    # the full acceptance suite (tests/golden.rs, tests/fixture_integrity.rs,
              # plus a real-world unit test against fixtures/real/ifm-test.nix)
cargo run -- --targets targets/golden.toml [--json]              # legacy, still works
cargo run -- check --root . --targets targets/golden.toml [--json]   # equivalent, preferred
cargo run -- check --root /path/to/nixpkgs --targets targets.toml    # a manifest applied to a checkout elsewhere

# compare one manifest against two independent checkouts -- exit 0 means
# "the comparison was produced" (a real Changed is not a failure), 2 means
# either side's own analysis was inconclusive, 3 is a tool/input error;
# there is no exit 1 for diff at all -- see the PR D2 README section:
cargo run -- diff --base-root ./base --head-root ./head --targets targets.toml [--json]

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

`cargo test` — 62 tests, all passing (34 at the H1.3b/H1-freeze point
below, +28 from H2: gate-1 fix, pure Predicate IR, `eval_pred`
refinement, lexical alias resolver, the scope-aware safety-gate fix, and
the counterfactual gate 4 wiring with the davis acceptance case + 4
synthetic adversarial cases — see "H2" above; H2 hasn't added new
`cN`-numbered fixtures yet, so criterion C's case list below stays as of
the H1 freeze): 5 from the original spike, 5 from H1
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
