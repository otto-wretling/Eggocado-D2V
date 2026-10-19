{ username }:
{ pkgs, ... }:

{
  programs.nix-ld.enable = true;

  environment.systemPackages = [
    pkgs.cmake
    pkgs.espeak-ng
    pkgs.ffmpeg-full
    pkgs.gcc
    pkgs.gnumake
    pkgs.n8n
    pkgs.uv
  ];

  networking.firewall.trustedInterfaces = [ "docker0" ];

  users.users.${username}.extraGroups = [ "docker" ];

  virtualisation.docker.enable = true;
}
