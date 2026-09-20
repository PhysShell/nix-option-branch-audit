{ config, lib, ... }:

let
  cfg = config.services.demo;
in
{
  options.services.demo = {
    enable = lib.mkEnableOption "demo";
  };

  config = lib.mkIf cfg.enable {
    environment.etc."demo.conf".text = "enabled";
  };
}
