#!/usr/bin/env python3
"""S5-F2-R section 7: cross-instance ambiguity pre-scan, restricted to
targets flagged eligible:true in eligibility-manifest.json. TEXT-LEVEL
enumeration over the real fetched head-side test source -- deliberately
NOT a re-implementation of oba's own AST walker (that would be circular:
checking oba's own matching using oba's own matching logic). Two
supported concrete shapes, matched conservatively; anything else is
flagged ambiguous/not-extracted rather than guessed:

  (a) fully flat-dotted: `<prefix...>."<instance>".<suffix...> = <value>;`
      or `<prefix...>.<instance>.<suffix...> = <value>;` (bare instance key)
  (b) instance-block form: `<prefix...>."<instance>" = { ... };` (or bare
      instance key), with the suffix's OWN flat-dotted form
      (`<suffix...> = <value>;`) found via bracket-balanced extraction of
      that instance's own attrset body, then a regex search within it.
      Nested-attrset suffix forms (`gitHttpBackend = { enable = ...; };`)
      are NOT specifically handled -- flagged as
      parse_status=ambiguous_suffix_not_flat_within_instance_block if the
      instance block exists but the flat-dotted suffix regex finds
      nothing inside it, rather than silently reporting zero candidates.

A "node" is approximated as the nearest enclosing top-level `nodes.<x>`
or `nodes.<x> = { ... }:` construct found by scanning backward from the
match for the last `nodes.<ident>` occurrence before it, or the nearest
enclosing `{ ... }: { ... }` lambda body under `nodes = { <ident> = ... }`
-- best-effort, reported as-is, not claimed authoritative.
"""
import base64
import json
import re
import subprocess
from pathlib import Path

ROOT = Path("/home/tandem/nix-option-branch-audit")
D = ROOT / "fixtures/s5-f2-r"
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
            content = base64.b64decode(data["content"]).decode("utf-8", errors="replace")
            content_cache[key] = content
            return content
        except Exception:
            if attempt == 2:
                content_cache[key] = None
                return None
    return None


SEG_RE = r'(?:"[^"]*"|[A-Za-z_][A-Za-z0-9_\'-]*)'


def seg_literal(seg):
    # produce a regex alternation matching this literal segment either
    # bare or quoted, escaping regex metacharacters in the literal text
    esc = re.escape(seg)
    return rf'(?:"{esc}"|{esc})'


def strip_quotes(s):
    if s.startswith('"') and s.endswith('"'):
        return s[1:-1]
    return s


def find_enclosing_line_start(src, pos):
    line = src.count("\n", 0, pos) + 1
    return line


def nearest_node_name(src, pos):
    # best-effort: last `nodes.<ident>` or `<ident> =` immediately after
    # a `nodes = {` block start, before `pos`
    m = None
    for mm in re.finditer(r'nodes\.([A-Za-z_][A-Za-z0-9_\'-]*)', src[:pos]):
        m = mm
    if m:
        return m.group(1)
    # fallback: look for `nodes = { <ident>` pattern
    nodes_block = re.search(r'nodes\s*=\s*\{', src[:pos])
    if nodes_block:
        after = src[nodes_block.end():pos]
        idents = re.findall(r'([A-Za-z_][A-Za-z0-9_\'-]*)\s*=', after)
        if idents:
            return idents[0]
    return None


def extract_balanced_block(src, brace_open_pos):
    # brace_open_pos points at the '{' character
    depth = 0
    i = brace_open_pos
    n = len(src)
    while i < n:
        c = src[i]
        if c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0:
                return src[brace_open_pos:i + 1]
        i += 1
    return None


def scan_target(src, option_prefix, suffix_segs):
    prefix_pattern = r'\.'.join(seg_literal(s) for s in option_prefix)
    suffix_pattern = r'\.'.join(seg_literal(s) for s in suffix_segs)
    candidates = []

    # (a) fully flat-dotted
    flat_re = re.compile(
        prefix_pattern + r'\.(' + SEG_RE + r')\.' + suffix_pattern + r'\s*=\s*([^;]+);'
    )
    for m in flat_re.finditer(src):
        instance_raw = m.group(1)
        value_text = m.group(2).strip()
        line = find_enclosing_line_start(src, m.start())
        node = nearest_node_name(src, m.start())
        candidates.append({
            "form": "flat_dotted",
            "instance_key": strip_quotes(instance_raw),
            "instance_key_raw": instance_raw,
            "value_text": value_text,
            "node": node,
            "line": line,
        })

    # (b) instance-block form
    block_re = re.compile(prefix_pattern + r'\.(' + SEG_RE + r')\s*=\s*\{')
    for m in block_re.finditer(src):
        instance_raw = m.group(1)
        brace_pos = src.rfind('{', m.start(), m.end())
        block = extract_balanced_block(src, brace_pos)
        if block is None:
            continue
        line = find_enclosing_line_start(src, m.start())
        node = nearest_node_name(src, m.start())
        suf_re = re.compile(suffix_pattern + r'\s*=\s*([^;]+);')
        suf_matches = list(suf_re.finditer(block))
        if not suf_matches:
            candidates.append({
                "form": "instance_block",
                "instance_key": strip_quotes(instance_raw),
                "instance_key_raw": instance_raw,
                "value_text": None,
                "node": node,
                "line": line,
                "parse_status": "ambiguous_suffix_not_flat_within_instance_block",
            })
            continue
        for sm in suf_matches:
            candidates.append({
                "form": "instance_block",
                "instance_key": strip_quotes(instance_raw),
                "instance_key_raw": instance_raw,
                "value_text": sm.group(1).strip(),
                "node": node,
                "line": line + src[:brace_pos].count("\n") - src[:m.start()].count("\n") + block[:sm.start()].count("\n"),
            })
    return candidates


def main():
    elig = json.loads((D / "eligibility-manifest.json").read_text())
    manifest = {e["pr"]: e for e in json.loads((D / "applicable-manifest.json").read_text())}

    out = []
    eligible_targets = [r for r in elig if r.get("eligible") is True]
    print(f"scanning {len(eligible_targets)} eligible targets for cross-instance candidates")

    # group by pr to fetch test source once per pr
    from collections import defaultdict
    by_pr = defaultdict(list)
    for r in eligible_targets:
        by_pr[r["pr"]].append(r)

    for pr, targets in by_pr.items():
        e = manifest.get(pr)
        if e is None:
            continue
        head_sha = e["head_sha"]
        orig_dir = ROOT / e["orig_dir"]
        import tomllib
        parsed = tomllib.loads((orig_dir / "targets.toml").read_text())
        toml_targets = {t["name"]: t for t in parsed.get("target", [])}

        for r in targets:
            tname = r["target_name"]
            tt = toml_targets.get(tname)
            if tt is None:
                out.append({**r, "candidates": [], "error": "target not found in targets.toml"})
                continue
            test_path = tt["test"]
            src = gh_content(test_path, head_sha)
            if src is None:
                out.append({**r, "candidates": [], "error": "fetch failed"})
                continue
            watch_list = tt.get("watch", [])
            pr_candidates = {}
            for w in watch_list:
                suffix_segs = w.split(".")
                cands = scan_target(src, r["option_prefix"], suffix_segs)
                distinct_values = {c["value_text"] for c in cands if c.get("value_text") is not None}
                pr_candidates[w] = {
                    "candidates": cands,
                    "textually_distinct_values": len(distinct_values) > 1,
                }
            out.append({
                "pr": pr, "cohort": r["cohort"], "target_name": tname,
                "option_prefix": r["option_prefix"], "watch": watch_list,
                "per_suffix": pr_candidates,
            })

    (D / "cross-instance-candidates.json").write_text(json.dumps(out, indent=2) + "\n")

    multi = [o for o in out if any(v.get("textually_distinct_values") for v in o.get("per_suffix", {}).values())]
    print(f"wrote cross-instance-candidates.json: {len(out)} eligible-target records, {len(multi)} with textually_distinct_values on some suffix")
    for o in multi:
        print(f"  PR {o['pr']} target={o['target_name']} option_prefix={o['option_prefix']}")


if __name__ == "__main__":
    main()
