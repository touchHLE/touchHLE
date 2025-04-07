{
  description = "A calculator program/website";

  outputs = { self, nixpkgs }:
    let
      systems =
        [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);

      nixpkgsFor = forAllSystems (system:
        import nixpkgs {
          inherit system;
          overlays = [ self.overlay ];
        });
    in rec {
      overlay = final: prev: {
        touchHLE = final.rustPlatform.buildRustPackage {
          pname = "touchHLE";
          version = "unstable";
          description = "High-level iOS emulator";

          src = self;

          # Runtime dependencies
          buildInputs = with final; [ 
            boost
            openal
            SDL2
            sndio
            
            wayland
            wayland-protocols
            wayland-scanner
            wayland-utils
            xorg.libX11
            xorg.libxcb
            xorg.libXdmcp
            xorg.libXext
            xorg.libXfixes
            xorg.libXrandr
          ];

          # Compile time dependencies
          nativeBuildInputs = with final; [ 
            cmake
            gcc
            pkg-config
          ];

          # Static linking breaks the graphical environment
          # Therefore, we're forcing a dynamic link here
          # See: https://github.com/touchHLE/touchHLE/blob/trunk/dev-docs/building.md#non-android-platforms
          buildNoDefaultFeatures = true;

          # Skip tests as they fail because they're not in a dev environment
          doCheck = false;

          outputs = [ "out" "lib" ];

          postInstall = ''
            moveToOutput "lib" "$lib"
          '';

          cargoDeps = final.rustPlatform.importCargoLock {
            lockFile = self + "/Cargo.lock";
            outputHashes = {
              "sdl2-0.37.0" = "sha256-zHra4VwC2ARQzXoRKhi/r2uvOrjpDvCqumnRnhpmlNs=";
            };
          };

          CARGO_FEATURE_USE_SYSTEM_LIBS = "1";
        };
      };

      packages =
        forAllSystems (system: { inherit (nixpkgsFor.${system}) touchHLE; });

      defaultPackage = forAllSystems (system: self.packages.${system}.touchHLE);

      apps = forAllSystems (system: {
        touchHLE = {
          type = "app";
          program = "${self.packages.${system}.touchHLE}/bin/touchHLE";
        };
      });

      defaultApp = forAllSystems (system: self.apps.${system}.touchHLE);

      devShell = forAllSystems (system:
        nixpkgs.legacyPackages.${system}.mkShell {
          inputsFrom = builtins.attrValues (packages.${system});
        });
    };
}

