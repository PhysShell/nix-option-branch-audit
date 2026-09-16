{ ... }:
{
  # The NESTED nodes form (`nodes = { machine = ...; };`), not the flat
  # form (`nodes.machine = ...;`) used everywhere else in this corpus.
  # Both are real, common nixosTest idioms -- 559 files under nixpkgs's
  # nixos/tests use `nodes = {` (H1.3 review). An explicit opposite-outcome
  # assignment here must still be found and produce PASS.
  nodes = {
    machine =
      { ... }:
      {
        services.synth.foo = null;
      };
  };
}
