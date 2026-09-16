{ ... }:
{
  # Assigns foo to the exact same value as its declared default. Under the
  # old textual-equality is_default_class() this counted as "non-default
  # evidence" (the value isn't the null literal) even though it produces
  # the *same* predicate outcome as leaving the option unset -- no branch
  # transition is actually proven. Must be OBA001, not PASS.
  services.synth.foo = "/run/default.sock";
}
