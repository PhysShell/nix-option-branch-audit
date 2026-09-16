{ lib, ... }:

{
  name = "kimai";
  meta.maintainers = with lib.maintainers; [ peat-psuwit ];

  containers.machine =
    { ... }:
    {
      services.kimai.sites."localhost" = {
        database.createLocally = true;
      };
    };

  # Adversarial mutation C5: right prefix (services.kimai.sites.*), a
  # sibling option under the same `database` submodule IS activated with a
  # non-default value (database.host), but database.socket itself is never
  # touched. The predicate under watch is database.socket specifically --
  # this must not borrow activation evidence from a neighboring option.
  containers.socketMachine =
    { ... }:
    {
      services.kimai.sites."localhost" = {
        database.createLocally = true;
        database.host = "127.0.0.1";
      };
      services.mysql.settings.mysqld.skip-networking = true;
    };

  testScript = ''
    start_all()

    machine.wait_for_unit("phpfpm-kimai-localhost.service")
    machine.wait_for_unit("nginx.service")
    machine.wait_for_open_port(80)
    machine.succeed("curl -v --location --fail http://localhost/")

    socketMachine.wait_for_unit("phpfpm-kimai-localhost.service")
  '';
}
