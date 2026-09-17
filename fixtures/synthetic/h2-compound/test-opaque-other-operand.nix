{ ... }:
{
  # H2.2 Finding 1 regression: `createLocally` is explicitly assigned at
  # this instance, but to a non-literal (opaque) expression -- NOT left
  # unassigned. An earlier version of `evaluate_predicate_witness` only
  # distinguished "known value" vs "nothing at all for this option",
  # falling back through `.or_else(declared_defaults.get(r))` whenever
  # the lookup produced `None` for *any* reason -- including an explicit
  # assignment whose value just couldn't be classified. That silently
  # substituted createLocally's declared default (`true`) for what is
  # actually an unknown value, which could fabricate a false PASS.
  #
  # watch = database.driver. `driver` itself flips cleanly (sqlite ->
  # mysql), but `p = createLocally && driver == "mysql"` can only be
  # evaluated once createLocally's own value is known at THIS instance,
  # and it isn't. Correct result: TestValueUnresolved, never PASS.
  nodes.machine =
    { pkgs, ... }:
    {
      services.synthCompound.database = {
        createLocally = builtins.pathExists "/etc/synth-compound-flag";
        driver = "mysql";
      };
    };
}
