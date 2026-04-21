{
  self,
  lib,
  pkg-config,
  rustPlatform,
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

  date = fmtDate (self.lastModifiedDate or "19700101");
  shortRev = self.shortRev or "dirty";
in
rustPlatform.buildRustPackage (final: {
  pname = "wallpaper-rs";
  version = "unstable-${date}-${shortRev}";

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
    description = "A minimal wallpaper daemon for Wayland, written in Rust.";
    homepage = "https://github.com/hambosto/wallpaper-rs";
    license = lib.licenses.mit;
    mainProgram = "wallpaper-rs";
    maintainers = [ ];
    platforms = lib.platforms.linux;
  };
})
