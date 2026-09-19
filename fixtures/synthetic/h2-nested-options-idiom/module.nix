{ config, lib, pkgs, ... }:
let
  cfg = config.services.synthNestedIdiom;
in
{
  options = {
    services.synthNestedIdiom = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = ''
          A plain `mkOption` declared inside the nested `options = {
          services.X = { ... }; };` idiom -- proves GAP-2's fix alone,
          isolated from mkEnableOption (P1) and the nested-submodule
          collision (P0/GAP-4).
        '';
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.etc."synth-nested-idiom-probe".text = "on";
  };
}
