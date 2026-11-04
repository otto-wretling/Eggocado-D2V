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
  dockerDesktopCli = "/usr/bin/docker";
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

    services.ollama.enable = false;

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
      linuxUserServices.services // {
      d2v-docker-services = {
        description = "Eggocado D2V Docker Compose services on Docker Desktop";
        unitConfig = {
          ConditionUser = cfg.user;
          StartLimitIntervalSec = 0;
        };
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = "${dockerDesktopCli} compose up --detach --force-recreate";
          Restart = "on-failure";
          RestartSec = 5;
          WorkingDirectory = cfg.projectRoot;
        };
        wantedBy = [ "default.target" ];
      };
    };
  };
}
