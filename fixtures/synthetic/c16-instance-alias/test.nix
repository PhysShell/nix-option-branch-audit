{ ... }:
let
  # A perfectly legitimate nixosTest pattern: factor a node's config out
  # into a local binding. Whatever this actually contains is irrelevant --
  # the point is `nodes.machine`'s value is a bare identifier, not a
  # literal attrset, so the walker (which doesn't resolve `let` bindings)
  # can never see into it regardless of what's inside.
  machineConfig = {
    services.synth.foo = "/run/default.sock";
  };
in
{
  nodes.machine = machineConfig;
}
