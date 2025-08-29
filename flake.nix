{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ 
            (import rust-overlay)
          ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          inherit (pkgs) lib;

          # Read rust version from rust-version file
          rustVersion = lib.strings.removeSuffix "\n" (builtins.readFile ./rust-version);

          rustToolchain = pkgs.rust-bin.stable.${rustVersion}.default.override {
            extensions = [ "rust-src" "clippy" ];
          };

          nativeBuildInputs = with pkgs; [ rustToolchain pkg-config clang ];
          buildInputs = with pkgs; [ openssl openssl.dev ];

        in
        {
          devShells.default = pkgs.mkShell {
            inherit buildInputs nativeBuildInputs;

            RUST_VERSION = rustVersion;
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.pkg-config}/lib/pkgconfig";
            OPENSSL_DIR = "${pkgs.openssl.dev}";
            OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
            OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";

            # Environment variables for integration tests
            BITCOIND_EXEC = "${pkgs.bitcoind}/bin/bitcoind";
            ELEMENTSD_EXEC = "${pkgs.elementsd}/bin/elementsd";

          };
        }
      );
} 