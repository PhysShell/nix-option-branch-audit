{ ... }:
{
  # Cases 2 and 3 share this single-instance scenario, watching different
  # options from the SAME compound predicate `p = createLocally &&
  # driver == "mysql"`:
  #
  #   case 2 (watch = database.driver): default(driver=sqlite,
  #   createLocally=false[this instance]) = false; test(driver=mysql,
  #   createLocally=false) = false. Same -> OBA001, NOT PASS -- the
  #   watched option's own change is *absorbed* by the other operand
  #   already being false. Proves presence of `driver = "mysql"` alone
  #   isn't treated as sufficient evidence.
  #
  #   case 3 (watch = database.createLocally): default(createLocally=true,
  #   driver=mysql[this instance]) = true; test(createLocally=false,
  #   driver=mysql) = false. true -> false -> PASS -- proves the system
  #   correctly attributes causation to whichever option is actually
  #   being watched, not just "the compound predicate was true somewhere".
  nodes.machine =
    { ... }:
    {
      services.synthCompound.database = {
        createLocally = false;
        driver = "mysql";
      };
    };
}
