{ ... }:
{
  # Case 4: per-instance anti-cross-contamination. nodeA and nodeB are
  # two SEPARATE, real machines that never share a Nix evaluation with
  # each other -- combining nodeB's createLocally=true with nodeA's
  # driver="mysql" into one synthetic environment would "prove" a
  # transition (false&&false=false -> true&&true=true) that no actual
  # Nix evaluation ever produced. Only nodeA explicitly assigns
  # database.driver, so only nodeA is considered for watch =
  # database.driver; nodeB's own createLocally=true must never leak into
  # nodeA's evaluation. Correct result: default(driver=sqlite,
  # createLocally=false[nodeA's own]) = false; test(driver=mysql,
  # createLocally=false[nodeA's own]) = false. Same -> OBA001, not PASS.
  nodes.nodeA =
    { ... }:
    {
      services.synthCompound.database = {
        driver = "mysql";
        createLocally = false;
      };
    };
  nodes.nodeB =
    { ... }:
    {
      services.synthCompound.database = {
        createLocally = true;
      };
    };
}
