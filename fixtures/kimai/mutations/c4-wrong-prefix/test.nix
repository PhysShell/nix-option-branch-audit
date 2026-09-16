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

  # Adversarial mutation C4: database.socket IS assigned a non-null value
  # somewhere in the test file, but under an unrelated option namespace
  # (services.notKimai, not services.kimai). A naive grep for
  # "database.socket = " would wrongly call this covered; a structurally
  # path-bound matcher must not.
  containers.socketMachine =
    { ... }:
    {
      services.notKimai.sites."localhost" = {
        database.createLocally = true;
        database.socket = "/run/mysqld/mysqld.sock";
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
