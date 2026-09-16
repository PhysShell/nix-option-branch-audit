{ ... }:
{
  # A perfectly ordinary, fully visible node -- no opposite-outcome
  # evidence in it (this assigns the SAME value as the default), so the
  # walker's own view of the world has nothing that would justify PASS.
  nodes.machine =
    { ... }:
    {
      services.synth.foo = "/run/default.sock";
    };

  # H1.3a review: a second, completely live test scenario sitting right
  # next to it, built through a helper this tool doesn't know, with its
  # own real opposite-outcome assignment inside. Before this fix,
  # `walk_test_spec_root` treated any key that wasn't `nodes`/`containers`
  # as harmless test-harness metadata (like `name` or `meta`) and silently
  # discarded it -- `found_any_instance` was already true from
  # `nodes.machine` above, so the whole-file fallback opacity never fired
  # either. The result was OBA001: the walker had *positive* evidence it
  # could see (foo == default) and no signal at all that anything else in
  # the file was invisible to it, even though `hiddenScenario` visibly
  # activates the exact opposite outcome. Must be TestConfigUnresolved.
  hiddenScenario = runTest {
    nodes.other =
      { ... }:
      {
        services.synth.foo = null;
      };
  };
}
