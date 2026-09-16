{ ... }:
{
  # An explicit, provable transition (default is non-null; this sets
  # null -- the opposite predicate outcome) lives in one node...
  nodes.machine =
    { ... }:
    {
      services.synth.foo = null;
    };

  # ...while a *different*, unrelated node has an import. Positive
  # evidence beats incompleteness found elsewhere in the same file: this
  # must be PASS, not TestConfigUnresolved. Existential opposite evidence
  # is a stronger claim than "some other opacity also exists somewhere".
  nodes.other =
    { ... }:
    {
      imports = [ ./unrelated.nix ];
    };
}
