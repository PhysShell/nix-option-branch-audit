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
      description = "Default is a real expression, not a literal true/false -- its outcome under a Truthy predicate can't be statically classified. (c7: DefaultUnresolved)";
    };
    baz = mkOption {
      type = types.nullOr types.str;
      default = if builtins.pathExists /etc/synth-baz then null else "/run/other.sock";
      description = "Default is an if-expression, not a literal null -- classify_value() must call this Unknown, not 'definitely non-null' from a raw-text mismatch against the string \"null\". (c10: DefaultUnresolved, at the default side)";
    };
    qux = mkOption {
      type = types.bool;
      default = false;
      description = "A known (literal) default, paired with an unresolvable *test* value in c11 -- to isolate TestValueUnresolved from DefaultUnresolved.";
    };
  };

  config = mkIf true {
    systemd.services.synth.script = if cfg.foo != null then "use ${cfg.foo}" else "no socket";
    systemd.services.synth-bar.enable = mkIf cfg.bar true;
    systemd.services.synth-baz.script = if cfg.baz != null then "use ${cfg.baz}" else "no socket";
    systemd.services.synth-qux.enable = mkIf cfg.qux true;
  };
}
