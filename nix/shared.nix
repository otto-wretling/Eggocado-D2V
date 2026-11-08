let
  options = { config, lib, ... }:
    let
      cfg = config.eggocado-d2v;
      absolutePath = lib.types.strMatching "/.*";
    in
    {
      options.eggocado-d2v = {
        user = lib.mkOption {
          type = lib.types.str;
          description = "Account that runs Eggocado D2V's host services.";
        };

        homeDirectory = lib.mkOption {
          type = absolutePath;
          description = "Home directory of the account in eggocado-d2v.user.";
        };

        projectRoot = lib.mkOption {
          type = absolutePath;
          default = "${cfg.homeDirectory}/Documents/GitHub/Eggocado-D2V";
          defaultText = lib.literalExpression ''"''${config.eggocado-d2v.homeDirectory}/Documents/GitHub/Eggocado-D2V"'';
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
      ports = {
        n8n = 5679;
        n8nTaskRunnerBroker = 5680;
        acceleratedKokoro = 8881;
        cpuKokoro = 8882;
      };

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
        uvicornArguments = [ "api.src.main:app" "--host" "0.0.0.0" "--port" (toString port) ];
      };
    in
    {
      inherit ports;

      n8nEnvironment = { startCpuTtsCommand, stopCpuTtsCommand }: {
        D2V_START_CPU_TTS_COMMAND = startCpuTtsCommand;
        D2V_STOP_CPU_TTS_COMMAND = stopCpuTtsCommand;
        N8N_PORT = toString ports.n8n;
      };

      n8nCommandExecutionEnvironment = {
        N8N_ENABLE_EXECUTE_COMMAND = "true";
        N8N_RESTRICT_FILE_ACCESS_TO = "/tmp";
        N8N_RUNNERS_BROKER_PORT = toString ports.n8nTaskRunnerBroker;
        NODES_EXCLUDE = "[]";
      };

      acceleratedKokoro = kokoroInstance { port = ports.acceleratedKokoro; useGpu = true; };
      cpuKokoro = kokoroInstance { port = ports.cpuKokoro; useGpu = false; };
    };
in
{
  inherit options services;
}
