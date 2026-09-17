{ ... }:
{
  # Case 5: unknown context. The watched option's own test assignment is
  # perfectly well known (driver = "mysql"), but the predicate's OTHER
  # operand (flag) can be obtained from neither this instance (never
  # assigned) nor a classifiable declared default (Unknown). Must be
  # inconclusive -- never silently treated as OBA001 (which would falsely
  # claim "no evidence, nothing to see") and never silently treated as
  # PASS (which would fabricate a transition this tool has no basis for).
  nodes.machine =
    { ... }:
    {
      services.synthUnknown.database = {
        driver = "mysql";
      };
    };
}
