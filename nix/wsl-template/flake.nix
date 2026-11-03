{
  description = "NixOS-WSL distribution running Eggocado D2V's host services";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    nixos-wsl = {
      url = "github:nix-community/NixOS-WSL/release-26.05";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    eggocado-d2v.url = "github:otto-wretling/Eggocado-D2V";
  };

  outputs = { nixpkgs, nixos-wsl, eggocado-d2v, ... }: {
    nixosConfigurations.eggocado-d2v = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        nixos-wsl.nixosModules.default
        eggocado-d2v.nixosModules.wsl
        ./configuration.nix
      ];
    };
  };
}
