#!/usr/bin/env python3
"""GO-H target-validity V-check: deterministic, text-only, runs BEFORE any oba call.

Usage: validate_targets.py <manifest.toml> <source_root>

Reads each [[target]] and checks contract invariants against the module source
under <source_root> (the head or base checkout). Never consults oba output.
Writes one JSON object per target to stdout. Verdicts: VALID / INVALID / AMBIGUOUS.

Rules (see work/go-h/target-validity-rubric.json for the human-readable spec):
  V1 module file exists under source_root                       -> else INVALID
  V2 option_prefix is a walkable root: its LAST segment is a plain attrset
     container, not an option declared with mk*Option            -> else INVALID
     (the P1 / GO-F defect: option_prefix ending in an interior mkOption container)
  V3 every watch segment is declared as an option or container key in the module
     source; a leaf declared with mk*Option counts as an option    -> else INVALID
  V4 ambiguity: the last option_prefix segment is not found as a container key
     at all (e.g. only reachable through a helper)                -> AMBIGUOUS
Semantic remainder (does the target match the PR's intended source change) is NOT
checked here; it is the reviewer's job (rubric item S1).
"""
import json
import pathlib
import re
import sys
import tomllib

MK = r"(?:lib\.)?mk(?:Option|EnableOption|PackageOption)\b"


def key_pat(name):
    return re.compile(r"(?<![\w'\"-])" + re.escape(name) + r"""["']?\s*=""")


def declared_as_option(src, name):
    return re.search(r"(?<![\w'\"-])" + re.escape(name) + r"\s*=\s*" + MK, src) is not None


def declared_as_container(src, name):
    return re.search(r"(?<![\w'\"-])" + re.escape(name) + r"""["']?\s*=\s*\{""", src) is not None


def validate(target, root):
    reasons = []
    verdict = "VALID"
    mod = pathlib.Path(root) / target["module"]
    if not mod.is_file():
        return "INVALID", ["V1: module file not found under source root"]
    src = mod.read_text()
    prefix = target["option_prefix"]
    last = prefix[-1]
    if declared_as_option(src, last):
        return "INVALID", [f"V2: option_prefix last segment {last!r} is declared with mk*Option (interior option container, not a walkable root)"]
    if not (declared_as_container(src, last) or key_pat(last).search(src)):
        verdict = "AMBIGUOUS"
        reasons.append(f"V4: option_prefix segment {last!r} not found as a container key in module text")
    for w in target["watch"]:
        segs = w.split(".")
        for seg in segs[:-1]:
            if re.search(r"(?<![\w'\"-])" + re.escape(seg) + r"""["']?\s*[=.]""", src):
                continue
            return "INVALID", [f"V3: watch segment {seg!r} (in {w!r}) not declared in module source"]
        leaf = segs[-1]
        if declared_as_option(src, leaf) or key_pat(leaf).search(src):
            continue
        return "INVALID", [f"V3: watch leaf {leaf!r} (in {w!r}) not declared in module source"]
    if verdict == "VALID":
        reasons.append("V1-V3 pass")
    return verdict, reasons


def main():
    manifest, root = sys.argv[1], sys.argv[2]
    data = tomllib.loads(pathlib.Path(manifest).read_text())
    out = []
    for t in data.get("target", []):
        v, r = validate(t, root)
        out.append({"name": t["name"], "module": t["module"], "option_prefix": t["option_prefix"], "watch": t["watch"], "verdict": v, "reasons": r})
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
