# C-E1.2c fresh holdout: population and draw

Recorded BEFORE any candidate below was inspected, per the protocol's
own requirement.

## Amendment, disclosed (not silently retrofitted)

`protocol.md` said the seed uses "the same `int(freeze_sha, 16)`
convention E1 itself used, with `freeze_sha` = `984b366`'s own full
40-char SHA." Re-reading E1's own real protocol
(`fixtures/e1-holdout-audit/protocol.md`) while actually running the
draw shows E1's real precedent used the SHORT SHA (`int("c8e42a1",
16)`, 7 hex chars), not the full 40-char one -- my own protocol
mis-stated E1's own precedent. Followed the REAL precedent (short SHA)
here, since "the same convention E1 itself used" is the actual intent:
`seed = int("984b366", 16) = 159691622`.

## Population

1. Real `PhysShell/nixpkgs` tree `68740713a1d5904edf9ba92a998a522b1b6ce080`
   (the same pin this entire project uses throughout) fetched via
   `nix eval`'s own `fetchTarball`, grepped directly from the resulting
   local store path (not `gh api search/code` -- faster and complete,
   no search-API pagination/rate-limit risk for a full-tree grep).
2. Every real file under `nixos/modules/services/` containing a
   `formats.(toml|yaml|yamlOrJson|json|ini|elixirConf|keyValue) {`
   call -- 412 real files, reduced to 202 distinct app names after
   requiring a matching top-level `nixos/tests/<name>.nix` (the same
   module.nix+test.nix-pair population shape E1 itself used).
3. Removed every name in the combined exclusion list: E1's own 40
   candidates (`ls fixtures/e1-holdout-audit/`, minus `_bisect`) UNION
   K1-K5's own real 77-name exclusion list (copied verbatim from
   `fixtures/e1-holdout-audit/protocol.md`'s own recorded list -- C-E1.1's
   14 and C-E1.2a/b's 11 implemented + `vault` are already subsets of
   E1's 40, since every one of them was originally drawn from it).
   125 unique excluded names; 32 of the 202 formats-using candidates
   were in fact already-excluded names (a real, useful cross-check that
   the population-construction method is consistent with E1's own).
4. **170 real, genuinely untouched candidates remain.** Full list:
   `fresh-holdout-population-170.txt`.

## Draw

`random.Random(159691622).shuffle(names)` (Python stdlib, Mersenne
Twister, names pre-sorted alphabetically before shuffling for a
reproducible starting order) -- first 10 after shuffle:

```
librechat
meilisearch
anubis
loki
nomad
gocron
clatd
redmine
nats
gancio
```

**These 10 are the fresh holdout.** No re-roll, no post-hoc
substitution -- investigated exactly as drawn, in this order.
