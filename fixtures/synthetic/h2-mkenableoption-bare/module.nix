{ config, lib, pkgs, ... }:
let
  cfg = config.services.synthEnableBare;
in
{
  options.services.synthEnableBare = {
    enable = lib.mkEnableOption "synth bare probe";
  };

  config = lib.mkIf cfg.enable {
    environment.etc."synth-enable-bare-probe".text = "on";
  };
}
