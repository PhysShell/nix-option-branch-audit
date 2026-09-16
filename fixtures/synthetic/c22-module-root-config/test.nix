{ ... }:
{
  nodes.machine =
    { ... }:
    {
      # `config = { ... };` is a real, common NixOS module idiom
      # (explicit instead of the top-level shorthand kimai/davis's tests
      # use). Its contents must normalize into the SAME option-path
      # namespace as a bare `services.synth.foo = ...;` at module root --
      # `config.services.synth.foo`, not `services.synth.foo`, would be
      # wrong and would never match the watched option's real path.
      config = {
        services.synth.foo = null;
      };
    };
}
