#!/usr/bin/env bash
# GO-H1C.12 bounded historical probes. Read-only. Every excerpt is printed from an exact SHA.
# Output is evidence for review, not a verdict. Probes marked SYNTHETIC have no historical SHA.
set -uo pipefail
R=/home/tandem/.cache/go-h/nixpkgs-exact
S=/home/tandem/nix-option-branch-audit/work/go-h/exact_tree_search.sh
E=e4c7d977153965496cbe73ff5631ca6a1118b347
B=e2497c3a5262687ff7aada0707bc754cca66396f
F=fdc3396349781c0dc2b495b6a320f62918bd63f6
RUN=b063b8f9b2a655f72cec13f1877522c3e4066ac2

ex() { # id label sha path from to
  echo "### $1 $2"
  echo "# $3:$4 lines $5-$6"
  git -C "$R" show "$3:$4" | sed -n "$5,$6p" | awk -v s="$5" '{printf "%5d| %s\n", s+NR-1, $0}'
  echo
}

echo "## P01 direct if predicate (watched services.beszel.agent.environment)"
ex P01 "if predicate in value position" $B nixos/modules/services/monitoring/beszel-agent.nix 204 216

echo "## P02 assertion condition inside a gated block (watched programs.joycond-cemuhook.enable; foreign services.joycond.enable)"
ex P02 "assertion" $E nixos/modules/programs/joycond-cemuhook.nix 12 18

echo "## P03 selection predicate lambda over option-derived list (watched services.beszel.agent.environment.GPU_COLLECTOR)"
ex P03 "filter predicate" $F nixos/modules/services/monitoring/beszel-agent.nix 48 54

echo "## P04 transform lambda without a boolean (negative control)"
ex P04 "map transform" $F nixos/modules/services/monitoring/beszel-agent.nix 53 53

echo "## P04b transform lambda containing a conditional (negative control, watched environment)"
ex P04b "mapAttrs transform with if" $F nixos/modules/services/monitoring/beszel-agent.nix 242 249

echo "## P05 mixed local and foreign operands in a stored attribute value (watched services.beszel.agent.smartmon.enable; foreign config.boot.zfs.enabled)"
ex P05 "attribute value, mixed operands" $B nixos/modules/services/monitoring/beszel-agent.nix 185 188

echo "## P05 foreign-only gating (watched not in predicate)"
ex P05f "lib.optionals foreign gating" $B nixos/modules/services/monitoring/beszel-agent.nix 136 140

echo "## P05s SYNTHETIC mixed operands in gating position (no historical SHA)"
echo "  lib.mkIf (cfg.smartmon.enable && config.boot.zfs.enabled) { ... }"
echo

echo "## P06 let alias around an option path (device-tree; watched hardware.deviceTree.dtboBuildExtraPreprocessorFlags)"
ex P06 "let alias" $E nixos/modules/hardware/device-tree.nix 92 110

echo "## P07 let-bound default (watched programs.tmux.keyMode; changed binding defaultKeyMode)"
ex P07a "let binding" $E nixos/modules/programs/tmux.nix 17 20
ex P07b "option default uses binding" $E nixos/modules/programs/tmux.nix 155 157
ex P07c "gating use of the option value" $E nixos/modules/programs/tmux.nix 44 49

echo "## P08 boolean serialized into shell text (watched services.locate.pruneBindMounts)"
ex P08a "shell text use" $E nixos/modules/misc/locate.nix 238 246
ex P08b "second use" $E nixos/modules/misc/locate.nix 272 278

echo "## P09 exact-SHA absence, owner key gophernicus (watched services.gophernicus.rootDir)"
echo "# owner-key query (A3)"
"$S" "$R" "$E" nixos/ '\bgophernicus\b' nixos/modules/services/misc/gophernicus.nix
echo "# receiver closure check (A5), whole-namespace and computed-key forms over config.services"
"$S" "$R" "$E" nixos/ 'config\.services *(;|\)|\}|$)|config\.services\.\$\{|cfgs *= *config\.services'
echo

echo "## P09b owner key rundeck at a historical SHA (watched services.rundeck.aclPolicies; not decided by the script)"
"$S" "$R" "$RUN" nixos/ '\brundeck\b' nixos/modules/services/web-apps/rundeck.nix
echo

echo "## P10 library binding: lib.boolToYesNo (lambda definition, not an alias)"
ex P10 "lib function definition" $E lib/trivial.nix 306 308

echo "## P10b library binding: lib.filter (alias chain)"
ex P10b "lib alias to builtin (lists)" $E lib/lists.nix 14 27
ex P10c "lib aggregation includes lists" $E lib/default.nix 266 270

echo "## P11 SYNTHETIC callee controls (no historical SHA)"
echo "  (a) let f = lib.filter; in f pred xs"
echo "  (b) with lib; filter pred xs"
echo "  (c) let filter = myFilter; in filter pred xs   (myFilter is a local helper)"
echo
