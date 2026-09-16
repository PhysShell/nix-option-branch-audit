{ ... }:
{
  # H1.3b review: a close relative of c25, one level down. The visible
  # instance here assigns the SAME value as the default -- no opposite
  # evidence the walker can see.
  nodes = {
    machine =
      { ... }:
      {
        services.synth.foo = "/run/default.sock";
      };

    # A multi-segment attrpath sitting directly under `nodes = { ... };`
    # is not a single-segment instance-name binding this walker
    # understands. Before this fix, the inner loop only handled
    # `inst_segs.len() == 1` and silently fell through for anything else
    # -- this genuinely live opposite-outcome assignment vanished with no
    # trace anywhere in the report, even though a normal sibling instance
    # (`machine`) was also present and correctly recognized. Must be
    # TestConfigUnresolved, not OBA001.
    hidden.services.synth.foo = null;
  };
}
