# S6 target-construction protocol (frozen before any P1 PR is examined)

Purpose: make NC2 ("target derivability without large amounts of
manual invention") a checkable fact, not a post-hoc impression. For
each of the 15 P1-sampled PRs, apply these exact steps, in order, and
record which step the process stopped at. No step below permits a
judgment call beyond what it itself specifies.

**Ordering guarantee this protocol exists to enforce**: steps 1-6 (diff
inspection, target freezing) happen BEFORE step 7 (running `oba`).
Nothing in steps 1-6 ever runs the analyzer or looks at its output.
This is the mechanical enforcement of S6-R0's own item 9 ("never: run
OBA broadly, find an interesting output, construct a target around
it").

## Steps

1. **Fetch the real PR diff** (`gh pr view <n> --repo NixOS/nixpkgs
   --json files,baseRefOid,headRefOid` or equivalent), restricted to
   files under `nixos/modules/**` or `nixos/tests/**` (the same two
   path prefixes the cheap relevance filter already used to select this
   PR).

2. **Identify candidate module file(s)**: every changed file under
   `nixos/modules/**` that contains at least one `mkOption`/
   `mkEnableOption`/`mkPackageOption` call in either its base or head
   version (a syntactic grep, not a semantic judgment). If none, record
   `target_not_mechanically_derivable: no option-declaring file
   changed` and STOP for this PR (this PR still counts toward the NC3/
   NC4 denominators as "no substantive target," not silently dropped).

3. **Identify the candidate watched option**: among the option
   declarations in the changed module file(s), select the one(s)
   the diff itself directly touches (the declaration's own source
   lines appear in the diff — added, removed, or modified). If the
   diff touches more than one independent option declaration in the
   same file, construct ONE target per touched declaration (do not
   pick "the most interesting one" — every touched declaration in
   scope becomes its own target, mechanically). If the diff touches
   zero option declarations directly (e.g., only touches `config =`
   logic, not `options =`), record `target_not_mechanically_derivable:
   no option declaration in the diff itself` and STOP.

4. **Derive `option_prefix`/`watch`**: `option_prefix` is the
   declaration's own path up to but not including its final segment;
   `watch` is that final segment (dotted, if the declaration is nested
   under further sub-attributes within the same `options = {...}`
   block, mirroring this project's own existing `watch` convention).
   `cfg_ident`: the local `let`-bound alias for `config.<option_prefix>`
   if one exists in the module (grep for `= config\.<option_prefix
   joined by "\.">` ), else the literal string `"cfg"` if the module
   itself never introduces its own alias (matching this project's own
   default-fallback convention used throughout S1-S5).

5. **Identify the wired test file**: the corresponding
   `nixos/tests/**` file changed by the SAME PR, if any; else, grep
   `nixos/tests/all-tests.nix` at the PR's own head SHA for a reference
   to a test file whose own name matches the module's own service name
   (same heuristic S5's own protocol used, e.g. `gonic = runTest
   ./gonic.nix;`). If neither exists, record
   `target_not_mechanically_derivable: no wired test file found` and
   STOP.

6. **Freeze the target**: write `{name, module, test, cfg_ident,
   option_prefix, watch}` to this PR's own frozen target record,
   together with the exact base/head SHAs the PR diff was read from.
   This record is written BEFORE step 7 and is never edited after
   step 7 (S6-R0 item 9's own ordering guarantee).

7. **Only now**, run `oba check`/`oba audit-diff` against the real,
   live-fetched base/head source at the frozen SHAs, using the frozen
   target record from step 6, exactly as every prior S-round's own
   replay tooling already does.

## NC2's own operational definition, restated

A PR counts as a **successful mechanical derivation** (contributes to
the NC2 numerator) if it reaches step 6 for at least one target,
without any step above requiring information or discretion beyond what
that step itself specifies. A PR that stops at step 2, 3, or 5 is a
**mechanical non-derivation** (does NOT count toward the NC2
numerator, but DOES count toward the fixed 15-PR denominator — it is
not discarded or replaced).

## What this protocol deliberately does NOT attempt

- Judging whether the changed option is "interesting" or "likely to
  reveal a defect" — every option touched by the diff becomes a
  target, mechanically, per step 3.
- Resolving `imports=`, wildcards, or any other already-disclosed
  residual limitation specially — this protocol is blind to which
  known-limitation class (if any) a given PR might touch; that
  determination happens only during adjudication (see
  `adjudication-rubric.md`), never during target construction.
- Guaranteeing every PR yields a target — a PR that stops at step 2/3/5
  is real, honest information about NC2's own success rate, not a
  protocol failure to paper over.
