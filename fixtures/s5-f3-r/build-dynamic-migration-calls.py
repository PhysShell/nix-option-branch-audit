#!/usr/bin/env python3
"""S5-F3-R section 8 (mandate item 12): text-level cross-check --
search raw module source (base+head, all 180 PRs, from the persisted
sources/ dir) for mkRenamedOptionModule / mkRenamedOptionModuleWith /
mkRemovedOptionModule call sites via a simple regex, and cross-reference
against the migration-edge manifest (which line numbers actually
produced a real MigrationEdge). A call site with no matching edge in
the manifest is flagged as a candidate dynamic/unresolved case -- a
MECHANICAL, textual observation only, not a definitive AST-level
judgment."""
import json
import re
from pathlib import Path

ROOT = Path("/home/tandem/nix-option-branch-audit")
D = ROOT / "fixtures/s5-f3-r"
SOURCES = D / "sources"

HELPER_RE = re.compile(r"\b(mkRenamedOptionModuleWith|mkRenamedOptionModule|mkRemovedOptionModule)\b")


def main():
    manifest = json.loads((D / "applicable-manifest.json").read_text())
    edges = json.loads((D / "migration-edge-manifest.json").read_text())
    edge_lines = {(e["pr"], e["side"], e["source_file"], int(e["span"]["line"])) for e in edges if e["span"]["line"] is not None}

    candidates = []
    total_call_sites = 0
    for e in manifest:
        pr = e["pr"]
        cohort = e["cohort"]
        for side in ("base", "head"):
            side_root = SOURCES / cohort / str(pr) / side
            if not side_root.exists():
                continue
            for nix_file in side_root.rglob("*.nix"):
                rel = str(nix_file.relative_to(side_root))
                text = nix_file.read_text(errors="replace")
                for i, line in enumerate(text.splitlines(), start=1):
                    if HELPER_RE.search(line):
                        total_call_sites += 1
                        # a real call may span multiple lines (args on
                        # following lines) -- the manifest's own span is
                        # the APPLY node's own start line, which for a
                        # multi-line curried call is the line the helper
                        # NAME itself appears on, matching this scan's
                        # own line here.
                        key = (pr, side, rel, i)
                        if key not in edge_lines:
                            candidates.append({
                                "pr": pr, "side": side, "file": rel, "line": i,
                                "raw_text": line.strip(),
                            })

    (D / "dynamic-migration-calls.json").write_text(json.dumps(candidates, indent=2) + "\n")
    print(f"total helper call sites found (text scan): {total_call_sites}")
    print(f"call sites with NO corresponding manifest edge (candidate dynamic/unresolved): {len(candidates)}")
    for c in candidates:
        print(f"  PR {c['pr']} {c['side']} {c['file']}:{c['line']}: {c['raw_text']}")


if __name__ == "__main__":
    main()
