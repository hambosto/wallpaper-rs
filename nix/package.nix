{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage (final: {
  pname = "wallpaper-rs";
  version = "v26.3.0";

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../src
      ../Cargo.lock
      ../Cargo.toml
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  meta = {
    description = "A very small, very simple, yet very wayland wallpaper setter written in rust.";
    homepage = "https://github.com/hambosto/wallpaper-rs";
    license = lib.licenses.mit;
    mainProgram = "wallpaper-rs";
    platforms = lib.platforms.unix;
  };
})
