{ nixpkgs, username }:
{ pkgs, ... }:

let
  d2vPkgs = import nixpkgs {
    system = pkgs.stdenv.hostPlatform.system;
    config.allowUnfreePredicate = package: builtins.elem (pkgs.lib.getName package) [ "n8n" ];
  };
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
    d2vPkgs.rustc
    d2vPkgs.uv
    pkgs.n8n
  ];

  networking.firewall.trustedInterfaces = [ "docker0" "d2v-bridge" ];

  services.ollama = {
    enable = true;
    package = d2vPkgs.ollama;
  };

  nixpkgs.overlays = [
    (final: prev: {
      n8n = prev.n8n.overrideAttrs (old: {
        pnpmDeps = old.pnpmDeps.overrideAttrs (_: {
          outputHash = "sha256-kSv/8YxvwCFxrSRTqYHP+TAmDgu5OPbcwolHVIqCGDA=";
        });
      });
    })
  ];

  users.users.${username}.extraGroups = [ "docker" ];

  virtualisation.docker.enable = true;
}
