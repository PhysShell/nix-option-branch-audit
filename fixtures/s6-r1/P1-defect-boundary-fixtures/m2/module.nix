{ config, lib, pkgs, ... }:
let
  cfg = config.services.synthBoundary;
in
{
  options.services.synthBoundary = {
    enable = lib.mkEnableOption "boundary probe";
    outer = {
      inner = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "M2: flat dotted attrpath, NOT inside any submodule/mkOption wrapping at all";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.etc."synth-boundary-probe" = lib.mkIf cfg.outer.inner { text = "on"; };
  };
}
