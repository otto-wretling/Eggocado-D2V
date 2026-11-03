{ lib, ... }:

{
  wsl.enable = true;
  wsl.defaultUser = "nixos";

  networking.hostName = "eggocado-d2v";

  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  nixpkgs.config.allowUnfreePredicate = package: builtins.elem (lib.getName package) [ "n8n" ];

  eggocado-d2v.projectRoot = throw "Set eggocado-d2v.projectRoot in /etc/nixos/configuration.nix to your Eggocado-D2V checkout's WSL path, for example /mnt/c/Users/<you>/Documents/GitHub/Eggocado-D2V.";

  eggocado-d2v.nvidia.computeCapability = null;

  system.stateVersion = "26.05";
}
