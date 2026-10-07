import json
import re

excl = {}


def add(numbers, source):
    for n in numbers:
        n = int(n)
        excl.setdefault(n, []).append(source)


s5 = json.load(open('fixtures/s6-r0/excluded-pr-identities.json'))
add(s5['excluded_pr_numbers'], 'S5-369-ledger')

for w in (1, 2, 3):
    d = json.load(open(f'fixtures/s6-r0/P0-window{w}-raw-log.json'))
    nums = [e['pr_number'] for e in d['inspection_log']]
    add(nums, f'S6-R0-window{w}-census')

d = json.load(open('fixtures/s6-r1/P0-window1-raw-log.json'))
nums = [e['pr_number'] for e in d['inspection_log']]
add(nums, 'S6-R1-window1-census-750')

text = open('fixtures/s6-r1/P1-sample.md').read()
m_pool = re.search(r'\*\*Pool, chronological order\*\* \(26\): `([^`]+)`', text)
m_sample = re.search(r'\*\*P1 sample\*\* \(15, deterministic\): `([^`]+)`', text)
m_unused = re.search(r'\*\*Not selected\*\* \(11.*?\): `([^`]+)`', text)
for m, label in [(m_pool, 'S6-R1-P1-pool-26'), (m_sample, 'S6-R1-P1-sample-15'), (m_unused, 'S6-R1-P1-unused-11')]:
    nums = [x.strip() for x in m.group(1).split(',')]
    add(nums, label)

add([443747, 568048], 'GO-E-replay')

print('total unique excluded PR numbers:', len(excl))
out = {
    'excluded_pr_numbers': sorted(excl.keys()),
    'provenance': {str(k): v for k, v in sorted(excl.items())},
}
json.dump(out, open('work/go-f/exclusion-set.json', 'w'), indent=2)
print('written work/go-f/exclusion-set.json')
