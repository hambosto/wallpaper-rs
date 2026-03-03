{
  lib,
  pkg-config,
  rustPlatform,
  libxkbcommon,
}:
rustPlatform.buildRustPackage (final: {
  pname = "wallpaper-rs";
  version = "26.3.0";

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../src
      ../Cargo.lock
      ../Cargo.toml
    ];
  };

  buildInputs = [
    libxkbcommon
  ];

  nativeBuildInputs = [
    pkg-config
  ];

  doCheck = false;

  cargoLock.lockFile = ../Cargo.lock;

  meta = {
    description = "A lightweight, daemonless wallpaper tool for Wayland compositors, written in Rust.";
    homepage = "https://github.com/hambosto/wallpaper-rs";
    license = lib.licenses.mit;
    mainProgram = "wallpaper-rs";
    maintainers = [ ];
    platforms = lib.platforms.linux;
  };
})
