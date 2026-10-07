{ config, lib, pkgs, ... }:
let
  cfg = config.services.synthBoundary;
in
{
  options.services.synthBoundary = {
    enable = lib.mkEnableOption "boundary probe";
    outer = lib.mkOption {
      default = { };
      type = lib.types.submodule {
        options = {
          inner = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "nested leaf under test (M0: plain submodule, no freeformType)";
          };
        };
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.etc."synth-boundary-probe" = lib.mkIf cfg.outer.inner { text = "on"; };
  };
}
