{ lib, config, ... }:
with lib;
let
  cfg = config.services.synthCompound;
  db = cfg.database;
  # The exact shape of davis's real mysqlLocal, isolated: a compound
  # predicate over TWO options, so a watched option's own value alone is
  # never sufficient -- the other operand matters too. H2's adversarial
  # cases 2-4 (absorbed watched change, symmetric watch, per-instance
  # anti-cross-contamination) all exercise this same module.
  p = db.createLocally && db.driver == "mysql";
in
{
  options.services.synthCompound = {
    database = {
      createLocally = mkOption {
        type = types.bool;
        default = true;
      };
      driver = mkOption {
        type = types.nullOr types.str;
        default = "sqlite";
      };
    };
  };

  config = mkIf true {
    systemd.services.synthCompound.script = if p then "mysql-mode" else "other-mode";
  };
}
