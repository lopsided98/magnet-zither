{ pkgs ? import <nixpkgs> {} }:
with pkgs;

mkShell {
  buildInputs = [
    rustup
    openocd
    pkgsCross.arm-embedded.buildPackages.gdb
  ] ++ (with pkgs.callPackages ./nix { }; [ 
    cargo-hf2
    cmdebug
  ]);
}
