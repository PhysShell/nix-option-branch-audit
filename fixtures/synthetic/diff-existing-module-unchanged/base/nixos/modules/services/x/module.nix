{ lib, config, ... }:
let
  cfg = config.services.g1demo;
in
{
  options.services.g1demo = {
    enable = lib.mkEnableOption "g1demo";
    settings = lib.mkOption {
      type = lib.types.submodule {
        options = {
          foo.bar = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "G1 kill-test: option nested inside another option's own submodule block, in a brand-new module file.";
          };
          baz = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "G1 kill-test: a second, sibling option in the same new nested block.";
          };
        };
      };
      default = { };
      description = "settings";
    };
  };
  config = lib.mkIf cfg.enable {
    environment.etc."g1demo.conf".text =
      "${lib.boolToString cfg.settings.foo.bar} ${lib.boolToString cfg.settings.baz}";
  };
}
