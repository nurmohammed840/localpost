{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.dbus ];
}