#!/usr/bin/env bash
# GO-H1B.3 controls for the exact-tree search procedure. Output is evidence, not a verdict.
set -uo pipefail
S=/home/tandem/nix-option-branch-audit/work/go-h/exact_tree_search.sh
R=/home/tandem/.cache/go-h/nixpkgs-exact
E=e4c7d977153965496cbe73ff5631ca6a1118b347
BESZEL=$(git -C "$R" rev-parse e2497c3a)
ZFS=$(git -C "$R" rev-parse e812ab65)
HEAD563=b063b8f9b2a655f72cec13f1877522c3e4066ac2
BASE563=$(git -C "$R" rev-parse 3d35b67b)

echo "### P1 positive control: config.boot.zfs.enabled consumers (beszel-agent.nix @ e2497c3a)"
"$S" "$R" "$BESZEL" nixos/modules/services/monitoring/beszel-agent.nix 'config\.boot\.zfs\.enabled'
echo "### P2 positive control: zfs.nix declaration (@ e812ab65)"
"$S" "$R" "$ZFS" nixos/modules/tasks/filesystems/zfs.nix 'enabled = lib\.mkOption'
echo "### N1 known negative, owner-key identifier (gophernicus outside own module @ e4c7d977)"
"$S" "$R" "$E" nixos/ '\bgophernicus\b' nixos/modules/services/misc/gophernicus.nix \
  '\(config\.services\)|config\.services *[;)]|config\.services\.\$\{|with config\.services'
echo "### N2 leaf-only diagnostic (rootDir selections outside own module @ e4c7d977)"
"$S" "$R" "$E" nixos/ '\.rootDir\b' nixos/modules/services/misc/gophernicus.nix
echo "### N3 container-path-only diagnostic (services.gophernicus. outside own module @ e4c7d977)"
"$S" "$R" "$E" nixos/ 'services\.gophernicus\.' nixos/modules/services/misc/gophernicus.nix
echo "### PC1 prior case aclPolicies (head b063b8f9)"
"$S" "$R" "$HEAD563" nixos/ 'aclPolicies'
echo "### PC2 prior case aclPolicies (base 3d35b67b)"
"$S" "$R" "$BASE563" nixos/ 'aclPolicies'
