{ ... }:
{
  # Same nested `nodes = { machine = ...; };` form as c18, but the
  # assignment matches the default's predicate outcome -- must correctly
  # produce OBA001 through this form too, not just the flat one.
  nodes = {
    machine =
      { ... }:
      {
        services.synth.foo = "/run/default.sock";
      };
  };
}
