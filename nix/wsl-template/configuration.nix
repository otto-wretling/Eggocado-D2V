{ lib, ... }:

{
  wsl.enable = true;
  wsl.defaultUser = "nixos";

  networking.hostName = "eggocado-d2v";

  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  nixpkgs.config.allowUnfreePredicate = package: builtins.elem (lib.getName package) [ "n8n" ];

  system.stateVersion = "26.05";
}
