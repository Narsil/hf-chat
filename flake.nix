{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
      ];
    in
    {
      devShells = forAllSystems (system: {
        default =
          with nixpkgs.legacyPackages.${system}.pkgs;
          mkShell {
            buildInputs =
              with pkgs;
              [
                rustup
                rust-analyzer
                pkg-config
                trunk
              ]
              ++ lib.optionals stdenv.isDarwin (
                with darwin.apple_sdk.frameworks;
                [
                  libiconv
                  SystemConfiguration
                  CoreServices
                  WebKit
                  lld
                ]
              );
          };
      });
    };
}
