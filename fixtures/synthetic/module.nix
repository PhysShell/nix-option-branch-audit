{ lib, config, ... }:
with lib;
let
  cfg = config.services.synth;
in
{
  options = {
    foo = mkOption {
      type = types.nullOr types.str;
      default = "/run/default.sock";
      description = "Deliberately a non-null default, to isolate the outcome-transition fix from the H1 review.";
    };
    bar = mkOption {
      type = types.bool;
      default = builtins.elem "x" [ "x" "y" ];
      description = "Default is a real expression, not a literal true/false -- its outcome under a Truthy predicate can't be statically classified.";
    };
  };

  config = mkIf true {
    systemd.services.synth.script = if cfg.foo != null then "use ${cfg.foo}" else "no socket";
    systemd.services.synth-bar.enable = mkIf cfg.bar true;
  };
}
