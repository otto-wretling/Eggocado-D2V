{ username }:
{ pkgs, ... }:

let
  nixpkgsUnstable = import <nixpkgs-unstable> { system = pkgs.stdenv.hostPlatform.system; config.allowUnfree = true; };
in
{
  programs.nix-ld.enable = true;

  environment.systemPackages = [
    nixpkgsUnstable.antigravity-cli
    pkgs.cmake
    pkgs.espeak-ng
    pkgs.ffmpeg-full
    pkgs.gcc
    pkgs.gnumake
    pkgs.n8n
    pkgs.uv
  ];

  users.users.${username}.extraGroups = [ "docker" ];

  virtualisation.docker.enable = true;
}
