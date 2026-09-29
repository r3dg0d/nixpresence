{ lib
, rustPlatform
, pkg-config
, dbus
, nixpresenceSrc ? null
}:
let
  version = "0.1.0";
  srcRoot =
    if nixpresenceSrc != null then nixpresenceSrc
    else ./../..;
in
rustPlatform.buildRustPackage {
  pname = "nixpresence";
  inherit version;
  src = lib.cleanSourceWith {
    src = srcRoot;
    filter = path: type:
      let base = baseNameOf path; in
      !(lib.hasInfix "/target/" path)
      && !(lib.hasInfix "/.git/" path)
      && base != "target"
      && base != ".git"
      && base != "result";
  };
  cargoLock.lockFile = srcRoot + "/Cargo.lock";
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ dbus ];
  # NVML is dlopen'd at runtime from /run/opengl-driver/lib on NixOS.
  doCheck = true;
  meta = {
    description = "NixOS VRChat chatbox + Discord presence daemon";
    license = lib.licenses.mit;
    mainProgram = "nixpresence";
    platforms = lib.platforms.linux;
  };
}
