#!/usr/bin/env python3
"""S4-F1-R stage 2: for every corpus PR, fetch the REAL nixpkgs content
its manifest's own module/test paths need, at the REAL frozen base_sha
and head_sha recorded by S4 (never a moving branch or current PR head).
Resumable: skips any file already present in the scratch cache. A
genuine 404 (the real historical fact that a file does not exist at
that exact commit -- e.g. #543492's enso-os.nix at head) is recorded,
not treated as an error: it's the whole point of the fixture.
"""
import base64
import json
import subprocess
import sys
import time
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
CORPUS_PATH = Path(__file__).resolve().parent / "corpus.json"
SCRATCH = Path("/home/tandem/s4-f1-r-scratch")
REPO = "NixOS/nixpkgs"


def gh_api_content(path: str, ref: str, attempts: int = 4) -> bytes | None:
    """Returns real file bytes, or None if the file genuinely does not
    exist at `ref` (a real 404). Any OTHER failure raises after retries.
    """
    for attempt in range(attempts):
        proc = subprocess.run(
            ["gh", "api", f"repos/{REPO}/contents/{path}?ref={ref}", "--jq", ".content"],
            capture_output=True, text=True,
        )
        if proc.returncode == 0:
            content_b64 = proc.stdout.strip()
            if not content_b64:
                # Real API oddity (directory, not a file) -- treat as a
                # hard error, not a graceful absence.
                raise RuntimeError(f"empty content for {path}@{ref}: {proc.stdout!r} {proc.stderr!r}")
            return base64.b64decode(content_b64)
        stderr = proc.stderr
        if "Not Found" in stderr or "404" in stderr:
            return None
        # Rate limit / transient network -- back off and retry.
        if attempt < attempts - 1:
            time.sleep(2 * (attempt + 1))
            continue
        raise RuntimeError(f"gh api failed for {path}@{ref} after {attempts} attempts: {stderr}")
    raise RuntimeError(f"unreachable: {path}@{ref}")


def ensure_file(path: str, ref: str, dest_root: Path, log: list[str]) -> str:
    """Returns 'present', 'absent', or 'cached'."""
    dest = dest_root / path
    marker_absent = dest_root / (path + ".ABSENT")
    if dest.exists():
        return "cached"
    if marker_absent.exists():
        return "cached-absent"
    content = gh_api_content(path, ref)
    if content is None:
        marker_absent.parent.mkdir(parents=True, exist_ok=True)
        marker_absent.write_text("")
        log.append(f"ABSENT  {path}@{ref}")
        return "absent"
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(content)
    log.append(f"fetched {path}@{ref} ({len(content)} bytes)")
    return "present"


def main():
    corpus = json.loads(CORPUS_PATH.read_text())
    total = len(corpus)
    for i, c in enumerate(corpus, 1):
        manifest_path = ROOT / c["manifest_path"]
        with manifest_path.open("rb") as f:
            doc = tomllib.load(f)
        paths = set()
        for t in doc["target"]:
            paths.add(t["module"])
            paths.add(t["test"])

        pr_dir = SCRATCH / str(c["pr"])
        base_root = pr_dir / "base-root"
        head_root = pr_dir / "head-root"
        log: list[str] = []
        for p in sorted(paths):
            ensure_file(p, c["base_sha"], base_root, log)
            ensure_file(p, c["head_sha"], head_root, log)
        if log:
            print(f"[{i}/{total}] PR #{c['pr']}: " + "; ".join(log))
        else:
            print(f"[{i}/{total}] PR #{c['pr']}: all cached", flush=True)
        sys.stdout.flush()


if __name__ == "__main__":
    main()
