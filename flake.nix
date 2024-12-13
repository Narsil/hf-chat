{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    android-nixpkgs = {
      url = "github:tadfisher/android-nixpkgs/stable";

      # The main branch follows the "canary" channel of the Android SDK
      # repository. Use another android-nixpkgs branch to explicitly
      # track an SDK release channel.
      #
      # url = "github:tadfisher/android-nixpkgs/stable";
      # url = "github:tadfisher/android-nixpkgs/beta";
      # url = "github:tadfisher/android-nixpkgs/preview";
      # url = "github:tadfisher/android-nixpkgs/canary";

      # If you have nixpkgs as an input, this will replace the "nixpkgs" input
      # for the "android" flake.
      #
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      android-nixpkgs,
      ...
    }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
      ];

      android-sdk = forAllSystems (
        system:
        android-nixpkgs.sdk.${system} (
          sdkPkgs:
          with sdkPkgs;
          [
            # Useful packages for building and testing.
            build-tools-34-0-0
            cmdline-tools-latest
            emulator
            platform-tools
            platforms-android-34

            # Other useful packages for a development environment.
            ndk-26-1-10909125
            # skiaparser-3
            # sources-android-34
          ]
          ++ lib.optionals (system == "aarch64-darwin") [
            # system-images-android-34-google-apis-arm64-v8a
            # system-images-android-34-google-apis-playstore-arm64-v8a
          ]
          ++ lib.optionals (system == "x86_64-darwin" || system == "x86_64-linux") [
            # system-images-android-34-google-apis-x86-64
            # system-images-android-34-google-apis-playstore-x86-64
          ]
        )
      );
      inherit (nixpkgs) lib;
    in
    {
      devShells = forAllSystems (system: {
        default =
          with nixpkgs.legacyPackages.${system}.pkgs;
          mkShell {
            nativeBuildInputs = with darwin.apple_sdk.frameworks; [
              libiconv
            ];
            buildInputs =
              with pkgs;
              [
                rustup
                rust-analyzer
                android-sdk.${system}
                pkg-config
                trunk
                gradle
                jdk
                openssl
                glib
                glib-networking
                gtk3
                webkitgtk_4_1
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

            GIO_MODULE_DIR = "${pkgs.glib-networking}/lib/gio/modules/";
            XDG_DATA_DIRS = "${gsettings-desktop-schemas}/share/gsettings-schemas/${gsettings-desktop-schemas.name}:${gtk3}/share/gsettings-schemas/${gtk3.name}:$XDG_DATA_DIRS";
          };
      });
    };
}
