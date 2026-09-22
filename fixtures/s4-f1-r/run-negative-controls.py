#!/usr/bin/env python3
"""S4-F1-R: fatal negative controls, run against BOTH binaries. Uses
the committed S4-F1 hostile fixtures (targets/s4f1-*.toml,
fixtures/synthetic/root-escape/*) from the repo working tree at HEAD
(940e388) -- these did not exist at v0.4.4, so the baseline binary is
run against the SAME real fixture content, proving each condition was
ALREADY fatal before S4-F1 and REMAINS fatal after it. Not part of the
94-PR corpus regression (which only exercises "genuine absence");
these are the security/manifest-fatal boundary this fix must never
have widened.
"""
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
BASELINE_BIN = Path("/tmp/verify-v044-scratch/oba-x86_64-unknown-linux-musl/oba")
CANDIDATE_BIN = Path("/tmp/s4f1r-candidate-target/release/oba")


def run_oba(binary, args, cwd):
    proc = subprocess.run([str(binary)] + args, capture_output=True, cwd=str(cwd))
    return proc.returncode, proc.stdout, proc.stderr


def sha256(b):
    return hashlib.sha256(b).hexdigest()


CASES = [
    {
        "name": "absolute_module_path",
        "args": ["check", "--root", "fixtures/synthetic", "--targets", "fixtures/synthetic/root-escape/t-absolute.toml"],
        "expect_exit": 3,
        "expect_stderr_contains": b"must be relative to --root",
    },
    {
        "name": "dotdot_escape",
        "args": ["check", "--root", "fixtures/synthetic", "--targets", "fixtures/synthetic/root-escape/t-dotdot.toml"],
        "expect_exit": 3,
        "expect_stderr_contains": b"escapes --root",
    },
    {
        "name": "symlink_escape",
        "args": ["check", "--root", "fixtures/synthetic/root-escape/root", "--targets", "fixtures/synthetic/root-escape/t-symlink.toml"],
        "expect_exit": 3,
        "expect_stderr_contains": b"escapes --root",
    },
    {
        "name": "missing_root",
        "args": ["check", "--root", "this-root-does-not-exist-anywhere", "--targets", "targets/golden.toml"],
        "expect_exit": 3,
        "expect_stderr_contains": None,
    },
    {
        "name": "s4f1_negative_control_absolute_path_alongside_deleted_target",
        "args": ["check", "--root", ".", "--targets", "targets/s4f1-negative-control-escape.toml"],
        "expect_exit": 3,
        "expect_stderr_contains": b"must be relative to --root",
    },
    {
        "name": "s4f1_malformed_manifest",
        "args": ["check", "--root", ".", "--targets", "targets/s4f1-malformed-manifest.toml"],
        "expect_exit": 3,
        "expect_stderr_contains": b"parsing targets manifest",
    },
    {
        "name": "s4f1_all_targets_unavailable_multi_target",
        "args": ["check", "--root", ".", "--targets", "targets/s4f1-all-deleted.toml"],
        "expect_exit": 3,
        "expect_stderr_contains": None,
    },
]


def main():
    results = []
    for case in CASES:
        b_exit, b_out, b_err = run_oba(BASELINE_BIN, case["args"], ROOT)
        c_exit, c_out, c_err = run_oba(CANDIDATE_BIN, case["args"], ROOT)
        # The literal acceptance criterion (#8) is "previously-fatal
        # non-absence conditions remain fatal" -- exit code and empty
        # stdout, not byte-identical stderr TEXT. A manifest combining a
        # soft-missing target with a hard-error target can legitimately
        # have baseline and candidate name a DIFFERENT target as the
        # cause (old code aborts on whichever problem it scans first,
        # regardless of kind; new code scans past soft-missing targets,
        # so a later hard error is no longer masked by an earlier
        # incidental one) -- recorded separately as `stderr_differs`,
        # informational, never a pass/fail criterion on its own.
        fatal_status_ok = b_exit == case["expect_exit"] == c_exit and b_out == b"" == c_out
        stderr_contains_ok = True
        if case["expect_stderr_contains"]:
            stderr_contains_ok = (
                case["expect_stderr_contains"] in c_err
            )
        ok = fatal_status_ok and stderr_contains_ok
        results.append({
            "name": case["name"],
            "args": case["args"],
            "baseline_exit": b_exit,
            "candidate_exit": c_exit,
            "baseline_stdout_sha256": sha256(b_out),
            "candidate_stdout_sha256": sha256(c_out),
            "baseline_stderr_sha256": sha256(b_err),
            "candidate_stderr_sha256": sha256(c_err),
            "stderr_differs": b_err != c_err,
            "baseline_stderr": b_err.decode(errors="replace"),
            "candidate_stderr": c_err.decode(errors="replace"),
            "both_fatal_and_matching": ok,
        })
        note = " (stderr TEXT differs -- disclosed, non-blocking, see notes)" if b_err != c_err else ""
        print(f"{'OK ' if ok else 'FAIL'} {case['name']}: baseline exit={b_exit} candidate exit={c_exit}{note}")

    out_path = Path(__file__).resolve().parent / "negative-controls.json"
    out_path.write_text(json.dumps(results, indent=2) + "\n")
    failures = [r for r in results if not r["both_fatal_and_matching"]]
    print(f"\n{len(results)} negative controls, {len(failures)} failures")
    if failures:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
