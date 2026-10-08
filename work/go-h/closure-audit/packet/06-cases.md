# Cases for classification

Pinned commits (nixpkgs; repo at /home/tandem/.cache/go-h/nixpkgs-exact):
- E = e4c7d977153965496cbe73ff5631ca6a1118b347
- B = e2497c3a5262687ff7aada0707bc754cca66396f
- F = fdc3396349781c0dc2b495b6a320f62918bd63f6

Source excerpts for the real cases are in 05-source-excerpts.txt (exact-SHA, with line numbers). You may read more of the same file at the same SHA with `git -C <repo> show <sha>:<path>` or `git -C <repo> grep -n <pattern> <sha> -- <path>`. Every query must name a SHA. Do not use a working tree.

Conventions for every case:
- The watched option W and the changed hunk are as stated. Evaluate the target validity V for the target W names, using the validity rules, unless the case says otherwise.
- A synthetic case declares only what it states. Where a synthetic case names an operand "declared in another module", classify it as foreign on that basis; do not search for its file.
- Classify synthetic cases as written. Do not look for a historical commit for them.

## K01 control-only dependence under a watched gate (real, base B)
File nixos/modules/services/monitoring/beszel-agent.nix at B. W = services.beszel.agent.openFirewall (declared in this file). Hunk: the if expression at line 211 is changed. It sits inside the list argument of lib.mkIf cfg.openFirewall at line 209.

## K02 watched and foreign mixed predicate (synthetic)
Module declares services.foo.port (type port, default 80). Changed line:
    config = lib.mkIf (config.services.foo.port > 0 && config.boot.zfs.enabled) { ... };
boot.zfs.enabled is declared in another module. W = services.foo.port. Hunk: this line is changed.

## K03 foreign-only predicate (synthetic)
Module declares services.foo.port. Changed line:
    config = lib.mkIf config.boot.zfs.enabled { ... };
boot.zfs.enabled is declared in another module. W = services.foo.port. Hunk: this line is changed.

## K04 declared nested option versus freeform-only key (real, E)
File nixos/modules/services/accessibility/speechd.nix at E, lines 34-50. The option services.speechd.modules is declared with type submodule { freeformType = attrsOf lines; } and default { }.
- K04a: W = services.speechd.modules (declared). Hunk: the default of services.speechd.modules is changed.
- K04b: W = services.speechd.modules.generic-epos (a key matched only by freeformType, not declared as an option). Hunk: the default of services.speechd.modules is changed.

## K05 exact-SHA resolution of lib.filter (real, F)
File nixos/modules/services/monitoring/beszel-agent.nix at F, line 54: lib.filter (name: gpuCollectors.${name} ? package) activeCollectors. The lib binding for filter at F is in 05-source-excerpts.txt (lib/default.nix and lib/lists.nix at F). W = services.beszel.agent.environment.GPU_COLLECTOR. Hunk: the lambda body on line 54 is changed (gpuCollectors.${name} ? package).

## K06 unresolved and shadowable higher-order callee (synthetic)
Module declares services.foo.enable and services.foo.items (a listOf of attrs with an enable field). W = services.foo.enable. Hunk: the lambda line is changed in each case.
- K06a: let filter = myFilter; in filter (x: x.enable) config.services.foo.items. myFilter has no definition in the module.
- K06b: with lib; filter (x: x.enable) config.services.foo.items. The module has no local binding named filter.

## K07 selection lambda versus transform lambda (synthetic)
Module declares services.foo.mode (enum [ "a" "b" ]) and services.foo.names (listOf str). W = services.foo.mode. Hunk: the lambda body is changed in each case.
- K07a: selected = lib.filter (n: config.services.foo.mode == "a") config.services.foo.names;
- K07b: mapped = map (n: if config.services.foo.mode == "a" then n else "") config.services.foo.names;

## K08 transparent let alias (synthetic)
Module declares services.foo.enable. W = services.foo.enable. Changed line:
    config = let p = config.services.foo.enable; in lib.mkIf p { ... };
Hunk: the let binding line is changed.

## K09 let-bound semantic property (real, F)
File nixos/modules/services/monitoring/beszel-agent.nix at F. The let binding activeCollectors is on line 48 and the lambda that iterates it is on line 54. W = services.beszel.agent.environment.GPU_COLLECTOR. Hunk: line 48 is changed.

## K10 whole-namespace receiver (synthetic)
Module declares services.foo.enable. W = services.foo.enable. Changed line:
    config = lib.mkIf (builtins.elem "foo" (builtins.attrNames config.services)) { ... };
Hunk: this line is changed.

## K11 unresolved computed key (synthetic)
Module declares services.foo.enable. W = services.foo.enable. Changed line:
    config = lib.mkIf config.services.${builtins.getEnv "X"}.enable { ... };
Hunk: this line is changed.

## K12 finite enum computed key (real, E)
File nixos/modules/services/web-apps/movim.nix at E. services.movim.database.type is declared at lines 476-484 with type enum [ "mariadb" "postgresql" ] (default "postgresql" at line 483). Line 628 reads config.services.${cfg.database.type}.settings.port. W = services.movim.database.type. Hunk: the default at line 483 is changed.

## K13 generic helper with option-dependent call context (real, E)
- K13a: lib/trivial.nix at E, line 308: boolToYesNo = b: if b then "yes" else "no";. W = services.locate.pruneBindMounts (declared at nixos/modules/misc/locate.nix lines 191-193 at E). Hunk: line 308 is changed (the if expression).
- K13b: nixos/modules/misc/locate.nix at E, line 244 interpolates lib.boolToYesNo cfg.pruneBindMounts. Same W. Hunk: the interpolated text on line 244 is changed.

## K14 equivalence of a rewritten conditional (synthetic)
Module declares services.foo.enable. W = services.foo.enable. Changed line in an expression:
    before: "${lib.optionalString config.services.foo.enable "--flag"}"
    after:  "${if config.services.foo.enable then "--flag" else ""}"
lib.optionalString at E is in lib/strings.nix line 776. Hunk: the rewrite. State the equivalence level under the equivalence rules, and the resulting category.

## K15 gating predicate versus data-only boolean (synthetic)
Module declares services.foo.enable and services.foo.label (str). W = services.foo.enable.
- K15a: config = lib.mkIf config.services.foo.enable { ... }; Hunk: this line is changed.
- K15b: services.foo.label = if config.services.foo.enable then "on" else "off"; Hunk: this line is changed.

## K16 exact-SHA absence: T_NONE versus T_UNKNOWN (synthetic, searched at E)
Each case changes the default of a declared option P in a changed module. Consumers of P are searched at E with the procedure in 04-exact-tree-search.sh (usage in its header), scope nixos/, owner key as stated. Decide the result under the absence rules.
- K16a: P = services.zzgohprobe.enable (owner key zzgohprobe; declared in the changed module).
- K16b: P = services.foo.port (owner key port; declared in the changed module).

## K17 invalid target without an E classification
- K17a (synthetic): module declares services.foo.enable only. Changed line:
      config = lib.mkIf config.services.foo.bar { ... };
  W = services.foo.bar, which the module does not declare. Hunk: this line is changed.
- K17b (real, E): W = services.speechd.modules.generic-epos (see K04b). Hunk: the predicate or option default that reads it, as in K04b.

## D1 deliberately unresolved: mpd V3 submodule container (real, E)
File nixos/modules/services/audio/mpd.nix at E, lines 150-186. services.mpd.settings is declared with type submodule that has both freeformType and options. W = services.mpd.settings.music_directory (declared at line 178 inside that submodule's options). Hunk: the default of music_directory is changed (the default line follows line 178).
The case asks: is W a valid target, and what does the specification say about whether this declared-in-submodule option is an intermediate container under the V3 rule?

## D2 deliberately unresolved: boolToYesNo equivalence and EQ3 (real, E)
File nixos/modules/misc/locate.nix at E, line 276: (lib.boolToYesNo cfg.pruneBindMounts). W = services.locate.pruneBindMounts. Hunk: the call is rewritten to (if cfg.pruneBindMounts then "yes" else "no"). State whether the rewrite is equivalent, under which class, and the resulting category.
