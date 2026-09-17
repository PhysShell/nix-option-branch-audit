{ ... }:
{
  # driver: default "sqlite" -> test "mysql". `direct`'s own predicate
  # (driver != null) is true either way -- no transition, no evidence
  # from that candidate alone. Without a fix, this would fall straight
  # through to a false OBA001. `hidden`'s condition is reachably tied to
  # database.driver too (through its alias binding, not its own syntax)
  # and its own lowering genuinely failed -- correct result:
  # TestValueUnresolved, not OBA001.
  nodes.machine =
    { ... }:
    {
      services.synthHidden.database.driver = "mysql";
    };
}
