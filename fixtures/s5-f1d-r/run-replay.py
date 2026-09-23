#!/usr/bin/env python3
"""S5-F1D-R replay executor. For each applicable PR, fetch real base/head
content at the frozen SHAs, run the candidate (25c5b54) binary against the
original committed targets.toml, save outputs. No comparison/judgment
here -- see compare-replay.py. Adapted verbatim from fixtures/s5-f1-r/run-replay.py
(same replay mechanics), only the candidate binary path and output
directory differ.
"""
import base64
import json
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path("/home/tandem/nix-option-branch-audit")
D = ROOT / "fixtures/s5-f1d-r"
CANDIDATE = "/home/tandem/f1dr-clean-checkout/target/release/oba"
SCRATCH = Path("/home/tandem/f1dr-scratch")
REPO = "NixOS/nixpkgs"

content_cache = {}
fail_count = 0
incomplete = []


def gh_content(path, sha):
    key = (path, sha)
    if key in content_cache:
        return content_cache[key]
    for attempt in range(3):
        try:
            out = subprocess.run(
                ["gh", "api", f"repos/{REPO}/contents/{path}?ref={sha}"],
                capture_output=True, text=True, timeout=30,
            )
            if out.returncode != 0:
                if attempt < 2:
                    continue
                content_cache[key] = None
                return None
            data = json.loads(out.stdout)
            content = base64.b64decode(data["content"])
            content_cache[key] = content
            return content
        except Exception:
            if attempt == 2:
                content_cache[key] = None
                return None
    return None


def main():
    global fail_count
    manifest = json.loads((D / "applicable-manifest.json").read_text())
    total = len(manifest)
    for i, entry in enumerate(manifest, 1):
        pr = entry["pr"]
        cohort = entry["cohort"]
        base_sha = entry["base_sha"]
        head_sha = entry["head_sha"]
        orig_dir = ROOT / entry["orig_dir"]
        out_dir = D / "replay" / cohort / str(pr)

        if (out_dir / "raw.json").exists():
            print(f"[{i}/{total}] pr={pr} SKIP (already done)")
            continue

        targets_toml_path = orig_dir / "targets.toml"
        if not targets_toml_path.exists():
            print(f"[{i}/{total}] pr={pr} MISSING targets.toml")
            out_dir.mkdir(parents=True, exist_ok=True)
            (out_dir / "INCOMPLETE.txt").write_text("missing original targets.toml\n")
            incomplete.append(pr)
            fail_count += 1
            continue

        manifest_text = targets_toml_path.read_text()
        parsed = tomllib.loads(manifest_text)
        targets = parsed.get("target", [])

        pr_scratch = SCRATCH / cohort / str(pr)
        base_root = pr_scratch / "base-root"
        head_root = pr_scratch / "head-root"
        if pr_scratch.exists():
            shutil.rmtree(pr_scratch)
        base_root.mkdir(parents=True)
        head_root.mkdir(parents=True)

        ok = True
        paths_needed = set()
        for t in targets:
            paths_needed.add(t["module"])
            paths_needed.add(t["test"])

        for rel_path in paths_needed:
            base_content = gh_content(rel_path, base_sha)
            head_content = gh_content(rel_path, head_sha)
            if base_content is None or head_content is None:
                ok = False
                break
            (base_root / rel_path).parent.mkdir(parents=True, exist_ok=True)
            (base_root / rel_path).write_bytes(base_content)
            (head_root / rel_path).parent.mkdir(parents=True, exist_ok=True)
            (head_root / rel_path).write_bytes(head_content)

        if not ok:
            print(f"[{i}/{total}] pr={pr} FETCH FAILED")
            out_dir.mkdir(parents=True, exist_ok=True)
            (out_dir / "INCOMPLETE.txt").write_text(f"fetch failure for one or more of {sorted(paths_needed)}\n")
            incomplete.append(pr)
            fail_count += 1
            if fail_count > 5:
                print("TOO MANY FAILURES -- stopping per fail-closed requirement")
                break
            continue

        shutil.copy(targets_toml_path, pr_scratch / "targets.toml")

        cmds = [
            [CANDIDATE, "audit-diff", "--base-root", "base-root", "--head-root", "head-root",
             "--targets", "targets.toml", "--format", "json", "--summary-path", "summary.md"],
            [CANDIDATE, "check", "--root", "base-root", "--targets", "targets.toml", "--json"],
            [CANDIDATE, "check", "--root", "head-root", "--targets", "targets.toml", "--json"],
        ]
        outfiles = ["raw.json", "check-base.json", "check-head.json"]
        out_dir.mkdir(parents=True, exist_ok=True)
        cmd_log = []
        for cmd, outfile in zip(cmds, outfiles):
            result = subprocess.run(cmd, cwd=pr_scratch, capture_output=True, text=True, timeout=60)
            (pr_scratch / outfile).write_text(result.stdout)
            cmd_log.append(f"cd {pr_scratch}\n{' '.join(cmd)} > {outfile}  (exit {result.returncode})")

        for outfile in outfiles:
            shutil.copy(pr_scratch / outfile, out_dir / outfile)
        if (pr_scratch / "summary.md").exists():
            shutil.copy(pr_scratch / "summary.md", out_dir / "summary.md")
        (out_dir / "command.txt").write_text("\n".join(cmd_log) + "\n")

        shutil.rmtree(pr_scratch)
        print(f"[{i}/{total}] pr={pr} OK")

    print(f"\ndone. failures: {fail_count}, incomplete PRs: {incomplete}")
    if fail_count > 5:
        sys.exit(1)


if __name__ == "__main__":
    main()
