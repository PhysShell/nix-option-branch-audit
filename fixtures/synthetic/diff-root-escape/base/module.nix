{ lib, ... }:
{
  options.services.synth.foo = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
  };
}
