#!/usr/bin/env python3
"""S4-F1-R: mechanically derive the regression corpus (every S4
pr_summary with applicable==true) from the committed, frozen
adjudication-ledger.jsonl -- never hand-typed. Reads only:
  fixtures/s4-live-pr-shadow/adjudication-ledger.jsonl (untouched by this)
Writes: fixtures/s4-f1-r/corpus.json
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent  # repo root
LEDGER = ROOT / "fixtures/s4-live-pr-shadow/adjudication-ledger.jsonl"


def main():
    records = [json.loads(line) for line in LEDGER.read_text().splitlines() if line.strip()]
    pr_summaries = [r for r in records if r.get("record_type") == "pr_summary"]
    applicable = [r for r in pr_summaries if r.get("applicable") is True]

    corpus = []
    for r in applicable:
        # Locate the committed S4 artifact directory for this PR so we can
        # reuse its real targets.toml (identical manifest S4 itself used)
        # without regenerating anything.
        ref = r["raw_json_ref"]  # e.g. "fixtures/s4-live-pr-shadow/adjudication/batch2/526840/raw.json"
        assert ref.startswith("fixtures/s4-live-pr-shadow/adjudication/"), ref
        artifact_dir = str(Path(ref).parent)
        corpus.append({
            "pr": r["pr"],
            "cohort": r["cohort"],
            "position": r["position"],
            "base_sha": r["base_sha"],
            "head_sha": r["head_sha"],
            "module_paths": r["module_paths"],
            "test_paths": r["test_paths"],
            "s4_artifact_dir": artifact_dir,
            "s4_targets_toml": f"{artifact_dir}/targets.toml",
            "s4_raw_json": f"{artifact_dir}/raw.json",
        })

    out = Path(__file__).resolve().parent / "corpus.json"
    out.write_text(json.dumps(corpus, indent=2) + "\n")
    print(f"total pr_summary records: {len(pr_summaries)}")
    print(f"applicable (S4-F1-R corpus): {len(applicable)}")
    print(f"  S4-A: {sum(1 for c in corpus if c['cohort']=='A')}")
    print(f"  S4-B: {sum(1 for c in corpus if c['cohort']=='B')}")
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
