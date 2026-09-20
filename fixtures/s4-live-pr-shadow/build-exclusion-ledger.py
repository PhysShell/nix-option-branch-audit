#!/usr/bin/env python3
"""S4: generate the exclusion ledger from repository evidence.

Per the S4 mandate: "Do not construct this exclusion set manually
from memory. Generate it from repository evidence into a
machine-readable artifact and commit it before drawing S4. Record
exact source files and counts used to construct it."

Real, deterministic, re-runnable. Every entry in the output ledger
carries its own provenance (which source file(s) produced it) so this
can be independently checked or regenerated, not merely trusted.

Sources actually read:
  - fixtures/s1-live-pr-shadow/sample.md   (S1's own drawn 30 PR numbers)
  - fixtures/s2-live-pr-shadow/sample.md   (S2's own drawn 50 PR numbers,
    A+B cohorts)
  - fixtures/s3-live-pr-shadow/sample.md   (S3's own drawn 50 PR numbers,
    A+B cohorts, including the 2 later-excluded-but-still-examined ones)
  - fixtures/s1-live-pr-shadow/raw-population-196.jsonl (S1's own raw
    population -- real file paths for every S1-examined PR)
  - fixtures/s2-live-pr-shadow/population-134.json (S2's own raw
    population -- real file paths, superset covering S2's drawn 50)
  - fixtures/s3-live-pr-shadow/population-94.json +
    fixtures/s3-live-pr-shadow/stress-eligible-52.json (S3's own raw
    population -- real file paths, covering S3's drawn 50)
  - fixtures/s2-f3-exporters-census/exporter-names-93.txt (S2-F3's own
    real census of every Prometheus exporter module name in nixpkgs)
  - fixtures/synthetic/**, fixtures/real/**, fixtures/kimai/** (this
    repo's own golden/synthetic fixture subjects -- real subject names
    already baked into this project's own test suite, never eligible
    for a "genuinely unseen" claim regardless of which round used them)
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def drawn_pr_numbers_from_sample(path: Path) -> set[int]:
    text = path.read_text()
    return {int(n) for n in re.findall(r"\[#(\d+)\]\(https://github\.com/NixOS/nixpkgs/pull/\d+\)", text)}


def load_population(path: Path) -> dict[int, dict]:
    if path.suffix == ".jsonl":
        rows = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    else:
        rows = json.loads(path.read_text())
    return {row["number"]: row for row in rows}


def subject_from_path(path: str) -> str | None:
    """Best-effort real-subject extraction from a real nixpkgs file path:
    the basename of a nixos/modules/services/**/<name>.nix or
    nixos/tests/<name>.nix path, stripping the .nix extension and any
    default/index-style filename that doesn't itself carry a subject
    name (default.nix, index.nix)."""
    p = Path(path)
    if p.suffix != ".nix":
        return None
    stem = p.stem
    if stem in ("default", "index"):
        # the real subject is usually the PARENT directory name for
        # these (e.g. nixos/modules/services/networking/kea/default.nix)
        stem = p.parent.name
    if not stem or stem in ("default", "index"):
        return None
    return stem.lower()


def main() -> None:
    provenance: dict[str, list[str]] = {}

    def record(name: str, source: str) -> None:
        name = name.strip().lower()
        if not name:
            return
        provenance.setdefault(name, [])
        if source not in provenance[name]:
            provenance[name].append(source)

    pr_numbers: dict[int, list[str]] = {}

    def record_pr(n: int, source: str) -> None:
        pr_numbers.setdefault(n, [])
        if source not in pr_numbers[n]:
            pr_numbers[n].append(source)

    # 1. Drawn PR numbers, per round, from each round's own frozen sample.md.
    s1_dir = ROOT / "fixtures/s1-live-pr-shadow"
    s2_dir = ROOT / "fixtures/s2-live-pr-shadow"
    s3_dir = ROOT / "fixtures/s3-live-pr-shadow"

    s1_prs = drawn_pr_numbers_from_sample(s1_dir / "sample.md")
    s2_prs = drawn_pr_numbers_from_sample(s2_dir / "sample.md")
    s3_prs = drawn_pr_numbers_from_sample(s3_dir / "sample.md")

    for n in s1_prs:
        record_pr(n, "S1 sample.md")
    for n in s2_prs:
        record_pr(n, "S2 sample.md")
    for n in s3_prs:
        record_pr(n, "S3 sample.md")

    # 2. Real module/test paths + subject names for every drawn PR,
    # cross-referenced against each round's own real population file
    # (which carries the real `files` list GitHub reported for it).
    s1_pop = load_population(s1_dir / "raw-population-196.jsonl")
    s2_pop = load_population(s2_dir / "population-134.json")
    s3_pop_a = load_population(s3_dir / "population-94.json")
    s3_pop_b = load_population(s3_dir / "stress-eligible-52.json")
    s3_pop = {**s3_pop_a, **s3_pop_b}

    module_paths: dict[str, list[str]] = {}
    test_paths: dict[str, list[str]] = {}

    def record_path(bucket: dict, path: str, source: str) -> None:
        bucket.setdefault(path, [])
        if source not in bucket[path]:
            bucket[path].append(source)

    for prs, pop, label in (
        (s1_prs, s1_pop, "S1"),
        (s2_prs, s2_pop, "S2"),
        (s3_prs, s3_pop, "S3"),
    ):
        for n in prs:
            row = pop.get(n)
            if row is None:
                continue  # drawn but not in this round's own raw population dump (rare, e.g. a top-up fetch) -- handled by PR-number exclusion regardless
            for f in row.get("files", []):
                path = f["path"]
                if "nixos/modules/" in path:
                    record_path(module_paths, path, f"{label} PR #{n}")
                elif "nixos/tests/" in path:
                    record_path(test_paths, path, f"{label} PR #{n}")
                subj = subject_from_path(path)
                if subj:
                    record(subj, f"{label} PR #{n} ({path})")

    # 3. S2-F3's own real exporter census (93 real module names).
    exporter_file = ROOT / "fixtures/s2-f3-exporters-census/exporter-names-93.txt"
    exporter_names = [
        line.strip() for line in exporter_file.read_text().splitlines() if line.strip()
    ]
    for name in exporter_names:
        record(name, "S2-F3 exporter census")

    # 4. This repo's own golden/synthetic fixture subjects -- real
    # subject names already baked into the test suite itself.
    fixture_dirs = ["fixtures/synthetic", "fixtures/real", "fixtures/kimai"]
    fixture_subject_hits: list[str] = []
    for d in fixture_dirs:
        base = ROOT / d
        if not base.exists():
            continue
        for p in base.rglob("*.nix"):
            subj = subject_from_path(str(p.relative_to(ROOT)))
            if subj and subj not in ("module", "test", "targets"):
                record(subj, f"repo fixture ({p.relative_to(ROOT)})")
                fixture_subject_hits.append(subj)

    # 5. The two earlier, disclosed, hand-compiled exclusion lists
    # themselves (S1's own 113-name list, S3's own 254/255-name merged
    # list) -- included as an additional, cross-checking source, not
    # the primary mechanism this time (per the S4 mandate: this ledger
    # must be GENERATED from repository evidence, not reused wholesale
    # from a prior round's own hand-compiled list). Any name in the old
    # lists NOT independently reproduced by the mechanical extraction
    # above is flagged separately below, for disclosure, not silently
    # dropped or silently trusted.
    old_lists = {
        "S1 113-name list": s1_dir / "exclusion-name-list.txt",
        "S3 254-name list": s3_dir / "exclusion-name-list-253.txt",
    }
    old_only: dict[str, list[str]] = {}
    for label, path in old_lists.items():
        names = {n.strip().lower() for n in path.read_text().splitlines() if n.strip()}
        for n in names:
            if n not in provenance:
                old_only.setdefault(n, []).append(label)
            else:
                provenance[n].append(f"(also in {label})")

    for n in old_only:
        record(n, "; ".join(old_only[n]) + " (present in an earlier round's hand-compiled list, NOT independently reproduced by this round's own mechanical extraction from real file paths -- kept for continuity, flagged here)")

    out = {
        "generated_by": "fixtures/s4-live-pr-shadow/build-exclusion-ledger.py",
        "sources": {
            "s1_sample_pr_count": len(s1_prs),
            "s2_sample_pr_count": len(s2_prs),
            "s3_sample_pr_count": len(s3_prs),
            "s1_population_rows": len(s1_pop),
            "s2_population_rows": len(s2_pop),
            "s3_population_rows": len(s3_pop),
            "exporter_census_names": len(exporter_names),
            "repo_fixture_subject_hits": len(fixture_subject_hits),
            "old_list_names_not_independently_reproduced": len(old_only),
        },
        "excluded_pr_numbers": sorted(pr_numbers.keys()),
        "excluded_pr_numbers_provenance": {str(k): v for k, v in sorted(pr_numbers.items())},
        "excluded_subject_names": sorted(provenance.keys()),
        "excluded_subject_names_provenance": {k: v for k, v in sorted(provenance.items())},
        "excluded_module_paths": sorted(module_paths.keys()),
        "excluded_module_paths_provenance": {k: v for k, v in sorted(module_paths.items())},
        "excluded_test_paths": sorted(test_paths.keys()),
        "excluded_test_paths_provenance": {k: v for k, v in sorted(test_paths.items())},
    }

    out_path = ROOT / "fixtures/s4-live-pr-shadow/exclusion-ledger.json"
    out_path.write_text(json.dumps(out, indent=2, sort_keys=False) + "\n")

    print(f"PR numbers excluded: {len(pr_numbers)}")
    print(f"Subject names excluded: {len(provenance)}")
    print(f"Module paths excluded: {len(module_paths)}")
    print(f"Test paths excluded: {len(test_paths)}")
    print(f"Names present in an old hand-compiled list but NOT independently reproduced: {len(old_only)}")
    if old_only:
        print("  ->", ", ".join(sorted(old_only)[:30]), "..." if len(old_only) > 30 else "")


if __name__ == "__main__":
    main()
