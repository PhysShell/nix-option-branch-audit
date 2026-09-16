{ pkgs, ... }:
import ./make-test-python.nix (
  { ... }:
  {
    # This is the actual, common shape of a real nixosTest file: an outer
    # lambda whose body is `import ./make-test-python.nix (<lambda>)`, and
    # the real test spec (nodes, testScript, ...) lives inside that
    # second argument. resolve_test_root() must unwrap through this to
    # find the real, provable opposite-outcome assignment below -- this
    # isn't a synthetic construct invented for this fixture, it's the
    # dominant convention across nixpkgs's nixos/tests (95 files use this
    # wrapper directly, per the H1.3 review).
    nodes.machine =
      { ... }:
      {
        services.synth.foo = null;
      };
  }
)
