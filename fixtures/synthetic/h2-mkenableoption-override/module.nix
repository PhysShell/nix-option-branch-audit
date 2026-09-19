{ config, lib, pkgs, ... }:
let
  cfg = config.services.synthEnableOverride;
in
{
  options.services.synthEnableOverride = {
    enable = lib.mkEnableOption "synth probe" // {
      default = true;
      description = ''
        An mkEnableOption whose `//` override changes the default to
        true -- proves the override wins over the synthesized `false`,
        the same real shape libinput's own module uses (there with a
        cross-module `config.services.xserver.enable` default instead
        of a literal).
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    environment.etc."synth-enable-override-probe".text = "on";
  };
}
