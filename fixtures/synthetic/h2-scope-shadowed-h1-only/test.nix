{ ... }:
{
  # flag: declared default false, test true -- a clean opposite-outcome
  # transition, but the ONLY predicate that could witness it is visible
  # to H1's spelling-based scanner alone (see module.nix). If H1's own
  # gate-4 opposite-outcome detection were ever silently broken, no H2
  # candidate exists here to redundantly rescue the verdict -- correct
  # result: PASS, or this target fails loudly instead of silently.
  nodes.machine =
    { ... }:
    {
      services.synthScopeShadow.flag = true;
    };
}
