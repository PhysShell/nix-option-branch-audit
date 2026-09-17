{ lib, config, ... }:
let
  cfg = config.services.synthSingleRefH2Only;
in
{
  options.services.synthSingleRefH2Only = {
    x = lib.mkOption {
      type = lib.types.bool;
      default = false;
    };
  };

  # `cfg.x == true` -- H1's `scan_predicates` only recognizes bare
  # selects (`Truthy`) and null comparisons (`NullEq`/`NullNeq`), never a
  # general `== <literal>` comparison, so this predicate is entirely
  # invisible to H1 (`discovered_predicates` is empty for this module).
  # H2's `lower_pred_chained` handles `BinOpKind::Equal` generically and
  # finds it: `Eq(Ref(["x"]), Literal(Bool(true)))`, with a SINGLE-entry
  # `refs` list (unlike the compound `h2-compound` module's two-ref
  # predicates, where a broken `refs` filter can still find the same
  # predicate through its OTHER ref). Isolates
  # `diagnostic_default_outcome`'s H2-only fallback lookup specifically.
  config = lib.mkIf true {
    systemd.services.synthSingleRefH2Only-x.enable = if cfg.x == true then "on" else "off";
  };
}
