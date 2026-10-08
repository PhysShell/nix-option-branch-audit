{ config, lib, pkgs, ... }:
let
  cfg = config.services.demo;
in
{
  options.services.demo = {
    enable = lib.mkEnableOption "demo";

    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      description = "Port demo listens on.";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open the firewall for the port.";
    };

    package = lib.mkPackageOption pkgs "demo" { };
  };

  config = lib.mkIf cfg.enable {
    networking.firewall.allowedTCPPorts = lib.optional cfg.openFirewall cfg.port;
    systemd.services.demo.serviceConfig.ExecStart = "${lib.getExe cfg.package} --port ${toString cfg.port}";
  };
}
