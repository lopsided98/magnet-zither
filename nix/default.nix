{ pkgs, defaultCrateOverrides, python3Packages, pkgconfig, libusb }: let 
  cargo = import ./Cargo.nix {
    inherit pkgs;
    defaultCrateOverrides = defaultCrateOverrides // {
      hidapi = attrs: {
        nativeBuildInputs = [ pkgconfig ];
        buildInputs = [ libusb ];
      };
    };
  };
in {
  cargo-hf2 = cargo.workspaceMembers.cargo-hf2.build;
  cmdebug = python3Packages.callPackage ./cmdebug.nix { };
}
