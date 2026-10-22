{ config, lib, pkgs, ... }:

let
  d2vRoot = "${config.home.homeDirectory}/Documents/GitHub/Eggocado-D2V";
  n8nExecuteCommandPath = lib.concatStringsSep ":" (
    map
      (path: lib.replaceStrings [ "$HOME" ] [ config.home.homeDirectory ] path)
      config.home.sessionPath
    ++ [
      "/etc/profiles/per-user/${config.home.username}/bin"
      "/run/current-system/sw/bin"
      "/run/wrappers/bin"
    ]
  );
  kokoroRoot = "${config.home.homeDirectory}/Documents/GitHub/Kokoro-FastAPI";
in
{
  local.mcp.servers = {
    n8n-baremetal.url = "http://localhost:5679/mcp-server/http";
    n8n-d2v.url = "http://localhost:5678/mcp-server/http";
  };

  systemd.user.services.kokoro-fastapi = {
    Unit.Description = "Kokoro-FastAPI text-to-speech (CPU inference)";
    Service = {
      Environment = [
        "ESPEAK_DATA_PATH=${pkgs.espeak-ng}/share/espeak-ng-data"
        "MODEL_DIR=src/models"
        "PORT=8881"
        "PYTHONPATH=${kokoroRoot}:${kokoroRoot}/api"
        "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
        "USE_GPU=false"
        "VOICES_DIR=src/voices/v1_0"
        "WEB_PLAYER_PATH=${kokoroRoot}/web"
      ];
      ExecStart = "${pkgs.uv}/bin/uv run --no-sync uvicorn api.src.main:app --host 0.0.0.0 --port 8881";
      Restart = "on-failure";
      RestartSec = 3;
      WorkingDirectory = "${kokoroRoot}";
    };
    Install.WantedBy = [ "default.target" ];
  };

  systemd.user.services.n8n-d2v-baremetal = {
    Unit.Description = "Eggocado D2V bare-metal n8n instance";
    Service = {
      Environment = [
        "N8N_ENABLE_EXECUTE_COMMAND=true"
        "N8N_PORT=5679"
        "N8N_RESTRICT_FILE_ACCESS_TO=/tmp"
        "N8N_RUNNERS_BROKER_PORT=5680"
        "N8N_USER_FOLDER=${config.home.homeDirectory}/.n8n-d2v-baremetal"
        "NODES_EXCLUDE=[]"
        "PATH=${n8nExecuteCommandPath}"
      ];
      ExecStart = "${pkgs.n8n}/bin/n8n start";
      Restart = "on-failure";
      RestartSec = 3;
      WorkingDirectory = "${d2vRoot}";
    };
    Install.WantedBy = [ "default.target" ];
  };
}
