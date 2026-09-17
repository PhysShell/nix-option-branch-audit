{ lib, config, ... }:
let
  cfg = config.services.synthScopeShadow;
in
{
  options.services.synthScopeShadow = {
    flag = lib.mkOption {
      type = lib.types.bool;
      default = false;
    };
  };

  # A LOCALLY shadowed `cfg`, unrelated to the module's own top-level
  # `cfg = config.services.synthScopeShadow;` above. H1's `scan_predicates`
  # is purely spelling-based (`root_name == cfg_ident`) and finds a
  # `Truthy` predicate on `flag` here regardless of which `cfg` this
  # actually is. H2's `resolve_cfg_root` scope check (exact, lexical,
  # used whenever `option_prefix` is concrete) correctly resolves THIS
  # occurrence to `someUnrelatedScope.thing` -- not
  # `config.services.synthScopeShadow` -- and excludes it from
  # `resolved_predicates` entirely (a confident negative, not an
  # unresolved site, since it lowers fine, it just isn't about this
  # option_prefix). This predicate is visible to H1's gate 4 ALONE: no
  # H2 candidate exists for `flag` in this module at all, isolating
  # whether H1's own opposite-outcome detection is actually still doing
  # anything post-H2-unification, rather than being silently shadowed
  # by H2 rediscovering the same select on every other existing fixture.
  config = lib.mkIf true {
    systemd.services.synthScopeShadow-flag.enable =
      let
        cfg = someUnrelatedScope.thing;
      in
      if cfg.flag then "on" else "off";
  };
}
