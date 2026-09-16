{ ... }:
{
  # Assigns foo to the exact same value as its declared default. Under the
  # old textual-equality is_default_class() this counted as "non-default
  # evidence" (the value isn't the null literal) even though it produces
  # the *same* predicate outcome as leaving the option unset -- no branch
  # transition is actually proven. Must be OBA001, not PASS.
  #
  # Wrapped in nodes.machine like a real nixosTest node, not a bare
  # top-level assignment (H1.3 review: the walker's TestSpecRoot context
  # only recognizes nodes/containers as option-relevant; a bare top-level
  # assignment isn't how real nixosTest files are shaped and was only ever
  # a fixture-authoring shortcut, not something the tool should special-case).
  nodes.machine =
    { ... }:
    {
      services.synth.foo = "/run/default.sock";
    };
}
