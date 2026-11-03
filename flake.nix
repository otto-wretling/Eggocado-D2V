{
  description = "Eggocado D2V: Document to Video";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    nixos-wsl = {
      url = "github:nix-community/NixOS-WSL/release-26.05";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    kokoro-fastapi = {
      url = "github:remsky/Kokoro-FastAPI/release";
      flake = false;
    };
    ollama-release = {
      url = "https://github.com/ollama/ollama/releases/latest/download/ollama-linux-amd64.tar.zst";
      flake = false;
    };
    pyproject-nix = {
      url = "github:pyproject-nix/pyproject.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    uv2nix = {
      url = "github:pyproject-nix/uv2nix";
      inputs.pyproject-nix.follows = "pyproject-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pyproject-build-systems = {
      url = "github:pyproject-nix/build-system-pkgs";
      inputs.pyproject-nix.follows = "pyproject-nix";
      inputs.uv2nix.follows = "uv2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, nixos-wsl, kokoro-fastapi, ollama-release, pyproject-nix, uv2nix, pyproject-build-systems, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forLinuxSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ];
      moduleFrom = nixpkgs.lib.setDefaultModuleLocation;
      nixos = import ./nix/nixos.nix { inherit nixpkgs pyproject-nix uv2nix pyproject-build-systems kokoro-fastapi; };
      kokoroFor = nixos.kokoroFor;
      mac = import ./nix/mac.nix;
      checkUser = "d2v-check";
      checkSystemBaseline = {
        nixpkgs.config.allowUnfreePredicate = package: builtins.elem (nixpkgs.lib.getName package) [ "n8n" ];
        system.stateVersion = "26.05";
      };
    in
    {
      nixosModules.packages = { username }: moduleFrom ./nix/nixos.nix (nixos.packages { inherit username; });
      nixosModules.services = moduleFrom ./nix/nixos.nix nixos.services;
      nixosModules.wsl = moduleFrom ./nix/wsl.nix (import ./nix/wsl.nix { inherit nixos ollama-release; });

      darwinModules.homebrew = moduleFrom ./nix/mac.nix mac.homebrew;
      darwinModules.services = moduleFrom ./nix/mac.nix mac.services;

      templates.wsl = {
        path = ./nix/wsl-template;
        description = "NixOS-WSL system flake running Eggocado D2V's host services";
      };

      packages = forLinuxSystems (system: {
        kokoro-fastapi-cpu = (kokoroFor system "cpu").environment;
        kokoro-fastapi-gpu = (kokoroFor system "gpu").environment;
        kokoro-model = (kokoroFor system "cpu").modelDirectory;
      });

      checks.x86_64-linux = {
        nixos-system-with-packages-module = (nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            (self.nixosModules.packages { username = checkUser; })
            checkSystemBaseline
            {
              boot.loader.grub.enable = false;
              fileSystems."/" = {
                device = "none";
                fsType = "tmpfs";
              };
              users.users.${checkUser}.isNormalUser = true;
            }
          ];
        }).config.system.build.toplevel;

        nixos-wsl-system-with-wsl-module = (nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            nixos-wsl.nixosModules.default
            self.nixosModules.wsl
            checkSystemBaseline
            {
              wsl.enable = true;
              wsl.defaultUser = checkUser;
            }
          ];
        }).config.system.build.toplevel;
      };

      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            packages = [ pkgs.cmake pkgs.espeak-ng pkgs.ffmpeg-full pkgs.gcc pkgs.gnumake pkgs.uv ];
          };
        });
    };
}
