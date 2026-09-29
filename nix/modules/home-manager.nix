{ config, lib, pkgs, ... }:
let
  cfg = config.services.nixpresence;
in
{
  options.services.nixpresence = {
    enable = lib.mkEnableOption "nixpresence user daemon";
    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.nixpresence or (throw "pkgs.nixpresence missing");
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];
    systemd.user.services.nixpresence = {
      Unit = {
        Description = "nixpresence VRChat chatbox + Discord presence";
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${cfg.package}/bin/nixpresence daemon";
        Restart = "on-failure";
        RestartSec = "3";
        NoNewPrivileges = true;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
