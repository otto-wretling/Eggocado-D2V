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

        kokoroRoot = lib.mkOption {
          type = absolutePath;
          default = "${cfg.homeDirectory}/Documents/GitHub/Kokoro-FastAPI";
          defaultText = lib.literalExpression ''"''${config.eggocado-d2v.homeDirectory}/Documents/GitHub/Kokoro-FastAPI"'';
          description = "Kokoro-FastAPI checkout, including its uv environment and downloaded model.";
        };

        acceleration = {
          platformDefault = lib.mkOption {
            type = lib.types.enum [ "automatic" "cpu" ];
            default = "automatic";
            description = "Acceleration a conversion uses when its preference is Use platform default.";
          };

          kokoroDevice = lib.mkOption {
            type = lib.types.nullOr (lib.types.enum [ "cuda" "mps" ]);
            default = null;
            description = "Device the accelerated Kokoro instance requests. Null selects Kokoro's detection order: MPS, then CUDA, then CPU.";
          };
        };
      };
    };

  services = { lib, cfg }:
    let
      ports = {
        n8n = 5679;
        n8nTaskRunnerBroker = 5680;
        acceleratedKokoro = 8881;
        cpuKokoro = 8882;
      };

      kokoroInstance = { port, useGpu }: {
        environment = {
          MODEL_DIR = "src/models";
          PORT = toString port;
          PYTHONPATH = "${cfg.kokoroRoot}:${cfg.kokoroRoot}/api";
          USE_GPU = lib.boolToString useGpu;
          VOICES_DIR = "src/voices/v1_0";
          WEB_PLAYER_PATH = "${cfg.kokoroRoot}/web";
        }
        // lib.optionalAttrs (useGpu && cfg.acceleration.kokoroDevice != null) {
          DEVICE_TYPE = cfg.acceleration.kokoroDevice;
        };
        serverArguments = [ "uvicorn" "api.src.main:app" "--host" "0.0.0.0" "--port" (toString port) ];
      };
    in
    {
      inherit ports;

      n8nEnvironment = { startCpuTtsCommand, stopCpuTtsCommand }: {
        D2V_PLATFORM_DEFAULT_ACCELERATION = cfg.acceleration.platformDefault;
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

  hostPackages = pkgs: [
    pkgs.cargo
    pkgs.cmake
    pkgs.espeak-ng
    pkgs.ffmpeg-full
    pkgs.gcc
    pkgs.gnumake
    pkgs.rustc
    pkgs.uv
  ];

  linuxHost = { nixpkgs }: { config, lib, pkgs, ... }:
    let
      cfg = config.eggocado-d2v;
      d2vPkgs = import nixpkgs {
        system = pkgs.stdenv.hostPlatform.system;
        config.allowUnfreePredicate = package: builtins.elem (pkgs.lib.getName package) [ "n8n" ];
      };
    in
    {
      imports = [ options ];

      options.eggocado-d2v.ollamaPackage = lib.mkOption {
        type = lib.types.package;
        default = d2vPkgs.ollama;
        defaultText = lib.literalMD "Ollama from this flake's pinned nixpkgs";
        description = "Ollama package serving local transcription models.";
      };

      config = {
        eggocado-d2v.homeDirectory = lib.mkDefault "/home/${cfg.user}";

        programs.nix-ld.enable = true;

        environment.variables.NPM_CONFIG_PREFIX = "${cfg.homeDirectory}/.local/share/npm";

        environment.systemPackages = hostPackages d2vPkgs ++ [ pkgs.n8n ];

        services.ollama = {
          enable = true;
          package = cfg.ollamaPackage;
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
      };
    };

  linuxUserServices = { lib, pkgs, cfg, userExecutableDirectories, acceleratorLibraryDirectories ? [ ] }:
    let
      shared = services { inherit lib cfg; };
      cpuKokoroUnit = "kokoro-fastapi-cpu.service";
      executableSearchPath = lib.concatStringsSep ":" (
        userExecutableDirectories
        ++ [
          "/etc/profiles/per-user/${cfg.user}/bin"
          "/run/current-system/sw/bin"
          "/run/wrappers/bin"
        ]
      );
      acceleratorLibraryEnvironment = lib.optionalAttrs (acceleratorLibraryDirectories != [ ]) {
        LD_LIBRARY_PATH = lib.concatStringsSep ":" acceleratorLibraryDirectories;
      };

      kokoroService = { description, instance, startsWithSession }: {
        inherit description startsWithSession;
        environment = instance.environment // acceleratorLibraryEnvironment // {
          ESPEAK_DATA_PATH = "${pkgs.espeak-ng}/share/espeak-ng-data";
          SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
        };
        command = [ "${pkgs.uv}/bin/uv" "run" "--no-sync" ] ++ instance.serverArguments;
        workingDirectory = cfg.kokoroRoot;
      };
    in
    {
      inherit (shared) ports;

      services = {
        n8n-d2v-baremetal = {
          description = "Eggocado D2V bare-metal n8n instance";
          startsWithSession = true;
          environment =
            shared.n8nEnvironment {
              startCpuTtsCommand = "systemctl --user start ${cpuKokoroUnit}";
              stopCpuTtsCommand = "systemctl --user stop ${cpuKokoroUnit}";
            }
            // shared.n8nCommandExecutionEnvironment
            // acceleratorLibraryEnvironment
            // {
              N8N_USER_FOLDER = "${cfg.homeDirectory}/.n8n-d2v-baremetal";
              PATH = executableSearchPath;
            };
          command = [ "${pkgs.n8n}/bin/n8n" "start" ];
          workingDirectory = cfg.projectRoot;
        };

        kokoro-fastapi = kokoroService {
          description = "Kokoro-FastAPI text-to-speech (accelerated inference, CPU when no accelerator is available)";
          instance = shared.acceleratedKokoro;
          startsWithSession = true;
        };

        kokoro-fastapi-cpu = kokoroService {
          description = "Kokoro-FastAPI text-to-speech (CPU inference, started on demand)";
          instance = shared.cpuKokoro;
          startsWithSession = false;
        };
      };
    };
in
{
  inherit options services hostPackages linuxHost linuxUserServices;
}
