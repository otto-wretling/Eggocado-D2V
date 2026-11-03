{ nixos }:
{ config, lib, pkgs, utils, ... }:

let
  cfg = config.eggocado-d2v;
  linuxUserServices = nixos.linuxUserServices {
    inherit lib pkgs cfg;
    userExecutableDirectories = [ "${cfg.homeDirectory}/.local/share/npm/bin" ];
    acceleratorLibraryDirectories = [ "/run/opengl-driver/lib" ];
  };
  dockerDesktopIntegrationCommands = [ "chmod" "install" "mkdir" "mv" "rm" "stat" "test" ];

  ollamaRelease = pkgs.stdenv.mkDerivation rec {
    pname = "ollama";
    version = "0.40.14";
    src = pkgs.fetchurl {
      url = "https://github.com/ollama/ollama/releases/download/v${version}/ollama-linux-amd64.tar.zst";
      sha256 = "993e83d08c3d13d17c55009a3e2664c8a25445e644aec3a68bef5abb6abce67d";
    };
    nativeBuildInputs = [ pkgs.zstd pkgs.autoPatchelfHook ];
    buildInputs = [ pkgs.stdenv.cc.cc.lib ];
    autoPatchelfIgnoreMissingDeps = true;
    appendRunpaths = [ "$ORIGIN" "/run/opengl-driver/lib" ];
    sourceRoot = ".";
    unpackPhase = "tar --zstd -xf $src";
    installPhase = ''
      mkdir -p $out
      cp -r bin lib $out/
    '';
    meta.mainProgram = "ollama";
  };
in
{
  imports = [ nixos.linuxHost ];

  config = {
    assertions = [
      {
        assertion = !config.virtualisation.docker.enable;
        message = "Eggocado D2V on NixOS-WSL runs its containers on Docker Desktop through wsl.docker-desktop. Set virtualisation.docker.enable = false.";
      }
    ];

    eggocado-d2v.user = lib.mkDefault config.wsl.defaultUser;

    eggocado-d2v.ollamaPackage = lib.mkDefault ollamaRelease;

    services.ollama.environmentVariables.LD_LIBRARY_PATH = "/run/opengl-driver/lib";

    eggocado-d2v.kokoro.torchVariant = lib.mkDefault "gpu";

    wsl = {
      docker-desktop.enable = true;
      useWindowsDriver = true;
      extraBin = map
        (command: { src = "${pkgs.coreutils}/bin/${command}"; })
        dockerDesktopIntegrationCommands;
    };

    networking.firewall.allowedTCPPorts = with linuxUserServices.ports; [ n8n acceleratedKokoro cpuKokoro ];

    users.users.${cfg.user}.linger = true;

    systemd.user.services = lib.mapAttrs
      (_: service: {
        inherit (service) description;
        environment = service.environment // lib.optionalAttrs (service.environment ? PATH) {
          PATH = lib.mkForce service.environment.PATH;
        };
        unitConfig.ConditionUser = cfg.user;
        serviceConfig = {
          ExecStart = utils.escapeSystemdExecArgs service.command;
          Restart = "on-failure";
          RestartSec = 3;
          WorkingDirectory = service.workingDirectory;
        };
        wantedBy = lib.optionals service.startsWithSession [ "default.target" ];
      })
      linuxUserServices.services;
  };
}
