{ config, lib, ... }:

let
  cfg = config.services.demo;
in
{
  options.services.demo = {
    enable = lib.mkEnableOption "demo";
    # S3-F3's own real reproducer shape: a brand-new option, born in
    # this exact diff, inside an already-existing module (not a whole
    # new module file -- module.nix already existed on both sides, only
    # this option is new). Real #516128 (tinyauth): `enableUnixSocket`
    # is added the same way, in an otherwise-untouched module.
    newOption = lib.mkEnableOption "new option, born in this diff";
  };

  config = lib.mkIf cfg.enable {
    environment.etc."demo.conf".text = "enabled";
    systemd.services.demo.path = lib.mkIf cfg.newOption [ "/run/wrappers" ];
  };
}
