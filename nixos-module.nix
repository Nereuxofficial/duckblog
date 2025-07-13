{
  pkgs,
  config,
  lib,
  ...
}:
let
  cfg = config.services.duckblog;
in
{
  options = {
    services.duckblog = {
      enable = lib.mkEnableOption "duckblog";
      package = lib.mkOption {
        description = "duckblog package to use";
        type = lib.types.package;
      };
    };
  };
  config = lib.mkIf cfg.enable {
    systemd.services.duckblog = {
      description = "duckblog at the user on the console";
      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${cfg.package}/bin/duckblog";
        StandardOutput = "journal+console";
      };
      wantedBy = [ "multi-user.target" ];
    };
  };
}
