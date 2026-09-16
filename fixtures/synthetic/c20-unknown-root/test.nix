{ ... }:
someHelperFunctionThisToolDoesNotKnow {
  # The whole test spec is wrapped in an unrecognized function call -- not
  # a literal attrset, not the known `import ./make-test-python.nix (...)`
  # wrapper. resolve_test_root() must not guess at this. The scanner
  # genuinely can't see past this wrapper at all, regardless of what's
  # inside it (even a real opposite-outcome assignment, as here) -- the
  # correct result is TestConfigUnresolved (root-scope opacity), never a
  # false OBA001 claiming the option was never touched.
  nodes.machine =
    { ... }:
    {
      services.synth.foo = null;
    };
}
