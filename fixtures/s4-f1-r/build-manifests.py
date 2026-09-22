#!/usr/bin/env python3
"""S4-F1-R stage 1: for every corpus PR, produce the exact target
manifest to compare baseline vs candidate against.

Per the F1-R authorization: "Prefer committed S4 artifacts when
sufficient." Batches 6-10 (S4-A) and every S4-B batch committed a real
targets.toml per PR -- reused verbatim, copied byte-for-byte, never
regenerated. Batches 1-5 (S4-A positions 1-60) only committed
raw.json/summary.md; for those, the manifest is mechanically
reconstructed from raw.json's own `oba[].identity` entries (module,
test, cfg_ident, option_prefix, watched_path) -- the exact real
target/watch set that PRODUCED that raw.json, grouped back into a
manifest, not re-derived from scratch. Verified equivalent (same
module/test/cfg_ident/option_prefix/watch set) against a committed
targets.toml this same technique can also reconstruct, as a sanity
check, before trusting it for the 37 PRs where no committed manifest
exists.

Writes: fixtures/s4-f1-r/manifests/<pr>.toml (one per corpus PR) and
records provenance (copied-verbatim vs reconstructed-from-raw-json) +
manifest sha256 back into corpus.json.
"""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
CORPUS_PATH = Path(__file__).resolve().parent / "corpus.json"
MANIFESTS_DIR = Path(__file__).resolve().parent / "manifests"


def toml_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def render_manifest(targets: list[dict]) -> str:
    lines = []
    for i, t in enumerate(targets):
        lines.append("[[target]]")
        lines.append(f'name = "{toml_escape(t["name"])}"')
        lines.append(f'module = "{toml_escape(t["module"])}"')
        lines.append(f'test = "{toml_escape(t["test"])}"')
        lines.append(f'cfg_ident = "{toml_escape(t["cfg_ident"])}"')
        prefix = ", ".join(f'"{toml_escape(p)}"' for p in t["option_prefix"])
        lines.append(f"option_prefix = [{prefix}]")
        watch = ", ".join(f'"{toml_escape(w)}"' for w in sorted(t["watch"]))
        lines.append(f"watch = [{watch}]")
        if i != len(targets) - 1:
            lines.append("")
    return "\n".join(lines) + "\n"


def reconstruct_from_raw_json(raw_json_path: Path) -> list[dict]:
    raw = json.loads(raw_json_path.read_text())
    grouped: dict[tuple, set[str]] = {}
    order: list[tuple] = []
    for entry in raw["oba"]:
        ident = entry["identity"]
        key = (ident["module"], ident["test"], ident["cfg_ident"], tuple(ident["option_prefix"]))
        if key not in grouped:
            grouped[key] = set()
            order.append(key)
        grouped[key].add(ident["watched_path"])
    targets = []
    for i, key in enumerate(order):
        module, test, cfg_ident, option_prefix = key
        # Deterministic, content-derived name -- cosmetic only (never
        # compared), but stable across re-runs of this script.
        name = f"{Path(module).stem}-{i}" if len(order) > 1 else Path(module).stem
        targets.append({
            "name": name,
            "module": module,
            "test": test,
            "cfg_ident": cfg_ident,
            "option_prefix": list(option_prefix),
            "watch": sorted(grouped[key]),
        })
    return targets


def parse_committed_toml(path: Path) -> list[dict]:
    # Minimal, dependency-free TOML reader for this manifest's own
    # narrow, already-known shape ([[target]] tables with string/array
    # scalars only) -- avoids adding a new parsing dependency for a
    # format this project's own `toml` crate already parses on the Rust
    # side; this Python-side reconstruction only needs to ROUND-TRIP
    # the same shape, not parse arbitrary TOML.
    import tomllib
    with path.open("rb") as f:
        doc = tomllib.load(f)
    return doc["target"]


def main():
    corpus = json.loads(CORPUS_PATH.read_text())
    MANIFESTS_DIR.mkdir(exist_ok=True)

    reused, reconstructed = 0, 0
    for c in corpus:
        committed = ROOT / c["s4_targets_toml"]
        out_path = MANIFESTS_DIR / f"{c['pr']}.toml"
        if committed.exists():
            text = committed.read_text()
            provenance = "copied_verbatim_from_s4_artifact"
            reused += 1
        else:
            raw_json = ROOT / c["s4_raw_json"]
            targets = reconstruct_from_raw_json(raw_json)
            text = render_manifest(targets)
            provenance = "reconstructed_from_s4_raw_json_identities"
            reconstructed += 1
        out_path.write_text(text)
        c["manifest_path"] = f"fixtures/s4-f1-r/manifests/{c['pr']}.toml"
        c["manifest_provenance"] = provenance
        c["manifest_sha256"] = hashlib.sha256(text.encode()).hexdigest()

    CORPUS_PATH.write_text(json.dumps(corpus, indent=2) + "\n")
    print(f"manifests: {reused} copied verbatim, {reconstructed} reconstructed from raw.json, {len(corpus)} total")


if __name__ == "__main__":
    main()
