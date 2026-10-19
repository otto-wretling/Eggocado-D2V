{ config, pkgs, ... }:

let
  d2vRoot = "${config.home.homeDirectory}/Documents/GitHub/Eggocado-D2V";
  kokoroRoot = "${config.home.homeDirectory}/Documents/GitHub/Kokoro-FastAPI";
in
{
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
      ];
      ExecStart = "${pkgs.n8n}/bin/n8n start";
      Restart = "on-failure";
      RestartSec = 3;
      WorkingDirectory = "${d2vRoot}";
    };
    Install.WantedBy = [ "default.target" ];
  };
}
