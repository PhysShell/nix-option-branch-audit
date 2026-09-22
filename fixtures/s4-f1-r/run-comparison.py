#!/usr/bin/env python3
"""S4-F1-R stage 3: the actual regression comparison. For every corpus
PR, runs BOTH the frozen v0.4.4 baseline and the candidate (940e388)
binary against the SAME real, frozen content and the SAME manifest, for:
  - `check --root base-root`
  - `check --root head-root`
  - `audit-diff --base-root base-root --head-root head-root` (negative
    control: audit-diff's own code was never touched by S4-F1)

Comparison contract (frozen BEFORE examining any corpus differences,
per the F1-R authorization): for `check` JSON, the candidate's own
`summary.unavailable`/`unavailable_targets` fields are stripped ONLY
when they are the default (0 / []) before comparing against baseline --
no other field is ever normalized away. `audit-diff` output is compared
with NO normalization (nothing additive was ever added there).

Writes fixtures/s4-f1-r/ledger.jsonl -- one JSON object per invocation
comparison, machine-generated, the sole source every count/table in the
final report is computed from.
"""
import copy
import hashlib
import json
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
CORPUS_PATH = Path(__file__).resolve().parent / "corpus.json"
LEDGER_PATH = Path(__file__).resolve().parent / "ledger.jsonl"
SCRATCH = Path("/home/tandem/s4-f1-r-scratch")

BASELINE_BIN = Path("/tmp/verify-v044-scratch/oba-x86_64-unknown-linux-musl/oba")
BASELINE_SHA256 = "388ea5ab8147c1bc5d71948ff63afe0c8ee3fec3ac4abb9fbc53ca14293d9225"
CANDIDATE_BIN = Path("/tmp/s4f1r-candidate-target/release/oba")


def sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def sha256_file(p: Path) -> str:
    return sha256_bytes(p.read_bytes())


def run_oba(binary: Path, args: list[str]) -> tuple[int, bytes, bytes]:
    proc = subprocess.run([str(binary)] + args, capture_output=True)
    return proc.returncode, proc.stdout, proc.stderr


def normalize_check_json(obj: dict) -> dict:
    """Strip S4-F1's two additive fields ONLY when they carry the
    default 'nothing unavailable' value -- the one, narrowly-defined
    normalization this comparison is allowed to make. Any other
    difference stays visible.
    """
    obj = copy.deepcopy(obj)
    summary = obj.get("summary", {})
    if summary.get("unavailable") == 0 and obj.get("unavailable_targets") == []:
        summary.pop("unavailable", None)
        obj.pop("unavailable_targets", None)
    return obj


def try_parse_json(b: bytes):
    try:
        return json.loads(b)
    except (json.JSONDecodeError, UnicodeDecodeError):
        return None


def classify_check(baseline_exit, baseline_out, baseline_err, candidate_exit, candidate_out, candidate_err):
    """Returns (classification, notes)."""
    baseline_json = try_parse_json(baseline_out) if baseline_out else None
    candidate_json = try_parse_json(candidate_out) if candidate_out else None

    if baseline_json is not None and candidate_json is not None:
        b_norm = normalize_check_json(baseline_json)
        c_norm = normalize_check_json(candidate_json)
        if b_norm == c_norm and baseline_exit == candidate_exit and baseline_err == candidate_err:
            return "identical_after_additive_normalization", "byte-identical after stripping only the two additive default-valued fields"

    # The one pre-registered expected delta: baseline TOOL_ERROR (exit 3,
    # empty stdout) on a target genuinely absent under --root; candidate
    # exit 2, with the SURVIVING targets' own verdicts/evidence otherwise
    # untouched and the absent ones now named under unavailable_targets.
    if (
        baseline_exit == 3
        and baseline_out == b""
        and (b"module:" in baseline_err or b"test:" in baseline_err)
        and candidate_exit == 2
        and candidate_json is not None
    ):
        unavailable = candidate_json.get("unavailable_targets", [])
        if unavailable and candidate_json.get("summary", {}).get("unavailable") == len(unavailable):
            return "expected_missing_target_recovery", (
                f"baseline TOOL_ERROR(3, empty stdout) on a genuinely-absent target; "
                f"candidate exit 2 with {len(candidate_json.get('targets', []))} target(s) analyzed "
                f"and {len(unavailable)} correctly reported unavailable: "
                f"{[u['name'] for u in unavailable]}"
            )

    return "unexpected_difference", (
        f"baseline exit={baseline_exit} candidate exit={candidate_exit}; "
        f"baseline stdout sha256={sha256_bytes(baseline_out)[:16]} candidate stdout sha256={sha256_bytes(candidate_out)[:16]}; "
        f"baseline stderr sha256={sha256_bytes(baseline_err)[:16]} candidate stderr sha256={sha256_bytes(candidate_err)[:16]}"
    )


def classify_audit_diff(baseline_exit, baseline_out, baseline_err, candidate_exit, candidate_out, candidate_err):
    if baseline_exit == candidate_exit and baseline_out == candidate_out and baseline_err == candidate_err:
        return "identical_after_additive_normalization", "byte-identical, no normalization needed (audit-diff's own schema was never touched by S4-F1)"
    return "unexpected_difference", (
        f"baseline exit={baseline_exit} candidate exit={candidate_exit}; "
        f"baseline stdout sha256={sha256_bytes(baseline_out)[:16]} candidate stdout sha256={sha256_bytes(candidate_out)[:16]}"
    )


def main():
    assert BASELINE_BIN.exists(), f"baseline binary missing: {BASELINE_BIN}"
    assert CANDIDATE_BIN.exists(), f"candidate binary missing: {CANDIDATE_BIN}"
    baseline_actual_sha = sha256_file(BASELINE_BIN)
    assert baseline_actual_sha == BASELINE_SHA256, (
        f"baseline binary sha256 mismatch: expected {BASELINE_SHA256}, got {baseline_actual_sha}"
    )
    candidate_sha256 = sha256_file(CANDIDATE_BIN)
    print(f"baseline sha256: {baseline_actual_sha}")
    print(f"candidate sha256: {candidate_sha256}")

    corpus = json.loads(CORPUS_PATH.read_text())
    unexpected = []
    rows_written = 0

    with LEDGER_PATH.open("w") as ledger:
        for i, c in enumerate(corpus, 1):
            pr = c["pr"]
            pr_dir = SCRATCH / str(pr)
            base_root = pr_dir / "base-root"
            head_root = pr_dir / "head-root"
            manifest_path = (ROOT / c["manifest_path"]).resolve()

            for side, root in (("base", base_root), ("head", head_root)):
                b_exit, b_out, b_err = run_oba(BASELINE_BIN, ["check", "--root", str(root), "--targets", str(manifest_path), "--json"])
                c_exit, c_out, c_err = run_oba(CANDIDATE_BIN, ["check", "--root", str(root), "--targets", str(manifest_path), "--json"])
                classification, notes = classify_check(b_exit, b_out, b_err, c_exit, c_out, c_err)
                if classification == "unexpected_difference":
                    unexpected.append((pr, side, "check", notes))
                row = {
                    "pr": pr,
                    "cohort": c["cohort"],
                    "kind": "check",
                    "side": side,
                    "frozen_sha": c["base_sha"] if side == "base" else c["head_sha"],
                    "manifest_sha256": c["manifest_sha256"],
                    "manifest_provenance": c["manifest_provenance"],
                    "baseline_binary_sha256": baseline_actual_sha,
                    "candidate_binary_sha256": candidate_sha256,
                    "baseline_exit_code": b_exit,
                    "candidate_exit_code": c_exit,
                    "baseline_stdout_sha256": sha256_bytes(b_out),
                    "baseline_stderr_sha256": sha256_bytes(b_err),
                    "candidate_stdout_sha256": sha256_bytes(c_out),
                    "candidate_stderr_sha256": sha256_bytes(c_err),
                    "delta_classification": classification,
                    "notes": notes,
                }
                ledger.write(json.dumps(row) + "\n")
                rows_written += 1

            b_exit, b_out, b_err = run_oba(BASELINE_BIN, ["audit-diff", "--base-root", str(base_root), "--head-root", str(head_root), "--targets", str(manifest_path), "--format", "json"])
            c_exit, c_out, c_err = run_oba(CANDIDATE_BIN, ["audit-diff", "--base-root", str(base_root), "--head-root", str(head_root), "--targets", str(manifest_path), "--format", "json"])
            classification, notes = classify_audit_diff(b_exit, b_out, b_err, c_exit, c_out, c_err)
            if classification == "unexpected_difference":
                unexpected.append((pr, "both", "audit-diff", notes))
            row = {
                "pr": pr,
                "cohort": c["cohort"],
                "kind": "audit-diff",
                "side": "both",
                "frozen_sha": f"{c['base_sha']}..{c['head_sha']}",
                "manifest_sha256": c["manifest_sha256"],
                "manifest_provenance": c["manifest_provenance"],
                "baseline_binary_sha256": baseline_actual_sha,
                "candidate_binary_sha256": candidate_sha256,
                "baseline_exit_code": b_exit,
                "candidate_exit_code": c_exit,
                "baseline_stdout_sha256": sha256_bytes(b_out),
                "baseline_stderr_sha256": sha256_bytes(b_err),
                "candidate_stdout_sha256": sha256_bytes(c_out),
                "candidate_stderr_sha256": sha256_bytes(c_err),
                "delta_classification": classification,
                "notes": notes,
            }
            ledger.write(json.dumps(row) + "\n")
            rows_written += 1

            print(f"[{i}/{len(corpus)}] PR #{pr}: done ({rows_written} ledger rows so far)", flush=True)

    print(f"\nTotal ledger rows: {rows_written}")
    print(f"Unexpected differences: {len(unexpected)}")
    for u in unexpected:
        print(f"  UNEXPECTED: PR #{u[0]} side={u[1]} kind={u[2]}: {u[3]}")
    if unexpected:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
