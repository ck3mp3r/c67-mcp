{
  description = "Rust Context7 MCP Server with Devshell and Fenix";

  inputs = {
    nixpkgs.url = "github:NixOs/nixpkgs";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell.url = "github:numtide/devshell";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rustnix = {
      url = "github:ck3mp3r/flakes?dir=rustnix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    self,
    flake-parts,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["aarch64-darwin" "aarch64-linux" "x86_64-linux"];
      perSystem = {
        config,
        system,
        pkgs,
        ...
      }: let
        overlays = [
          inputs.fenix.overlays.default
          inputs.devshell.overlays.default
        ];
        pkgs = import inputs.nixpkgs {inherit system overlays;};

        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        cargoLock = {lockFile = ./Cargo.lock;};

        # Install data for pre-built releases
        installData = {
          aarch64-darwin = builtins.fromJSON (builtins.readFile ./data/aarch64-darwin.json);
          aarch64-linux = builtins.fromJSON (builtins.readFile ./data/aarch64-linux.json);
          x86_64-linux = builtins.fromJSON (builtins.readFile ./data/x86_64-linux.json);
        };

        # Build regular packages (no archives)
        regularPackages = inputs.rustnix.lib.rust.buildTargetOutputs {
          inherit
            cargoToml
            cargoLock
            overlays
            pkgs
            system
            installData
            ;
          fenix = inputs.fenix;
          nixpkgs = inputs.nixpkgs;
          src = ./.;
          packageName = "c67-mcp";
          archiveAndHash = false;
          supportedTargets = ["aarch64-darwin" "aarch64-linux" "x86_64-linux"];
        };

        # Build archive packages (creates archive with system name)
        archivePackages = inputs.rustnix.lib.rust.buildTargetOutputs {
          inherit
            cargoToml
            cargoLock
            overlays
            pkgs
            system
            installData
            ;
          fenix = inputs.fenix;
          nixpkgs = inputs.nixpkgs;
          src = ./.;
          packageName = "archive";
          archiveAndHash = true;
          supportedTargets = ["aarch64-darwin" "aarch64-linux" "x86_64-linux"];
        };
      in {
        apps = {
          default = {
            type = "app";
            program = "${config.packages.default}/bin/c67-mcp";
          };
        };

        packages =
          regularPackages
          // archivePackages;

        devShells = {
          default = pkgs.devshell.mkShell {
            packages = [inputs.fenix.packages.${system}.stable.toolchain];
            imports = [
              (pkgs.devshell.importTOML ./devshell.toml)
              "${inputs.devshell}/extra/git/hooks.nix"
            ];
          };
        };

        formatter = pkgs.alejandra;
      };

      flake = {
        overlays.default = final: prev: {
          c67-mcp = self.packages.default;
        };
      };
    };
}
