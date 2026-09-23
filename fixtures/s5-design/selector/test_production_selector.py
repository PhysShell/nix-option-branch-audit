#!/usr/bin/env python3
"""Hostile tests for `production_selector.py`'s fail-closed
completeness handling and the frozen R4_mkoption_line_edit_v1 rule
itself. Run with: python3 fixtures/s5-design/selector/test_production_selector.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from production_selector import select  # noqa: E402


def _relevant_file(filename="nixos/modules/services/web-servers/x.nix", status="modified", patch=None):
    f = {"filename": filename, "status": status}
    if patch is not None:
        f["patch"] = patch
    return f


def test_missing_patch_on_relevant_file_is_unresolved():
    record = {"pr": 1, "base_sha": "a", "head_sha": "b", "files": [_relevant_file(patch=None)]}
    result = select(record)
    assert result["unresolved"] is True
    assert result["selected"] is None
    assert any("missing 'patch'" in r for r in result["reasons"])


def test_missing_patch_on_relevant_file_unresolved_even_if_another_file_has_the_token():
    """A resolved file elsewhere must NOT paper over an unresolved one --
    the whole PR is unresolved if ANY relevant file lacks patch data."""
    record = {
        "pr": 2, "base_sha": "a", "head_sha": "b",
        "files": [
            _relevant_file("nixos/modules/services/a/a.nix", patch="+  mkOption {}\n"),
            _relevant_file("nixos/modules/services/b/b.nix", patch=None),
        ],
    }
    result = select(record)
    assert result["unresolved"] is True
    assert result["selected"] is None


def test_exactly_300_files_without_proof_is_unresolved():
    files = [_relevant_file(f"nixos/modules/services/{i}/f.nix", patch="") for i in range(300)]
    record = {"pr": 3, "base_sha": "a", "head_sha": "b", "files": files}
    result = select(record)
    assert result["unresolved"] is True
    assert any("300" in r for r in result["reasons"])


def test_exactly_300_files_with_independently_proven_completeness_is_not_blocked_by_truncation():
    files = [_relevant_file(f"nixos/modules/services/{i}/f.nix", patch="") for i in range(300)]
    record = {"pr": 4, "base_sha": "a", "head_sha": "b", "files": files, "files_complete": True}
    result = select(record)
    # Not blocked by the truncation check specifically -- resolves normally
    # (no mkOption tokens present here, so selected=False, not unresolved).
    assert result["unresolved"] is False
    assert result["selected"] is False


def test_missing_patch_on_irrelevant_file_does_not_block_selection():
    record = {
        "pr": 5, "base_sha": "a", "head_sha": "b",
        "files": [
            {"filename": "pkgs/by-name/xx/foo/package.nix", "status": "modified"},  # no patch key at all
            _relevant_file(patch="+  mkEnableOption \"x\";\n"),
        ],
    }
    result = select(record)
    assert result["unresolved"] is False
    assert result["selected"] is True
    assert result["reasons"] == ["mkenableoption_edit_count=1"]


def test_missing_patch_on_binary_file_does_not_block_selection():
    """GitHub itself omits `patch` for binary files -- must never block."""
    record = {
        "pr": 6, "base_sha": "a", "head_sha": "b",
        "files": [
            {"filename": "nixos/doc/manual/images/logo.png", "status": "modified"},
            _relevant_file(patch="no tokens here\n"),
        ],
    }
    result = select(record)
    assert result["unresolved"] is False
    assert result["selected"] is False


def test_ordinary_complete_diff_with_mkoption_edit_selects_positive_control():
    record = {
        "pr": 7, "base_sha": "a", "head_sha": "b",
        "files": [_relevant_file(patch="@@ -1,3 +1,4 @@\n+  someOption = mkOption {\n   type = types.bool;\n")],
    }
    result = select(record)
    assert result["unresolved"] is False
    assert result["selected"] is True
    assert result["reasons"] == ["mkoption_edit_count=1"]


def test_ordinary_complete_diff_with_no_matching_tokens_does_not_select_negative_control():
    record = {
        "pr": 8, "base_sha": "a", "head_sha": "b",
        "files": [_relevant_file(patch="@@ -1,2 +1,2 @@\n-  ExecStart = \"old\";\n+  ExecStart = \"new\";\n")],
    }
    result = select(record)
    assert result["unresolved"] is False
    assert result["selected"] is False
    assert result["reasons"] == []


def test_removed_context_only_mkoption_is_not_counted_as_added_removed_line():
    """A context line (no leading +/-) that happens to mention mkOption
    must not count -- only real +/- diff content lines do."""
    record = {
        "pr": 9, "base_sha": "a", "head_sha": "b",
        "files": [_relevant_file(patch="@@ -1,3 +1,3 @@\n   existingOption = mkOption {\n-  x = 1;\n+  x = 2;\n")],
    }
    result = select(record)
    assert result["unresolved"] is False
    assert result["selected"] is False


def test_malformed_response_missing_files_key_is_unresolved_not_a_crash():
    record = {"pr": 10, "base_sha": "a", "head_sha": "b"}
    result = select(record)
    assert result["unresolved"] is True
    assert any("malformed_input" in r for r in result["reasons"])


def test_malformed_response_files_not_a_list_is_unresolved_not_a_crash():
    record = {"pr": 11, "base_sha": "a", "head_sha": "b", "files": "not-a-list"}
    result = select(record)
    assert result["unresolved"] is True
    assert any("malformed_input" in r for r in result["reasons"])


def test_malformed_file_entry_missing_filename_is_unresolved_not_a_crash():
    record = {"pr": 12, "base_sha": "a", "head_sha": "b", "files": [{"status": "modified", "patch": "x"}]}
    result = select(record)
    assert result["unresolved"] is True
    assert any("malformed_input" in r for r in result["reasons"])


def test_removed_relevant_file_with_patch_is_evaluated_normally():
    record = {
        "pr": 13, "base_sha": "a", "head_sha": "b",
        "files": [_relevant_file(status="removed", patch="-  opt = mkEnableOption \"x\";\n")],
    }
    result = select(record)
    assert result["unresolved"] is False
    assert result["selected"] is True


def test_no_relevant_files_at_all_resolves_to_not_selected():
    record = {
        "pr": 14, "base_sha": "a", "head_sha": "b",
        "files": [{"filename": "nixos/tests/foo.nix", "status": "modified", "patch": "+  services.foo.mkOption\n"}],
    }
    result = select(record)
    assert result["unresolved"] is False
    assert result["selected"] is False


if __name__ == "__main__":
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
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
