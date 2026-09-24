#!/usr/bin/env python3
"""S5-F3-R section 5: migration-edge manifest for the whole frozen
corpus. Runs the throwaway diagnostic binary (env-gated eprintln! right
after run_target's own `let migrations = scan_migrations(...)` call,
reusing the REAL function verbatim -- never used for the actual
replay/comparison results) against each PR's already-fetched module
source (persisted by run-replay.py under sources/), for both base and
head sides, and parses OBA_DEBUG_MIGRATION lines from stderr.

For each edge, additionally computes (mechanically): does it match some
target's own watched full path (option_prefix ++ watch[i])? How many
edges (any kind) share this from_path within the same (pr, side) scan?
"""
import json
import subprocess
import tomllib
from collections import defaultdict
from pathlib import Path

ROOT = Path("/home/tandem/nix-option-branch-audit")
D = ROOT / "fixtures/s5-f3-r"
DIAG = "/home/tandem/f3r-diag-checkout/target/release/oba"
SOURCES = D / "sources"


def parse_debug_lines(stderr_text):
    edges = []
    for line in stderr_text.splitlines():
        if not line.startswith("OBA_DEBUG_MIGRATION "):
            continue
        # key=value pairs; from_path/to_path are JSON, may contain spaces? no -- json.dumps default has no spaces after separators by default in Rust serde_json::to_string (compact, no spaces)
        rest = line[len("OBA_DEBUG_MIGRATION "):]
        fields = {}
        # split on whitespace but json values have no spaces (serde_json::to_string is compact)
        for tok in rest.split(" "):
            if "=" not in tok:
                continue
            k, v = tok.split("=", 1)
            fields[k] = v
        edges.append(fields)
    return edges


def main():
    manifest = json.loads((D / "applicable-manifest.json").read_text())
    all_edges = []
    errors = []

    for e in manifest:
        pr = e["pr"]
        cohort = e["cohort"]
        pr_src = SOURCES / cohort / str(pr)
        targets_toml_path = pr_src / "targets.toml"
        if not targets_toml_path.exists():
            errors.append({"pr": pr, "error": "missing sources/targets.toml"})
            continue
        parsed = tomllib.loads(targets_toml_path.read_text())
        targets = parsed.get("target", [])
        watched_full_paths = set()
        for t in targets:
            prefix = tuple(t["option_prefix"])
            for w in t.get("watch", []):
                watched_full_paths.add(prefix + tuple(w.split(".")))

        for side in ("base", "head"):
            side_root = pr_src / side
            if not side_root.exists():
                errors.append({"pr": pr, "side": side, "error": "missing source root"})
                continue
            env = dict(__import__("os").environ)
            env["OBA_DEBUG_PR"] = str(pr)
            env["OBA_DEBUG_SIDE"] = side
            result = subprocess.run(
                [DIAG, "check", "--root", str(side_root), "--targets", str(targets_toml_path), "--json"],
                capture_output=True, text=True, timeout=30, env=env,
            )
            raw_edges = parse_debug_lines(result.stderr)
            for re_ in raw_edges:
                from_path = json.loads(re_.get("from_path", "[]"))
                to_path_raw = re_.get("to_path", "null")
                to_path = json.loads(to_path_raw) if to_path_raw != "null" else None
                all_edges.append({
                    "pr": pr,
                    "side": side,
                    "target": re_.get("target"),
                    "kind": re_.get("kind"),
                    "from_path": from_path,
                    "to_path": to_path,
                    "source_file": re_.get("source_file"),
                    "span": {"line": re_.get("line"), "col": re_.get("col")},
                    "helper_form": re_.get("helper_form"),
                    "matches_watched_path": tuple(from_path) in watched_full_paths,
                })

    # ambiguity count: group by (pr, side, tuple(from_path))
    groups = defaultdict(list)
    for edge in all_edges:
        key = (edge["pr"], edge["side"], tuple(edge["from_path"]))
        groups[key].append(edge)
    for edge in all_edges:
        key = (edge["pr"], edge["side"], tuple(edge["from_path"]))
        edge["ambiguity_count_for_from_path"] = len(groups[key])

    (D / "migration-edge-manifest.json").write_text(json.dumps(all_edges, indent=2) + "\n")
    if errors:
        (D / "migration-edge-manifest-errors.json").write_text(json.dumps(errors, indent=2) + "\n")

    by_helper = defaultdict(int)
    by_kind = defaultdict(int)
    for edge in all_edges:
        by_helper[edge["helper_form"]] += 1
        by_kind[edge["kind"]] += 1
    matches_watched = sum(1 for e in all_edges if e["matches_watched_path"])
    ambiguous = sum(1 for e in all_edges if e["ambiguity_count_for_from_path"] > 1)

    print(f"total edges: {len(all_edges)}")
    print(f"by helper_form: {dict(by_helper)}")
    print(f"by kind: {dict(by_kind)}")
    print(f"matches_watched_path=true: {matches_watched}")
    print(f"ambiguity_count>1: {ambiguous}")
    print(f"errors: {len(errors)}")


if __name__ == "__main__":
    main()
