#!/usr/bin/env python3
"""GO-G DIAGNOSTIC / NON-SCORING.

Classifies the frozen GO-F and P1 PR samples by the protocol stage at which each
PR stopped (frozen records, copied verbatim from
reports/go-f-f2-1-target-construction.md and fixtures/s6-r1/P1-target-construction.md),
plus a live read-only declaration-span check against NixOS/nixpkgs base/head sources.

Does not run oba, does not change any frozen artifact, does not re-score GO-F.
"""
import base64
import json
import pathlib
import re
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[2]
DECL = re.compile(r"\b(mkOption|mkEnableOption|mkPackageOption)\b")
HUNK = re.compile(r"@@ -(\d+)(?:,\d+)? \+(\d+)")

# Frozen GO-F stop points (reports/go-f-f2-1-target-construction.md, a7931cb).
# wired: step-5 result as judged in this diagnostic (None = step 5 not reached).
# config: PR assigns to an existing option without declaring it (diagnostic-only class).
GOF = [
    (570480, "derived", 1, True, "frozen derived", False),
    (565935, "step5", 0, False, "all-tests.nix at head has no tabby entry; no nixos/tests file changed", False),
    (483650, "step3", 0, False, "all-tests.nix at head has no syncoid entry (only borgbackup/mysql-* backup entries); no nixos/tests file changed", True),
    (568883, "step2", 0, None, "step 2: no nixos/modules file", False),
    (510072, "step2", 0, None, "step 2: no nixos/modules file", False),
    (565712, "step2", 0, None, "step 2: no nixos/modules file", False),
    (519655, "derived", 5, True, "frozen derived", False),
    (569962, "step3", 0, True, "all-tests.nix at head: netbird = runTest ./netbird.nix (service-name mapping, judgment)", False),
    (569876, "step3", 0, True, "all-tests.nix at head: radicle = runTest ./radicle.nix", False),
    (564819, "derived", 2, True, "frozen derived", False),
    (569875, "derived", 1, True, "frozen derived", False),
    (566204, "step2", 0, None, "step 2: no nixos/modules file", False),
    (565592, "step2", 0, None, "step 2: no nixos/modules file", False),
    (556686, "step3", 0, True, "PR changes nixos/tests/pgbackrest/sftp.nix; all-tests.nix at head: pgbackrest = import ./pgbackrest", True),
    (557977, "derived", 6, True, "frozen derived", False),
]

# Frozen P1 stop points (fixtures/s6-r1/P1-target-construction.md). Harness-repair
# reclassification (fixtures/s6-r1/P1-harness-repair.md) applied as a separate column.
P1 = [
    (565943, "step3", 0, None, "", False, None),
    (566007, "step3", 0, None, "", False, None),
    (569867, "step3", 0, None, "", False, None),
    (508090, "derived", 2, True, "frozen derived", False, None),
    (568429, "derived", 4, True, "frozen derived", False, None),
    (563823, "derived", 1, True, "frozen derived", False, None),
    (566696, "step3", 0, None, "", False, None),
    (443747, "derived", 10, True, "frozen derived", False, "NOT_EVALUABLE_BY_CURRENT_PROTOCOL (NEW_MODULE_NO_BASE)"),
    (556752, "step3", 0, None, "", False, None),
    (564688, "step3", 0, None, "", False, None),
    (568782, "step5", 0, False, "all-tests.nix at head has no dnscache entry; no nixos/tests file changed", False, None),
    (567915, "step2", 0, None, "step 2: no nixos/modules file", False, None),
    (568245, "step2", 0, None, "step 2: no nixos/modules file", False, None),
    (561242, "step2", 0, None, "step 2: no nixos/modules file", False, None),
    (568048, "derived", 11, True, "frozen derived", False, "NOT_EVALUABLE_BY_CURRENT_PROTOCOL (NEW_MODULE_NO_BASE)"),
]


def gh(args):
    r = subprocess.run(["gh", "api", *args], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def file_at(path, ref):
    out = gh([f"repos/NixOS/nixpkgs/contents/{path}?ref={ref}", "--jq", ".content"])
    return base64.b64decode(out).decode() if out and out.strip() else None


def enclosed_by_decl(lines, idx):
    depth = 0
    for i in range(idx - 1, -1, -1):
        for ch in reversed(lines[i]):
            if ch == "}":
                depth += 1
            elif ch == "{":
                if depth == 0 and DECL.search(lines[i]):
                    return True
                if depth:
                    depth -= 1
    return False


def line_in_decl(lines, idx):
    if idx >= len(lines):
        return False
    return bool(DECL.search(lines[idx])) or enclosed_by_decl(lines, idx)


def span(path, base, head, patch):
    bl = (file_at(path, base) or "").split("\n") if base else [""]
    hl = (file_at(path, head) or "").split("\n") if head else [""]
    changed = inside = 0
    bi = hi = None
    for line in patch.split("\n"):
        m = HUNK.match(line)
        if m:
            bi, hi = int(m.group(1)) - 1, int(m.group(2)) - 1
            continue
        if bi is None:
            continue
        if line.startswith("-"):
            changed += 1
            inside += line_in_decl(bl, bi)
            bi += 1
        elif line.startswith("+"):
            changed += 1
            inside += line_in_decl(hl, hi)
            hi += 1
        elif line.startswith(" "):
            bi += 1
            hi += 1
    return changed, inside


def classify(pr, group, stop, derived_targets, wired, wired_evidence, config, p1_note, meta_base=None, meta_head=None):
    info = json.loads(gh([f"repos/NixOS/nixpkgs/pulls/{pr}"]))
    base, head = info["base"]["sha"], info["merge_commit_sha"]
    files = json.loads(gh([f"repos/NixOS/nixpkgs/pulls/{pr}/files?per_page=100"]))
    module_files = [f for f in files if f["filename"].startswith("nixos/modules/")]
    test_files = [f["filename"] for f in files if f["filename"].startswith("nixos/tests/")]
    span_by_file = {}
    for f in module_files:
        ch, ins = span(f["filename"], base, head, f.get("patch") or "")
        span_by_file[f["filename"]] = {"changed_lines": ch, "lines_inside_declaration": ins}
    inside_total = sum(v["lines_inside_declaration"] for v in span_by_file.values())
    step2 = bool(module_files)
    step3 = inside_total > 0
    uniform = stop == "derived" or (step2 and step3 and wired is True)
    meta_ok = None
    if meta_base is not None:
        meta_ok = (meta_base == base and meta_head == head)
    return {
        "group": group,
        "pr": pr,
        "title": info["title"],
        "base_sha": base,
        "head_sha": head,
        "meta_matches_api": meta_ok,
        "files_total": len(files),
        "module_files": [f["filename"] for f in module_files],
        "test_files_changed": test_files,
        "span_check": span_by_file,
        "frozen_stop": stop,
        "frozen_derived_targets": derived_targets,
        "step2_module_file_present": step2,
        "step3_declaration_lines_touched": inside_total,
        "step3_declaration_touched": step3,
        "step5_wired_test": wired,
        "step5_evidence": wired_evidence,
        "config_level_candidate": config,
        "uniform_rule_derived": uniform,
        "frozen_consistent_with_span": (stop != "derived") or step3,
        "p1_harness_repair": p1_note,
        "classification_label": p1_note or stop,
    }


def meta(pr):
    p = ROOT / f"work/go-f/pr-meta/{pr}-meta.json"
    m = json.loads(p.read_text())
    return m["base_sha"], m["merge_commit_sha"]


def main():
    records = []
    for pr, stop, n, wired, ev, cfg in GOF:
        mb, mh = meta(pr)
        records.append(classify(pr, "GO-F", stop, n, wired, ev, cfg, None, mb, mh))
    for pr, stop, n, wired, ev, cfg, note in P1:
        records.append(classify(pr, "P1", stop, n, wired, ev, cfg, note))

    gof = [r for r in records if r["group"] == "GO-F"]
    p1 = [r for r in records if r["group"] == "P1"]

    def stops(rs):
        out = {}
        for r in rs:
            out[r["frozen_stop"]] = out.get(r["frozen_stop"], 0) + 1
        return out

    uniform_set = {r["pr"] for r in gof if r["uniform_rule_derived"]}
    step5_relax = {r["pr"] for r in gof if r["step2_module_file_present"] and r["step3_declaration_touched"]}
    config_wired = {r["pr"] for r in gof if r["config_level_candidate"] and r["step5_wired_test"] is True}
    config_any = {r["pr"] for r in gof if r["config_level_candidate"]}
    ceilings = [
        ("R0 frozen GO-F (as recorded)", {r["pr"] for r in gof if r["frozen_stop"] == "derived"}),
        ("R1 uniform syntactic declaration rule + wired test (no protocol change beyond consistent application)", uniform_set),
        ("R2 R1 + step-5 relaxation (declaration-touched PRs admitted without a wired test)", uniform_set | step5_relax),
        ("R3 R1 + config-level assignment admission (wired-test PRs only)", uniform_set | config_wired),
        ("R4 R1 + step-5 relaxation + config-level admission (wired-test PRs) = minimal PASS path", uniform_set | step5_relax | config_wired),
        ("R5 R4 + config-level admission for PR without wired test", uniform_set | step5_relax | config_any),
    ]
    ceiling_rows = [{"row": name, "count": len(s), "prs": sorted(s), "nc2_pass_threshold": 9, "nc2_pass": len(s) >= 9, "NON_SCORING_COUNTERFACTUAL": True} for name, s in ceilings]

    out = {
        "schema": "go-g-pr-classification/1",
        "status": "DIAGNOSTIC / NON-SCORING",
        "source_head": "a7931cb",
        "frozen_inputs": [
            "reports/go-f-f2-1-target-construction.md",
            "reports/go-f-evaluation-final.md",
            "fixtures/s6-r1/P1-target-construction.md",
            "fixtures/s6-r1/P1-harness-repair.md",
            "work/go-f/pr-meta/*-meta.json",
        ],
        "live_inputs": "NixOS/nixpkgs base/head sources and PR file patches, fetched read-only via gh api at classification time",
        "rules": {
            "step2": "PR has at least one nixos/modules/** file changed",
            "step3_syntactic": "at least one changed line lies inside, or is, a line containing mkOption/mkEnableOption/mkPackageOption (heuristic brace walk)",
            "step5": "wired test: nixos/tests file changed, or all-tests.nix entry at head (manual mapping, judgment)",
            "uniform_rule_derived": "step2 AND step3_syntactic AND step5 (frozen derived PRs count as derived)",
        },
        "summary": {
            "GO-F": {
                "frozen_stop_counts": stops(gof),
                "frozen_derived_pr_count": sum(1 for r in gof if r["frozen_stop"] == "derived"),
                "frozen_derived_target_count": sum(r["frozen_derived_targets"] for r in gof),
                "uniform_rule_derived_pr_count": len(uniform_set),
                "frozen_records_inconsistent_with_span": [r["pr"] for r in gof if not r["frozen_consistent_with_span"]],
                "ceilings": ceiling_rows,
            },
            "P1": {
                "frozen_stop_counts": stops(p1),
                "frozen_derived_pr_count_original": sum(1 for r in p1 if r["frozen_stop"] == "derived"),
                "frozen_derived_pr_count_corrected": sum(1 for r in p1 if r["frozen_stop"] == "derived" and not r["p1_harness_repair"]),
                "uniform_rule_derived_pr_count_corrected": sum(1 for r in p1 if r["uniform_rule_derived"] and not r["p1_harness_repair"]),
                "note": "P1 cohort shown separately; not pooled with GO-F",
            },
        },
        "records": records,
    }
    dest = ROOT / "work/go-g/pr-classification.json"
    dest.write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps(out["summary"], indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
