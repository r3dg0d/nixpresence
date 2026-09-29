{ config, lib, pkgs, ... }:
let
  cfg = config.services.nixpresence;
in
{
  options.services.nixpresence = {
    enable = lib.mkEnableOption "nixpresence system package + optional user service hint";
    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.nixpresence or (throw "pkgs.nixpresence missing — overlay/flake package required");
      description = "nixpresence package";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
    # User unit is installed via XDG data; enable with:
    #   systemctl --user enable --now nixpresence.service
  };
}
