{ nodes, ... }:
{
  name = "g1demo";
  nodes.machine = {
    services.g1demo.enable = true;
    services.g1demo.settings.foo.bar = true;
    services.g1demo.settings.baz = true;
  };
}
