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
        freeformType = lib.types.attrsOf lib.types.str;
        options = {
          inner = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "M1: submodule WITH freeformType (matches real #568429/#508090 shape)";
          };
        };
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.etc."synth-boundary-probe" = lib.mkIf cfg.outer.inner { text = "on"; };
  };
}
