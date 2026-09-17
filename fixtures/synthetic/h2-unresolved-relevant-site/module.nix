{ lib, config, ... }:
with lib;
let
  cfg = config.services.synthUnresolvedSite;
in
{
  options.services.synthUnresolvedSite = {
    watched = mkOption {
      type = types.bool;
      default = false;
      description = "The option under test: its OWN direct predicate never flips in test.nix, so absent any other consideration this target would clean-resolve to OBA001.";
    };
  };

  config = mkIf true {
    systemd.services.synthUnresolvedSite-watched.enable = mkIf cfg.watched true;
    # `someUnsupportedHelper cfg.watched` -- unlowerable as a whole (an
    # arbitrary helper call isn't a supported Pred shape), but its
    # ARGUMENT is a direct `cfg.watched` select, so `collect_reachable_refs`
    # finds `refs = [["watched"]]` for this site even though the
    # condition as a whole failed to lower. This is what makes the site
    # genuinely per-option relevant to `watched` specifically (not just
    # "some unrelated cfg-rooted site exists somewhere in the target"),
    # proving the gate isn't dead code under per-option matching.
    systemd.services.synthUnresolvedSite-extra.enable = mkIf (someUnsupportedHelper cfg.watched) true;
  };
}
