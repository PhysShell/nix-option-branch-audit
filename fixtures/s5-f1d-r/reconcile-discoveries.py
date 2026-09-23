#!/usr/bin/env python3
"""S5-F1D-R: declaration-level provenance-conservation reconciliation.

F1D deliberately changes how a promoted declaration's own `path` is
computed (bare -> qualified by its real embedding). A raw byte-diff of
`discovered_options` therefore CANNOT be used as the acceptance
criterion on its own -- per this round's own mandate, a path change is
only acceptable when proven to be the SAME underlying source
declaration, requalified, not a lost or invented one.

The reconciliation key used here is a declaration's own real SOURCE
SPAN (file, line, col) -- the same literal `mkOption {...}` call
produces the exact same span in both the historical and the candidate
binary's own output, REGARDLESS of what `path` either one recorded for
it (both binaries parse the exact same, frozen, live-fetched source
text). This is a mechanical, reproducible reconciliation -- not a
judgment call -- classifying every historical/candidate declaration
pair into exactly one of:

  same_identity          -- same span, same path, on both sides
  requalified             -- same span, DIFFERENT path (F1D's own fix
                              doing its job -- bare -> embedding-qualified)
  removed_false_duplicate -- span appeared MORE THAN ONCE historically
                              (the exact double-recording SHAPE F1D's
                              own bounded-traversal fix targets) and
                              LESS often (or once) in the candidate
  newly_reachable          -- span present in candidate only (a real
                              declaration the one-hop cap, or a related
                              limitation, previously excluded)
  unexplained_loss         -- span present historically only, not
                              explained by any of the above -- BLOCKS
                              PASS unless independently classified as a
                              pre-existing, unrelated limitation by hand
                              (recorded separately, never silently)
  unexplained_gain         -- (rare) a candidate span with no historical
                              counterpart that does not fit
                              "newly_reachable" cleanly -- flagged for
                              manual review

This script performs the MECHANICAL classification only (span-based
matching, counting). It does NOT decide "unexplained_loss is actually
fine" -- any such case must be traced by hand (real source fetch) and
recorded in classification-notes.json, exactly like every previous S5
round's own real semantic deltas.
"""
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
D = Path(__file__).resolve().parent


def load(path):
    return json.loads(path.read_text())


def span_key(o):
    s = o["span"]
    return (s["file"], s["line"], s["col"])


def reconcile_side(hist_opts, cand_opts):
    hist_by_span = defaultdict(list)
    for o in hist_opts:
        hist_by_span[span_key(o)].append(o)
    cand_by_span = defaultdict(list)
    for o in cand_opts:
        cand_by_span[span_key(o)].append(o)

    all_spans = set(hist_by_span) | set(cand_by_span)
    rows = []
    for span in sorted(all_spans):
        hlist = hist_by_span.get(span, [])
        clist = cand_by_span.get(span, [])
        hpaths = [tuple(o["path"]) for o in hlist]
        cpaths = [tuple(o["path"]) for o in clist]

        if not hlist and clist:
            rows.append({"span": span, "class": "newly_reachable", "hist_paths": [], "cand_paths": cpaths})
            continue
        if hlist and not clist:
            rows.append({"span": span, "class": "unexplained_loss", "hist_paths": hpaths, "cand_paths": []})
            continue
        # both sides have this span
        if len(hlist) > 1 and len(clist) <= 1:
            rows.append({"span": span, "class": "removed_false_duplicate", "hist_paths": hpaths, "cand_paths": cpaths})
            continue
        if set(hpaths) == set(cpaths) and len(hlist) == len(clist):
            rows.append({"span": span, "class": "same_identity", "hist_paths": hpaths, "cand_paths": cpaths})
            continue
        if set(hpaths) != set(cpaths):
            rows.append({"span": span, "class": "requalified", "hist_paths": hpaths, "cand_paths": cpaths})
            continue
        rows.append({"span": span, "class": "unexplained_gain", "hist_paths": hpaths, "cand_paths": cpaths})
    return rows


def main():
    manifest = {e["pr"]: e for e in json.loads((D / "applicable-manifest.json").read_text())}
    comparison_rows = [json.loads(l) for l in (D / "comparison-report.jsonl").read_text().splitlines() if l.strip()]
    changed_prs = [r["pr"] for r in comparison_rows if r.get("status") == "complete" and not r["identical"]]

    out = {}
    class_totals = Counter()
    for pr in changed_prs:
        e = manifest[pr]
        orig_dir = ROOT / e["orig_dir"]
        cand_dir = D / "replay" / e["cohort"] / str(pr)
        pr_rows = {}
        for side, fname in (("base", "check-base.json"), ("head", "check-head.json")):
            hist = load(orig_dir / fname)
            cand = load(cand_dir / fname)
            side_rows = []
            for ht, ct in zip(hist["targets"], cand["targets"]):
                assert ht["name"] == ct["name"]
                rows = reconcile_side(ht["discovered_options"], ct["discovered_options"])
                for r in rows:
                    class_totals[r["class"]] += 1
                side_rows.append({"target": ht["name"], "reconciliation": rows})
            pr_rows[side] = side_rows
        out[str(pr)] = pr_rows

    (D / "provenance-reconciliation.json").write_text(json.dumps(out, indent=2) + "\n")
    print(f"reconciled {len(changed_prs)} changed PRs")
    for k, v in sorted(class_totals.items()):
        print(f"  {k}: {v}")

    unexplained = class_totals.get("unexplained_loss", 0) + class_totals.get("unexplained_gain", 0)
    if unexplained:
        print(f"\n{unexplained} unexplained span-level deltas require manual tracing before PASS is possible.")
        sys.exit(1)


if __name__ == "__main__":
    main()
