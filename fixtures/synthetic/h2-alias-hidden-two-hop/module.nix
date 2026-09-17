{ lib, config, ... }:
let
  cfg = config.services.synthTwoHop;

  # `direct`'s own predicate never transitions -- see test.nix.
  direct = cfg.database.driver != null;

  # TWO hops of aliasing between the condition site and the cfg-rooted
  # reference: `hidden` -> `mid` -> `someUnsupportedHelper cfg.database.driver`.
  # `collect_reachable_refs`'s own alias-following cycle-guard
  # (`!chain.iter().any(|n| n == &name)`) must correctly let resolution
  # continue past the SECOND hop, not just the first -- a mutation
  # testing survivor found this specific depth wasn't exercised by the
  # single-hop `h2-alias-hidden-unresolved` fixture (chain stays empty
  # for that one's own alias-following step, so a broken guard there
  # happened to coincide with the correct answer).
  mid = someUnsupportedHelper cfg.database.driver;
  hidden = mid;
in
{
  options.services.synthTwoHop = {
    database = {
      driver = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = "sqlite";
      };
    };
  };

  config = lib.mkMerge [
    (lib.mkIf direct {
      systemd.services.synthTwoHop-direct.enable = true;
    })
    (lib.mkIf hidden {
      systemd.services.synthTwoHop-hidden.enable = true;
    })
  ];
}
