{ lib, config, ... }:
{
  options.services.g1broken = {
    enable = lib.mkEnableOption "g1broken";
  }
  # G1 kill-test: deliberately missing closing `;` / brace below, a
  # real parse error -- must still fail normally (parse_errors
  # populated / inconclusive), never silently treated as a valid new
  # module just because it's paired with an absent base side.
