#!/usr/bin/env python3
"""S5-B FROZEN production selector: `R4_mkoption_line_edit_v1`.

Genuinely separate from `../extract-features.py`, which is explicitly
insufficient for real fresh-PR selection: its own `main()` reads the
S4 adjudication ledger for (pr, base_sha, head_sha) coordinates. This
module has NO import of, and no path reference anywhere in its source
to, any adjudication ledger, `raw.json`, `summary.md`, `check-*.json`,
or the `oba` binary. It is fully self-contained (its own copies of the
diff-line/regex helpers, deliberately NOT imported from
`extract-features.py`) so it can be copied alone into an empty
directory and still run correctly -- see `test_selector_isolation.py`.

Frozen rule (verbatim from `enrichment-rules.py`'s own
`r4_mkoption_line_edit_v1` docstring): select iff at least one ADDED
or REMOVED diff line in a `nixos/modules/services/**` file contains
the literal token `mkOption` or `mkEnableOption`. Lexical line-edit
detection only -- no AST, no add/remove/lifecycle claim.

Input record shape (one per PR), a light normalization of GitHub's own
Compare API response plus one extra field this module itself requires
for fail-closed completeness proof:

    {
      "pr": int,
      "base_sha": str,
      "head_sha": str,
      "files": [
        {"filename": str, "status": str, "patch": str | absent},
        ...
      ],
      "files_complete": bool  # OPTIONAL. True only when the caller has
        # INDEPENDENTLY proven the `files` list is the PR's complete
        # file list (e.g. len(files) == the PR's own `changed_files`
        # count from a separate, non-Compare-API source) -- never a
        # value this module infers from `files` alone. Absent/False/
        # None all mean "not proven".
    }

Fail-closed completeness (this is the actual bug being fixed -- the
historical `extract-features.py` used `f.get("patch", "")`, silently
treating a missing patch as "no edit"; confirmed empirically NOT to
have corrupted the historical 187-PR corpus, but a real risk for any
unvetted fresh PR):

  - A `nixos/modules/services/**` file (the only files R4 examines)
    with a missing/None `patch` makes the WHOLE PR's selection
    `unresolved`, regardless of what any other file shows.
  - Exactly 300 entries in `files` is GitHub's documented Compare API
    truncation ceiling. Unless `files_complete` is `True`, this makes
    the PR `unresolved` too -- the true file list might extend past
    what was returned, and a relevant file could be hiding beyond it.
  - A missing `patch` on any file OUTSIDE `nixos/modules/services/**`
    (an irrelevant file, or a binary file GitHub itself omits `patch`
    for) never blocks selection -- R4 never looks at those files.
  - A malformed record (`files` missing, or not a list, or a file
    entry missing `filename`) is `unresolved` with an explicit reason,
    never a silent `False` and never an uncaught crash.
"""
import json
import re
import sys

RE_MKOPTION = re.compile(r"\bmkOption\b")
RE_MKENABLEOPTION = re.compile(r"\bmkEnableOption\b")

RELEVANT_PREFIX = "nixos/modules/services/"
TRUNCATION_CEILING = 300

SELECTED_BY = "R4_mkoption_line_edit_v1"


def diff_lines(patch: str) -> list[str]:
    """Added/removed content lines only, `+++`/`---` headers stripped."""
    out = []
    for line in patch.splitlines():
        if line.startswith("+++") or line.startswith("---"):
            continue
        if line.startswith("+") or line.startswith("-"):
            out.append(line)
    return out


def count_matches(pattern: re.Pattern, lines: list[str]) -> int:
    return sum(1 for l in lines if pattern.search(l))


class MalformedRecordError(Exception):
    pass


def select(record: dict) -> dict:
    """Returns a `selection_record`-shaped dict (see
    `ledger-schema-draft.json`'s `selection_record` type), extended
    with `unresolved`/`unresolved_reasons`. Never raises for a
    malformed record -- callers get `unresolved=True` with a reason
    instead, so a batch run over many PRs can't be aborted by one bad
    record.
    """
    pr = record.get("pr")
    try:
        files = record["files"]
        if not isinstance(files, list):
            raise MalformedRecordError(f"'files' is not a list: {type(files).__name__}")
        for f in files:
            if not isinstance(f, dict) or "filename" not in f:
                raise MalformedRecordError("a file entry is missing 'filename'")
    except KeyError:
        return _unresolved(pr, ["malformed_input: 'files' key missing from record"])
    except MalformedRecordError as e:
        return _unresolved(pr, [f"malformed_input: {e}"])

    unresolved_reasons = []

    if len(files) == TRUNCATION_CEILING and record.get("files_complete") is not True:
        unresolved_reasons.append(
            f"files list has exactly {TRUNCATION_CEILING} entries (Compare API truncation "
            "ceiling) and 'files_complete' was not independently proven True"
        )

    relevant_files = [f for f in files if f["filename"].startswith(RELEVANT_PREFIX)]
    for f in relevant_files:
        patch = f.get("patch")
        if patch is None:
            unresolved_reasons.append(
                f"missing 'patch' on relevant file {f['filename']!r} (status={f.get('status', '?')})"
            )

    if unresolved_reasons:
        return _unresolved(pr, unresolved_reasons)

    module_lines: list[str] = []
    for f in relevant_files:
        module_lines.extend(diff_lines(f.get("patch", "")))

    mkoption_edit_count = count_matches(RE_MKOPTION, module_lines)
    mkenableoption_edit_count = count_matches(RE_MKENABLEOPTION, module_lines)
    selected = mkoption_edit_count > 0 or mkenableoption_edit_count > 0

    reasons = []
    if mkoption_edit_count > 0:
        reasons.append(f"mkoption_edit_count={mkoption_edit_count}")
    if mkenableoption_edit_count > 0:
        reasons.append(f"mkenableoption_edit_count={mkenableoption_edit_count}")

    return {
        "record_type": "selection_record",
        "pr": pr,
        "selected_by": SELECTED_BY,
        "score": int(selected),
        "selected": selected,
        "unresolved": False,
        "reasons": reasons,
    }


def _unresolved(pr, reasons: list[str]) -> dict:
    return {
        "record_type": "selection_record",
        "pr": pr,
        "selected_by": SELECTED_BY,
        "score": None,
        "selected": None,
        "unresolved": True,
        "reasons": reasons,
    }


def main():
    """CLI: reads a JSON array of input records from stdin (or a file
    path given as argv[1]), writes one selection_record JSON line per
    input record to stdout. No other I/O, no network, no filesystem
    access beyond this one input.
    """
    if len(sys.argv) > 1:
        with open(sys.argv[1]) as f:
            records = json.load(f)
    else:
        records = json.load(sys.stdin)
    if not isinstance(records, list):
        print(json.dumps({"error": "top-level input must be a JSON array of records"}), file=sys.stderr)
        raise SystemExit(1)
    for record in records:
        print(json.dumps(select(record)))


if __name__ == "__main__":
    main()
