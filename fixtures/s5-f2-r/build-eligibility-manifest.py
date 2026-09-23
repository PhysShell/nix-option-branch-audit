#!/usr/bin/env python3
"""S5-F2-R section 5: eligibility manifest. For every applicable PR's
every target, determine whether option_prefix_is_instance_keyed_submodule
would return true under c647d1b's own real logic -- using a throwaway,
strictly-additive diagnostic binary (env-var-gated eprintln reusing the
EXACT existing function, built from a separate clean c647d1b checkout at
/home/tandem/f2r-diag-checkout/repo, src/main.rs diff is 6 lines, purely
additive, no logic touched). This diagnostic binary is NEVER used for the
actual replay/comparison results -- only for this factual eligibility
question. Fetches head-root content fresh per PR (module content only is
strictly needed, but test content is fetched too since `check` requires
both paths to exist)."""
import base64
import json
import re
import shutil
import subprocess
import tomllib
from pathlib import Path

ROOT = Path("/home/tandem/nix-option-branch-audit")
D = ROOT / "fixtures/s5-f2-r"
DIAG_BIN = "/home/tandem/f2r-diag-checkout/target/release/oba"
SCRATCH = Path("/home/tandem/f2r-elig-scratch")
REPO = "NixOS/nixpkgs"

content_cache = {}


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
    manifest = json.loads((D / "applicable-manifest.json").read_text())
    total = len(manifest)
    results = []
    fail_count = 0
    for i, entry in enumerate(manifest, 1):
        pr = entry["pr"]
        cohort = entry["cohort"]
        head_sha = entry["head_sha"]
        orig_dir = ROOT / entry["orig_dir"]
        targets_toml_path = orig_dir / "targets.toml"
        if not targets_toml_path.exists():
            print(f"[{i}/{total}] pr={pr} MISSING targets.toml -- skip")
            continue
        parsed = tomllib.loads(targets_toml_path.read_text())
        targets = parsed.get("target", [])

        pr_scratch = SCRATCH / cohort / str(pr)
        head_root = pr_scratch / "head-root"
        if pr_scratch.exists():
            shutil.rmtree(pr_scratch)
        head_root.mkdir(parents=True)

        ok = True
        paths_needed = set()
        for t in targets:
            paths_needed.add(t["module"])
            paths_needed.add(t["test"])
        for rel_path in paths_needed:
            content = gh_content(rel_path, head_sha)
            if content is None:
                ok = False
                break
            (head_root / rel_path).parent.mkdir(parents=True, exist_ok=True)
            (head_root / rel_path).write_bytes(content)

        if not ok:
            print(f"[{i}/{total}] pr={pr} FETCH FAILED")
            fail_count += 1
            for t in targets:
                results.append({
                    "pr": pr, "cohort": cohort, "target_name": t["name"],
                    "option_prefix": t["option_prefix"], "watch": t["watch"],
                    "eligible": None, "container_form": None,
                    "method": "fetch_failed",
                })
            shutil.rmtree(pr_scratch)
            continue

        shutil.copy(targets_toml_path, pr_scratch / "targets.toml")
        env = {"OBA_DEBUG_ELIGIBILITY": "1"}
        import os
        full_env = dict(os.environ)
        full_env.update(env)
        proc = subprocess.run(
            [DIAG_BIN, "check", "--root", "head-root", "--targets", "targets.toml", "--json"],
            cwd=pr_scratch, capture_output=True, text=True, timeout=60, env=full_env,
        )
        stderr = proc.stderr

        # Parse lines like:
        # OBA_DEBUG_ELIGIBILITY target=<name> option_prefix=["a", "b"] allow_instance_key=true
        parsed_by_name = {}
        for line in stderr.splitlines():
            m = re.match(
                r'OBA_DEBUG_ELIGIBILITY target=(\S+) option_prefix=(\[.*?\]) allow_instance_key=(true|false)',
                line,
            )
            if not m:
                continue
            name, prefix_dbg, elig = m.group(1), m.group(2), m.group(3)
            parsed_by_name[name] = elig == "true"

        for t in targets:
            name = t["name"]
            eligible = parsed_by_name.get(name)
            results.append({
                "pr": pr, "cohort": cohort, "target_name": name,
                "option_prefix": t["option_prefix"], "watch": t["watch"],
                "eligible": eligible,
                "container_form": None,  # not distinguished by the diagnostic; see raw stderr if needed
                "method": "diagnostic_binary" if eligible is not None else "diagnostic_binary_no_output",
            })

        shutil.rmtree(pr_scratch)
        print(f"[{i}/{total}] pr={pr} OK ({len(targets)} targets)")

    (D / "eligibility-manifest.json").write_text(json.dumps(results, indent=2) + "\n")
    n_eligible = sum(1 for r in results if r["eligible"] is True)
    n_total = len(results)
    print(f"\nwrote eligibility-manifest.json: {n_total} target records, {n_eligible} eligible, fetch failures: {fail_count}")


if __name__ == "__main__":
    main()
