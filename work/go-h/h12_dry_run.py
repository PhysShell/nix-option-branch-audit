#!/usr/bin/env python3
"""GO-H H12 dry run: run the candidate target-validity V-check on the frozen P1 manifests.

Diagnostic only. Fetches module and test sources at each PR's base and head SHA
(read-only, gh api), then runs validate_targets.validate() per target against both
sides. Does NOT run oba. Does NOT compute any historical verdict.

Usage: h12_dry_run.py <out_dir>
"""
import base64
import json
import pathlib
import subprocess
import sys
import tomllib

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from validate_targets import validate  # noqa: E402

REPO = "NixOS/nixpkgs"
SHAS = {
    "508090": ("d4e00cc559c325edc7f8ec7947dd81ef2761c3c4", "461071d7ab456653ceba20f6f2675cee0020ff14"),
    "568429": ("6adce641b596aac2c69f7217d061019642834c4d", "e5ebd085e2692963fe3ac47624e0bfa30aa0300e"),
    "563823": ("3d35b67b0b8051a9080c09ffdd9fa4f41e845d15", "6c6973cdf55fbe86579846439ec5ddd9a1198e46"),
    "443747": ("1acfd66170c28f3271d806d82348f0bd53ff4685", "82c6719c63bf9544fd336d8febb0cca9ff638ff8"),
    "568048": ("803c63ce3bb749922969cc7edc346e48897d8029", "e97af004f41f80ea370d11f010af15d653fa6931"),
}
MANIFESTS = {
    "as-frozen": "targets/s6-r1-p1.toml",
    "corrected": "targets/s6-r1-p1-corrected.toml",
}
ROOT = pathlib.Path("/home/tandem/nix-option-branch-audit")


def pr_of(name):
    return name.split("_", 1)[0][2:]


def fetch(path, sha, dest):
    dest.parent.mkdir(parents=True, exist_ok=True)
    r = subprocess.run(
        ["gh", "api", f"repos/{REPO}/contents/{path}?ref={sha}"],
        capture_output=True, text=True,
    )
    if r.returncode != 0:
        return "absent"
    content = json.loads(r.stdout).get("content", "")
    dest.write_bytes(base64.b64decode(content))
    return "present"


def main():
    out = pathlib.Path(sys.argv[1])
    src = out / "src"
    fetched = {}
    results = []
    for label, rel in MANIFESTS.items():
        data = tomllib.loads((ROOT / rel).read_text())
        for t in data["target"]:
            pr = pr_of(t["name"])
            base_sha, head_sha = SHAS[pr]
            for side, sha in (("base", base_sha), ("head", head_sha)):
                for key in ("module", "test"):
                    p = t[key]
                    k = (side, sha, p)
                    if k not in fetched:
                        dest = src / side / sha[:10] / p
                        fetched[k] = (fetch(p, sha, dest), dest)
                root = src / side / sha[:10]
                # validate() reads module from root/module; test presence is recorded separately
                v, reasons = validate(t, str(root))
                test_status = fetched[(side, sha, t["test"])][0]
                results.append({
                    "manifest": label, "name": t["name"], "pr": pr, "side": side,
                    "verdict": v, "reasons": reasons,
                    "test_on_side": test_status,
                    "option_prefix": t["option_prefix"], "watch": t["watch"],
                })
    (out / "h12-results.json").write_text(json.dumps(results, indent=2))
    fetch_summary = {f"{s}:{sha[:10]}:{p}": st for (s, sha, p), (st, _) in fetched.items()}
    (out / "h12-fetch.json").write_text(json.dumps(fetch_summary, indent=2))
    for label in MANIFESTS:
        rows = [r for r in results if r["manifest"] == label]
        print(label, len(rows), "rows")
        for r in rows:
            print(f'  {r["name"]:<34} {r["side"]:<5} {r["verdict"]:<10} test={r["test_on_side"]:<7} {"; ".join(r["reasons"])[:110]}')


if __name__ == "__main__":
    main()
