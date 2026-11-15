let
  absolutePath = lib: lib.types.strMatching "/.*";

  unfreePackageNames = [ "n8n" ];

  isAllowedUnfree = lib: package: builtins.elem (lib.getName package) unfreePackageNames;

  hostToolNames = [ "espeak-ng" "ffmpeg-full" "uv" ];

  restartDelaySeconds = 3;

  repositoriesFolderName = "Documents/GitHub";

  projectFolderName = "Eggocado-D2V";

  bareMetalN8nFolderName = ".n8n-d2v-baremetal";

  n8nRestrictedFileAccessDirectory = "/tmp";

  kokoroListenAddress = "0.0.0.0";

  ports = {
    mainN8n = 5678;
    n8n = 5679;
    n8nTaskRunnerBroker = 5680;
    acceleratedKokoro = 8881;
    cpuKokoro = 8882;
  };

  d2vServices = [
    { key = "kokoro"; name = "Kokoro-FastAPI"; stopsWith = [ "cpuKokoro" ]; }
    { key = "n8n"; name = "Bare-metal n8n"; stopsWith = [ ]; }
    { key = "dockerServices"; name = "Docker services"; stopsWith = [ ]; }
  ];

  options = { config, lib, ... }:
    let
      cfg = config.eggocado-d2v;
    in
    {
      options.eggocado-d2v = {
        user = lib.mkOption {
          type = lib.types.str;
          description = "Account that runs Eggocado D2V's host services.";
        };

        homeDirectory = lib.mkOption {
          type = absolutePath lib;
          description = "Home directory of the account in eggocado-d2v.user.";
        };

        projectRoot = lib.mkOption {
          type = absolutePath lib;
          default = "${cfg.homeDirectory}/${repositoriesFolderName}/${projectFolderName}";
          defaultText = lib.literalExpression ''"''${config.eggocado-d2v.homeDirectory}/${repositoriesFolderName}/${projectFolderName}"'';
          description = "Eggocado D2V checkout holding the temp/ and output/ working folders.";
        };

        acceleration = {
          kokoroDevice = lib.mkOption {
            type = lib.types.nullOr (lib.types.enum [ "cuda" "mps" ]);
            default = null;
            description = "Device the accelerated Kokoro instance requests. Null selects Kokoro's detection order: MPS, then CUDA, then CPU.";
          };
        };
      };
    };

  services = { lib, cfg, kokoroPaths }:
    let
      kokoroInstance = { port, useGpu }: {
        environment = {
          MODEL_DIR = kokoroPaths.modelDirectory;
          PORT = toString port;
          PYTHONPATH = "${kokoroPaths.source}:${kokoroPaths.source}/api";
          USE_GPU = lib.boolToString useGpu;
          VOICES_DIR = kokoroPaths.voicesDirectory;
          WEB_PLAYER_PATH = "${kokoroPaths.source}/web";
        }
        // lib.optionalAttrs (useGpu && cfg.acceleration.kokoroDevice != null) {
          DEVICE_TYPE = cfg.acceleration.kokoroDevice;
        };
        uvicornArguments = [ "api.src.main:app" "--host" kokoroListenAddress "--port" (toString port) ];
      };
    in
    {
      inherit ports;

      n8nEnvironment = { startCpuTtsCommand, stopCpuTtsCommand }: {
        D2V_START_CPU_TTS_COMMAND = startCpuTtsCommand;
        D2V_STOP_CPU_TTS_COMMAND = stopCpuTtsCommand;
        N8N_PORT = toString ports.n8n;
        N8N_USER_FOLDER = "${cfg.homeDirectory}/${bareMetalN8nFolderName}";
      };

      n8nCommandExecutionEnvironment = {
        N8N_ENABLE_EXECUTE_COMMAND = "true";
        N8N_RESTRICT_FILE_ACCESS_TO = n8nRestrictedFileAccessDirectory;
        N8N_RUNNERS_BROKER_PORT = toString ports.n8nTaskRunnerBroker;
        NODES_EXCLUDE = "[]";
      };

      acceleratedKokoro = kokoroInstance { port = ports.acceleratedKokoro; useGpu = true; };
      cpuKokoro = kokoroInstance { port = ports.cpuKokoro; useGpu = false; };
    };

  dockerComposeControls = { dockerCommand, projectRoot }:
    let
      compose = "${dockerCommand} compose --project-directory ${projectRoot}";
    in
    {
      start = [ "${compose} up --detach" ];
      stop = [ "${compose} stop" ];
      status = "${compose} ps --status running --quiet | grep -q .";
    };

  d2vCommand = { lib, pkgs, controlsFor }:
    let
      usage = "Usage: d2v start|stop|status";
      services = map (service: { inherit (service) name; } // controlsFor service) d2vServices;
      eachService = render: lib.concatMapStringsSep "\n" render;
      startAll = eachService
        (service: ''
          echo "Starting ${service.name}"
          ${lib.concatStringsSep "\n" service.start}
        '')
        services;
      stopAll = eachService
        (service: ''
          echo "Stopping ${service.name}"
          ${lib.concatMapStringsSep "\n" (command: "${command} || true") service.stop}
        '')
        (lib.reverseList services);
      reportAll = eachService
        (service: ''
          if ${service.status}; then
            echo "${service.name}: running"
          else
            echo "${service.name}: stopped"
          fi
        '')
        services;
    in
    pkgs.writeShellApplication {
      name = "d2v";
      text = ''
        case "''${1:-}" in
          start)
            ${startAll}
            ;;
          stop)
            ${stopAll}
            ;;
          status)
            ${reportAll}
            ;;
          *)
            echo "${usage}" >&2
            exit 1
            ;;
        esac
      '';
    };
in
{
  inherit absolutePath d2vCommand dockerComposeControls hostToolNames isAllowedUnfree options repositoriesFolderName restartDelaySeconds services;
}
