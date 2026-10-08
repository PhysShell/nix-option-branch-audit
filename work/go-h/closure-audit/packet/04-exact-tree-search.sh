#!/usr/bin/env bash
# GO-H1B.1 exact-tree search. Searches one committed tree; never a working tree or a code-search index.
# Usage: exact_tree_search.sh <repo_dir> <required_full_sha> <scope_path> <extended_regex> [exclude_path] [receiver_regex]
# Output: key=value record. status=ZERO_COMPLETE is the only status that supports an absence claim.
#   MATCH          one or more matches (positive evidence per occurrence)
#   ZERO_COMPLETE  zero matches, tree resolved to the required SHA, scope non-empty, git grep exit 1
#   ERROR          anything else (bad SHA, missing object, empty scope, git grep failure): never negative evidence
# receiver_regex (optional) lists whole-attrset or dynamic receivers in the same scope. Their hits are printed
# (they are few) so a reviewer can resolve each binding. Generic computed access \.${ is reported as a count only.
set -uo pipefail

repo="$1"; sha="$2"; scope="$3"; pattern="$4"; excl="${5:-}"; recv="${6:-}"
pathspec=("$scope")
[[ -n "$excl" ]] && pathspec+=(":(exclude)$excl")

cd "$repo" || { echo "status=ERROR reason=repo_unreadable"; exit 0; }

if [[ ! "$sha" =~ ^[0-9a-f]{40}$ ]]; then
  echo "status=ERROR reason=sha_not_full_40_hex"; exit 0
fi

resolved="$(git rev-parse --verify --quiet "${sha}^{commit}" 2>/dev/null || true)"
if [[ "$resolved" != "$sha" ]]; then
  echo "status=ERROR reason=sha_not_resolved required=$sha resolved=${resolved:-none}"; exit 0
fi

files="$(git ls-tree -r --name-only "$sha" -- "$scope" 2>/dev/null | wc -l)"
if [[ "$files" -eq 0 ]]; then
  echo "status=ERROR reason=empty_or_missing_scope scope=$scope"; exit 0
fi

out="$(git grep -a -n -E -e "$pattern" "$sha" -- "${pathspec[@]}" 2>&1)"
rc=$?

case "$rc" in
  0) status=MATCH; matches="$(printf '%s\n' "$out" | grep -c .)" ;;
  1) status=ZERO_COMPLETE; matches=0 ;;
  *) status=ERROR; matches=-1 ;;
esac

dyn_count="$(git grep -a -c -E -e '\.\$\{' "$sha" -- "${pathspec[@]}" 2>/dev/null | wc -l)"

echo "status=$status sha=$sha scope=$scope exclude=${excl:-none} files_in_scope=$files pattern=$pattern grep_exit=$rc matches=$matches generic_computed_files=$dyn_count"
echo "command=git grep -a -n -E -e '$pattern' $sha -- $scope ${excl:+:(exclude)$excl}"
if [[ "$status" == MATCH ]]; then printf '%s\n' "$out"; fi
if [[ "$status" == ERROR ]]; then printf 'detail=%s\n' "$out"; fi
if [[ -n "$recv" ]]; then
  rout="$(git grep -a -n -E -e "$recv" "$sha" -- "${pathspec[@]}" 2>/dev/null)"
  printf 'receiver_hits=%s\n' "$(printf '%s' "$rout" | grep -c . || true)"
  [[ -n "$rout" ]] && printf '%s\n' "$rout"
fi
