{ nixos }:
{ config, lib, pkgs, utils, ... }:

let
  shared = import ./shared.nix;
  cfg = config.eggocado-d2v;
  windowsDriveMountRoot = lib.removeSuffix "/" config.wsl.wslConf.automount.root;
  windowsSystemFolderInDrive = "Windows/System32";
  windowsPowerShellFolderInSystemFolder = "WindowsPowerShell/v1.0";
  windowsShellProgram = "cmd.exe";
  wslInteropDirectory = "/bin";
  runWithWindowsInteropPath = pkgs.writeShellApplication {
    name = "run-with-windows-interop-path";
    text = ''
      for windows_system_folder in ${windowsDriveMountRoot}/*/${windowsSystemFolderInDrive}; do
        if [ -x "$windows_system_folder/${windowsShellProgram}" ]; then
          PATH="$PATH:$windows_system_folder:$windows_system_folder/${windowsPowerShellFolderInSystemFolder}"
          break
        fi
      done
      export PATH="$PATH:${wslInteropDirectory}"
      exec "$@"
    '';
  };
  linuxUserServices = nixos.linuxUserServices {
    inherit lib pkgs cfg;
    userExecutableDirectories = [ "${cfg.homeDirectory}/.local/share/npm/bin" ];
    acceleratorLibraryDirectories = [ "/run/opengl-driver/lib" ];
  };
  windowsPowerShellCommand = "powershell.exe";
  windowsTaskKillCommand = "taskkill.exe";
  windowsTaskListCommand = "tasklist.exe";
  windowsOllamaServerProcess = "ollama.exe";
  windowsOllamaTrayProcess = "ollama app.exe";
  windowsOllamaControls = {
    start = [ ''${windowsPowerShellCommand} -NoProfile -Command "Start-Process -WindowStyle Hidden ollama -ArgumentList serve"'' ];
    stop = map
      (process: ''${windowsTaskKillCommand} /F /IM "${process}"'')
      [ windowsOllamaTrayProcess windowsOllamaServerProcess ];
    status = ''${windowsTaskListCommand} /FI "IMAGENAME eq ${windowsOllamaServerProcess}" | grep -qi "${windowsOllamaServerProcess}"'';
  };
  dockerDesktopIntegrationCommands = [ "chmod" "install" "mkdir" "mv" "rm" "stat" "test" ];
  dockerDesktopCli = "/usr/bin/docker";
  publicRegistryDockerConfiguration = pkgs.writeText "docker-config.json" "{}";
  dockerConfigurationStateDirectory = "eggocado-d2v-docker";
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

    environment.systemPackages = [
      (shared.d2vCommand {
        inherit lib pkgs;
        controlsFor = service:
          if service.key == "ollama" then windowsOllamaControls else nixos.systemdUserControls service;
      })
    ];

    systemd.user.services = lib.mapAttrs
      (unit: service: {
        inherit (service) description;
        environment = service.environment // lib.optionalAttrs (service.environment ? PATH) {
          PATH = lib.mkForce service.environment.PATH;
        };
        unitConfig.ConditionUser = cfg.user;
        serviceConfig = nixos.restartPolicy // {
          ExecStart = utils.escapeSystemdExecArgs (
            lib.optional (unit == nixos.userServiceUnits.n8n) (lib.getExe runWithWindowsInteropPath)
            ++ service.command
          );
          WorkingDirectory = service.workingDirectory;
        };
      })
      linuxUserServices.services // {
      ${nixos.userServiceUnits.dockerServices} = {
        description = "Eggocado D2V Docker Compose services on Docker Desktop";
        unitConfig = {
          ConditionUser = cfg.user;
          StartLimitIntervalSec = 0;
        };
        environment.DOCKER_CONFIG = "%S/${dockerConfigurationStateDirectory}";
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          StateDirectory = dockerConfigurationStateDirectory;
          ExecStartPre = "${pkgs.coreutils}/bin/install -m 644 ${publicRegistryDockerConfiguration} %S/${dockerConfigurationStateDirectory}/config.json";
          ExecStart = "${dockerDesktopCli} compose up --detach --force-recreate";
          ExecStop = "${dockerDesktopCli} compose stop";
          Restart = "on-failure";
          RestartSec = 5;
          WorkingDirectory = cfg.projectRoot;
        };
      };
    };
  };
}
