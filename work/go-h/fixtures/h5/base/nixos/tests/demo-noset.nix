{ lib, ... }:
{
  name = "demo-noset";
  nodes.machine = { ... }: {
    services.demo.enable = true;
  };
  testScript = "";
}
