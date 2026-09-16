#!/usr/bin/env bash
# Re-derives each fixture's sha256 directly from the upstream commit named
# in fixtures/integrity-lock.toml's repo/commit/upstream_path fields, via
# `git show`, and compares against the locked hash AND the local fixture
# file's actual hash. This is the actual provenance check -- "this hash
# really did come from that exact commit" -- as opposed to
# tests/fixture_integrity.rs, which only checks the local fixture still
# matches the locked hash (an integrity check, not a provenance check; see
# that file's header for why they're deliberately separate).
#
# Requires a local checkout of the named repo (not fetched automatically --
# this deliberately stays out of `cargo test` and off the network by
# default). Usage:
#   scripts/verify-upstream.sh /path/to/nixpkgs-checkout
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: $0 <path-to-repo-checkout>" >&2
  echo "  the checkout must have the commits named in fixtures/integrity-lock.toml" >&2
  echo "  reachable (e.g. a full clone, or one with those refs fetched)" >&2
  exit 2
fi

REPO_CHECKOUT="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
LOCK_FILE="$ROOT_DIR/fixtures/integrity-lock.toml"

if [ ! -d "$REPO_CHECKOUT/.git" ]; then
  echo "error: $REPO_CHECKOUT is not a git checkout" >&2
  exit 2
fi

fail=0
count=0

# Parses the flat [[fixture]] table by reading four consecutive key = "value"
# lines per entry -- fragile if the TOML is reformatted, but this file is
# machine-generated/-maintained, not hand-edited freely, so that's an
# acceptable trade for not pulling in a TOML parser for a shell script.
while IFS= read -r path && IFS= read -r repo && IFS= read -r commit && IFS= read -r upstream_path && IFS= read -r locked_sha; do
  count=$((count + 1))
  echo "checking $path (claimed: $repo@$commit:$upstream_path)"

  if ! upstream_sha=$(git -C "$REPO_CHECKOUT" show "$commit:$upstream_path" 2>/dev/null | sha256sum | cut -d' ' -f1); then
    echo "  FAIL: could not read $commit:$upstream_path from $REPO_CHECKOUT (commit not present locally?)" >&2
    fail=1
    continue
  fi

  local_path="$ROOT_DIR/$path"
  if [ ! -f "$local_path" ]; then
    echo "  FAIL: local fixture $path does not exist" >&2
    fail=1
    continue
  fi
  local_sha=$(sha256sum "$local_path" | cut -d' ' -f1)

  if [ "$upstream_sha" != "$locked_sha" ]; then
    echo "  FAIL: upstream commit content does not match the locked hash" >&2
    echo "    locked:   $locked_sha" >&2
    echo "    upstream: $upstream_sha" >&2
    fail=1
  elif [ "$local_sha" != "$locked_sha" ]; then
    echo "  FAIL: local fixture does not match the locked hash (run cargo test, tests/fixture_integrity.rs will also catch this)" >&2
    fail=1
  else
    echo "  OK"
  fi
done < <(
  awk '
    /^path *=/     { gsub(/.*= *"|"$/, ""); path=$0 }
    /^repo *=/     { gsub(/.*= *"|"$/, ""); repo=$0 }
    /^commit *=/   { gsub(/.*= *"|"$/, ""); commit=$0 }
    /^upstream_path *=/ { gsub(/.*= *"|"$/, ""); upath=$0 }
    /^sha256 *=/ {
      gsub(/.*= *"|"$/, "");
      print path; print repo; print commit; print upath; print $0
    }
  ' "$LOCK_FILE"
)

echo
echo "checked $count fixture(s)"
if [ "$fail" -ne 0 ]; then
  echo "verify-upstream: FAILED" >&2
  exit 1
fi
echo "verify-upstream: all fixtures verified against upstream"
