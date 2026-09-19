{ ... }:
{
  name = "synth-enable-override";
  nodes.machine = {
    services.synthEnableOverride.enable = false;
  };
  testScript = "machine.succeed('true')";
}
