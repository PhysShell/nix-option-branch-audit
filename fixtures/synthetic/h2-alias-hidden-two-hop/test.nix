{ ... }:
{
  # driver: default "sqlite" -> test "mysql". `direct`'s own predicate
  # (driver != null) never transitions -- true either way. Without a
  # working 2-hop alias walk in `collect_reachable_refs`, `hidden`'s
  # unresolved condition would contribute NO refs at all, and this
  # target would wrongly resolve to OBA001. Correct result:
  # TestValueUnresolved.
  nodes.machine =
    { ... }:
    {
      services.synthTwoHop.database.driver = "mysql";
    };
}
