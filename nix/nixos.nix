{ nixpkgs }:

let
  shared = import ./shared.nix;
in
{
  packages = { username }: { ... }: {
    imports = [ (shared.linuxHost { inherit nixpkgs; }) ];

    eggocado-d2v.user = username;

    networking.firewall.trustedInterfaces = [ "docker0" "d2v-bridge" ];

    users.users.${username}.extraGroups = [ "docker" ];

    virtualisation.docker.enable = true;
  };

  services = { config, lib, pkgs, ... }:
    let
      cfg = config.eggocado-d2v;
      linuxUserServices = shared.linuxUserServices {
        inherit lib pkgs cfg;
        userExecutableDirectories = map
          (path: lib.replaceStrings [ "$HOME" ] [ config.home.homeDirectory ] path)
          config.home.sessionPath;
      };
      systemdEnvironmentAssignment = name: value:
        if lib.hasInfix " " value then ''"${name}=${value}"'' else "${name}=${value}";
    in
    {
      imports = [ shared.options ];

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
          linuxUserServices.services;
      };
    };
}
