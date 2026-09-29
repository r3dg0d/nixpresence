{ pkgs, lib, config, ... }:
let
  cfg = config.services.nixpresence;
  # Prefer flake input / callPackage from the package set; override src when packaging from a local tree.
  pkg = cfg.package;
in {
  options.services.nixpresence = {
    enable = lib.mkEnableOption "nixpresence (VRChat chatbox + Discord presence)";
    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.callPackage ../../packaging/nix/package.nix { };
      description = "nixpresence package";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ pkg ];

    systemd.user.services.nixpresence = {
      description = "nixpresence VRChat chatbox + Discord presence";
      after = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" ];
      partOf = [ "graphical-session.target" ];
      serviceConfig = {
        Type = "simple";
        ExecStart = "${pkg}/bin/nixpresence daemon";
        Restart = "on-failure";
        RestartSec = 3;
        NoNewPrivileges = true;
        Environment = [ "LD_LIBRARY_PATH=/run/opengl-driver/lib" ];
      };
    };
  };
}
