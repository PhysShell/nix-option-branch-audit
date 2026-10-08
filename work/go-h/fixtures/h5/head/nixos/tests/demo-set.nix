{ lib, ... }:
{
  name = "demo-set";
  nodes.machine = { ... }: {
    services.demo.enable = true;
    services.demo.openFirewall = true;
  };
  testScript = "";
}
