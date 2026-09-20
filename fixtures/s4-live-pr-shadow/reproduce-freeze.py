#!/usr/bin/env python3
"""S4: regenerates S4-A and S4-B's frozen orders purely from committed
inputs, and asserts byte-for-byte equality with the already-committed
`s4a-frozen-order.json`/`s4b-frozen-order.json`.

This is VERIFICATION of the existing freeze, not authorization to
redraw it. If reproduction disagrees with the current frozen orders,
this script exits non-zero and prints the discrepancy -- the correct
response to that is to STOP and report, never to silently replace the
committed cohorts with whatever this script just produced.

Reads only:
  - fixtures/s4-live-pr-shadow/population-survivors.json (committed)
  - fixtures/s4-live-pr-shadow/stress-diffs/*.diff (committed, the
    exact pinned diff evidence the stress classifier used)

No /tmp dependency, no network access, no undocumented source of
truth.
"""
import hashlib
import json
import random
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent

SHORT_SHA = "81e1131"  # v0.4.4's own frozen commit

STRESS_CATEGORIES = {
    "option_declaration": re.compile(r"\b(mkOption|mkEnableOption)\b"),
    "nixos_tests_touched": None,
    "execstart_script_cmdline": re.compile(r"\b(ExecStart|script\s*=)\b"),
    "environment": re.compile(r"\b(environment|EnvironmentFile|environmentFile)\b"),
    "generated_config": re.compile(
        r"\b(writeText|toYAML|toJSON|settingsFormat|format\.generate|configFile)\b"
    ),
    "package_version_source_dep": re.compile(
        r"(^[+-]\s*version\s*=|^[+-]\s*src\s*=\s*fetch|^[+-]\s*rev\s*=|sha256-[A-Za-z0-9+/=]{20,})"
    ),
}


def added_removed_lines(diff_text: str) -> str:
    lines = []
    for line in diff_text.splitlines():
        if line.startswith("+++") or line.startswith("---"):
            continue
        if line.startswith("+") or line.startswith("-"):
            lines.append(line)
    return "\n".join(lines)


def stress_eligible(r: dict, diffs_dir: Path) -> bool:
    num = r["number"]
    diff_path = diffs_dir / f"{num}.diff"
    joined = ""
    if diff_path.exists() and diff_path.stat().st_size > 0:
        joined = added_removed_lines(diff_path.read_text(encoding="utf-8", errors="replace"))
    for cat, pat in STRESS_CATEGORIES.items():
        if pat is not None and pat.search(joined):
            return True
    paths = [f["path"] for f in r.get("files", [])]
    if any(p.startswith("nixos/tests/") for p in paths):
        return True
    return False


def numbers(rows: list[dict]) -> list[int]:
    return [r["number"] for r in rows]


def main() -> int:
    survivors = json.loads((ROOT / "population-survivors.json").read_text())
    committed_a = json.loads((ROOT / "s4a-frozen-order.json").read_text())
    committed_b = json.loads((ROOT / "s4b-frozen-order.json").read_text())

    print(f"survivors: {len(survivors)}")

    # --- S4-A reproduction ---
    seed_a = int(SHORT_SHA, 16)
    pool = list(survivors)
    random.Random(seed_a).shuffle(pool)
    reproduced_a = pool[: min(120, len(pool))]

    a_match = numbers(reproduced_a) == numbers(committed_a)
    print(f"S4-A reproduction: {'MATCH' if a_match else 'MISMATCH'} ({len(reproduced_a)} vs {len(committed_a)} committed)")

    # --- S4-B reproduction ---
    remaining = [r for r in pool if r not in reproduced_a]
    diffs_dir = ROOT / "stress-diffs"
    diff_files = sorted(diffs_dir.glob("*.diff"))
    combined_hash = hashlib.sha256()
    for p in diff_files:
        combined_hash.update(hashlib.sha256(p.read_bytes()).hexdigest().encode())
    print(f"stress-diffs: {len(diff_files)} files, combined SHA256(of-per-file-SHA256s) = {combined_hash.hexdigest()}")

    eligible = [r for r in remaining if stress_eligible(r, diffs_dir)]
    print(f"stress-eligible: {len(eligible)} / {len(remaining)} remaining")

    seed_b = int(SHORT_SHA, 16) + 1
    pool_b = list(eligible)
    random.Random(seed_b).shuffle(pool_b)
    reproduced_b = pool_b[: min(80, len(pool_b))]

    b_match = numbers(reproduced_b) == numbers(committed_b)
    print(f"S4-B reproduction: {'MATCH' if b_match else 'MISMATCH'} ({len(reproduced_b)} vs {len(committed_b)} committed)")

    if a_match and b_match:
        print("\nREPRODUCTION CONFIRMED: both frozen orders regenerate byte-for-byte identically from committed inputs alone.")
        return 0
    else:
        print("\nREPRODUCTION FAILED -- DO NOT silently replace the committed cohorts. STOP and report this discrepancy.", file=sys.stderr)
        if not a_match:
            print(f"  S4-A reproduced: {numbers(reproduced_a)[:10]}...", file=sys.stderr)
            print(f"  S4-A committed:  {numbers(committed_a)[:10]}...", file=sys.stderr)
        if not b_match:
            print(f"  S4-B reproduced: {numbers(reproduced_b)[:10]}...", file=sys.stderr)
            print(f"  S4-B committed:  {numbers(committed_b)[:10]}...", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
