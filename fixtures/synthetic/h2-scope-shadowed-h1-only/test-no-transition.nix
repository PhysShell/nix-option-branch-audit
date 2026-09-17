{ ... }:
{
  # flag held at its own declared default (false) -- no transition, so
  # this target must resolve to OBA001 rather than PASS. Since H2 has
  # ZERO candidates for `flag` in this module (see module.nix -- the
  # only predicate site is scope-shadowed away from H2 entirely),
  # `diagnostic_default_outcome` is computed PURELY from H1's own lookup,
  # with no H2 `.or_else` fallback able to rescue a broken one. Isolates
  # that lookup specifically (see the h2_case12 golden test).
  nodes.machine =
    { ... }:
    {
      services.synthScopeShadow.flag = false;
    };
}
