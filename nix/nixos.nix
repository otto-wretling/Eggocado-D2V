{ nixpkgs, pyproject-nix, uv2nix, pyproject-build-systems, kokoro-fastapi }:

let
  shared = import ./shared.nix;
  lib = nixpkgs.lib;

  hostPackages = pkgs: [
    pkgs.cargo
    pkgs.cmake
    pkgs.espeak-ng
    pkgs.ffmpeg-full
    pkgs.gcc
    pkgs.git
    pkgs.gnumake
    pkgs.nodejs
    pkgs.rustc
    pkgs.uv
  ];

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
        config.allowUnfreePredicate = package: builtins.elem (pkgs.lib.getName package) [ "n8n" ];
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

      kokoroService = { name, description, instance, startsWithSession }: {
        inherit description startsWithSession;
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
        n8n-d2v-baremetal = {
          description = "Eggocado D2V bare-metal n8n instance";
          startsWithSession = true;
          environment =
            sharedServices.n8nEnvironment {
              startCpuTtsCommand = "systemctl --user start ${cpuKokoroUnit}";
              stopCpuTtsCommand = "systemctl --user stop ${cpuKokoroUnit}";
            }
            // sharedServices.n8nCommandExecutionEnvironment
            // acceleratorLibraryEnvironment
            // {
              N8N_USER_FOLDER = "${cfg.homeDirectory}/.n8n-d2v-baremetal";
              PATH = executableSearchPath;
            };
          command = [ "${pkgs.n8n}/bin/n8n" "start" ];
          workingDirectory = cfg.projectRoot;
        };

        kokoro-fastapi = kokoroService {
          name = "kokoro-fastapi";
          description = "Kokoro-FastAPI text-to-speech (accelerated inference, CPU when no accelerator is available)";
          instance = sharedServices.acceleratedKokoro;
          startsWithSession = true;
        };

        kokoro-fastapi-cpu = kokoroService {
          name = "kokoro-fastapi-cpu";
          description = "Kokoro-FastAPI text-to-speech (CPU inference, started on demand)";
          instance = sharedServices.cpuKokoro;
          startsWithSession = false;
        };
      };
    };
in
{
  inherit kokoroFor linuxHost linuxUserServices;

  packages = { username }: { ... }: {
    imports = [ linuxHost ];

    eggocado-d2v.user = username;

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

        local.mcp.servers = {
          n8n-baremetal.url = "http://localhost:5679/mcp-server/http";
          n8n-d2v.url = "http://localhost:5678/mcp-server/http";
        };

        systemd.user.services = lib.mapAttrs
          (_: service:
            {
              Unit.Description = service.description;
              Service = {
                Environment = lib.mapAttrsToList systemdEnvironmentAssignment service.environment;
                ExecStart = lib.concatStringsSep " " service.command;
                Restart = "on-failure";
                RestartSec = 3;
                WorkingDirectory = service.workingDirectory;
              };
            }
            // lib.optionalAttrs service.startsWithSession {
              Install.WantedBy = [ "default.target" ];
            })
          userServices.services;
      };
    };
}
