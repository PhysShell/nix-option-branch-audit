#!/usr/bin/env python3
"""Build the independent closure-audit packet from the committed rubrics.

The packet omits controller verdicts, case results, gap classifications and
probe status, so the reviewer sees the rules and the source evidence only.
Source excerpts are read from pinned commits with `git show`, never a worktree.
"""
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent / "packet"
REPO = "/home/tandem/.cache/go-h/nixpkgs-exact"
E = "e4c7d977153965496cbe73ff5631ca6a1118b347"
B = "e2497c3a5262687ff7aada0707bc754cca66396f"
F = "fdc3396349781c0dc2b495b6a320f62918bd63f6"

EXCERPTS = [
    (B, "nixos/modules/services/monitoring/beszel-agent.nix", 204, 216, "beszel, base B"),
    (F, "nixos/modules/services/monitoring/beszel-agent.nix", 44, 56, "beszel, head F"),
    (E, "nixos/modules/services/accessibility/speechd.nix", 34, 50, "speechd, E"),
    (E, "nixos/modules/services/audio/mpd.nix", 150, 186, "mpd settings submodule, E"),
    (E, "lib/trivial.nix", 302, 310, "boolToYesNo, E"),
    (E, "nixos/modules/misc/locate.nix", 188, 200, "locate pruneBindMounts, E"),
    (E, "nixos/modules/misc/locate.nix", 240, 246, "locate updatedb.conf, E"),
    (E, "nixos/modules/misc/locate.nix", 270, 278, "locate command, E"),
    (E, "nixos/modules/services/web-apps/movim.nix", 474, 486, "movim database.type, E"),
    (E, "nixos/modules/services/web-apps/movim.nix", 620, 630, "movim computed key, E"),
    (E, "lib/strings.nix", 774, 778, "optionalString, E"),
    (E, "lib/default.nix", 266, 280, "lib.lists binding, E"),
    (F, "lib/default.nix", 266, 280, "lib.lists binding, F"),
    (F, "lib/lists.nix", 16, 26, "lists.nix inherit, F"),
    (E, "lib/lists.nix", 16, 26, "lists.nix inherit, E"),
]

SCRUB_PATTERN = re.compile(
    r"\bC\d[a-z]\b|\bgap \d|closure gap|controller|READY|PARTIAL|GO-G|GO-H1D|verdict|"
    r"reviewer's|controller-drafted|not been reviewed|closed by|CLOSED"
)


def git_show(sha, path, lo, hi):
    text = subprocess.run(
        ["git", "-C", REPO, "show", f"{sha}:{path}"],
        check=True, capture_output=True, text=True,
    ).stdout.splitlines()
    return [f"{i:>5}  {text[i - 1]}" for i in range(lo, hi + 1)]


def sanitize_eligibility(d):
    out = {}
    drop = {"amendments", "h1d_probe_status", "h1d_verdict_criterion", "h1d_verdict", "h1d_closure"}
    for k, v in d.items():
        if k in drop:
            continue
        out[k] = v
    out["status"] = (
        "Finalized rubric, schema go-h-eligibility-rubric/6.0. Sanitized for "
        "independent audit: controller results and history removed."
    )
    closure = d["h1d_closure"]
    out["h1d_owner_ruling_and_clarifications"] = {
        "owner_ruling_2026_10_08": closure["owner_ruling_2026_10_08"],
        "clarifications": closure["clarifications"],
    }
    out["h1d_control_dependence"].pop("worked_examples_diagnostic", None)
    out["h1d_gating_primitives"]["per_commit_blobs"].pop("closed_by_blob", None)
    text = json.dumps(out, indent=2, ensure_ascii=False)
    for old, new in ELIG_REPLACEMENTS:
        assert text.count(old) == 1, old[:80]
        text = text.replace(old, new)
    return json.loads(text)


ELIG_REPLACEMENTS = [
    (' "PARENT_DEFAULT_REACH (h1d_closure gap 9)"', ' "PARENT_DEFAULT_REACH"'),
    ("derivation_scope (GO-H1D closure). A hunk", "derivation_scope. A hunk"),
    (
        "For C1a / beszel-agent.nix:211: the inner predicate is not a valid target; "
        "V_INVALID with NO_WATCHED_PREDICATE_DEPENDENCY / CONTROL_DEPENDENT_ONLY; no E category is assigned from the invalid target; ",
        "A control-only inner predicate is not a valid target for W: it is V_INVALID with "
        "NO_WATCHED_PREDICATE_DEPENDENCY / CONTROL_DEPENDENT_ONLY, and no E category is assigned from the invalid target. ",
    ),
    (", as activeCollectors is in C4d.", "."),
    (" as in C7c.", "."),
]


def sanitize_v2(text):
    text = text.replace(
        "CANDIDATE (GO-H1D; closed by the owner ruling of 2026-10-08, see eligibility-rubric h1d_closure). Forward-only.",
        "FINAL candidate rubric. Forward-only.",
    )
    text = text.replace(
        ', "PARENT_DEFAULT_REACH (eligibility-rubric h1d_closure gap 9)"', ""
    )
    text = text.replace(',\n    "PARENT_DEFAULT_REACH (eligibility-rubric h1d_closure gap 9)"', "")
    text = text.replace(
        "\n    \"PARENT_DEFAULT_REACH (eligibility-rubric h1d_closure gap 9)\"", ""
    )
    text = text.replace(" (owner-independent GO-H1D closure clarification)", "")
    return text


def scan(name, obj_text):
    hits = []
    for m in SCRUB_PATTERN.finditer(obj_text):
        s = max(0, m.start() - 70)
        hits.append(obj_text[s:m.end() + 70].replace("\n", " "))
    return hits


def main():
    OUT.mkdir(parents=True, exist_ok=True)

    elig = json.loads((ROOT / "work/go-h/eligibility-rubric.json").read_text(encoding="utf-8"))
    elig_s = sanitize_eligibility(elig)
    elig_text = json.dumps(elig_s, indent=2, ensure_ascii=False) + "\n"
    (OUT / "01-eligibility-rubric.json").write_text(elig_text, encoding="utf-8")

    v1 = (ROOT / "work/go-h/target-validity-rubric.json").read_text(encoding="utf-8")
    (OUT / "02-target-validity-rubric-v1.json").write_text(v1, encoding="utf-8")

    v2_raw = (ROOT / "work/go-h/target-validity-rubric.v2.json").read_text(encoding="utf-8")
    v2 = sanitize_v2(v2_raw)
    json.loads(v2)
    (OUT / "03-target-validity-rubric-v2.json").write_text(v2, encoding="utf-8")

    readme = (ROOT / "README.md").read_text(encoding="utf-8").splitlines()
    product = ["# Product definition (README.md at the audited commit, verbatim)", ""]
    product += [f"{i + 1:>4}  {readme[i]}" for i in range(0, 24)]
    product += ["", "...", ""]
    product += [f"{i + 1:>4}  {readme[i]}" for i in range(162, 180)]
    product += ["", "# Scope object (eligibility-rubric scope, verbatim)", ""]
    product += [json.dumps(elig["scope"], indent=2, ensure_ascii=False)]
    (OUT / "00-product-definition.md").write_text("\n".join(product) + "\n", encoding="utf-8")

    ex = [
        "# Exact-SHA source excerpts. Read from pinned commits with git show; no working tree.",
        f"# repo: {REPO}",
        "",
    ]
    for sha, path, lo, hi, label in EXCERPTS:
        ex.append(f"=== {label}: {sha[:12]}:{path} lines {lo}-{hi}")
        ex += git_show(sha, path, lo, hi)
        ex.append("")
    (OUT / "05-source-excerpts.txt").write_text("\n".join(ex) + "\n", encoding="utf-8")

    (OUT / "04-exact-tree-search.sh").write_text((ROOT / "work/go-h/exact_tree_search.sh").read_text(encoding="utf-8"), encoding="utf-8")

    hits = []
    for name, text in [
        ("01", elig_text),
        ("02", v1),
        ("03", v2),
        ("00", "\n".join(product)),
    ]:
        for h in scan(name, text):
            hits.append(f"{name}: ...{h}...")
    (OUT.parent / "scrub-scan.txt").write_text("\n".join(hits) + "\n", encoding="utf-8")
    print(f"packet written to {OUT}; scrub hits: {len(hits)}")


if __name__ == "__main__":
    main()
