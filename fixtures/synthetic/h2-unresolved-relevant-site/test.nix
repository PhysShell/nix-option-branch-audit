{ ... }:
{
  # `watched` is explicitly assigned but held at its own default value --
  # no transition, so H1's own predicate contributes a clean
  # `witnessed = Some(false)`, no `TestValueUnresolved` from that side.
  # Without H2.2 Finding 3, this target would resolve to OBA001 for
  # `watched`; the unresolved `someUnsupportedHelper cfg.watched` guard
  # elsewhere in config -- whose collected refs include `watched` itself
  # -- must still be enough to weaken that to TestValueUnresolved.
  nodes.machine =
    { ... }:
    {
      services.synthUnresolvedSite.watched = false;
    };
}
