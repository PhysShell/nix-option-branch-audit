{ ... }:
{
  # qux's default is a known literal (false), so gate 3 resolves cleanly --
  # unlike c7, this isolates the test-value side of the same fail-closed
  # bug. This assignment IS `true` at runtime (builtins.elem "x" [ "x" "y" ]
  # trivially holds) but is statically opaque to classify_value() (it's a
  # NODE_APPLY, not a literal). The old H1 code silently filtered this out
  # of `transitions` and fell through to OBA001 -- a false "no evidence"
  # on a value that may well be the actual opposite outcome. Must be
  # TestValueUnresolved, not OBA001 and not a guessed PASS.
  #
  # Wrapped in nodes.machine, see c6a's fixture for why.
  nodes.machine =
    { ... }:
    {
      services.synth.qux = builtins.elem "x" [ "x" "y" ];
    };
}
