{ ... }:
{
  # x held at its own declared default (false) -- no transition, so
  # OBA001. H1 finds nothing at all for `x` (see module.nix), so
  # `diagnostic_default_outcome` is computed PURELY through H2's
  # single-ref `.or_else` fallback lookup, with no H1 result able to
  # mask a broken one (see the h2_case13 golden test).
  nodes.machine =
    { ... }:
    {
      services.synthSingleRefH2Only.x = false;
    };
}
