#!/usr/bin/env bash
# GO-H H5 DIAGNOSTIC / NON-SCORING: wired-test semantics on a synthetic fixture.
# Runs the frozen oba candidate (sha256 406a27f5...3498) unchanged.
set -euo pipefail
cd "$(dirname "$0")/../.."
OBA=work/go-f/oba-candidate-9ff7c04
FX=work/go-h/fixtures/h5
OUT=work/go-h/raw/h5
MAN=work/go-h/manifests
test -x "$OBA"
for variant in set noset empty absent; do
  case $variant in
    set) tf=nixos/tests/demo-set.nix ;;
    noset) tf=nixos/tests/demo-noset.nix ;;
    empty) tf=nixos/tests/demo-empty.nix ;;
    absent) tf=nixos/tests/demo-absent.nix ;;
  esac
  m="$MAN/h5-$variant.toml"
  : > "$m"
  for opt in openFirewall port package; do
    cat >> "$m" <<T
[[target]]
name = "$opt-$variant"
module = "nixos/modules/demo.nix"
test = "$tf"
cfg_ident = "cfg"
option_prefix = ["services", "demo"]
watch = ["$opt"]

T
  done
  "$OBA" diff --base-root "$FX/base" --head-root "$FX/head" --targets "$m" --json > "$OUT/diff-$variant.json" 2> "$OUT/diff-$variant.stderr" && rc=0 || rc=$?
  echo "$rc" > "$OUT/diff-$variant.exit"
  "$OBA" check --root "$FX/head" --targets "$m" --json > "$OUT/check-head-$variant.json" 2> "$OUT/check-head-$variant.stderr" && rc=0 || rc=$?
  echo "$rc" > "$OUT/check-head-$variant.exit"
done
echo done
