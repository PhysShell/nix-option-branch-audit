{ ... }:
{
  name = "bisect";
  nodes.machine = {
    services.bisect.enable = true;
  };
  testScript = "machine.succeed('true')";
}
