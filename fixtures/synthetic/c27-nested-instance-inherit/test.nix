{ ... }:
{
  nodes = {
    machine =
      { ... }:
      {
        services.synth.foo = "/run/default.sock";
      };

    # `inherit hidden;` directly under `nodes = { ... };` -- before this
    # fix, the inner loop only special-cased NODE_ATTRPATH_VALUE entries
    # and silently `continue`d past anything else (NODE_INHERIT included),
    # exactly like c24's inherit case one level up, but this inner loop
    # never got the equivalent fix in H1.3. Must be TestConfigUnresolved,
    # not OBA001 -- the walker genuinely doesn't know what `hidden` binds.
    inherit hidden;
  };
}
