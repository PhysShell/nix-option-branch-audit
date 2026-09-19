{ ... }:
{
  name = "synth-enable-bare";
  nodes.machine = {
    services.synthEnableBare.enable = true;
  };
  testScript = "machine.succeed('true')";
}
