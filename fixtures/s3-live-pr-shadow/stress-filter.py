import json, re, os

with open('/tmp/s3-remaining-for-stress.json') as f:
    remaining = json.load(f)

# EXACT same categories as S2-B, unchanged, per the protocol's own
# explicit "reused verbatim, never re-tuned" requirement.
categories = {
    "option_declaration": re.compile(r'\b(mkOption|mkEnableOption)\b'),
    "nixos_tests_touched": None,
    "execstart_script_cmdline": re.compile(r'\b(ExecStart|script\s*=)\b'),
    "environment": re.compile(r'\b(environment|EnvironmentFile|environmentFile)\b'),
    "generated_config": re.compile(r'\b(writeText|toYAML|toJSON|settingsFormat|format\.generate|configFile)\b'),
    "package_version_source_dep": re.compile(r'(^[+-]\s*version\s*=|^[+-]\s*src\s*=\s*fetch|^[+-]\s*rev\s*=|sha256-[A-Za-z0-9+/=]{20,})'),
}

def added_removed_lines(diff_text):
    lines = []
    for line in diff_text.splitlines():
        if line.startswith('+++') or line.startswith('---'):
            continue
        if line.startswith('+') or line.startswith('-'):
            lines.append(line)
    return lines

results = []
for r in remaining:
    num = r['number']
    path = f"/tmp/s3-diffs/{num}.diff"
    if not os.path.exists(path) or os.path.getsize(path) == 0:
        results.append((r, [], False))
        continue
    with open(path, encoding='utf-8', errors='replace') as f:
        diff_text = f.read()
    lines = added_removed_lines(diff_text)
    joined = "\n".join(lines)
    matched = []
    for cat, pat in categories.items():
        if pat is None:
            continue
        if pat.search(joined):
            matched.append(cat)
    paths = [fl["path"] for fl in r.get("files", [])]
    if any(p.startswith("nixos/tests/") for p in paths):
        matched.append("nixos_tests_touched")
    results.append((r, matched, len(matched) > 0))

stress_eligible = [(r, m) for (r, m, ok) in results if ok]
not_eligible = [(r, m) for (r, m, ok) in results if not ok]

print(f"stress-eligible: {len(stress_eligible)} / {len(remaining)}")
print(f"not eligible: {len(not_eligible)}")

from collections import Counter
cat_counts = Counter()
for r, m in stress_eligible:
    for c in m:
        cat_counts[c] += 1
print("category hit counts:")
for c, n in cat_counts.most_common():
    print(f"  {c}: {n}")

with open('/tmp/s3-stress-eligible.json', 'w') as f:
    json.dump([r for r, m in stress_eligible], f, indent=2)
with open('/tmp/s3-stress-eligible-with-cats.json', 'w') as f:
    json.dump([{"number": r["number"], "title": r["title"], "categories": m} for r, m in stress_eligible], f, indent=2)
