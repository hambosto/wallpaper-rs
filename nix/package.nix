{
  self,
  lib,
  pkg-config,
  rustPlatform,
  rust-jemalloc-sys,
  libxkbcommon,
}:
let
  fmtDate =
    raw:
    let
      year = builtins.substring 0 4 raw;
      month = builtins.substring 4 2 raw;
      day = builtins.substring 6 2 raw;
    in
    "${year}-${month}-${day}";
in
rustPlatform.buildRustPackage (final: {
  pname = "wallpaper-rs";
  version = "unstable-${fmtDate self.lastModifiedDate}-${self.shortRev or "dirty"}";

  src = lib.cleanSourceWith {
    filter =
      name: _:
      let
        baseName = baseNameOf (toString name);
      in
      !(lib.hasSuffix ".nix" baseName);
    src = lib.cleanSource ../.;
  };

  cargoLock.lockFile = ../Cargo.lock;

  doCheck = false;

  buildInputs = [
    libxkbcommon
    rust-jemalloc-sys
  ];

  nativeBuildInputs = [
    pkg-config
  ];

  WALLPAPER_BUILD_VERSION = "unstable ${fmtDate self.lastModifiedDate} (commit ${self.rev or "dirty"})";

  meta = {
    description = "A minimal wallpaper daemon for Wayland, written in Rust.";
    homepage = "https://github.com/hambosto/wallpaper-rs";
    license = lib.licenses.mit;
    mainProgram = "wallpaper-rs";
    platforms = lib.platforms.linux;
  };
})
