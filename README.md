# oba — Option Branch Activation evidence

Layer 1 only, of the three explicitly separated layers agreed on before
writing any code:

```
A. option branch activation    — was this branch ever exercised with a
   (OBA, this tool)              non-default value by a test?
B. consumer contract drift      — does the emitted key name match what
   (CDC, not built)               the pinned consumer actually recognizes?
C. runtime observability        — does the value actually change observed
   (ROB, not built)               behavior, or is it masked by a fallback?
```

`B PASS` does not imply `C PASS`, and this tool does not attempt either —
it answers exactly one question: **was this option ever assigned a
non-default value by a test, at all.** That's it. A `PASS` verdict from
this tool is *activation evidence*, not a correctness proof — see the
`kimai-after` golden result below for a real illustration of exactly why
that distinction matters.

## What it does

Given a NixOS module and a test file:

1. Finds `mkOption { default = ...; }` declarations nested under any
   `options = { ... };` block anywhere in the module (handles both a
   module's own top-level options and a separately-defined submodule's,
   e.g. kimai's `siteOpts`).
2. Finds branch predicates that reference `cfg.<path>` directly (`cfg_ident`
   is configurable) via a fixed, narrow grammar: `!= null`, `== null`,
   bare truthy, `!cfg.foo`, and `lib.{mkIf,optional,optionals,
   optionalString,optionalAttrs} cfg.foo`.
3. Walks the test file's attrset tree (transparently unwrapping
   `containers.<name>` / `nodes.<name>` lambda wrappers, which are
   nixosTest scaffolding, not part of the option namespace) and finds
   every leaf assignment.
4. For each explicitly `watch`ed option, structurally matches test
   assignments against `option_prefix ++ predicate_path` (prefix supports
   a `*` wildcard for `attrsOf`-submodule instance names) and classifies
   each match as inside or outside the option's default-value class.

Verdicts: `PredicateNotFound` (no direct `cfg.<path>` branch found — most
often a `let`-bound alias, explicitly out of scope, see below), `OBA001`
(predicate found, no non-default test evidence), `PASS` (predicate found,
non-default test evidence exists, with the matching assignment(s) attached
as evidence).

## Non-goals (deliberate)

- **No `let`-bound alias resolution.** `foo = cfg.x; if foo != null then
  ...` is invisible to this MVP. Reported as `PredicateNotFound`, never
  silently absorbed into a `PASS`.
- **No VM tests, no consumer knowledge.** The tool doesn't run anything
  and doesn't know what Doctrine, or any other consumer, is.
- **No claim about runtime behavior.** `PASS` means "a test assigns a
  non-default value here", full stop — never "the fix works", never
  "this is safe".

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
| `davis-before` | `PredicateNotFound` on `database.driver` | ✅ |
| `davis-after` | `PredicateNotFound` on `database.driver` | ✅ |

davis's branch is gated by `mysqlLocal = db.createLocally && db.driver ==
"mysql";` — a `let`-bound alias, not a direct `cfg.foo` select. That's the
real reason it stays `PredicateNotFound` on *both* commits: this MVP is
honestly telling you it didn't look hard enough to have an opinion, rather
than quietly passing something it never actually checked.

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

## Running

```
cargo test    # the full acceptance suite (tests/golden.rs)
cargo run -- --targets targets/golden.toml [--json]
```

## Acceptance criteria (as specified, all met)

- [x] A. reproduces the historical finding on the exact parent commit
- [x] B. clears it on the exact fix commit, with evidence attached
- [x] C. survives 4 adversarial mutations
- [x] D. produces source spans + evidence (file:line:col, matched assignment)
- [x] E. does not invoke VM tests
- [x] F. does not know anything about Doctrine

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

## Explicit follow-ups (not this spike)

- `let`-bound alias resolution (unblocks davis's actual `database.driver`
  predicate)
- flat-dotted `options.a.b.c = { ... };` recognition, done consistently
  with `cfg_ident` scope resolution
- CDC (consumer contract drift) and ROB (runtime observability) as
  separate tools/layers, per the explicit non-conflation this spec insisted
  on from the start
