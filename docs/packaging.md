# Packaging

Project lives at `~/Projects/nixpresence`. Nix expression:

```
packaging/nix/package.nix   # rustPlatform.buildRustPackage (MatrixShot pattern)
nix/modules/nixos.nix       # system module + optional user systemd
nix/modules/home-manager.nix
flake.nix                   # packages / apps / devShells / nixosModules / homeManagerModules
```

On zionsec (no Home Manager on live `/etc/nixos`), integrate via a **NixOS module** under
`/etc/nixos/packages/nixpresence/` that calls into this project's `packaging/nix/package.nix`
with `nixpresenceSrc = /home/neo/Projects/nixpresence;` — same pattern as MatrixShot /
desktop-tools. Wire-up into `/etc/nixos` is done by the parent agent later; the flake
already exports the modules.
