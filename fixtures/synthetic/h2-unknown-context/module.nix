{ lib, config, ... }:
with lib;
let
  cfg = config.services.synthUnknown;
  db = cfg.database;
  # `flag`'s default is deliberately a non-literal expression (Unknown
  # ValueClass, same construct as the H1-era `bar`/`baz` options in
  # fixtures/synthetic/module.nix) -- statically unclassifiable, and never
  # touched by the test file either, so no environment can ever supply a
  # KnownValue for it.
  p = db.flag && db.driver == "mysql";
in
{
  options.services.synthUnknown = {
    database = {
      flag = mkOption {
        type = types.bool;
        default = builtins.pathExists /etc/synth-unknown-flag;
      };
      driver = mkOption {
        type = types.nullOr types.str;
        default = "sqlite";
      };
    };
  };

  config = mkIf true {
    systemd.services.synthUnknown.script = if p then "mysql-mode" else "other-mode";
  };
}
