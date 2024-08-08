{
  nixConfig = {
    extra-trusted-substituters = [ "https://nix-community.cachix.org" ];
    extra-trusted-public-keys = [ "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs=" ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs@{ self, fenix, crane, flake-parts, advisory-db, ... }:
    flake-parts.lib.mkFlake { inherit self inputs; } ({ withSystem, ... }: {
      systems = [
        "x86_64-linux"
        "x86_64-darwin"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      perSystem = { lib, config, self', inputs', pkgs, system, ... }:
        let
          rustToolchain = fenix.packages.${system}.stable.toolchain;
          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

          # include .md and .json files for the build
          markdownFilter = path: _type: builtins.match ".*md$" path != null;
          jsonFilter = path: _type: builtins.match ".*json$" path != null;
          markdownOrJSONOrCargo = path: type:
          (markdownFilter path type) ||
          (jsonFilter path type) ||
          (craneLib.filterCargoSources path type);

          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;

          commonArgs = {
            src = lib.cleanSourceWith {
              src = ./.;
              filter = markdownOrJSONOrCargo;
              name = "source";
            };
            pname = "rs1090";
            version = version;

            nativeBuildInputs = with pkgs; [ pkg-config openssl python3 bzip2 ] ++
              lib.optionals pkgs.stdenv.isLinux [ clang mold ]
            ;
            buildInputs = [ ] ++ lib.optionals pkgs.stdenv.isDarwin
            [
              pkgs.libiconv
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ];

          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        {
          devShells.default = pkgs.mkShell {
            inputsFrom = builtins.attrValues self.checks;
            buildInputs = [ rustToolchain pkgs.pkg-config pkgs.openssl ];
            shellHook = if pkgs.stdenv.isDarwin then ''
              export NIX_LDFLAGS="-L${lib.makeLibraryPath [pkgs.libiconv]} $NIX_LDFLAGS"
              export NIX_LDFLAGS="-F${pkgs.darwin.apple_sdk.frameworks.SystemConfiguration}/Library/Frameworks -framework SystemConfiguration $NIX_LDFLAGS"
              export NIX_LDFLAGS="-F${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks -framework Security $NIX_LDFLAGS"
            '' else if pkgs.stdenv.isLinux then ''
              export RUSTFLAGS="-C linker=clang -C link-arg=-fuse-ld=${pkgs.mold}/bin/mold"
            ''
            else "";
          };

          packages =
            {
              default =  craneLib.buildPackage (commonArgs // {  # TODO  this does not build for now
                pname = "jet1090";
                cargoExtraFlags = "-p jet1090";
                meta.mainProgram = "jet1090";
                inherit cargoArtifacts;
              });

              # docs = pkgs.callPackage ./docs {};
            };

          checks =
            {
              fmt = craneLib.cargoFmt (commonArgs);
              audit = craneLib.cargoAudit (commonArgs // { inherit advisory-db; });
              rustdoc = craneLib.cargoDoc (commonArgs // { inherit cargoArtifacts; });

              clippy-check = craneLib.cargoClippy (commonArgs // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--all-features -- --deny warnings";
              });

              test-check = craneLib.cargoNextest (commonArgs // {
                inherit cargoArtifacts;
                partitions = 1;
                partitionType = "count";
              });
            }
            # build packages as part of the checks
            // (lib.mapAttrs' (key: value: lib.nameValuePair (key + "-package") value) self'.packages);

          formatter = pkgs.nixpkgs-fmt;
        };
    });
}
