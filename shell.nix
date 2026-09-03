{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = [
    pkgs.cargo
    pkgs.pkg-config
    pkgs.rustc
  ];

  buildInputs = [
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
}