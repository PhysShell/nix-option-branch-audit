{ ... }:
{
  # Assigns foo to null -- genuinely the opposite predicate outcome from
  # the non-null default (default_outcome = true since default != null;
  # this assignment's outcome = false). This is the positive control for
  # c6a: proves the outcome-transition model actually fires PASS when a
  # real transition exists, not just "always OBA001 now".
  services.synth.foo = null;
}
