#!/usr/bin/env python3
"""S5 design round: pure source-diff feature extractor.

CRITICAL INVARIANT (leakage prevention): this script NEVER imports,
invokes, or reads output from the `oba` binary, and NEVER reads any
S1-S4 result/adjudication file (raw.json, summary.md, check-*.json,
adjudication-ledger.jsonl) for the PR being featurized. It reads only:
  - PR metadata (number, base_sha, head_sha) from the S4 ledger's own
    `pr_summary` records -- used ONLY as (pr, base_sha, head_sha)
    coordinates, never for `applicable`/`actionable_count`/any outcome
    field.
  - The real diff between base_sha and head_sha, fetched fresh via
    `gh api repos/NixOS/nixpkgs/compare/<base>...<head>` (file list +
    unified-diff patches), independent of whether the PR is still open.

See `test_no_leakage.py` for a machine-enforced proof of this
invariant (this module is statically inspected + run against a
poisoned fixture directory to confirm it fails closed if oba-shaped
data is present where it shouldn't be).
"""
import json
import re
import subprocess
import time
from pathlib import Path

REPO = "NixOS/nixpkgs"


def gh_compare(base_sha: str, head_sha: str, attempts: int = 4) -> dict:
    for attempt in range(attempts):
        proc = subprocess.run(
            ["gh", "api", f"repos/{REPO}/compare/{base_sha}...{head_sha}"],
            capture_output=True, text=True,
        )
        if proc.returncode == 0:
            return json.loads(proc.stdout)
        if attempt < attempts - 1:
            time.sleep(2 * (attempt + 1))
            continue
        raise RuntimeError(f"gh api compare failed for {base_sha}...{head_sha}: {proc.stderr}")
    raise RuntimeError("unreachable")


# --- regex vocabularies, applied ONLY to +/- diff lines (never full
# file content, never oba's own output) ------------------------------

RE_MKOPTION = re.compile(r"\bmkOption\b")
RE_MKENABLEOPTION = re.compile(r"\bmkEnableOption\b")
RE_OPTIONS_BLOCK = re.compile(r"\boptions\s*=")
RE_OPTIONS_DOTTED = re.compile(r"\boptions\.[a-zA-Z_][\w.]*\s*=")
RE_DEFAULT_BOOL = re.compile(r"\bdefault\s*=\s*(true|false)\b")
RE_TYPE_DECL = re.compile(r"\btype\s*=\s*(types\.|lib\.types\.)")
RE_SUBMODULE = re.compile(r"\b(submodule|attrsOf)\b")

RE_MKIF = re.compile(r"\bmkIf\b")
RE_OPTIONALATTRS = re.compile(r"\boptionalAttrs\b")
RE_OPTIONALSTRING = re.compile(r"\boptionalString\b")
RE_LIB_OPTIONALS = re.compile(r"\blib\.optionals\b|\boptionals\b")
RE_MKMERGE = re.compile(r"\bmkMerge\b")
RE_CFG_REF = re.compile(r"\bcfg\.[a-zA-Z_]")
RE_CONFIG_SERVICES = re.compile(r"\bconfig\.services\.[a-zA-Z_]")
RE_COMPARE_LITERAL = re.compile(r"==\s*(\{\}|\[\]|null|true|false|\"[^\"]*\")|!=\s*(\{\}|\[\]|null|true|false)")

RE_TEST_SERVICES_ASSIGN = re.compile(r"\bservices\.[a-zA-Z_][\w.]*\s*=")
RE_SPECIALISATION = re.compile(r"\bspecialisation\.[^\s.]+\.configuration\b")
RE_ALL_TESTS_WIRING = re.compile(r"runTest\b")

RE_EXECSTART = re.compile(r"\bExecStart\b")
RE_ENVIRONMENT = re.compile(r"\bEnvironment(File)?\b|\benvironment\.[a-zA-Z]")
RE_SYSTEMD_SERVICE = re.compile(r"\bsystemd\.(services|sockets|timers)\.[a-zA-Z_]")
RE_FIREWALL = re.compile(r"\bopenFirewall\b|\bfirewall\.[a-zA-Z]|\ballowedTCPPorts\b|\ballowedUDPPorts\b")
RE_USERS_GROUPS = re.compile(r"\busers\.(users|groups)\.[a-zA-Z_]")
RE_SOCKET_ACTIVATION = re.compile(r"\bsocketActivation\b|\.socket\b")
RE_PACKAGE_VERSION_BUMP = re.compile(r"^[+-]\s*version\s*=|^[+-]\s*src\s*=\s*fetch|^[+-]\s*rev\s*=|sha256-[A-Za-z0-9+/=]{20,}")


def diff_lines(patch: str) -> list[str]:
    """+/- content lines only, headers stripped -- never the full file."""
    out = []
    for line in patch.splitlines():
        if line.startswith("+++") or line.startswith("---"):
            continue
        if line.startswith("+") or line.startswith("-"):
            out.append(line)
    return out


def count_matches(pattern: re.Pattern, lines: list[str]) -> int:
    return sum(1 for l in lines if pattern.search(l))


def basename_no_ext(path: str) -> str:
    return Path(path).stem


def extract_features(pr: int, base_sha: str, head_sha: str) -> dict:
    compare = gh_compare(base_sha, head_sha)
    files = compare.get("files", [])

    module_files = [f for f in files if f["filename"].startswith("nixos/modules/")]
    services_module_files = [f for f in files if f["filename"].startswith("nixos/modules/services/")]
    test_files = [f for f in files if f["filename"].startswith("nixos/tests/")]

    module_added = [f for f in services_module_files if f["status"] == "added"]
    module_removed = [f for f in services_module_files if f["status"] == "removed"]
    module_renamed = [f for f in services_module_files if f["status"] == "renamed"]
    test_added = [f for f in test_files if f["status"] == "added"]
    test_removed = [f for f in test_files if f["status"] == "removed"]

    module_subjects = {basename_no_ext(f["filename"]) for f in services_module_files}
    test_subjects = {basename_no_ext(f["filename"]) for f in test_files}
    same_subject_module_and_test = bool(module_subjects & test_subjects)

    # Aggregate +/- lines by file family, from real patches only.
    module_lines: list[str] = []
    test_lines: list[str] = []
    for f in services_module_files:
        module_lines.extend(diff_lines(f.get("patch", "")))
    for f in test_files:
        test_lines.extend(diff_lines(f.get("patch", "")))
    all_lines = module_lines + test_lines

    option_decl_changed = (
        count_matches(RE_MKOPTION, module_lines) > 0
        or count_matches(RE_MKENABLEOPTION, module_lines) > 0
        or count_matches(RE_OPTIONS_DOTTED, module_lines) > 0
    )
    predicate_logic_changed = (
        count_matches(RE_MKIF, module_lines) > 0
        or count_matches(RE_OPTIONALATTRS, module_lines) > 0
        or count_matches(RE_OPTIONALSTRING, module_lines) > 0
        or count_matches(RE_LIB_OPTIONALS, module_lines) > 0
        or count_matches(RE_MKMERGE, module_lines) > 0
    )
    test_assignment_changed = count_matches(RE_TEST_SERVICES_ASSIGN, test_lines) > 0
    specialisation_changed = count_matches(RE_SPECIALISATION, test_lines) > 0
    all_tests_wiring_changed = any(
        f["filename"] == "nixos/tests/all-tests.nix" for f in files
    )

    runtime_side_changed = (
        count_matches(RE_EXECSTART, module_lines) > 0
        or count_matches(RE_ENVIRONMENT, module_lines) > 0
        or count_matches(RE_SYSTEMD_SERVICE, module_lines) > 0
        or count_matches(RE_FIREWALL, module_lines) > 0
        or count_matches(RE_USERS_GROUPS, module_lines) > 0
        or count_matches(RE_SOCKET_ACTIVATION, module_lines) > 0
    )
    package_version_bump_lines = count_matches(RE_PACKAGE_VERSION_BUMP, all_lines)
    total_module_test_lines = len(all_lines)
    pure_package_bump = (
        package_version_bump_lines > 0
        and not option_decl_changed
        and not predicate_logic_changed
        and not test_assignment_changed
        and len(services_module_files) == 0
    )

    return {
        "pr": pr,
        "base_sha": base_sha,
        "head_sha": head_sha,
        # repository/path features
        "touches_nixos_modules": len(module_files) > 0,
        "touches_nixos_modules_services": len(services_module_files) > 0,
        "touches_nixos_tests": len(test_files) > 0,
        "num_module_files": len(services_module_files),
        "num_test_files": len(test_files),
        "same_subject_module_and_test": same_subject_module_and_test,
        "module_file_added": len(module_added) > 0,
        "module_file_removed": len(module_removed) > 0,
        "test_file_added": len(test_added) > 0,
        "test_file_removed": len(test_removed) > 0,
        "module_renamed_or_moved": len(module_renamed) > 0,
        "module_birth_or_death": len(module_added) > 0 or len(module_removed) > 0,
        # option-declaration diff features
        "option_declaration_changed": option_decl_changed,
        "mkoption_edit_count": count_matches(RE_MKOPTION, module_lines),
        "mkenableoption_edit_count": count_matches(RE_MKENABLEOPTION, module_lines),
        "default_bool_changed": count_matches(RE_DEFAULT_BOOL, module_lines) > 0,
        "type_decl_changed": count_matches(RE_TYPE_DECL, module_lines) > 0,
        "submodule_attrsof_changed": count_matches(RE_SUBMODULE, module_lines) > 0,
        # predicate / config-use diff features
        "predicate_logic_changed": predicate_logic_changed,
        "cfg_ref_edit_count": count_matches(RE_CFG_REF, module_lines),
        "config_services_ref_edit_count": count_matches(RE_CONFIG_SERVICES, module_lines),
        "literal_comparison_edit_count": count_matches(RE_COMPARE_LITERAL, module_lines),
        # test-assignment diff features
        "test_assignment_changed": test_assignment_changed,
        "specialisation_configuration_changed": specialisation_changed,
        "all_tests_wiring_changed": all_tests_wiring_changed,
        # runtime-side diff features
        "runtime_side_changed": runtime_side_changed,
        "execstart_changed": count_matches(RE_EXECSTART, module_lines) > 0,
        "environment_changed": count_matches(RE_ENVIRONMENT, module_lines) > 0,
        "systemd_service_attrs_changed": count_matches(RE_SYSTEMD_SERVICE, module_lines) > 0,
        "firewall_changed": count_matches(RE_FIREWALL, module_lines) > 0,
        "users_groups_changed": count_matches(RE_USERS_GROUPS, module_lines) > 0,
        # diff magnitude
        "files_changed_total": len(files),
        "module_diff_lines": len(module_lines),
        "test_diff_lines": len(test_lines),
        "package_version_bump_lines": package_version_bump_lines,
        "pure_package_bump_no_option_content": pure_package_bump,
    }


def main():
    import sys
    ledger_path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(
        "/home/tandem/nix-option-branch-audit/fixtures/s4-live-pr-shadow/adjudication-ledger.jsonl"
    )
    out_path = Path(__file__).resolve().parent / "features.jsonl"

    records = [json.loads(l) for l in ledger_path.read_text().splitlines() if l.strip()]
    pr_summaries = [r for r in records if r.get("record_type") == "pr_summary"]

    existing = {}
    if out_path.exists():
        for line in out_path.read_text().splitlines():
            if line.strip():
                row = json.loads(line)
                existing[row["pr"]] = row

    total = len(pr_summaries)
    with out_path.open("w") as out:
        for i, r in enumerate(pr_summaries, 1):
            pr = r["pr"]
            if pr in existing:
                out.write(json.dumps(existing[pr]) + "\n")
                print(f"[{i}/{total}] PR #{pr}: cached", flush=True)
                continue
            feats = extract_features(pr, r["base_sha"], r["head_sha"])
            feats["cohort"] = r["cohort"]
            out.write(json.dumps(feats) + "\n")
            print(f"[{i}/{total}] PR #{pr}: extracted", flush=True)


if __name__ == "__main__":
    main()
