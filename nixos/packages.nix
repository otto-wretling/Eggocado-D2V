{ username }:
{ pkgs, ... }:

{
  programs.nix-ld.enable = true;

  environment.variables.NPM_CONFIG_PREFIX = "/home/${username}/.local/share/npm";

  environment.systemPackages = [
    pkgs.cargo
    pkgs.cmake
    pkgs.espeak-ng
    pkgs.ffmpeg-full
    pkgs.gcc
    pkgs.gnumake
    pkgs.n8n
    pkgs.rustc
    pkgs.uv
  ];

  networking.firewall.trustedInterfaces = [ "docker0" "d2v-bridge" ];

  users.users.${username}.extraGroups = [ "docker" ];

  virtualisation.docker.enable = true;
}
