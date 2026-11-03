{ lib, ... }:

{
  wsl.enable = true;
  wsl.defaultUser = "nixos";

  networking.hostName = "eggocado-d2v";

  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  nixpkgs.config.allowUnfreePredicate = package: builtins.elem (lib.getName package) [ "n8n" ];

  eggocado-d2v.nvidia.computeCapability = null;

  system.stateVersion = "26.05";
}
