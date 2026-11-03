let
  shared = import ./shared.nix;
in
{
  homebrew = { ... }: {
    homebrew.brews = [
      "espeak-ng"
      "ffmpeg-full"
      "rust"
      "uv"
    ];

    homebrew.casks = [
      "ollama-app"
    ];
  };

  services = { config, lib, ... }:
    let
      cfg = config.eggocado-d2v;
      sharedServices = shared.services { inherit lib cfg; };
      cpuKokoroLabel = "io.github.remsky.kokoro-fastapi-cpu";

      kokoroAgent = { label, instance, startsAtLogin, logName }: {
        serviceConfig = {
          Label = label;
          ProgramArguments = [ "/opt/homebrew/bin/uv" "run" "--no-sync" ] ++ instance.serverArguments;
          RunAtLoad = startsAtLogin;
          KeepAlive = startsAtLogin;
          StandardOutPath = "/tmp/${logName}.log";
          StandardErrorPath = "/tmp/${logName}.err";
          WorkingDirectory = cfg.kokoroRoot;
          EnvironmentVariables = instance.environment // {
            HOME = cfg.homeDirectory;
            PYTORCH_ENABLE_MPS_FALLBACK = "1";
          };
        };
      };
    in
    {
      imports = [ shared.options ];

      eggocado-d2v = {
        user = lib.mkDefault config.system.primaryUser;
        homeDirectory = lib.mkDefault "/Users/${cfg.user}";
        acceleration.kokoroDevice = lib.mkDefault "mps";
      };

      launchd.user.agents.n8n = {
        serviceConfig = {
          Label = "io.n8n.n8n";
          ProgramArguments = [ "/nix/var/nix/profiles/system/sw/bin/n8n" "start" ];
          RunAtLoad = true;
          KeepAlive = true;
          StandardOutPath = "/tmp/n8n.log";
          StandardErrorPath = "/tmp/n8n.err";
          WorkingDirectory = cfg.projectRoot;
          EnvironmentVariables =
            sharedServices.n8nEnvironment {
              startCpuTtsCommand = "/bin/launchctl start ${cpuKokoroLabel}";
              stopCpuTtsCommand = "/bin/launchctl stop ${cpuKokoroLabel}";
            }
            // {
              HOME = cfg.homeDirectory;
              N8N_USER_FOLDER = "${cfg.homeDirectory}/.n8n";
            };
        };
      };

      launchd.user.agents.kokoro = kokoroAgent {
        label = "io.github.remsky.kokoro-fastapi";
        instance = sharedServices.acceleratedKokoro;
        startsAtLogin = true;
        logName = "kokoro";
      };

      launchd.user.agents.kokoro-cpu = kokoroAgent {
        label = cpuKokoroLabel;
        instance = sharedServices.cpuKokoro;
        startsAtLogin = false;
        logName = "kokoro-cpu";
      };
    };
}
