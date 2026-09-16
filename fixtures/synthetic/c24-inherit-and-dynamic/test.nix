{ ... }:
{
  nodes.machine =
    { ... }:
    {
      # Two constructs `walk_test_block` used to silently `continue`
      # past entirely, with neither a TestAssignment nor an Opacity
      # recorded: a keyed `inherit` (pulling bindings from an outer
      # scope this walker can't trace) and a dynamic ("${...}") attribute
      # name (can't be statically resolved to an option path at all).
      # Neither one touches services.synth.foo directly, but both must
      # still register as opacity -- the walker genuinely doesn't know
      # what either one might set, and staying silent about that is
      # exactly the "silent absence" failure mode this whole review round
      # targets. Must be TestConfigUnresolved, not OBA001.
      inherit (someOuterScope) unrelatedThing;
      services.${dynamicServiceName}.enabled = true;
    };
}
