{ ... }:
{
  # flag: default "sqlite" -> test "mysql". H1's own Truthy predicate on
  # `flag` can't classify its default at all (DefaultUnresolved, in
  # isolation) -- before the fix this short-circuited the whole option
  # and the separate, fully H2-resolvable `flag == "mysql"` predicate was
  # never even evaluated. Correct result: PASS, witnessed via the Eq
  # predicate, not a false DefaultUnresolved.
  nodes.machine =
    { ... }:
    {
      services.synthDefaultRescue.flag = "mysql";
    };
}
