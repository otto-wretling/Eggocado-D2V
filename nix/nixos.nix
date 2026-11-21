{ nixpkgs, pyproject-nix, uv2nix, pyproject-build-systems, kokoro-fastapi }:

let
  shared = import ./shared.nix;
  lib = nixpkgs.lib;

  hostPackages = pkgs:
    map (name: pkgs.${name}) shared.hostToolNames
    ++ [
      pkgs.cargo
      pkgs.cmake
      pkgs.gcc
      pkgs.git
      pkgs.gnumake
      pkgs.nodejs
      pkgs.rustc
    ];

  restartPolicy = {
    Restart = "on-failure";
    RestartSec = shared.restartDelaySeconds;
  };

  userServiceUnits = {
    n8n = "n8n-d2v-baremetal";
    kokoro = "kokoro-fastapi";
    cpuKokoro = "kokoro-fastapi-cpu";
    dockerServices = "d2v-docker-services";
  };

  systemdUserControls = service:
    let
      unit = userServiceUnits.${service.key};
      stoppedUnits = [ unit ] ++ map (key: userServiceUnits.${key}) service.stopsWith;
    in
    {
      start = [ "systemctl --user start ${unit}" ];
      stop = map (stoppedUnit: "systemctl --user stop ${stoppedUnit}") stoppedUnits;
      status = "systemctl --user is-active --quiet ${unit}";
    };

  ollamaSystemUnit = "ollama";

  ollamaSystemControls = {
    start = [ "systemctl start ${ollamaSystemUnit}" ];
    stop = [ "systemctl stop ${ollamaSystemUnit}" ];
    status = "systemctl is-active --quiet ${ollamaSystemUnit}";
  };

  polkitManageUnitsAction = "org.freedesktop.systemd1.manage-units";

  kokoroFor = system: torchVariant:
    let
      pkgs = nixpkgs.legacyPackages.${system};
      workspace = uv2nix.lib.workspace.loadWorkspace { workspaceRoot = kokoro-fastapi; };
      selectedExtras = { kokoro-fastapi = [ torchVariant ]; };

      lockedPackages = workspace.mkPyprojectOverlay {
        sourcePreference = "wheel";
        dependencies = selectedExtras;
      };

      loadsCudaLibrariesAtRuntime = name: name == "torch" || name == "triton" || lib.hasPrefix "nvidia-" name;

      wheelFixups = final: prev:
        lib.mapAttrs
          (name: package: package.overrideAttrs (_: {
            autoPatchelfIgnoreMissingDeps = true;
            appendRunpaths = [ "$ORIGIN" ];
          }))
          (lib.filterAttrs (name: _: loadsCudaLibrariesAtRuntime name) prev);

      undeclaredSourceBuildSystems = {
        docopt = { setuptools = [ ]; };
        jieba = { setuptools = [ ]; };
        unidic = { setuptools = [ ]; };
        pyopenjtalk = {
          setuptools = [ ];
          setuptools-scm = [ ];
          cython = [ ];
          numpy = [ ];
        };
      };

      sourceBuildFixups = final: prev:
        lib.mapAttrs
          (name: buildSystem: prev.${name}.overrideAttrs (old: {
            nativeBuildInputs = old.nativeBuildInputs
              ++ final.resolveBuildSystem buildSystem
              ++ lib.optionals (name == "pyopenjtalk") [ pkgs.cmake ];
            dontUseCmakeConfigure = true;
          }))
          (lib.filterAttrs (name: _: prev ? ${name}) undeclaredSourceBuildSystems);

      systemLibraryFixups = final: prev: {
        soundfile = prev.soundfile.overrideAttrs (_: {
          postInstall = ''
            substituteInPlace $out/${final.python.sitePackages}/soundfile.py \
              --replace-fail "_find_library('sndfile')" "'${pkgs.libsndfile.out}/lib/libsndfile.so'"
          '';
        });
      };

      pythonSet = (pkgs.callPackage pyproject-nix.build.packages { python = pkgs.python312; }).overrideScope (
        lib.composeManyExtensions [
          pyproject-build-systems.overlays.default
          lockedPackages
          wheelFixups
          sourceBuildFixups
          systemLibraryFixups
        ]
      );

      modelRelease = "https://github.com/remsky/Kokoro-FastAPI/releases/download/v0.1.4";

      modelWeights = pkgs.fetchurl {
        url = "${modelRelease}/kokoro-v1_0.pth";
        sha256 = "496dba118d1a58f5f3db2efc88dbdc216e0483fc89fe6e47ee1f2c53f18ad1e4";
      };

      modelConfig = pkgs.fetchurl {
        url = "${modelRelease}/config.json";
        sha256 = "5abb01e2403b072bf03d04fde160443e209d7a0dad49a423be15196b9b43c17f";
      };
    in
    {
      source = kokoro-fastapi;

      environment = pythonSet.mkVirtualEnv "kokoro-fastapi-${torchVariant}" selectedExtras;

      modelDirectory = pkgs.runCommand "kokoro-v1_0-model" { } ''
        mkdir -p $out/v1_0
        cp ${modelWeights} $out/v1_0/kokoro-v1_0.pth
        cp ${modelConfig} $out/v1_0/config.json
      '';
    };

  linuxOptions = { lib, ... }: {
    options.eggocado-d2v.kokoro.torchVariant = lib.mkOption {
      type = lib.types.enum [ "cpu" "gpu" ];
      default = "cpu";
      description = "PyTorch build in Kokoro-FastAPI's locked environment: cpu, or gpu for NVIDIA CUDA 12.6. The gpu build also runs the CPU instance.";
    };
  };

  linuxHost = { config, lib, pkgs, ... }:
    let
      cfg = config.eggocado-d2v;
      d2vPkgs = import nixpkgs {
        system = pkgs.stdenv.hostPlatform.system;
        config.allowUnfreePredicate = shared.isAllowedUnfree lib;
      };
    in
    {
      imports = [ shared.options linuxOptions ];

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

        environment.extraInit = ''
          export PATH="${cfg.homeDirectory}/.local/share/npm/bin:$PATH"
        '';

        environment.systemPackages = hostPackages d2vPkgs ++ [ pkgs.n8n ];

        services.ollama = {
          enable = lib.mkDefault true;
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
      kokoro = kokoroFor pkgs.stdenv.hostPlatform.system cfg.kokoro.torchVariant;
      sharedServices = shared.services {
        inherit lib cfg;
        kokoroPaths = {
          inherit (kokoro) source modelDirectory;
          voicesDirectory = "${kokoro.source}/api/src/voices/v1_0";
        };
      };
      kokoroStateDirectory = "${cfg.homeDirectory}/.local/state/eggocado-d2v";
      cpuKokoroUnit = "${userServiceUnits.cpuKokoro}.service";
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

      kokoroService = { name, description, instance }: {
        inherit description;
        environment = instance.environment // acceleratorLibraryEnvironment // {
          ESPEAK_DATA_PATH = "${pkgs.espeak-ng}/share/espeak-ng-data";
          HF_HUB_OFFLINE = "1";
          SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
          TEMP_FILE_DIR = "${kokoroStateDirectory}/${name}-temp-files";
        };
        command = [ "${kokoro.environment}/bin/uvicorn" ] ++ instance.uvicornArguments;
        workingDirectory = kokoro.source;
      };
    in
    {
      inherit (sharedServices) ports;

      services = {
        ${userServiceUnits.n8n} = {
          description = "Eggocado D2V bare-metal n8n instance";
          environment =
            sharedServices.n8nEnvironment {
              startCpuTtsCommand = "systemctl --user start ${cpuKokoroUnit}";
              stopCpuTtsCommand = "systemctl --user stop ${cpuKokoroUnit}";
            }
            // sharedServices.n8nCommandExecutionEnvironment
            // acceleratorLibraryEnvironment
            // {
              PATH = executableSearchPath;
            };
          command = [ "${pkgs.n8n}/bin/n8n" "start" ];
          workingDirectory = cfg.projectRoot;
        };

        ${userServiceUnits.kokoro} = kokoroService {
          name = userServiceUnits.kokoro;
          description = "Kokoro-FastAPI text-to-speech (accelerated inference, CPU when no accelerator is available)";
          instance = sharedServices.acceleratedKokoro;
        };

        ${userServiceUnits.cpuKokoro} = kokoroService {
          name = userServiceUnits.cpuKokoro;
          description = "Kokoro-FastAPI text-to-speech (CPU inference, started on demand)";
          instance = sharedServices.cpuKokoro;
        };
      };
    };
in
{
  inherit kokoroFor linuxHost linuxUserServices restartPolicy systemdUserControls userServiceUnits;

  packages = { username }: { lib, ... }: {
    imports = [ linuxHost ];

    eggocado-d2v.user = username;

    systemd.services.${ollamaSystemUnit}.wantedBy = lib.mkForce [ ];

    security.polkit.extraConfig = ''
      polkit.addRule(function (action, subject) {
        if (action.id == "${polkitManageUnitsAction}" && action.lookup("unit") == "${ollamaSystemUnit}.service" && subject.user == "${username}") {
          return polkit.Result.YES;
        }
      });
    '';

    networking.firewall.trustedInterfaces = [ "docker0" "d2v-bridge" ];

    users.users.${username}.extraGroups = [ "docker" ];

    virtualisation.docker.enable = true;
  };

  services = { config, lib, pkgs, ... }:
    let
      cfg = config.eggocado-d2v;
      userServices = linuxUserServices {
        inherit lib pkgs cfg;
        userExecutableDirectories = map
          (path: lib.replaceStrings [ "$HOME" ] [ config.home.homeDirectory ] path)
          config.home.sessionPath;
      };
      systemdEnvironmentAssignment = name: value:
        if lib.hasInfix " " value then ''"${name}=${value}"'' else "${name}=${value}";
      serviceControlOverrides = {
        ollama = ollamaSystemControls;
        dockerServices = shared.dockerComposeControls {
          dockerCommand = "docker";
          projectRoot = cfg.projectRoot;
        };
      };
    in
    {
      imports = [ shared.options linuxOptions ];

      options.local.mcp.servers = lib.mkOption {
        default = { };
        description = "Tool-neutral registry of MCP servers, keyed by name.";
        type = lib.types.attrsOf (
          lib.types.submodule {
            options.url = lib.mkOption {
              type = lib.types.str;
              description = "The MCP server's HTTP endpoint.";
            };
          }
        );
      };

      config = {
        eggocado-d2v = {
          user = lib.mkDefault config.home.username;
          homeDirectory = lib.mkDefault config.home.homeDirectory;
        };

        home.packages = [
          (shared.d2vCommand {
            inherit lib pkgs;
            controlsFor = service:
              serviceControlOverrides.${service.key} or (systemdUserControls service);
          })
        ];

        local.mcp.servers = {
          n8n-baremetal.url = "http://localhost:${toString userServices.ports.n8n}/mcp-server/http";
          n8n-d2v.url = "http://localhost:${toString userServices.ports.mainN8n}/mcp-server/http";
        };

        systemd.user.services = lib.mapAttrs
          (_: service: {
            Unit.Description = service.description;
            Service = restartPolicy // {
              Environment = lib.mapAttrsToList systemdEnvironmentAssignment service.environment;
              ExecStart = lib.concatStringsSep " " service.command;
              WorkingDirectory = service.workingDirectory;
            };
          })
          userServices.services;
      };
    };
}
