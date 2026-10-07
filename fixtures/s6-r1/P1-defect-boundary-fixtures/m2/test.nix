{ ... }:
{
  name = "synth-boundary";
  nodes.machine = {
    services.synthBoundary.enable = true;
  };
  testScript = "machine.succeed('true')";
}
