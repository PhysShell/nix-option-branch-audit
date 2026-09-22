#!/usr/bin/env python3
"""S5 design round: hostile self-tests proving the feature extractor
(`extract-features.py`) cannot leak `oba`-derived information into a
future S5-B selection decision.

Run with: python3 -m pytest fixtures/s5-design/test_no_leakage.py -v
(or `python3 fixtures/s5-design/test_no_leakage.py` for a standalone
run without pytest installed -- both paths exercised in CI, see
`.github/workflows/s5-design.yml`).
"""
import ast
import importlib.util
import inspect
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
EXTRACT_PATH = Path(__file__).resolve().parent / "extract-features.py"


def _code_only(path: Path) -> str:
    """Real code text with every docstring and `#` comment stripped --
    so a forbidden-pattern check below only fires on actual usage
    (a real `open("raw.json")`-shaped access), never on a docstring or
    comment that merely NAMES the forbidden thing while explaining why
    this code deliberately avoids it (which is exactly the style every
    module in this design round uses throughout). A first version of
    this test searched raw source text and flagged two false positives
    -- both docstring prose, not real usage -- caught immediately by
    actually running the test, not assumed correct from the code alone.
    """
    src = path.read_text()
    tree = ast.parse(src)
    docstring_ranges = []
    for node in ast.walk(tree):
        if isinstance(node, (ast.Module, ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            doc_node = ast.get_docstring(node, clean=False)
            if doc_node is not None and node.body and isinstance(node.body[0], ast.Expr):
                expr = node.body[0]
                docstring_ranges.append((expr.lineno, expr.end_lineno))
    lines = src.splitlines()
    out_lines = []
    for i, line in enumerate(lines, 1):
        if any(start <= i <= end for start, end in docstring_ranges):
            continue
        code_part = line.split("#", 1)[0]
        out_lines.append(code_part)
    return "\n".join(out_lines)


def _load_extract_module():
    spec = importlib.util.spec_from_file_location("ef", EXTRACT_PATH)
    mod = importlib.util.module_from_spec(spec)
    sys.modules["ef"] = mod
    spec.loader.exec_module(mod)
    return mod


def test_source_never_mentions_oba_binary_invocation():
    """Static check: the extractor's own source text must never
    construct a command line that runs the `oba` binary, and must
    never import `main`/`cdc` (this project's own analyzer modules).
    """
    src = EXTRACT_PATH.read_text()
    forbidden = [
        r"\boba_binary\b",
        r'"oba"\s*,',  # a subprocess arg list starting with the binary name
        r"\bcdc\.run_cdc_candidate\b",
        r"\bimport\s+main\b",
        r"verify-v0\d\d-scratch",  # the frozen S4/S4-F1-R binary paths
    ]
    for pattern in forbidden:
        assert not re.search(pattern, src), f"forbidden pattern found in extract-features.py: {pattern}"


def test_source_never_reads_oba_result_files():
    """Static check: the extractor's own source text must never open
    an oba-produced result file (raw.json, summary.md, check-*.json,
    the adjudication ledger) for the CANDIDATE PR it is featurizing.
    It's allowed to read the S4 LEDGER's own `pr`/`base_sha`/`head_sha`
    coordinates as input coordinates (that's how this design round's
    own historical corpus is addressed) -- but never `applicable`,
    `actionable_count`, or any other outcome field from it, and never
    any of the per-PR raw tool-output files at all.
    """
    code = _code_only(EXTRACT_PATH)
    forbidden_filenames = ["raw.json", "summary.md", "check-base.json", "check-head.json"]
    for name in forbidden_filenames:
        assert name not in code, f"extract-features.py's real code (not comments/docstrings) must never reference {name}"
    # The one legitimate ledger read (coordinates only) is confined to
    # main(); the actual per-PR feature function must never see the
    # ledger's own outcome fields as arguments.
    extract_fn_src = inspect.getsource(_load_extract_module().extract_features)
    forbidden_outcome_fields = [
        "applicable", "actionable_count", "pass_verdict_count",
        "tool_error_count", "false_pass", "false_finding",
    ]
    for field in forbidden_outcome_fields:
        assert field not in extract_fn_src, (
            f"extract_features() itself must never reference the outcome field {field!r} -- "
            "outcome labels belong only in extract-labels.py, joined AFTER feature extraction"
        )


def test_extract_features_signature_takes_no_outcome_arguments():
    """The only real inputs `extract_features()` accepts are (pr,
    base_sha, head_sha) -- a PR identity and two commit coordinates,
    nothing oba-shaped.
    """
    mod = _load_extract_module()
    sig = inspect.signature(mod.extract_features)
    params = list(sig.parameters.keys())
    assert params == ["pr", "base_sha", "head_sha"], (
        f"extract_features() must take exactly (pr, base_sha, head_sha), got {params}"
    )


def test_features_and_labels_are_physically_separate_files():
    """The leakage boundary this whole design insists on is easiest to
    audit if features and historical labels simply live in different
    files that no single script writes to both of.
    """
    d = Path(__file__).resolve().parent
    features_path = d / "features.jsonl"
    labels_path = d / "historical-labels.jsonl"
    assert features_path != labels_path
    extract_features_code = _code_only(EXTRACT_PATH)
    assert "historical-labels" not in extract_features_code, (
        "extract-features.py's real code must never read or write historical-labels.jsonl"
    )
    labels_script_code = _code_only(d / "extract-labels.py")
    assert "features.jsonl" not in labels_script_code, (
        "extract-labels.py's real code must never read or write features.jsonl"
    )


def test_enrichment_rules_module_never_touches_oba_or_result_files():
    """Static check over the candidate-rule implementation: the actual
    selector logic (the thing that would run for real on a fresh S5-B
    candidate) must be a pure function of the FEATURES dict alone --
    no filesystem/network access, no oba invocation, no ledger read.
    """
    rules_path = Path(__file__).resolve().parent / "enrichment-rules.py"
    src = rules_path.read_text()
    forbidden = [
        r"\bsubprocess\b", r"\bos\.system\b", r"\brequests\b",
        r'"oba"', r"raw\.json", r"summary\.md", r"adjudication-ledger",
        r"\bopen\(", r"Path\([^)]*\)\.read",
    ]
    for pattern in forbidden:
        assert not re.search(pattern, src), (
            f"enrichment-rules.py's own selector functions must be pure over a features dict -- "
            f"found forbidden pattern: {pattern}"
        )


def test_poisoned_features_file_is_rejected_by_the_validator():
    """Fail-closed proof: if a features.jsonl row is somehow poisoned
    with an oba-shaped key (e.g. a stray `verdict`/`actionable_count`
    field, simulating a future accidental merge of label data into the
    feature file), the schema validator used before any real S5-B
    selection must refuse it, not silently ignore the extra field.
    """
    mod = _load_extract_module()
    real_keys = set(mod.extract_features(1, "a" * 40, "b" * 40).keys()) if False else None
    # (real_keys intentionally unused/left None -- computing it would
    # require a real gh api call; the poisoned-key check below is
    # keyed off the FIXED, hand-audited allowlist in
    # validate_no_outcome_leakage, not a live call.)
    from importlib import import_module
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    validator = import_module("validate_features_schema")
    poisoned = {
        "pr": 1, "base_sha": "a" * 40, "head_sha": "b" * 40,
        "touches_nixos_modules": True,
        "actionable_count": 3,  # poisoned: an oba outcome field
    }
    errors = validator.validate_row(poisoned)
    assert errors, "a poisoned row with an outcome-shaped field must be rejected, not silently accepted"
    assert any("actionable_count" in e for e in errors)


if __name__ == "__main__":
    tests = [
        test_source_never_mentions_oba_binary_invocation,
        test_source_never_reads_oba_result_files,
        test_extract_features_signature_takes_no_outcome_arguments,
        test_features_and_labels_are_physically_separate_files,
        test_enrichment_rules_module_never_touches_oba_or_result_files,
        test_poisoned_features_file_is_rejected_by_the_validator,
    ]
    failures = 0
    for t in tests:
        try:
            t()
            print(f"PASS  {t.__name__}")
        except AssertionError as e:
            failures += 1
            print(f"FAIL  {t.__name__}: {e}")
    print(f"\n{len(tests) - failures}/{len(tests)} passed")
    if failures:
        raise SystemExit(1)
