{ lib, config, ... }:
with lib;
{
  options = {
    foo = mkOption {
      type = types.nullOr types.str;
      default = "/run/default.sock";
    ;
  };
}
