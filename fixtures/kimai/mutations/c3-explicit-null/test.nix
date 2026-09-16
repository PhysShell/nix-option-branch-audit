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

  # Adversarial mutation C3: assigns database.socket, but to null -- the
  # option's own default. A naive "was this key ever assigned" grep would
  # call this covered; it must not count as non-default activation evidence.
  containers.socketMachine =
    { ... }:
    {
      services.kimai.sites."localhost" = {
        database.createLocally = true;
        database.socket = null;
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
