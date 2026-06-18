{
  mkShell,
  wallpaper-rs,
}:
mkShell {
  inputsFrom = [ wallpaper-rs ];
}
