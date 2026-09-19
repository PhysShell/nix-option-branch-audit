{ config, lib, pkgs, ... }:
let
  cfg = config.services.bisect;
in
{
  options = {
    services.bisect = {
      enable = lib.mkEnableOption "bisect probe";

      nested = lib.mkOption {
        default = { };
        type = lib.types.submodule {
          options = {
            enable = lib.mkOption {
              type = lib.types.bool;
              default = true;
              description = "an unrelated nested enable, default TRUE (opposite of the top-level default)";
            };
          };
        };
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.etc."bisect-probe".text = "on";
  };
}
