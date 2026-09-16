{ ... }:
{
  nodes.machine =
    { ... }:
    {
      # config's value is a function call, not a literal attrset --
      # anything could be hidden inside, including a value that would
      # flip services.synth.foo's predicate. Must be TestConfigUnresolved,
      # not a silent "not activated" OBA001.
      config = someModuleGeneratingFunction { };
    };
}
