{ ... }:
{
  name = "synth-nested-idiom";
  nodes.machine = {
    services.synthNestedIdiom.enable = true;
  };
  testScript = "machine.succeed('true')";
}
