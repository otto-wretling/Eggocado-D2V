let
  shared = import ./shared.nix;

  homebrewPrefix = "/opt/homebrew";
  ffmpegFullBinDirectory = "${homebrewPrefix}/opt/ffmpeg-full/bin";
  dockerAppName = "Docker";
  dockerCliDirectory = "/Applications/${dockerAppName}.app/Contents/Resources/bin";
  openCommand = "/usr/bin/open";
  launchctl = "/bin/launchctl";
  launchdUserDomain = "gui/$(id -u)";
  launchctlRunningField = "PID";
  userExecutablesFolderName = ".local/bin";
  logDirectory = "/tmp";
  dockerPollIntervalSeconds = 5;
  dockerComposeRetryDelaySeconds = 60;

  kokoro = rec {
    checkoutFolderName = "Kokoro-FastAPI";
    repositoryUrl = "https://github.com/remsky/${checkoutFolderName}.git";
    virtualEnvironmentDirectory = ".venv";
    modelVersion = "v1_0";
    modelsDirectory = "src/models";
    voicesDirectory = "src/voices/${modelVersion}";
    downloadedModelsDirectory = "api/${modelsDirectory}/${modelVersion}";
    modelFile = "kokoro-${modelVersion}.pth";
    downloadScript = "docker/scripts/download_model.py";
    mpsFallbackEnvironment = { PYTORCH_ENABLE_MPS_FALLBACK = "1"; };
  };

  ollamaAppName = "Ollama";
  ollamaExecutable = "/Applications/${ollamaAppName}.app/Contents/Resources/ollama";

  agents = {
    n8n = { label = "io.n8n.n8n"; logName = "n8n"; };
    kokoro = { label = "io.github.remsky.kokoro-fastapi"; logName = "kokoro"; };
    cpuKokoro = { label = "io.github.remsky.kokoro-fastapi-cpu"; logName = "kokoro-cpu"; };
    dockerServices = { label = "io.github.eggocado.d2v-docker-services"; logName = "d2v-docker-services"; };
  };
in
{
  homebrew = { ... }: {
    homebrew.brews = shared.hostToolNames ++ [ "rust" ];

    homebrew.casks = [
      "docker-desktop"
      "ollama-app"
    ];
  };

  packages = { lib, pkgs, ... }: {
    environment.systemPackages = [ pkgs.n8n ];

    environment.systemPath = [ ffmpegFullBinDirectory ];

    nixpkgs.config.allowUnfreePredicate = shared.isAllowedUnfree lib;
  };

  services = { config, lib, pkgs, ... }:
    let
      cfg = config.eggocado-d2v;
      hostSearchPath = lib.concatStringsSep ":" [
        "${homebrewPrefix}/bin"
        "${cfg.homeDirectory}/${userExecutablesFolderName}"
        "/nix/var/nix/profiles/system/sw/bin"
        "/usr/bin"
        "/bin"
        "/usr/sbin"
        "/sbin"
      ];

      runKokoroFastApi = pkgs.writeShellApplication {
        name = "run-kokoro-fastapi";
        runtimeInputs = [ pkgs.git pkgs.uv ];
        text = ''
          if [ ! -d "${cfg.kokoroRoot}/.git" ]; then
            git clone ${kokoro.repositoryUrl} "${cfg.kokoroRoot}"
          fi
          cd "${cfg.kokoroRoot}"
          if [ ! -d ${kokoro.virtualEnvironmentDirectory} ]; then
            uv venv
            uv pip install -e .
          fi
          if [ ! -f ${kokoro.downloadedModelsDirectory}/${kokoro.modelFile} ]; then
            uv run --no-sync python ${kokoro.downloadScript} --output ${kokoro.downloadedModelsDirectory}
          fi
          exec uv run --no-sync uvicorn "$@"
        '';
      };

      startDockerServices = pkgs.writeShellApplication {
        name = "start-d2v-docker-services";
        text = ''
          export PATH="$PATH:${dockerCliDirectory}"
          ${openCommand} -ga ${dockerAppName}
          until docker info > /dev/null 2>&1; do
            sleep ${toString dockerPollIntervalSeconds}
          done
          cd "${cfg.projectRoot}"
          docker compose up -d
        '';
      };

      restartOnFailure = pkgs.writeShellApplication {
        name = "restart-on-failure";
        text = ''
          delaySeconds=$1
          shift
          until "$@"; do
            sleep "$delaySeconds"
          done
        '';
      };

      launchdAgent = { agent, command, workingDirectory ? cfg.homeDirectory, environment ? { }, restartDelaySeconds ? shared.restartDelaySeconds }: {
        serviceConfig = {
          Label = agent.label;
          ProgramArguments = [ "${restartOnFailure}/bin/restart-on-failure" (toString restartDelaySeconds) ] ++ command;
          RunAtLoad = false;
          KeepAlive = false;
          StandardOutPath = "${logDirectory}/${agent.logName}.log";
          StandardErrorPath = "${logDirectory}/${agent.logName}.err";
          WorkingDirectory = workingDirectory;
          EnvironmentVariables = {
            HOME = cfg.homeDirectory;
            PATH = hostSearchPath;
          } // environment;
        };
      };

      sharedServices = shared.services {
        inherit lib cfg;
        kokoroPaths = {
          source = cfg.kokoroRoot;
          modelDirectory = kokoro.modelsDirectory;
          voicesDirectory = kokoro.voicesDirectory;
        };
      };

      kokoroAgent = { agent, instance }: launchdAgent {
        inherit agent;
        command = [ "${runKokoroFastApi}/bin/run-kokoro-fastapi" ] ++ instance.uvicornArguments;
        environment = instance.environment // kokoro.mpsFallbackEnvironment;
      };

      startAgentCommand = agent: ''${launchctl} kickstart "${launchdUserDomain}/${agent.label}"'';

      stopAgentCommand = agent: ''${launchctl} stop "${agent.label}"'';

      agentIsRunningCommand = agent: ''${launchctl} list "${agent.label}" 2>/dev/null | grep -q ${launchctlRunningField}'';

      launchdControls = { agent, alsoStopAgents ? [ ] }: {
        start = [ (startAgentCommand agent) ];
        stop = map stopAgentCommand ([ agent ] ++ alsoStopAgents);
        status = agentIsRunningCommand agent;
      };

      dockerComposeServiceControls = shared.dockerComposeControls {
        dockerCommand = "${dockerCliDirectory}/docker";
        projectRoot = cfg.projectRoot;
      };

      serviceControlOverrides = {
        ollama = {
          start = [ "${openCommand} -ga ${ollamaAppName}" ];
          stop = [ ];
          status = "${ollamaExecutable} ps > /dev/null 2>&1";
        };
        dockerServices = {
          start = [ (startAgentCommand agents.dockerServices) ];
          stop = [ (stopAgentCommand agents.dockerServices) ] ++ dockerComposeServiceControls.stop;
          status = dockerComposeServiceControls.status;
        };
      };

      d2vCommand = shared.d2vCommand {
        inherit lib pkgs;
        controlsFor = service:
          serviceControlOverrides.${service.key} or (launchdControls {
            agent = agents.${service.key};
            alsoStopAgents = map (key: agents.${key}) service.stopsWith;
          });
      };
    in
    {
      imports = [ shared.options ];

      options.eggocado-d2v.kokoroRoot = lib.mkOption {
        type = shared.absolutePath lib;
        default = "${cfg.homeDirectory}/${shared.repositoriesFolderName}/${kokoro.checkoutFolderName}";
        defaultText = lib.literalExpression ''"''${config.eggocado-d2v.homeDirectory}/${shared.repositoriesFolderName}/${kokoro.checkoutFolderName}"'';
        description = "Kokoro-FastAPI checkout, including its uv environment and downloaded model.";
      };

      config.eggocado-d2v = {
        user = lib.mkDefault config.system.primaryUser;
        homeDirectory = lib.mkDefault "/Users/${cfg.user}";
        acceleration.kokoroDevice = lib.mkDefault "mps";
      };

      config.environment.systemPackages = [ d2vCommand ];

      config.launchd.user.agents.n8n = launchdAgent {
        agent = agents.n8n;
        command = shared.n8nServerCommand pkgs;
        workingDirectory = cfg.projectRoot;
        environment =
          sharedServices.n8nEnvironment {
            startCpuTtsCommand = "${launchctl} start ${agents.cpuKokoro.label}";
            stopCpuTtsCommand = stopAgentCommand agents.cpuKokoro;
          }
          // sharedServices.n8nCommandExecutionEnvironment;
      };

      config.launchd.user.agents.d2v-docker-services = launchdAgent {
        agent = agents.dockerServices;
        command = [ "${startDockerServices}/bin/start-d2v-docker-services" ];
        workingDirectory = cfg.projectRoot;
        restartDelaySeconds = dockerComposeRetryDelaySeconds;
      };

      config.launchd.user.agents.kokoro = kokoroAgent {
        agent = agents.kokoro;
        instance = sharedServices.acceleratedKokoro;
      };

      config.launchd.user.agents.kokoro-cpu = kokoroAgent {
        agent = agents.cpuKokoro;
        instance = sharedServices.cpuKokoro;
      };
    };
}
