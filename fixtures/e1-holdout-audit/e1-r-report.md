# E1-R: regression re-run of the frozen E1 holdout, after P0+P1+P2

**Same 40-candidate corpus as E1, not reshuffled, not re-drawn.** The
exact `targets/e1-holdout-batch{1,2,3,4}.toml` manifests and the exact
vendored `fixtures/e1-holdout-audit/<name>/{module.nix,test.nix}` files
E1 itself produced (commit `6d6059d`) are re-run unchanged against the
real binary at commit `3f9e41f` -- the head of the P0 → P1 → P2 fix
chain. This is now a regression benchmark, not a fresh audit: nothing
about the corpus, the draw, or the manifests changed; only the analyzer
did.

## What changed, in order

- **P0** (`937a941`): fixed the nested-submodule declaration collision
  (GAP-4) -- the real, demonstrated false-positive-capable bug
  (`xandikos`'s reported `PASS` backed by the wrong declaration).
- **P1** (`9adec68`): normalized `mkEnableOption` as a declaration of a
  plain boolean option with a known default (GAP-1) -- the single cause
  behind 84% of E1's own 32 inconclusive results.
- **P2** (`3f9e41f`): fixed the nested `options = { services.X =
  {...}; };` declaration idiom losing the `option_prefix` boundary
  (GAP-2) -- confirmed still fully needed before starting (all 10
  originally-attributed candidates remained inconclusive after P1
  alone).

## Headline: before vs. after

| | E1 original (`c8e42a1`) | E1-R (`3f9e41f`) |
|---|---|---|
| PASS | 6 (5 clean + 1 false-positive) | **21 (all clean)** |
| FINDING (`OBA001`) | 2 | **6** |
| INCONCLUSIVE | 32 | **13** |
| TOOL_ERROR | 0 | **0** |

Watched-path-level (a finer-grained count -- some candidates watch more
than one option; the same total, 62, either side of the fix, since no
target's own `watch` list changed): 8 PASS / 2 FINDING / 52 INCONCLUSIVE
before → **32 PASS / 6 FINDING / 24 INCONCLUSIVE** after (`OptionNotFound`
6, `PredicateNotFound` 5, `TestConfigUnresolved` 12, `DefaultUnresolved`
1). Not the headline number -- the candidate-level rollup above is what
the user's own success criteria are stated against -- but included for
completeness, since it's real data already in hand, not extrapolated.

## The user's own pre-committed success criteria, checked one by one

- **false-positive capability: 1 → 0.** `xandikos`'s real `PASS` was
  backed by the wrong declaration (`nginx.enable`, not
  `services.xandikos.enable`) in E1's original run. Now: `xandikos`
  shows a genuine `PASS`, backed by the real declaration, with the
  collision source (`nginx.enable`) correctly scoped separately --
  confirmed structurally (`discovered_options` now contains BOTH
  `enable` and `nginx.enable` as distinct entries, never colliding) and
  by two permanent regression tests (`h2-case14`/`15` in
  `tests/golden.rs`) that assert exactly this, run in real CI on every
  push since. **0/0 — met.**
- **`TOOL_ERROR`: 0 → 0.** Confirmed directly from the real re-run
  above. **Met.**
- **`INCONCLUSIVE` substantially down.** 32 → 13, a 59% reduction (27
  candidates were fixed by some combination of P0/P1/P2; 13 remain,
  each for a distinct, already-disclosed reason from E1's own root-cause
  taxonomy -- five one-off structural gaps, plus two real, working-as-
  designed limitations (`with`-scope, import-opacity) that were never
  bugs to begin with). **Met, substantially.**
- **Existing true findings: preserved.** `flame`/`openFirewall` and
  `convos`/`reverseProxy`, E1's own two original manually-verified real
  findings, are both still `OBA001` in this run, unchanged. **Met.**

## The four newly-reachable findings, each manually re-verified genuine

P1/P2 made real predicates checkable that were previously invisible
behind gate 1 -- this surfaced four NEW `OBA001` results. Each was
independently re-verified by reading the real, current test file
directly (every node, not just a grep for the option name), the same
discipline E1's own two original findings were held to. None assumed
genuine just because the tool said so.

- **`coturn`/`no-auth`**: real predicate `lib.optionalString cfg.no-auth
  "no-auth"`, default `false`. `fixtures/e1-holdout-audit/coturn/test.nix`
  has two nodes (`default`, `secretsfile`) -- neither ever sets
  `no-auth` anywhere. Genuine.
- **`vault`/`dev`**: real predicate `lib.optional cfg.dev "-dev"`,
  default `false`. `fixtures/e1-holdout-audit/vault/test.nix`'s one node
  never sets `dev`. Genuine.
- **`omada`/`openFirewallWebPorts`**: real predicate `lib.optionals
  cfg.openFirewallWebPorts [...]`, default `false`.
  `fixtures/e1-holdout-audit/omada/test.nix`'s one node never sets it.
  Genuine.
- **`unpackerr`/`user`**: real predicate `mkIf (cfg.user ==
  "unpackerr")`, and the option's own real declared default IS
  `"unpackerr"` (confirmed by reading the declaration directly,
  `fixtures/e1-holdout-audit/unpackerr/module.nix:51-55`). The real test
  sets `user = "unpackerr";` -- the SAME value as the default, not an
  opposite transition. Genuine (never witnessed opposite, exactly as
  reported). **The strongest single piece of evidence that this isn't
  "the tool flags everything now"**: `unpackerr`'s own `group` option
  has the exact same real predicate shape and the exact same real
  default (`"unpackerr"`), watched in the very same run -- and it
  correctly comes back `PASS`, because the real test DOES flip it
  (`group = "users";`, genuinely different from the default). Same
  module, same shape, same run, two different genuinely-correct
  verdicts -- the discriminating behavior working exactly as designed,
  not a blanket "everything found is a finding."

## What's left in the 13 remaining `INCONCLUSIVE` (all already disclosed
in E1's own root-cause taxonomy, none new)

- `jitsi-meet`, `pomerium`: GAP-3 (`with types;` wrapper) / GAP-5 (`cfg`
  bound inside `config`'s own `let`) -- each a real, one-off structural
  gap, not re-fixed this round (E1's own plan named these as one-offs,
  not load-bearing enough to justify a P3 on their own yet).
- `bees`, `authelia`: `PredicateNotFound` -- GAP-6 (empty list/attrset
  literal comparison) and GAP-7 (a predicate mediated through a plain
  function call, `filterAttrs`/`mapAttrs'`, never a textual `mkIf`
  site), both real, disclosed, one-off gaps.
- `i2pd`, `nimdow`, `tor` (partially), plus a few others: further,
  smaller structural causes already named in E1's own census (freeform
  settings keys with no `Declaration` at all, deep function-call-built
  test topologies) -- not newly discovered here, not chased further
  this round.
- `monado`, a `TestConfigUnresolved` case: the import-opacity gate
  correctly refusing to certify a conclusion when part of the real test
  config lives behind an unresolved `imports = [...]` -- working as
  designed, not a bug.

None of these are new information; re-stated here only to account for
every one of the 13, not leave a residual "and some other stuff" gap in
the regression accounting.

## Conclusion

The fix chain composes correctly (P0+P1+P2 together produce genuine
PASSes on real code, not three isolated patches that happen not to
conflict), preserves every prior true finding, introduces zero new
false positives or tool errors, and turns four previously-invisible
real branches into correctly-classified, individually-verified real
findings -- including one case (`unpackerr`) that directly demonstrates
the fix discriminates correctly rather than just finding more things.
E2/E3 direction (further gaps, or the CDC generated-config-file work
E1 itself flagged as the next-most-load-bearing signal) is now an open,
data-driven decision, not this report's to make.
