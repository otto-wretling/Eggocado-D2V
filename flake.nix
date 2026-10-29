{
  description = "Eggocado D2V: Document to Video";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  };

  outputs = { nixpkgs, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
    in
    {
      nixosModules.packages = { username }: import ./nixos/packages.nix { inherit nixpkgs username; };
      nixosModules.services = import ./nixos/services.nix;

      darwinModules.homebrew = import ./mac/homebrew.nix;
      darwinModules.services = import ./mac/services.nix;

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
