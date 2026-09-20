# S2-F3: `exporters.nix` census — scale and architecture, before any support

Research only, per the user's own explicit instruction: "сначала
census, а не сразу поддержку." Zero `src/` changes in this round —
this document exists to answer whether generic multi-file support is
justified, not to build it.

## The question

S2 found a real gap: `nixos/modules/services/monitoring/prometheus/
exporters.nix`'s shared submodule framework hides real `mkOption`
declarations (`privateKeyFile`/`environmentFile` on the `snowflake`
exporter, PR `#552038`) from `oba`'s own single-module-file scanner.
S2-F1 already fixed the general `//`-merge case this partially
resembles. The real, pre-registered question here: **is this a
one-off, or does the SAME architecture recur across enough real
modules that generic multi-file support would be a real milestone, not
a hole dug for one case?**

## Population: real, not estimated

`exporters.nix` registers exactly **93 real Prometheus exporter
modules** through one shared mechanism
(`fixtures/s2-f3-exporters-census/exporter-names-93.txt`, the real
`genAttrs [ ... ]` list read directly from the source, plus 6 further
`exportarr-*` aliases that all resolve to one shared `exportarr.nix`
file — not counted separately, since they share one file, not 93
distinct ones):

```
exporterOpts =
  (genAttrs [ "apcupsd" "artifactory" ... 93 names ... ] (
    name: import (./. + "/exporters/${name}.nix") { inherit config lib pkgs options utils; }
  ))
  // (mapAttrs (...) { exportarr-bazarr = {...}; ... });
```

All 93 real per-exporter files were fetched and inspected directly
(`fixtures/s2-f3-exporters-census/per-exporter-census.txt`, the raw
per-file counts):

```
87 / 93 (94%)  define a real, non-empty extraOpts block
92 / 93 (99%)  contain at least one real mkOption declaration of their own
 1 / 93        (unpoller) has zero real options beyond the shared framework's own
```

**This is real, decisive scale** — not a handful of edge cases, 93
real modules sharing the identical architecture. By the user's own
pre-registered bar ("десятки модулей с одной и той же архитектурой"),
this clears it.

## The real architecture, mapped precisely (three real crossings, not one)

Two concrete real files, read in full
(`nixos/modules/services/monitoring/prometheus/exporters.nix` and
`exporters/snowflake.nix`, `exporters/borgmatic.nix`), make the real
shape unambiguous:

```
exporters.nix (the SHARED framework file)
  options = mkSubModules;
  mkSubModules = foldl' (a: b: a // b) {} (
    mapAttrsToList (name: opts: mkSubModule {...}) exporterOpts
  );
  mkSubModule = { name, port, extraOpts, imports }: {
    ${name} = mkOption {
      type = types.submodule [
        {
          options = (
            mkExporterOpts { inherit name port; }   <- generic options
            // extraOpts                             <- per-exporter options
          );
        }
        ( { config, ... }: mkIf config.openFirewall {  <- the real predicate
            firewallFilter = mkDefault "...";
            firewallRules = mkDefault "...";
          }
        )
      ];
    };
  };
  mkExporterOpts = { name, port }: {
    enable = mkEnableOption "...";
    port = mkOption {...};
    listenAddress = mkOption {...};
    extraFlags = mkOption {...};
    openFirewall = mkOption {...};       <- the option the predicate above gates
    firewallFilter = mkOption {...};
    firewallRules = mkOption {...};
  };

exporters/<name>.nix (a SEPARATE file PER exporter, dynamically
  imported by string-interpolated path)
  { config, lib, pkgs, ... }: {
    port = 9975;
    extraOpts = {
      privateKeyFile = mkOption {...};   <- the option S2's own real
      environmentFile = mkOption {...};      shadow audit found hidden
    };
    serviceOpts = { serviceConfig = { ExecStart = "..."; }; };
  }

nixos/tests/prometheus-exporters.nix (a THIRD, separate shared file,
  one combined test with per-exporter node definitions inside it)
```

**Three real, distinct crossings**, not one:

1. **Generic options live in a function call, not a literal attrset**
   (`mkExporterOpts { inherit name port; }`) — this means `openFirewall`
   itself (a real, security-relevant predicate common to every single
   one of the 93 exporters — does this metrics endpoint get exposed to
   the firewall) is ALSO currently invisible, for all 93 modules, not
   only the ones with extra per-exporter secrets. **A real, larger
   finding than S2's own original one**: S2-F1's `//`-unwrap fix does
   NOT help here at all — `mkExporterOpts {...}` is the LHS of the
   merge, and it's a function call, which the fix deliberately never
   evaluates.
2. **Per-exporter options live in a separate file**, resolved via
   `genAttrs`'s own dynamic `import (./. + "/exporters/${name}.nix")` —
   a real file-path construction from a name list, not a literal
   `import` path a static scanner could simply follow.
3. **The predicate that GATES `openFirewall`** lives back in the shared
   file (`mkIf config.openFirewall { firewallFilter = ...; }`), a third
   location relative to where the option itself is declared.

## What generic support would actually require

Mapped directly against the layered design the user themselves
sketched:

```
module under analysis (exporters.nix)
      ↓
discover referenced/shared declaration sources
      ↓ -- REQUIRES: resolving `genAttrs [ 93 literal names ]
      ↓    (name: import (./. + "/exporters/${name}.nix") {...})`
      ↓    into 93 real file paths, i.e. a real, if small and
      ↓    bounded, STRING-INTERPOLATED PATH CONSTRUCTION over a
      ↓    literal name list -- genuinely new capability, not
      ↓    exercised anywhere in this project today.
bounded source set (94 files: exporters.nix + 93 per-exporter files)
      ↓
declarations + predicates
      ↓ -- REQUIRES: `mkExporterOpts { inherit name port; }` is a
      ↓    real FUNCTION CALL with known literal arguments at its
      ↓    own call site -- resolving it needs a real, bounded
      ↓    INLINE FUNCTION APPLICATION primitive (substitute known
      ↓    argument values into a small lambda's own body and walk
      ↓    the result) -- a second genuinely new capability class,
      ↓    and the riskier of the two: this is the exact shape of
      ↓    thing that silently grows into "our own Nix evaluator"
      ↓    if the bound isn't kept extremely tight (e.g. literal
      ↓    arguments only, no further indirection allowed one level
      ↓    deeper).
same existing H2/OBA logic (unchanged once declarations/predicates
  are actually visible)
```

Both new primitives are individually boundable in principle (a fixed
list of literal strings; a lambda applied to literal arguments, one
level, no further recursion into unknown call graphs) — but they are
real, new capability classes for this scanner, not an extension of
S2-F1's already-landed, much narrower `//`-unwrap.

## Recommendation

**Justified by scale** (93 real modules, identical architecture,
clears the user's own pre-registered bar decisively) — **but the
required mechanism is a genuine new capability class**, not a small
bounded fix. Two concrete, separately-assessable primitives would be
needed (dynamic literal-name-list file resolution; bounded single-level
function application), each carrying real risk of scope creep into a
de facto Nix evaluator if not held to a strict bound.

Given the user's own explicit caution ("иначе внезапно получится
собственный Nix evaluator, а это уже совсем другой способ испортить
выходные"), this census recommends: **real, but not undertaken during
S2-R** — a real candidate for its own dedicated, carefully-scoped round
with its own pre-registered bound (exactly like every other capability
this project has added), not bundled into the regression rerun. The
S2-R that follows this census stays at "exporters cases remain honestly
`INCONCLUSIVE`" for all 93 modules, exactly as today — a real,
disclosed limitation, not a silent gap.
