#!/usr/bin/env python3
"""Isolation self-tests for `production_selector.py`.

Two independent proofs:
1. RUNTIME isolation: copy ONLY `production_selector.py` into a fresh
   temp directory containing nothing else at all -- no ledger, no
   `oba` binary, no result-artifact directories PHYSICALLY PRESENT on
   the filesystem (not just empty/unreadable) -- and run it there as a
   real subprocess. If it can only work by silently reaching outside
   its own directory, this fails loudly (ImportError/FileNotFoundError)
   rather than passing by accident.
2. STATIC isolation: the selector's real code (docstrings/comments
   stripped, reusing `test_no_leakage.py`'s own `_code_only` helper via
   import, not a re-implementation) never mentions `oba`,
   `adjudication-ledger`, `raw.json`, `summary.md`, or `check-`.

Run with: python3 fixtures/s5-design/selector/test_selector_isolation.py
"""
import importlib.util
import json
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
SELECTOR_PATH = HERE / "production_selector.py"
DESIGN_DIR = HERE.parent


def _load_code_only_helper():
    spec = importlib.util.spec_from_file_location("_no_leakage", DESIGN_DIR / "test_no_leakage.py")
    mod = importlib.util.module_from_spec(spec)
    sys.modules["_no_leakage"] = mod
    spec.loader.exec_module(mod)
    return mod._code_only


def test_selector_runs_correctly_from_an_isolated_directory_with_no_other_files():
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        isolated_selector = tmp_path / "production_selector.py"
        isolated_selector.write_text(SELECTOR_PATH.read_text())

        # Prove physical absence, not just "unreadable": list the dir
        # and assert it holds ONLY the one file we just copied in.
        entries = sorted(p.name for p in tmp_path.iterdir())
        assert entries == ["production_selector.py"], f"isolated dir is not actually isolated: {entries}"

        input_records = [
            {
                "pr": 999001,
                "base_sha": "a" * 40,
                "head_sha": "b" * 40,
                "files": [
                    {
                        "filename": "nixos/modules/services/web-servers/example.nix",
                        "status": "modified",
                        "patch": "@@ -1,2 +1,3 @@\n+  enable = mkEnableOption \"example\";\n",
                    }
                ],
            },
            {
                "pr": 999002,
                "base_sha": "a" * 40,
                "head_sha": "b" * 40,
                "files": [
                    {
                        "filename": "nixos/modules/services/web-servers/other.nix",
                        "status": "modified",
                        "patch": "@@ -1,2 +1,2 @@\n-  x = 1;\n+  x = 2;\n",
                    }
                ],
            },
        ]
        input_path = tmp_path / "input.json"
        input_path.write_text(json.dumps(input_records))

        proc = subprocess.run(
            [sys.executable, str(isolated_selector), str(input_path)],
            cwd=tmp_path,
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert proc.returncode == 0, f"selector failed in isolation: stderr={proc.stderr}"
        lines = [json.loads(l) for l in proc.stdout.splitlines() if l.strip()]
        assert len(lines) == 2
        by_pr = {r["pr"]: r for r in lines}
        assert by_pr[999001]["selected"] is True
        assert by_pr[999002]["selected"] is False
        assert all(r["unresolved"] is False for r in lines)


def test_selector_source_never_mentions_oba_or_result_artifacts():
    code_only = _load_code_only_helper()
    code = code_only(SELECTOR_PATH)
    forbidden = ["oba", "adjudication-ledger", "raw.json", "summary.md", "check-"]
    for token in forbidden:
        assert token not in code, (
            f"production_selector.py's real code (docstrings/comments stripped) "
            f"must never mention {token!r} -- found a real usage"
        )


if __name__ == "__main__":
    tests = [
        test_selector_runs_correctly_from_an_isolated_directory_with_no_other_files,
        test_selector_source_never_mentions_oba_or_result_artifacts,
    ]
    failures = 0
    for t in tests:
        try:
            t()
            print(f"PASS  {t.__name__}")
        except AssertionError as e:
            failures += 1
            print(f"FAIL  {t.__name__}: {e}")
        except Exception as e:
            failures += 1
            print(f"ERROR {t.__name__}: {type(e).__name__}: {e}")
    print(f"\n{len(tests) - failures}/{len(tests)} passed")
    if failures:
        raise SystemExit(1)
