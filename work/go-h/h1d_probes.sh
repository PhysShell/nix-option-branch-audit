#!/usr/bin/env bash
# GO-H1D.12 bounded probes. Read-only. DIAGNOSTIC / NON-SCORING.
# Every excerpt is read from a pinned commit with git show or git grep; nothing reads a working tree.
# Synthetic controls are labelled SYNTHETIC and have no historical SHA.
set -uo pipefail
R=/home/tandem/.cache/go-h/nixpkgs-exact
S=/home/tandem/nix-option-branch-audit/work/go-h/exact_tree_search.sh
E=e4c7d977153965496cbe73ff5631ca6a1118b347
B=e2497c3a5262687ff7aada0707bc754cca66396f
F=fdc3396349781c0dc2b495b6a320f62918bd63f6
g() { git -C "$R" "$@"; }
show() { # sha path from to
  echo "# $1:$2 lines $3-$4"
  g show "$1:$2" | awk -v s="$3" -v e="$4" 'NR>=s && NR<=e {printf "%5d| %s\n", NR, $0}'
  echo
}
hdr() { # sha path line : nearest preceding "inherit (" header, the re-export source
  local h
  h=$(g show "$1:$2" | awk -v n="$3" 'NR<=n && /inherit \(/ {l=NR} END{print l}')
  echo "# nearest preceding inherit header for $2:$3 is line $h"
  show "$1" "$2" "$h" "$h"
}

echo "## A. G0(ii) primitives and alias chains at E ($E)"
echo "### A1 lib.mkIf: re-export header, definition, discharge documentation"
hdr $E lib/default.nix 496
show $E lib/default.nix 496 496
show $E lib/default.nix 71 71
show $E lib/modules.nix 1579 1582
show $E lib/modules.nix 1393 1398
echo "### A2 lib.optionals: re-export header, definition"
hdr $E lib/default.nix 289
show $E lib/default.nix 289 289
show $E lib/lists.nix 836 839
echo "### A3 lib.optionalString: re-export header, definition"
hdr $E lib/default.nix 350
show $E lib/default.nix 350 350
show $E lib/strings.nix 776 776
echo "### A4 lib.filter: builtins inherit, re-export, self/lists binding"
show $E lib/lists.nix 4 4
show $E lib/lists.nix 18 25
hdr $E lib/default.nix 278
show $E lib/default.nix 268 268
show $E lib/default.nix 278 278
show $E lib/default.nix 58 58
echo "### A5 lib.filterAttrs: definition, filter source inside attrsets.nix, re-export"
show $E lib/attrsets.nix 14 19
show $E lib/attrsets.nix 667 667
hdr $E lib/default.nix 218
show $E lib/default.nix 218 218
echo "### A6 lib.boolToYesNo: definition and re-export"
show $E lib/trivial.nix 305 308
hdr $E lib/default.nix 149
show $E lib/default.nix 149 149
echo "### A8 rebinding of lib / builtins in beszel-agent.nix at F (inherit or pattern forms)"
g grep -n -E 'inherit[^;]*\b(lib|builtins)\b|\}@|@ *\{' $F -- nixos/modules/services/monitoring/beszel-agent.nix | sed "s/^$F://" || true
echo "# (end A8)"

echo "## A7. Rebinding check in a caller at F ($F), beszel-agent.nix (lib.filter caller)"
show $F nixos/modules/services/monitoring/beszel-agent.nix 1 4
show $F nixos/modules/services/monitoring/beszel-agent.nix 48 54
echo "# rebinding grep for lib / builtins / let / with at F (none expected except with lib.<ns>)"
g grep -n -E '(^|[^A-Za-z0-9_.])(lib|builtins) *=|\blet\b|\bwith\b' $F -- nixos/modules/services/monitoring/beszel-agent.nix | sed "s/^$F://"
echo

echo "## B. Control dependence: enclosing gate versus inner predicate ($B)"
show $B nixos/modules/services/monitoring/beszel-agent.nix 22 26
show $B nixos/modules/services/monitoring/beszel-agent.nix 204 216
echo

echo "## C. Freeform keys versus declared suboptions ($E)"
show $E nixos/modules/services/audio/mpd.nix 153 185
show $E nixos/modules/services/accessibility/speechd.nix 36 48
echo "# count of freeformType declarations in nixos/modules/services at E"
g grep -c -E 'freeformType' $E -- nixos/modules/services/ | awk -F: '{s+=$NF} END{print "lines=" s}'
echo

echo "## D. Namespace closure ($E)"
echo "### D1 literal-leaf alias: ncdns whole-namespace alias with only literal accesses"
show $E nixos/modules/services/networking/ncdns.nix 8 9
show $E nixos/modules/services/networking/ncdns.nix 189 189
show $E nixos/modules/services/networking/ncdns.nix 207 209
echo "### D1b every use of the ncdns whole-namespace binding cfgs"
g grep -n -w cfgs $E -- nixos/modules/services/networking/ncdns.nix | sed "s/^$E://"
echo "### D2 whole-namespace iteration over config.services (expected: none)"
g grep -n -E '(attrNames|attrValues|mapAttrs|mapAttrs'"'"'|filterAttrs|attrByPath|getAttr)[^;]*config\.services([^A-Za-z0-9_.${-]|$)' $E -- nixos/ | sed "s/^$E://" | head -5
echo "# (end D2)"
echo "### D2s SYNTHETIC whole-namespace consumption (no historical SHA)"
echo "  let cfgs = config.services; in builtins.attrNames cfgs"
echo "  -> whole-namespace consumption; literal enumeration of cfgs.<name> does not close it"
echo "### D3 dynamic key with unknown domain: rancher config.services.\${name}"
show $E nixos/modules/services/cluster/rancher/default.nix 8 8
show $E nixos/modules/services/cluster/rancher/default.nix 22 25
echo "### D3b every caller of mkRancherModule in nixos/ (domain of the rancher name key)"
g grep -n -w mkRancherModule $E -- nixos/ | sed "s/^$E://"
show $E nixos/modules/services/cluster/rancher/default.nix 976 984
echo "### D4 finite literal-list domain: vault-agent config.services.\${flavour}"
show $E nixos/modules/services/security/vault-agent.nix 120 126
show $E nixos/modules/services/security/vault-agent.nix 129 149
echo "### D5 finite enum domain: movim config.services.\${cfg.database.type}"
show $E nixos/modules/services/web-apps/movim.nix 476 484
g grep -n -w 'database.type' $E -- nixos/modules/services/web-apps/movim.nix | sed "s/^$E://"
g grep -n -E 'cfg *= *config\.services\.movim|config\.services\.\$\{' $E -- nixos/modules/services/web-apps/movim.nix | sed "s/^$E://"
show $E nixos/modules/services/web-apps/movim.nix 622 628
echo

echo "## E. Helper with internal predicate and call-context identity ($E)"
show $E nixos/modules/misc/locate.nix 191 193
show $E nixos/modules/misc/locate.nix 242 245
show $E nixos/modules/misc/locate.nix 272 277
echo "# all boolToYesNo call sites in nixos/ at E"
g grep -n -w boolToYesNo $E -- nixos/ | sed "s/^$E://" | wc -l | sed 's/^/count=/'
echo "# boolToYesNo call sites in nixos/modules/misc/ and nixos/modules/config/ at E"
g grep -n -w boolToYesNo $E -- nixos/modules/misc/ nixos/modules/config/ | sed "s/^$E://"
echo

echo "## F. Exact-SHA sanity: definitions cited above are present at the pinned commits"
for sha in $E $B $F; do echo -n "$sha "; git -C "$R" rev-parse --verify --quiet "$sha^{commit}" >/dev/null && echo resolves || echo UNRESOLVED; done
echo
echo "## F2. Tree SHAs for the pinned commits (H1D.4 identity)"
for sha in $E $B $F; do echo -n "commit $sha tree "; g rev-parse --verify --quiet "$sha^{tree}" || echo UNRESOLVED; done
echo "# primitive definition blob SHAs at E"
for p in lib/modules.nix lib/lists.nix lib/strings.nix lib/attrsets.nix lib/trivial.nix lib/default.nix; do echo -n "$p blob "; g rev-parse --verify --quiet "$E:$p"; done
echo
echo "## G. Synthetic controls for H1D.3 callee identity (no historical SHA)"
echo "  (s1) let f = lib.filter; in f pred xs          -> callee resolves only if lib binding is exact-SHA proven"
echo "  (s2) let filter = myFilter; in filter pred xs  -> myFilter body unresolved -> P_UNKNOWN"
echo "  (s3) with lib; filter pred xs                  -> BARE, reachable through with -> P_UNKNOWN unless resolved"
