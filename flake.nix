{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rust-version = "1.68.0";
      in {
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs;
            [
              (rust-bin.stable.${rust-version}.default.override {
                extensions = [ "rust-src" "clippy" "rustfmt" ];
              })
            ];
        };
      });
}
