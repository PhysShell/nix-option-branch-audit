{ lib, config, ... }:
let
  cfg = config.services.synthHidden;

  # `direct`'s own predicate is fully lowerable, but its default (sqlite
  # != null -> true) and test value (mysql != null -> true) agree -- no
  # transition, so on its own this would be honest evidence-no-transition,
  # not TestValueUnresolved.
  direct = cfg.database.driver != null;

  # `hidden`'s condition SITE is just the bare identifier `hidden` --
  # its own syntax never mentions `cfg` at all. Only `hidden`'s BINDING,
  # reached by resolving the alias, references `cfg.database.driver`,
  # wrapped in a helper call this IR can't represent (`someUnsupportedHelper`
  # isn't null/true/false/a string/a select). This is the exact shape
  # H2.2 Finding 3's first fix (a purely syntactic "does the condition's
  # own text contain `cfg_ident`" check) missed: it would have read
  # `hidden`'s failed condition site and found no literal "cfg" anywhere
  # in it, wrongly treating this as irrelevant to `database.driver`.
  hidden = someUnsupportedHelper cfg.database.driver;
in
{
  options.services.synthHidden = {
    database = {
      driver = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = "sqlite";
      };
    };
  };

  config = lib.mkMerge [
    (lib.mkIf direct {
      systemd.services.synthHidden-direct.enable = true;
    })
    (lib.mkIf hidden {
      systemd.services.synthHidden-hidden.enable = true;
    })
  ];
}
