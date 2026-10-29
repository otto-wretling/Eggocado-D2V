{ nixpkgs, username }:
{ pkgs, ... }:

let
  d2vPkgs = nixpkgs.legacyPackages.${pkgs.stdenv.hostPlatform.system};
in

{
  programs.nix-ld.enable = true;

  environment.variables.NPM_CONFIG_PREFIX = "/home/${username}/.local/share/npm";

  environment.systemPackages = [
    d2vPkgs.cargo
    d2vPkgs.cmake
    d2vPkgs.espeak-ng
    d2vPkgs.ffmpeg-full
    d2vPkgs.gcc
    d2vPkgs.gnumake
    d2vPkgs.n8n
    d2vPkgs.rustc
    d2vPkgs.uv
  ];

  networking.firewall.trustedInterfaces = [ "docker0" "d2v-bridge" ];

  users.users.${username}.extraGroups = [ "docker" ];

  virtualisation.docker.enable = true;
}
