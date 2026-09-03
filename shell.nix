{ pkgs ? import <nixpkgs> {} }:

let
  runtimeLibraries = [
    pkgs.alsa-lib
    pkgs.dbus
    pkgs.fontconfig
    pkgs.libGL
    pkgs.libxkbcommon
    pkgs.wayland
    pkgs.wayland-protocols
    pkgs.libx11
    pkgs.libxcursor
    pkgs.libxi
    pkgs.libxrandr
  ];
in pkgs.mkShell {

  nativeBuildInputs = [
    pkgs.cargo
    pkgs.pkg-config
    pkgs.rustc
  ];

  buildInputs = runtimeLibraries;

  shellHook = ''
    export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath runtimeLibraries}:$LD_LIBRARY_PATH"
  '';
}