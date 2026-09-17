{ lib, config, ... }:
let
  cfg = config.services.synthDefaultRescue;
in
{
  options.services.synthDefaultRescue = {
    # A plain string default -- H1's own `classify_value` calls this
    # `DefinitelyNonNull` (enough for a null-check predicate, NOT enough
    # for `Truthy`/`NegTruthy`, which need an actual `Bool`).  H2's finer
    # `classify_known_value` calls the SAME node `Exact(Str("sqlite"))`.
    flag = lib.mkOption {
      type = lib.types.str;
      default = "sqlite";
    };
  };

  config = lib.mkMerge [
    # H1's own unary scanner finds a `Truthy` predicate here (a bare
    # `cfg.flag` used directly as a condition) -- deliberately
    # type-invalid Nix on its own (a string can't really be used as a
    # raw boolean condition at real `nix eval`), built ONLY to pin the
    # DefaultUnresolved-vs-H2-rescue fix: `predicate_outcome(Truthy,
    # DefinitelyNonNull)` returns `None`, so H1's OWN predicate alone
    # contributes nothing.
    (lib.mkIf cfg.flag {
      systemd.services.synthDefaultRescue-truthy.enable = true;
    })
    # A SEPARATE, fully H2-resolvable `Eq`-based predicate on the exact
    # same option. `flag`'s default lowers to `KnownValue::Exact(Str(
    # "sqlite"))` regardless of H1's own inability to classify it as a
    # `Bool` -- this predicate CAN witness a transition even though H1's
    # own predicate above cannot resolve its default outcome at all.
    (lib.mkIf (cfg.flag == "mysql") {
      systemd.services.synthDefaultRescue-eq.enable = true;
    })
  ];
}
